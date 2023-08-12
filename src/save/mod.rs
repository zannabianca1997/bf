use std::{
    borrow::Cow,
    io::{self, Read, Write},
    str::from_utf8,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ir;

/// Magic value to recognize compiled files
/// it starts with ']' so it's never valid bf
const MAGIC: [u8; 3] = *b"]bf";

/// Header of a compiled file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Header {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip)]
    pub compressed: bool,
    #[serde(flatten)]
    pub content: Content,
}
impl Header {
    pub fn of_plain_source() -> Header {
        Header {
            content: Content::Source,
            compressed: false,
            description: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(tag = "content")]
pub enum Content {
    Source,
    Ir {
        #[serde(default)]
        format: Format,
    },
}

impl Content {
    /// Returns `true` if the content is [`Source`].
    ///
    /// [`Source`]: Content::Source
    #[must_use]
    pub fn is_source(&self) -> bool {
        matches!(self, Self::Source)
    }

    /// Returns `true` if the content is [`Ir`].
    ///
    /// [`Ir`]: Content::Ir
    #[must_use]
    pub fn is_ir(&self) -> bool {
        matches!(self, Self::Ir { .. })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Default,
)]
pub enum Format {
    #[default]
    Json,
    CBOR,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Payload {
    Source(String),
    Ir(ir::Program),
}

impl Payload {
    #[must_use]
    pub fn as_ir(&self) -> Option<&ir::Program> {
        if let Self::Ir(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_source(&self) -> Option<&str> {
        if let Self::Source(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the payload is [`Source`].
    ///
    /// [`Source`]: Payload::Source
    #[must_use]
    pub fn is_source(&self) -> bool {
        matches!(self, Self::Source(..))
    }

    #[must_use]
    pub fn try_into_source(self) -> Result<String, Self> {
        if let Self::Source(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    /// Returns `true` if the payload is [`Ir`].
    ///
    /// [`Ir`]: Payload::Ir
    #[must_use]
    pub fn is_ir(&self) -> bool {
        matches!(self, Self::Ir(..))
    }

    #[must_use]
    pub fn try_into_ir(self) -> Result<ir::Program, Self> {
        if let Self::Ir(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct File {
    pub header: Header,
    pub payload: Payload,
}

#[derive(Debug, Error)]
pub enum ParseFileError {
    #[error("Error wjile reading the file")]
    Read(#[source] io::Error),
    #[error("Unrecognized compression flag {0}")]
    UnrecognizedCompression(u8),
    #[error("The header must be terminated with `...` on the line after the magic number")]
    UnterminatedHeader,
    #[error("The header must start with `---` alone on a line")]
    MissingHeaderStart,
    #[error("Error while decompressing")]
    DecompressError(#[source] io::Error),
    #[error("The header is not valid utf8")]
    HeaderNotUtf8(#[source] std::str::Utf8Error),
    #[error("Error while parsing yaml header")]
    Header(#[source] serde_yaml::Error),
    #[error("Error while parsing CBOR ir representation")]
    InvalidCBORIr(#[source] ciborium::de::Error<std::io::Error>),
    #[error("Error while parsing Json ir representation")]
    InvalidJsonIr(#[source] serde_json::Error),
}

/// Parse a file from the bytes
pub fn parse(mut source: impl io::Read) -> Result<File, ParseFileError> {
    let source = {
        let mut buf = vec![];
        source.read_to_end(&mut buf).map_err(ParseFileError::Read)?;
        buf
    };
    // check for magic number
    if let Some((source, compressed)) = {
        if source.len() >= 4 {
            let (magic, rest) = source.split_array_ref();
            if magic == &MAGIC {
                let (ch, rest) = rest.split_first().unwrap();
                match *ch {
                    b'c' => Some((rest, true)),
                    b'p' => Some((rest, false)),
                    _ => return Err(ParseFileError::UnrecognizedCompression(*ch)),
                }
            } else {
                None
            }
        } else {
            None
        }
    } {
        let source = if compressed {
            let mut decompressed = flate2::read::DeflateDecoder::new(source);
            let mut buf = vec![];
            decompressed
                .read_to_end(&mut buf)
                .map_err(ParseFileError::DecompressError)?;
            Cow::Owned(buf)
        } else {
            Cow::Borrowed(source)
        };
        // the file has our magic number on it!

        // splitting the header
        let (sep, rest) = source.split_array_ref();
        if sep != b"\n---" {
            return Err(ParseFileError::MissingHeaderStart);
        }
        let Some(hend) = rest.array_windows().position(|w| w==b"\n...\n") else {
            return Err(ParseFileError::UnterminatedHeader);
        };
        let (header, rest) = rest.split_at(hend);
        let (_, payload) = rest.split_at(b"\n...\n".len());

        // parsing the header
        let mut header: Header =
            serde_yaml::from_str(from_utf8(header).map_err(ParseFileError::HeaderNotUtf8)?)
                .map_err(ParseFileError::Header)?;
        header.compressed = compressed;

        // parsing the payload
        let payload = match header.content {
            Content::Source => Payload::Source(String::from_utf8_lossy(payload).into_owned()),
            Content::Ir { format } => Payload::Ir(match format {
                Format::Json => {
                    serde_json::from_slice(payload).map_err(ParseFileError::InvalidJsonIr)?
                }
                Format::CBOR => {
                    ciborium::from_reader(payload).map_err(ParseFileError::InvalidCBORIr)?
                }
            }),
        };

        Ok(File { header, payload })
    } else {
        let source = String::from_utf8_lossy(&source).into_owned();

        let mut header = Header::of_plain_source();

        // searching for beginner comment to include as a description
        header.description = {
            let source = source.trim_start();
            if source.starts_with('[') {
                let end = source
                    .char_indices()
                    .skip(1)
                    .scan(1usize, |depth, (idx, ch)| {
                        if *depth == 0 {
                            return None;
                        }
                        match ch {
                            '[' => {
                                *depth += 1;
                                Some(None)
                            }
                            ']' => {
                                *depth -= 1;
                                Some(Some(idx))
                            }
                            _ => Some(None),
                        }
                    })
                    .last()
                    .flatten()
                    .unwrap_or(source.len());
                Some(source[1..end].to_owned())
            } else {
                None
            }
        };

        let payload = Payload::Source(source);

        Ok(File { header, payload })
    }
}

/// Dump a source to file
pub fn write_source<'d>(
    mut dest: impl io::Write,
    source: impl AsRef<str>,
    compressed: bool,
    description: Option<impl Into<Cow<'d, str>>>,
) -> io::Result<()> {
    let header = serde_yaml::to_string(&Header {
        description: description.map(|d| d.into().into_owned()),
        compressed,
        content: Content::Source,
    })
    .unwrap();

    dest.write_all(&MAGIC)?;
    if compressed {
        write!(dest, "c")?;
        let mut dest = flate2::write::DeflateEncoder::new(dest, flate2::Compression::best());
        write!(dest, "\n---\n{header}\n...\n")?;
        write!(dest, "{}", source.as_ref())?;
        dest.finish()?;
    } else {
        write!(dest, "p")?;
        write!(dest, "\n---\n{header}\n...\n")?;
        write!(dest, "{}", source.as_ref())?;
    }
    Ok(())
}

/// Dump the intermediate representation to file
pub fn write_ir<'d>(
    mut dest: impl io::Write,
    ir: &ir::Program,
    compressed: bool,
    description: Option<impl Into<Cow<'d, str>>>,
    format: Format,
) -> io::Result<()> {
    let header = serde_yaml::to_string(&Header {
        description: description.map(|d| d.into().into_owned()),
        compressed,
        content: Content::Ir { format },
    })
    .unwrap();

    dest.write_all(&MAGIC)?;
    if compressed {
        write!(dest, "c")?;
        let mut dest = flate2::write::DeflateEncoder::new(dest, flate2::Compression::best());
        write!(dest, "\n---\n{header}\n...\n")?;
        match format {
            Format::Json => serde_json::to_writer(&mut dest, ir)?,
            Format::CBOR => {
                let mut buf = vec![];
                ciborium::into_writer(ir, &mut buf).expect("The ir should be always dumpable");
                dest.write_all(&buf)?;
            }
        }
        dest.finish()?;
    } else {
        write!(dest, "p")?;
        write!(dest, "\n---\n{header}\n...\n")?;
        match format {
            Format::Json => serde_json::to_writer(&mut dest, ir)?,
            Format::CBOR => {
                let mut buf = vec![];
                ciborium::into_writer(ir, &mut buf).expect("The ir should be always dumpable");
                dest.write_all(&buf)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::{parse, Content, File, Header, Payload};

    #[test]
    fn parse_source() {
        let src = "Some brainfuck: ++--";
        let file = parse(src.as_bytes()).expect("The file should be recognized as a source file");
        assert_matches!(
            file,
            File {
                header: Header {
                    description: None,
                    compressed: false,
                    content: Content::Source,
                },
                payload: Payload::Source(src)
            } if src == "Some brainfuck: ++--"
        )
    }
    #[test]
    fn parse_source_commented() {
        let src = "[Some brainfuck] ++--";
        let file = parse(src.as_bytes()).expect("The file should be recognized as a source file");
        assert_matches!(
            file,
            File {
                header: Header {
                    description: Some(descr),
                    compressed: false,
                    content: Content::Source,
                },
                payload: Payload::Source(src)
            } if src == "[Some brainfuck] ++--" && descr == "Some brainfuck"
        )
    }
}
