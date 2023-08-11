use std::{
    fs,
    io::{self, stdin, stdout, Write},
    path::PathBuf,
};

use anyhow::Context;
use bf::ir;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about="Read a bf input and print the optimized ir representation", long_about = None)]
struct Args {
    #[clap(long, short)]
    /// Input file. Default to read stdin
    input: Option<PathBuf>,
    #[clap(long, short)]
    /// Output file. Default to write on stdout
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let Args { input, output } = Args::parse();
    let input = if let Some(input) = input {
        fs::read_to_string(input).context("While reading input file")?
    } else {
        io::read_to_string(stdin()).context("While reading stdin")?
    };
    let program: ir::Program = input.parse().context("While parsing program")?;
    if let Some(output) = output {
        write!(
            fs::File::create(output).context("While creating output file")?,
            "{program}"
        )
        .context("While writing output file")?
    } else {
        write!(stdout(), "{program}").context("While writing to stdout")?
    }
    Ok(())
}
