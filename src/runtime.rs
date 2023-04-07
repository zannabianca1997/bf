#![feature(error_reporter)]
use std::{
    convert::TryFrom,
    error::{Error, Report},
    fmt::Display,
    io::{self, stdin, Read},
};

struct Memory(Vec<u8>);
impl Memory {
    fn new() -> Self {
        Self(vec![])
    }
    fn get(&self, index: isize) -> Result<u8, BFError> {
        let index = usize::try_from(index).map_err(|_| BFError::MemoryPointerUnderflow)?;
        Ok(*self.0.get(index).unwrap_or(&0))
    }
    fn set(&mut self, index: isize, value: u8) -> Result<(), BFError> {
        let index = usize::try_from(index).map_err(|_| BFError::MemoryPointerUnderflow)?;
        if self.0.len() <= index {
            self.0.resize(index + 1, 0)
        }
        self.0[index] = value;
        Ok(())
    }
}

#[derive(Debug)]
enum BFError {
    UnexpectedEOF,
    MemoryPointerUnderflow,
    #[allow(dead_code)]
    NonAsciiInput(char),
    IO(io::Error),
}
impl From<io::Error> for BFError {
    fn from(value: io::Error) -> Self {
        if value.kind() == io::ErrorKind::UnexpectedEof {
            Self::UnexpectedEOF
        } else {
            Self::IO(value)
        }
    }
}
impl Display for BFError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BFError::UnexpectedEOF => write!(f, "Unexpected end of file")?,
            BFError::MemoryPointerUnderflow => write!(f, "Memory pointer underflow")?,
            BFError::NonAsciiInput(ch) => write!(f, "Non ascii input {ch:?}")?,
            BFError::IO(_) => write!(f, "Error during input")?,
        }
        Ok(())
    }
}
impl Error for BFError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let BFError::IO(err) = self {
            Some(err)
        } else {
            None
        }
    }
}

#[allow(dead_code)]
fn read_char() -> Result<u8, BFError> {
    let mut byte = [0u8];
    stdin().read_exact(&mut byte)?;
    let byte = byte[0];
    if byte.is_ascii() {
        Ok(byte)
    } else {
        Err(BFError::NonAsciiInput(byte as char))
    }
}

#[allow(unused_mut)]
fn run(mut mem: Memory, mut mp: isize) -> Result<(), BFError> {
    todo!("<GENERATED CODE HERE>");
    Ok(())
}

fn main() -> Result<(), Report<BFError>> {
    run(Memory::new(), 0).map_err(|err| Report::new(err).pretty(true))
}
