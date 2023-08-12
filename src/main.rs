use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, stdin, stdout, Read, StdinLock, Write},
    path::PathBuf,
};

use anyhow::{bail, Context};
use bf::{
    engine::{self, Engine, ProgrammableEngine},
    save::Payload,
};
use clap::{Parser, ValueEnum};

/// Brainfuck optimizer and runner
#[derive(Debug, Clone, Parser)]
#[clap(name="bf", about = "Brainfuck optimizer and runner", long_about = None,version)]
enum Cli {
    /// Run the program
    Run {
        /// Run the program with no optimizations
        #[clap(long)]
        raw: bool,
        /// Input stream type
        #[clap(short, long, default_value = "bytes")]
        input: StreamType,
        /// Output stream type
        #[clap(short, long, default_value = "bytes")]
        output: StreamType,
        /// Program to run
        program: PathBuf,
    },
    /// Inspect a file, showing its header
    Inspect {
        /// File to inspect. Defaults to read stdin
        file: Option<PathBuf>,
    },
    /// Compile a file
    Compile {
        /// Source file. Defaults to read stdin
        #[clap(short, long)]
        input: Option<PathBuf>,
        /// Output file. Defaults to write stdout
        #[clap(short, long)]
        output: Option<PathBuf>,
        /// Format of the output representation
        #[clap(short, long, default_value = "binary")]
        format: Format,
        /// Use a compressed representation
        #[clap(short, long)]
        compress: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    /// Raw brainfuck
    Raw,
    /// Uncompressed binary form
    Binary,
    /// Human readable json
    Json,
}

impl Format {
    /// Returns `true` if the format is [`Raw`].
    ///
    /// [`Raw`]: Format::Raw
    #[must_use]
    fn is_raw(&self) -> bool {
        matches!(self, Self::Raw)
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StreamType {
    Bytes,
    Ascii,
}

struct InputStream {
    buf: VecDeque<u8>,
    typ: StreamType,
}
impl InputStream {
    fn read(&mut self) -> anyhow::Result<u8> {
        while self.buf.is_empty() {
            log::trace!("Filling input buffer");
            let mut buf = String::new();
            stdin().read_line(&mut buf)?;
            match self.typ {
                StreamType::Bytes => self.buf.extend(buf.as_bytes()),
                StreamType::Ascii => {
                    for num in buf.split_whitespace() {
                        let num = num.parse().context("Cannot parse integer")?;
                        self.buf.push_back(num)
                    }
                }
            }
        }
        Ok(self.buf.pop_front().unwrap())
    }
}
impl From<StreamType> for InputStream {
    fn from(value: StreamType) -> Self {
        Self {
            buf: VecDeque::new(),
            typ: value,
        }
    }
}

struct OutputStream {
    typ: StreamType,
}
impl OutputStream {
    fn write(&self, value: u8) -> io::Result<()> {
        match self.typ {
            StreamType::Bytes => stdout().write_all(&[value])?,
            StreamType::Ascii => writeln!(stdout(), "{value}")?,
        }
        stdout().flush()?;
        Ok(())
    }
}
impl From<StreamType> for OutputStream {
    fn from(value: StreamType) -> Self {
        Self { typ: value }
    }
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .with_level(log::LevelFilter::Warn)
        .env()
        .init()
        .context("Cannot init logging")?;
    match Cli::parse() {
        Cli::Run {
            mut raw,
            input,
            output,
            program,
        } => {
            log::info!("Reading file");
            let program = bf::save::parse(File::open(program).context("Cannot open program file")?)
                .context("Cannot parse program file")?;
            if raw && program.payload.is_ir() {
                log::warn!(
                    "The program in the file is already optimized, running with optimization on"
                );
                raw = false;
            }
            match (raw, program.payload) {
                (true, bf::save::Payload::Ir(_)) => unreachable!(),
                (true, bf::save::Payload::Source(src)) => {
                    let raw = src.parse().context("While parsing raw brainfuck")?;
                    run::<engine::raw::Engine>(raw, input.into(), output.into())?
                }
                (false, bf::save::Payload::Source(src)) => {
                    let ir = src.parse().context("While parsing raw brainfuck")?;
                    run::<engine::ir::Engine>(ir, input.into(), output.into())?
                }
                (false, bf::save::Payload::Ir(ir)) => {
                    run::<engine::ir::Engine>(ir, input.into(), output.into())?
                }
            }
        }
        Cli::Inspect { file } => {
            log::info!("Reading file");
            let header = if let Some(file) = file {
                bf::save::parse(File::open(file).context("Cannot open program file")?)
            } else {
                bf::save::parse(stdin())
            }
            .context("Cannot parse program file")?
            .header;
            serde_yaml::to_writer(stdout(), &header).context("While printing header")?;
        }
        Cli::Compile {
            input,
            output,
            compress,
            format,
        } => {
            let bf::save::File { header, payload } = if let Some(input) = input {
                log::info!("Reading file");
                bf::save::parse(File::open(input).context("Cannot open program file")?)
            } else {
                log::info!("Reading input");
                bf::save::parse(stdin())
            }
            .context("Cannot parse program file")?;
            if format.is_raw() {
                let Payload::Source(source) = payload else {bail!("Cannot conver compiled back into source brainfuck")};
                if let Some(output) = output {
                    bf::save::write_source(
                        File::create(output).context("Creating file")?,
                        source,
                        compress,
                        header.description,
                    )
                    .context("While writing to file")?
                } else {
                    bf::save::write_source(stdout(), source, compress, header.description)
                        .context("While writing to file")?
                }
            } else {
                let payload = match payload {
                    Payload::Source(src) => src.parse().context("Error doring compiling")?,
                    Payload::Ir(ir) => ir,
                };
                if let Some(output) = output {
                    bf::save::write_ir(
                        File::create(output).context("Creating file")?,
                        &payload,
                        compress,
                        header.description,
                        match format {
                            Format::Raw => unreachable!(),
                            Format::Binary => bf::save::Format::CBOR,
                            Format::Json => bf::save::Format::Json,
                        },
                    )
                    .context("While writing to file")?
                } else {
                    bf::save::write_ir(
                        stdout(),
                        &payload,
                        compress,
                        header.description,
                        match format {
                            Format::Raw => unreachable!(),
                            Format::Binary => bf::save::Format::CBOR,
                            Format::Json => bf::save::Format::Json,
                        },
                    )
                    .context("While writing to file")?
                }
            }
        }
    }
    Ok(())
}

fn run<E>(program: E::Program, mut input: InputStream, output: OutputStream) -> anyhow::Result<()>
where
    E: Engine + ProgrammableEngine,
{
    log::info!("Running raw brainfuck");
    let mut engine = E::new(program);
    'l: loop {
        match engine.run().context("Runtime error")? {
            engine::StopState::Halted => {
                log::trace!("Engine halted");
                break 'l;
            }
            engine::StopState::NeedInput => {
                log::trace!("Engine requested input");
                engine.give_input(input.read()?);
            }
            engine::StopState::HasOutput(ch) => {
                log::trace!("Engine emitted output");
                output.write(ch)?;
            }
        }
    }
    Ok(())
}
