//! Engine running ir brainfuck
//!
//! This is used to check all the steps of the optimization

use crate::ir::{self, Add, Block, Input, Loop, Output, Shift};

use super::{mem::Memory, ProgrammableEngine, RTError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Engine {
    stack: Vec<(Block, usize)>,
    mem: Memory,
    mp: isize,
    input: Option<u8>,
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
        let Self {
            stack,
            mem,
            mp,
            input,
        } = self;

        let advance = |stack: &mut Vec<(Block, usize)>| {
            stack.last_mut().unwrap().1 += 1;
            while stack.len() > 1 && {
                let (blk, pos) = stack.last().unwrap();
                blk.0.len() == *pos
            } {
                let (blk, _) = stack.pop().unwrap();
                let (sup, pos) = stack.last_mut().unwrap();
                match &mut sup.0[*pos] {
                    ir::Node::Loop(Loop { body, .. }) => {
                        // putting back the body
                        *body = blk;
                        // leaving pos as it is, so the loop is reexamined
                    }
                    other => {
                        unreachable!("{other:?} cannot be entered, so it should not be popped into")
                    }
                }
            }
        };

        let get_mem = |mem: &Memory, offset: isize| {
            let mp = *mp + offset;
            if mp < 0 {
                Err(RTError::MemNegativeOut)
            } else {
                Ok(*mem.get(mp as usize))
            }
        };

        let set_mem = |mem: &mut Memory, offset: isize, value: u8| {
            let mp = *mp + offset;
            if mp < 0 {
                Err(RTError::MemNegativeOut)
            } else {
                Ok(mem.set(mp as usize, value))
            }
        };

        match {
            let (blk, pos) = stack.last_mut().unwrap();
            &mut blk.0[*pos]
        } {
            ir::Node::Shift(Shift { amount }) => {
                *mp += amount.get();
                advance(stack);
                Ok(super::State::Running)
            }
            ir::Node::Add(Add { amount, offset }) => {
                set_mem(
                    mem,
                    *offset,
                    get_mem(mem, *offset)?.wrapping_add(amount.get()),
                )?;
                advance(stack);
                Ok(super::State::Running)
            }
            ir::Node::Output(Output { offset }) => {
                let out = get_mem(mem, *offset)?;
                advance(stack);
                Ok(super::State::Stopped(super::StopState::HasOutput(out)))
            }
            ir::Node::Input(Input { offset }) => {
                if let Some(input) = input.take() {
                    set_mem(mem, *offset, input)?;
                    advance(stack);
                    Ok(super::State::Running)
                } else {
                    Ok(super::State::Stopped(super::StopState::NeedInput))
                }
            }
            ir::Node::Loop(Loop { body, offset }) => {
                if get_mem(mem, *offset)? != 0 {
                    let blk = std::mem::take(body);
                    stack.push((blk, 0)); // opening the new frame
                    Ok(super::State::Running)
                } else {
                    advance(stack);
                    Ok(super::State::Running)
                }
            }
            ir::Node::Noop => {
                advance(stack);
                Ok(super::State::Running)
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
