//! Intermediate representation for optimized execution

use std::{
    mem,
    num::{NonZeroIsize, NonZeroU8},
};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Program(Block);
impl Program {
    fn from_raw(value: crate::raw::Program) -> Program {
        let mut stack: Vec<Vec<Node>> = vec![vec![]];
        for instr in value {
            match instr {
                crate::raw::Instruction::OpenLoop => stack.push(vec![]),
                crate::raw::Instruction::CloseLoop => {
                    let body = Block(stack.pop().unwrap());
                    stack.last_mut().unwrap().push(Node::Loop(Loop { body }))
                }

                crate::raw::Instruction::ShiftRight => {
                    stack.last_mut().unwrap().push(Node::Shift(Shift {
                        amount: NonZeroIsize::new(1).unwrap(),
                    }))
                }
                crate::raw::Instruction::ShiftLeft => {
                    stack.last_mut().unwrap().push(Node::Shift(Shift {
                        amount: NonZeroIsize::new(-1).unwrap(),
                    }))
                }
                crate::raw::Instruction::Add => stack.last_mut().unwrap().push(Node::Add(Add {
                    amount: NonZeroU8::new(1).unwrap(),
                })),
                crate::raw::Instruction::Sub => stack.last_mut().unwrap().push(Node::Add(Add {
                    amount: NonZeroU8::new(255).unwrap(),
                })),
                crate::raw::Instruction::Output => stack.last_mut().unwrap().push(Node::Output),
                crate::raw::Instruction::Input => stack.last_mut().unwrap().push(Node::Input),
            }
        }
        let [body] = &mut stack[..] else {unreachable!()};
        Program(Block(mem::take(body)))
    }
}

impl TryFrom<crate::raw::Program> for Program {
    type Error = !;

    fn try_from(value: crate::raw::Program) -> Result<Self, Self::Error> {
        Ok(Self::from_raw(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block(Vec<Node>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Node {
    Shift(Shift),
    Add(Add),
    Output,
    Input,
    Loop(Loop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Shift {
    amount: NonZeroIsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Add {
    amount: NonZeroU8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loop {
    body: Block,
}
