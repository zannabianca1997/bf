//! Unoptimized engine running raw brainfuck
//!
//! This is used as baseline, and to check outputs

use crate::raw;

use super::{mem::Memory, ProgrammableEngine, RTError, State, StopState};

/// Unoptimized engine running raw brainfuck
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Engine {
    program: raw::Program,
    ip: usize,
    mem: super::mem::Memory,
    mp: isize,
    input: Option<u8>,
}
impl Engine {
    #[inline]
    #[must_use]
    fn mem_curr(&self) -> Result<&u8, RTError> {
        if self.mp < 0 {
            Err(RTError::MemNegativeOut)
        } else {
            Ok(self.mem.get(self.mp as usize))
        }
    }
    #[inline]
    #[must_use]
    fn mem_curr_mut(&mut self) -> Result<&mut u8, RTError> {
        if self.mp < 0 {
            Err(RTError::MemNegativeOut)
        } else {
            Ok(self.mem.get_mut(self.mp as usize))
        }
    }
}

impl ProgrammableEngine for Engine {
    type Program = crate::raw::Program;

    fn new(program: Self::Program) -> Self
    where
        Self: Sized,
    {
        Self {
            program,
            ip: 0,
            mem: Memory::new(),
            mp: 0,
            input: None,
        }
    }
}

impl super::Engine for Engine {
    fn step(&mut self) -> Result<State, RTError> {
        if self.ip == self.program.len() {
            return Ok(State::Stopped(StopState::Halted));
        }
        Ok(match self.program[self.ip] {
            raw::Instruction::ShiftRight => {
                self.mp += 1;
                self.ip += 1;
                State::Running
            }
            raw::Instruction::ShiftLeft => {
                self.mp -= 1;
                self.ip += 1;
                State::Running
            }
            raw::Instruction::Add => {
                *self.mem_curr_mut()? = self.mem_curr()?.wrapping_add(1);
                self.ip += 1;
                State::Running
            }
            raw::Instruction::Sub => {
                *self.mem_curr_mut()? = self.mem_curr()?.wrapping_sub(1);
                self.ip += 1;
                State::Running
            }
            raw::Instruction::Output => {
                let out = *self.mem_curr()?;
                self.ip += 1;
                State::Stopped(StopState::HasOutput(out))
            }
            raw::Instruction::Input => match self.input.take() {
                Some(input) => {
                    *self.mem_curr_mut()? = input;
                    self.ip += 1;
                    State::Running
                }
                None => State::Stopped(StopState::NeedInput),
            },
            raw::Instruction::OpenLoop => {
                if *self.mem_curr()? == 0 {
                    let mut count = 1usize;
                    // go to the matching ]
                    while count > 0 {
                        self.ip += 1;
                        match self.program[self.ip] {
                            raw::Instruction::OpenLoop => count += 1,
                            raw::Instruction::CloseLoop => count -= 1,
                            _ => (),
                        }
                    }
                }
                // jump the [/]
                self.ip += 1;
                State::Running
            }
            raw::Instruction::CloseLoop => {
                if *self.mem_curr()? != 0 {
                    let mut count = 1usize;
                    // go to the matching [
                    while count > 0 {
                        self.ip -= 1;
                        match self.program[self.ip] {
                            raw::Instruction::OpenLoop => count -= 1,
                            raw::Instruction::CloseLoop => count += 1,
                            _ => (),
                        }
                    }
                }
                // jump the [/]
                self.ip += 1;
                State::Running
            }
        })
    }

    fn input(&self) -> Option<u8> {
        self.input
    }

    fn give_input(&mut self, input: u8) -> Option<u8> {
        self.input.replace(input)
    }

    fn try_give_input(&mut self, input: u8) -> Result<(), u8> {
        match self.input {
            Some(input) => Err(input),
            None => {
                self.input = Some(input);
                Ok(())
            }
        }
    }
}
