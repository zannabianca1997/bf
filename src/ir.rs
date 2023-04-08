//! Intermediate representation for BF

use std::num::NonZeroIsize;

use thiserror::Error;

use crate::{
    linear::{Affine, AffineToScalar, BiasPropToScalar},
    optimize, raw, runtime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// Destination cells that are summed to the readed value, with coefficients
    dest: BiasPropToScalar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Value to output, measured from memory
    value: AffineToScalar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemOp {
    /// General memory transformation
    transform: Affine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftingLoop {
    /// Condition to start the loop
    condition: AffineToScalar,
    /// Loop body
    body: Vec<Instruction>,
    /// shift of the memory pointer at the end of the loop
    shift: NonZeroIsize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    /// Condition to start the loop
    condition: AffineToScalar,
    /// Loop body
    body: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    /// Condition to execute the body
    condition: AffineToScalar,
    /// If body
    body: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shift(NonZeroIsize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diverge {
    /// loops forever with no side effect
    Loop,
    /// Runtime BF error
    Error(runtime::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// Shift the memory pointer of a given offset
    Shift(Shift),
    /// Add input readed to multiple cells
    Input(Input),
    /// Output an affine combinations of cells
    Output(Output),
    /// Execute an affine transformation of memory
    MemOp(MemOp),
    /// Loop, shifting at each turn
    ShiftingLoop(ShiftingLoop),
    Loop(Loop),
    If(If),
    Diverge(Diverge),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program(Vec<Instruction>);

impl Program {
    pub fn optimize(self) -> Self {
        Self(optimize::optimize_main_body(self.0))
    }
}

#[derive(Debug, Error)]
pub enum UnmatchedLoops {
    #[error("Unmatched [")]
    Open,
    #[error("Unmatched ]")]
    Close,
}

impl TryFrom<raw::Program> for Program {
    type Error = UnmatchedLoops;

    fn try_from(value: raw::Program) -> Result<Self, Self::Error> {
        let mut stack = vec![];
        let mut body = vec![];
        for instr in value {
            match instr {
                raw::Instruction::Left => {
                    body.push(Instruction::Shift(Shift(NonZeroIsize::new(-1).unwrap())))
                }
                raw::Instruction::Right => {
                    body.push(Instruction::Shift(Shift(NonZeroIsize::new(1).unwrap())))
                }
                raw::Instruction::Input => {
                    // set location to 0
                    body.push(Instruction::MemOp(MemOp {
                        transform: Affine::set(0, 0),
                    }));
                    // add inputted value
                    body.push(Instruction::Input(Input {
                        dest: BiasPropToScalar::to_single(0),
                    }))
                }
                raw::Instruction::Output => body.push(Instruction::Output(Output {
                    value: AffineToScalar::extract(0),
                })),
                raw::Instruction::Increase => body.push(Instruction::MemOp(MemOp {
                    transform: Affine::add(0, 1),
                })),
                raw::Instruction::Decrease => body.push(Instruction::MemOp(MemOp {
                    transform: Affine::add(0, u8::MAX),
                })),
                raw::Instruction::OpenLoop => {
                    stack.push(body);
                    body = Vec::new();
                }
                raw::Instruction::CloseLoop => {
                    let mut loop_body = body;
                    loop_body.shrink_to_fit();

                    body = stack.pop().ok_or(UnmatchedLoops::Close)?;
                    body.push(Instruction::Loop(Loop {
                        condition: AffineToScalar::extract(0),
                        body: loop_body,
                    }))
                }
            }
        }

        if !stack.is_empty() {
            Err(UnmatchedLoops::Open)
        } else {
            Ok(Program(body))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{Instruction, Loop, Program};
    use crate::{linear::AffineToScalar, raw};

    #[test]
    fn simple_loop() {
        let raw = "[]".parse::<raw::Program>().unwrap();
        let ir = Program::try_from(raw).expect("`[]` should be a valid program");
        assert_eq!(
            ir,
            Program(vec![Instruction::Loop(Loop {
                condition: AffineToScalar::extract(0),
                body: vec![]
            })])
        )
    }
}
