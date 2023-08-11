//! Engine running ir brainfuck
//!
//! This is used to check all the steps of the optimization

use crate::ir::{self, Add, Block, Loop, Shift};

use super::{mem::Memory, ProgrammableEngine, RTError};

pub struct Engine {
    stack: Vec<(Block, usize)>,
    mem: Memory,
    mp: isize,
    input: Option<u8>,
}

impl Engine {
    #[inline]
    #[must_use]
    fn get_mem_curr(&self) -> Result<&u8, RTError> {
        if self.mp < 0 {
            Err(RTError::MemNegativeOut)
        } else {
            Ok(self.mem.get(self.mp as usize))
        }
    }
    #[inline]
    #[must_use]
    fn set_mem_curr(&mut self, value: u8) -> Result<(), RTError> {
        if self.mp < 0 {
            Err(RTError::MemNegativeOut)
        } else {
            Ok(self.mem.set(self.mp as usize, value))
        }
    }

    fn advance(&mut self) {
        self.stack.last_mut().unwrap().1 += 1;
        while self.stack.len() > 1 && {
            let (blk, pos) = self.stack.last().unwrap();
            blk.0.len() == *pos
        } {
            let (blk, _) = self.stack.pop().unwrap();
            let (sup, pos) = self.stack.last_mut().unwrap();
            match &mut sup.0[*pos] {
                ir::Node::Loop(Loop { body }) => {
                    // putting back the body
                    *body = blk;
                    // leaving pos as it is, so the loop is reexamined
                }
                other => {
                    unreachable!("{other:?} cannot be entered, so it should not be popped into")
                }
            }
        }
    }
}

impl ProgrammableEngine for Engine {
    type Program = ir::Program;

    fn new(program: Self::Program) -> Self
    where
        Self: Sized,
    {
        Self {
            stack: vec![(program.0, 0)],
            mem: Memory::new(),
            mp: 0,
            input: None,
        }
    }
}

impl super::Engine for Engine {
    fn step(&mut self) -> Result<super::State, RTError> {
        if let [(blk, pos)] = &self.stack[..] {
            if *pos == blk.0.len() {
                return Ok(super::State::Stopped(super::StopState::Halted));
            }
        }
        // storing it in case we need to read it keeping a mutable ref to self
        let current_mem = self.get_mem_curr().map(|x| *x);
        match {
            let (blk, pos) = self.stack.last_mut().unwrap();
            &mut blk.0[*pos]
        } {
            ir::Node::Shift(Shift { amount }) => {
                self.mp += amount.get();
                self.advance();
                Ok(super::State::Running)
            }
            ir::Node::Add(Add { amount }) => {
                let amount = *amount;
                self.set_mem_curr(self.get_mem_curr()?.wrapping_add(amount.get()))?;
                self.advance();
                Ok(super::State::Running)
            }
            ir::Node::Output => {
                let out = *self.get_mem_curr()?;
                self.advance();
                Ok(super::State::Stopped(super::StopState::HasOutput(out)))
            }
            ir::Node::Input => {
                if let Some(input) = self.input.take() {
                    self.set_mem_curr(input)?;
                    self.advance();
                    Ok(super::State::Running)
                } else {
                    Ok(super::State::Stopped(super::StopState::NeedInput))
                }
            }
            ir::Node::Loop(Loop { body }) => {
                if current_mem? != 0 {
                    let blk = std::mem::take(body);
                    self.stack.push((blk, 0)); // opening the new frame
                    Ok(super::State::Running)
                } else {
                    self.advance();
                    Ok(super::State::Running)
                }
            }
        }
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
