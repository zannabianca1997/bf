//! Intermediate representation for optimized execution

use std::{
    mem,
    num::{NonZeroIsize, NonZeroU8},
    ops::{Index, IndexMut},
};

mod optimizations;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Program(pub Block);
impl Program {
    fn from_raw(value: crate::raw::Program) -> Program {
        let mut stack: Vec<Vec<Node>> = vec![vec![]];
        for instr in value {
            match instr {
                crate::raw::Instruction::OpenLoop => stack.push(vec![]),
                crate::raw::Instruction::CloseLoop => {
                    let body = Block(stack.pop().unwrap());
                    stack
                        .last_mut()
                        .unwrap()
                        .push(Node::Loop(Loop { body, offset: 0 }))
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
                    offset: 0,
                })),
                crate::raw::Instruction::Sub => stack.last_mut().unwrap().push(Node::Add(Add {
                    amount: NonZeroU8::new(255).unwrap(),
                    offset: 0,
                })),
                crate::raw::Instruction::Output => stack
                    .last_mut()
                    .unwrap()
                    .push(Node::Output(Output { offset: 0 })),
                crate::raw::Instruction::Input => stack
                    .last_mut()
                    .unwrap()
                    .push(Node::Input(Input { offset: 0 })),
            }
        }
        let [body] = &mut stack[..] else {unreachable!()};
        let mut body = Block(mem::take(body));
        body.optimize();
        Program(body)
    }
}

impl TryFrom<crate::raw::Program> for Program {
    type Error = !;

    fn try_from(value: crate::raw::Program) -> Result<Self, Self::Error> {
        Ok(Self::from_raw(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Block(pub Vec<Node>);

impl Block {
    /// Optimize the block
    ///
    /// Return if something changed
    pub fn optimize(&mut self) -> bool {
        let mut changed = true;
        let mut repeats = 0usize;
        while changed {
            changed = false;
            repeats += 1;
            self.0 = optimizations::optimize(mem::take(&mut self.0), &mut changed);
        }
        repeats > 1
    }
}

impl Index<usize> for Block {
    type Output = Node;

    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}
impl IndexMut<usize> for Block {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum Node {
    #[default]
    Noop,
    Shift(Shift),
    Add(Add),
    Output(Output),
    Input(Input),
    Loop(Loop),
}

impl Node {
    #[must_use]
    pub fn as_block(&self) -> Option<&Block> {
        if let Self::Loop(Loop { body, .. }) = self {
            Some(body)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Shift {
    pub amount: NonZeroIsize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Add {
    pub amount: NonZeroU8,
    pub offset: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input {
    pub offset: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Output {
    pub offset: isize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loop {
    pub body: Block,
    pub offset: isize,
}
