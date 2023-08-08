//! Brainfuck executors

use either::Either::{self, Left, Right};

use crate::raw::UnmatchedParentheses;

/// State of a stopped engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StopState {
    Halted,
    NeedInput,
    HasOutput(u8),
}

/// State of an engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
    Running,
    Stopped(StopState),
}

/// Runtime brainfuck error
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RTError {
    /// The memory pointer exited the memory from below
    MemNegativeOut,
}

/// A brainfuck engine
pub trait Engine {
    /// Step the engine
    fn step(&mut self) -> Result<State, RTError>;

    /// Run the engine until something stops it
    fn run(&mut self) -> Result<StopState, RTError> {
        loop {
            match self.step() {
                Ok(State::Running) => (),
                Ok(State::Stopped(state)) => return Ok(state),
                Err(err) => return Err(err),
            }
        }
    }

    /// Check if the engine has input
    fn has_input(&self) -> bool {
        self.input().is_some()
    }
    /// Check what input the engine has
    fn input(&self) -> Option<u8>;
    /// Give input to the engine
    /// If the engine has already some input, it is returned
    fn give_input(&mut self, input: u8) -> Option<u8>;
    /// Give input to the engine
    /// If the engine has already some input, do not do anything and return the input present as error
    fn try_give_input(&mut self, input: u8) -> Result<(), u8>;
}

/// A brainfuck engine that can be programmed
pub trait ProgrammableEngine {
    type Program;

    /// Create a new engine with the given program
    fn new(program: Self::Program) -> Self
    where
        Self: Sized;

    /// Create a new engine from raw breinfuck
    fn new_from_raw(
        program: crate::raw::Program,
    ) -> Result<Self, <Self::Program as TryFrom<crate::raw::Program>>::Error>
    where
        Self: Sized,
        Self::Program: TryFrom<crate::raw::Program>,
    {
        let program = program.try_into()?;
        Ok(Self::new(program))
    }

    /// Create a new engine from raw breinfuck string
    fn new_from_str<S>(
        program: S,
    ) -> Result<
        Self,
        Either<<Self::Program as TryFrom<crate::raw::Program>>::Error, UnmatchedParentheses>,
    >
    where
        S: AsRef<str>,
        Self: Sized,
        Self::Program: TryFrom<crate::raw::Program>,
    {
        let program =
            Self::Program::try_from(program.as_ref().parse().map_err(Right)?).map_err(Left)?;
        Ok(Self::new(program))
    }
}

pub mod raw;
