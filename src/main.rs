#![feature(error_reporter)]

use std::{
    error::Report,
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

use bf::{ir::UnmatchedLoops, IRProgram, RawProgram};
use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck source file
    source: PathBuf,
    /// What to do
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(Debug, Subcommand, Default)]
enum Command {
    /// Execute the brainfuck program
    #[default]
    Run,
}

#[derive(Debug, Error)]
enum MainError {
    #[error("Error while reading BF program")]
    ReadProgram(#[source] io::Error),
    #[error(transparent)]
    UnmatchedLoops(#[from] UnmatchedLoops),
}

fn run() -> Result<(), MainError> {
    // Parsing the command line error, and failing if impossible
    let Args { source, cmd } = Args::parse();
    // reading the source file
    let raw_program = {
        let mut buf = String::new();
        File::open(source)
            .and_then(|mut file| file.read_to_string(&mut buf))
            .map_err(MainError::ReadProgram)?;
        buf.parse::<RawProgram>().unwrap()
    };
    // parsing into a IR ast
    let ast = IRProgram::try_from(raw_program)?;

    todo!()
}

fn main() -> Result<(), Report<MainError>> {
    Ok(run()?)
}
