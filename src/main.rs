#![feature(never_type)]
#![feature(unboxed_closures)]
#![feature(error_reporter)]

use std::{
    cell::RefCell,
    collections::HashMap,
    error::{Error, Report},
    fs::File,
    io::{self, stdin, stdout, Read, Write},
    iter::once,
    num::{NonZeroIsize, NonZeroU8, TryFromIntError, Wrapping},
    ops::{Deref, Index, IndexMut},
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::{Parser, Subcommand};
use either::*;
use read_char::ReadIter;
use thiserror::Error;

mod linearmath;

/// Raw brainfuck instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BFInstruction {
    Add,
    Sub,
    Left,
    Right,
    Input,
    Output,
    OpenLoop,
    CloseLoop,
}

impl Into<char> for BFInstruction {
    fn into(self) -> char {
        use BFInstruction::*;
        match self {
            Add => '+',
            Sub => '-',
            Right => '>',
            Left => '<',
            Output => '.',
            Input => ',',
            OpenLoop => '[',
            CloseLoop => ']',
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid brainfuck instruction '{0:?}'")]
struct UnknowChar(char);
impl TryFrom<char> for BFInstruction {
    type Error = UnknowChar;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use BFInstruction::*;
        Ok(match value {
            '+' => Add,
            '-' => Sub,
            '>' => Right,
            '<' => Left,
            '[' => OpenLoop,
            ']' => CloseLoop,
            '.' => Output,
            ',' => Input,
            _ => return Err(UnknowChar(value)),
        })
    }
}

/// A raw brainfuck program
#[derive(Debug, Clone)]
struct BFProgram(Box<[BFInstruction]>);
impl Deref for BFProgram {
    type Target = [BFInstruction];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl FromIterator<BFInstruction> for BFProgram {
    fn from_iter<T: IntoIterator<Item = BFInstruction>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl IntoIterator for BFProgram {
    type Item = BFInstruction;

    type IntoIter = <Vec<BFInstruction> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}
impl FromStr for BFProgram {
    type Err = !;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.chars()
            .filter_map(|ch| BFInstruction::try_from(ch).ok())
            .collect())
    }
}

#[derive(Debug, Error)]
enum BFError {
    #[error("Unexpected EOF")]
    UnexpectedEOF,
    #[error("Unmatched [")]
    UnmatchedOpenLoop,
    #[error("Unmatched ]")]
    UnmatchedCloseLoop,
    #[error(transparent)]
    MemoryPointerUnderflow(#[from] MemoryPointerUnderflowError),
    #[error("Input char {0:?} out of ascii range")]
    NonAsciiInput(char),
}

#[derive(Debug, Error)]
#[error("Memory pointer underflow")]
struct MemoryPointerUnderflowError;
impl From<TryFromIntError> for MemoryPointerUnderflowError {
    fn from(value: TryFromIntError) -> Self {
        Self
    }
}

#[derive(Debug, Error)]
enum BFEngineError<IErr: Error, OErr: Error> {
    #[error(transparent)]
    BF(#[from] BFError),
    #[error("Error during input")]
    Inp(#[source] IErr),
    #[error("Error during output")]
    Out(#[source] OErr),
}
impl<IErr: Error, OErr: Error> From<MemoryPointerUnderflowError> for BFEngineError<IErr, OErr> {
    fn from(value: MemoryPointerUnderflowError) -> Self {
        Self::BF(value.into())
    }
}

impl BFProgram {
    /// Run a raw, unoptimize brainfuck source
    fn run<
        IErr: Error,
        OErr: Error,
        I: FnMut<(), Output = Result<Option<char>, IErr>>,
        O: FnMut<(char,), Output = Result<(), OErr>>,
    >(
        self,
        mut inp: I,
        mut out: O,
    ) -> Result<(), BFEngineError<IErr, OErr>> {
        // Instruction pointer
        let mut ip = 0;
        // Memory
        let mut mem = IRState::new();

        while let Some(instr) = self.get(ip) {
            use BFError::*;
            use BFInstruction::*;
            match instr {
                Add => mem.set(0, mem.get(0)? + Wrapping(1))?,
                Sub => {
                    if mp >= mem.len() {
                        mem.resize(mp + 1, 0)
                    }
                    mem[mp] = mem[mp].wrapping_sub(1)
                }
                Left => {
                    if mp > 0 {
                        mp -= 1
                    } else {
                        return Err(MemoryPointerUnderflowError.into());
                    }
                }
                Right => mp += 1,
                Input => {
                    let ch = inp().map_err(BFEngineError::Inp)?.ok_or(UnexpectedEOF)?;
                    if ch.is_ascii() {
                        if mp >= mem.len() {
                            mem.resize(mp + 1, 0)
                        }
                        mem[mp] = ch as u8;
                    } else {
                        return Err(NonAsciiInput(ch).into());
                    }
                }
                Output => out(*mem.get(mp).unwrap_or(&0) as char).map_err(BFEngineError::Out)?,
                OpenLoop => {
                    if *mem.get(mp).unwrap_or(&0) == 0 {
                        // skip to the corresponding ]
                        let mut depth = 1usize;
                        while depth > 0 {
                            ip += 1;
                            match self.get(ip).ok_or::<_>(UnmatchedOpenLoop)? {
                                OpenLoop => depth += 1,
                                CloseLoop => depth -= 1,
                                _ => (),
                            }
                        }
                    }
                }
                CloseLoop => {
                    if *mem.get(mp).unwrap_or(&0) != 0 {
                        // skip to the corresponding [
                        let mut depth = 1usize;
                        while depth > 0 {
                            ip -= 1;
                            match self.get(ip).ok_or::<_>(UnmatchedCloseLoop)? {
                                CloseLoop => depth += 1,
                                OpenLoop => depth -= 1,
                                _ => (),
                            }
                        }
                    }
                }
            }
            // advance to the next instruction
            ip += 1
        }
        Ok(())
    }
}

/// A linear combination of memory places
#[derive(Debug, Clone)]
struct LinCombination {
    addends: HashMap<isize, NonZeroU8>,
    constant: u8,
}

impl LinCombination {
    fn new(addends: HashMap<isize, NonZeroU8>, constant: u8) -> Self {
        Self { addends, constant }
    }
}

/// A node in the IR representation that does not move mp
#[derive(Debug, Clone)]
enum IRNode {
    Set {
        pos: isize,
        value: LinCombination,
    },
    Inp {
        pos: isize,
    },
    Out {
        pos: isize,
        constant: u8,
    },
    Block {
        body: Box<[IRNode]>,
        is_fixed: bool,
        does_input: bool,
        does_output: bool,
    },
    If {
        pos: isize,
        value: LinCombination,
        body: Box<IRNode>,
    },
    ShiftingLoop {
        pos: isize,
        shift: NonZeroIsize,
        body: Box<IRNode>,
    },
    Loop {
        pos: isize,
        body: Box<IRNode>,
    },
    InfiniteLoop,
}

impl IRNode {
    /// Create a new Set instruction
    fn set(pos: isize, value: LinCombination) -> Self {
        Self::Set { pos, value }
    }

    /// optimize this node
    /// Return Left if the optimization worked, Right if it's idempotent
    fn optimize(self) -> Either<Self, Self> {
        Right(self)
    }
    /// Run this node
    fn run<
        IErr: Error,
        OErr: Error,
        I: FnMut<(), Output = Result<Option<char>, IErr>>,
        O: FnMut<(char,), Output = Result<(), OErr>>,
    >(
        &self,
        state: &mut IRState,
        io: &mut IRio<IErr, OErr, I, O>,
    ) -> Result<(), BFEngineError<IErr, OErr>> {
        todo!()
    }
}

impl TryFrom<BFProgram> for IRNode {
    type Error = BFError;

    fn try_from(value: BFProgram) -> Result<Self, Self::Error> {
        todo!()
    }
}

struct IRState {
    mem: Vec<Wrapping<u8>>,
    mp: isize,
}
impl IRState {
    fn new() -> IRState {
        Self { mem: vec![], mp: 0 }
    }

    fn get(&self, idx: isize) -> Result<Wrapping<u8>, MemoryPointerUnderflowError> {
        let idx = usize::try_from(idx + self.mp)?;
        Ok(self.mem.get(idx).copied().unwrap_or(Wrapping(0)))
    }

    fn set(&mut self, idx: isize, value: Wrapping<u8>) -> Result<(), MemoryPointerUnderflowError> {
        let idx = usize::try_from(idx + self.mp)?;
        if idx >= self.mem.len() {
            // enlarge only if needed
            if value.0 != 0 {
                self.mem.resize(idx + 1, Wrapping(0));
                self.mem[idx] = value;
            }
        } else {
            self.mem[idx] = value;
        }
        Ok(())
    }

    fn shift(&mut ) {
        
    }
}

struct IRio<
    IErr: Error,
    OErr: Error,
    I: FnMut<(), Output = Result<Option<char>, IErr>>,
    O: FnMut<(char,), Output = Result<(), OErr>>,
> {
    inp: I,
    out: O,
}

trait IRCodeGenerator {
    // generate code
    fn generate_code(self, code: IRNode) -> io::Result<()>;
}

struct RustCG<O: Write> {
    out: O,
}
impl<O: Write> RustCG<O> {
    fn new(out: O) -> Self {
        Self { out }
    }
}

impl<O: Write> IRCodeGenerator for RustCG<O> {
    fn generate_code(self, code: IRNode) -> io::Result<()> {
        todo!()
    }
}

#[derive(Debug, Error)]
enum MainError {
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    BFEngine(#[from] BFEngineError<read_char::Error, !>),
    #[error(transparent)]
    BFError(#[from] BFError),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The brainfuck program
    #[arg(value_name = "FILE")]
    program: PathBuf,
    /// Action to take
    #[command(subcommand)]
    action: CLIAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
enum CLIAction {
    /// Run the program
    Run,
    /// Generate Rust code
    Generate {
        /// The output file
        #[arg(value_name = "OUTPUT_FILE")]
        out: PathBuf,
    },
}

fn execute(program: IRNode) -> Result<(), BFEngineError<read_char::Error, !>> {
    let mut inp = ReadIter::new(stdin());
    let out = RefCell::new(String::new());

    program.run(
        &mut IRState::new(),
        &mut IRio {
            inp: || {
                let mut out = out.borrow_mut();
                if !out.is_empty() {
                    print!("{out}");
                    *out = String::new();
                    stdout().flush().unwrap();
                }
                inp.next().transpose()
            },
            out: |ch| Ok(out.borrow_mut().push(ch)),
        },
    )?;
    let out = out.borrow();
    print!("{out}");
    Ok(())
}

fn generate(program: IRNode, out: &Path) -> io::Result<()> {
    RustCG::new(File::create(out)?).generate_code(program)
}

fn run() -> Result<(), MainError> {
    let args = Cli::parse();
    println!("Reading program...");
    let program = {
        let mut buf = String::new();
        File::open(args.program)?.read_to_string(&mut buf)?;
        buf.parse::<BFProgram>()
            .expect("Parsing should be infallible")
    };
    println!("Optimizing...");
    dbg!(&program);
    let program = IRNode::try_from(program)?.optimize().into_inner();
    dbg!(&program);
    match args.action {
        CLIAction::Run => {
            println!("Executing...");
            execute(program)?
        }
        CLIAction::Generate { out } => {
            println!("Generating code...");
            generate(program, &out)?
        }
    };
    Ok(())
}

fn main() -> Result<(), Report<MainError>> {
    Ok(run()?)
}
