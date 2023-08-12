//! Intermediate representation for optimized execution

use std::{
    fmt::{Display, Write},
    mem,
    num::{NonZeroIsize, NonZeroU8},
    ops::{Index, IndexMut},
    str::FromStr,
};

use indenter::indented;
use serde::{Deserialize, Serialize};

use crate::raw;

mod optimizations;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        while body.optimize() {
            // removing leading loops
            let mut s = 0;
            while matches!(body.0[s], Node::Loop(_)) {
                s += 1;
            }
            // removing tail with no side-effects or inputs
            let mut e = body.0.len().saturating_sub(1);
            while body.0[e].diverge() == Some(false) && !body.0[e].does_output() {
                e -= 1;
            }
            body = Block(body.0.drain(s..=e).collect())
        }

        Program(body)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for n in &self.0 .0 {
            writeln!(f, "{n}")?
        }
        Ok(())
    }
}

impl TryFrom<crate::raw::Program> for Program {
    type Error = !;

    fn try_from(value: crate::raw::Program) -> Result<Self, Self::Error> {
        Ok(Self::from_raw(value))
    }
}

impl FromStr for Program {
    type Err = <raw::Program as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::try_from(s.parse::<raw::Program>()?).unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
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
impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Noop => write!(f, "noop"),
            Node::Shift(c) => write!(f, "{c}"),
            Node::Add(c) => write!(f, "{c}"),
            Node::Output(c) => write!(f, "{c}"),
            Node::Input(c) => write!(f, "{c}"),
            Node::Loop(c) => write!(f, "{c}"),
        }
    }
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

    /// return the instruction shifted of the given amount
    /// >{instr}< ~ {instr.shifted(1)}
    fn shifted(self, additional_offset: isize) -> Self {
        match self {
            Node::Noop => Node::Noop,
            Node::Shift(shift) => Node::Shift(shift),
            Node::Add(Add { amount, offset }) => Node::Add(Add {
                amount,
                offset: offset + additional_offset,
            }),
            Node::Output(Output { offset }) => Node::Output(Output {
                offset: offset + additional_offset,
            }),
            Node::Input(Input { offset }) => Node::Input(Input {
                offset: offset + additional_offset,
            }),
            Node::Loop(Loop {
                body: Block(nodes),
                offset,
            }) => Node::Loop(Loop {
                body: Block(
                    nodes
                        .into_iter()
                        .map(|n| n.shifted(additional_offset))
                        .collect(),
                ),
                offset: offset + additional_offset,
            }),
        }
    }

    fn does_input(&self) -> bool {
        match self {
            Node::Output(_) => true,
            Node::Loop(Loop {
                body: Block(nodes), ..
            }) => nodes.iter().any(Node::does_output),
            Node::Noop | Node::Shift(_) | Node::Add(_) | Node::Input(_) => false,
        }
    }
    fn does_output(&self) -> bool {
        match self {
            Node::Output(_) => true,
            Node::Loop(Loop {
                body: Block(nodes), ..
            }) => nodes.iter().any(Node::does_output),
            Node::Noop | Node::Shift(_) | Node::Add(_) | Node::Input(_) => false,
        }
    }
    fn diverge(&self) -> Option<bool> {
        match self {
            Node::Noop | Node::Shift(_) | Node::Add(_) | Node::Output(_) | Node::Input(_) => {
                Some(false)
            }
            Node::Loop(_) => None, // TODO: More checks to identify diverging loops
        }
    }

    /// check if two nodes can be exchanged
    fn commute(&self, other: &Self) -> bool {
        match (self, other) {
            // Noop always commute
            (Node::Noop, _) | (_, Node::Noop) => true,
            // shift commute with himself, but with nothing else ( this will be handled with retarded shift)
            (Node::Shift(_), Node::Shift(_)) => true,
            (Node::Shift(_), Node::Add(_) | Node::Output(_) | Node::Input(_) | Node::Loop(_))
            | (Node::Add(_) | Node::Output(_) | Node::Input(_) | Node::Loop(_), Node::Shift(_)) => {
                false
            }
            // Add commute with IO and himself, but only if they refere to different memory positions
            (
                Node::Add(Add { offset: o1, .. }),
                Node::Add(Add { offset: o2, .. })
                | Node::Output(Output { offset: o2 })
                | Node::Input(Input { offset: o2 }),
            )
            | (
                Node::Output(Output { offset: o2 }) | Node::Input(Input { offset: o2 }),
                Node::Add(Add { offset: o1, .. }),
            ) => o1 != o2,
            // input and output will never exchange positions
            (Node::Output(_) | Node::Input(_), Node::Output(_) | Node::Input(_)) => false,

            // If uncertain, do not commute
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Shift {
    pub amount: NonZeroIsize,
}
impl Display for Shift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shift\t{}", self.amount)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Add {
    pub amount: NonZeroU8,
    pub offset: isize,
}
impl Display for Add {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "add\t{}\t@{}", self.amount, self.offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Input {
    pub offset: isize,
}
impl Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "input\t\t@{}", self.offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Output {
    pub offset: isize,
}
impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "output\t\t@{}", self.offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Loop {
    pub body: Block,
    pub offset: isize,
}
impl Display for Loop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "loop\t@{} [", self.offset)?;
        for node in &self.body.0 {
            writeln!(indented(f), "{}", node)?
        }
        write!(f, "]")?;
        Ok(())
    }
}
