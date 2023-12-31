//! Raw brainfuck utilities

use std::{
    fmt::Display,
    mem::size_of,
    ops::{Index, IndexMut},
    slice,
    str::{from_utf8_unchecked, FromStr},
    vec,
};

use static_assertions::const_assert_eq;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Instruction {
    ShiftRight = b'>',
    ShiftLeft = b'<',
    Add = b'+',
    Sub = b'-',
    Output = b'.',
    Input = b',',
    OpenLoop = b'[',
    CloseLoop = b']',
}

impl TryFrom<u8> for Instruction {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Instruction::*;
        match value {
            b'>' => Ok(ShiftRight),
            b'<' => Ok(ShiftLeft),
            b'+' => Ok(Add),
            b'-' => Ok(Sub),
            b'.' => Ok(Output),
            b',' => Ok(Input),
            b'[' => Ok(OpenLoop),
            b']' => Ok(CloseLoop),
            _ => Err(value),
        }
    }
}
impl From<Instruction> for u8 {
    fn from(value: Instruction) -> Self {
        value as u8
    }
}

impl TryFrom<char> for Instruction {
    type Error = char;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match u8::try_from(value).map(Instruction::try_from) {
            Ok(Ok(res)) => Ok(res),
            _ => Err(value),
        }
    }
}
impl From<Instruction> for char {
    fn from(value: Instruction) -> Self {
        (value as u8) as char
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", char::from(*self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Program {
    code: Box<[Instruction]>,
}

impl Program {
    /// Get the program as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        // This will fail to compile if instructions are not mere bytes
        const_assert_eq!(size_of::<Instruction>(), size_of::<u8>());

        let code = self.code.as_ref().as_ptr() as *const u8;
        unsafe {
            /*
               SAFETY: Instruction are representes as single bytes (thanks to `#[repr(u8)]`)
            */
            slice::from_raw_parts(code, self.code.len())
        }
    }
    /// Get the program as a str slice
    pub fn as_str(&self) -> &str {
        let code = self.as_bytes();
        unsafe {
            /*
               SAFETY: Instruction are all printable ascii bytes
            */
            from_utf8_unchecked(code)
        }
    }

    pub fn iter(&self) -> slice::Iter<'_, Instruction> {
        self.code.iter()
    }
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, Instruction> {
        self.code.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn from_chars(code: impl IntoIterator<Item = char>) -> Result<Self, UnmatchedParentheses> {
        Self::from_instrs(
            code.into_iter()
                .filter_map(|ch| Instruction::try_from(ch).ok()),
        )
    }

    pub fn from_bytes(code: impl IntoIterator<Item = u8>) -> Result<Self, UnmatchedParentheses> {
        Self::from_instrs(
            code.into_iter()
                .filter_map(|ch| Instruction::try_from(ch).ok()),
        )
    }

    pub fn from_instrs(
        code: impl IntoIterator<Item = Instruction>,
    ) -> Result<Self, UnmatchedParentheses> {
        let code: Box<_> = code.into_iter().collect();

        let mut par_count = 0usize;
        for instr in code.iter() {
            match instr {
                Instruction::OpenLoop => par_count += 1,
                Instruction::CloseLoop => {
                    par_count = par_count.checked_sub(1).ok_or(UnmatchedParentheses)?
                }
                _ => (),
            }
        }
        if par_count > 0 {
            return Err(UnmatchedParentheses);
        }

        Ok(Self { code })
    }
}

impl IntoIterator for Program {
    type Item = Instruction;

    type IntoIter = vec::IntoIter<Instruction>;

    fn into_iter(self) -> Self::IntoIter {
        self.code.into_vec().into_iter()
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Index<usize> for Program {
    type Output = Instruction;

    fn index(&self, index: usize) -> &Self::Output {
        self.code.index(index)
    }
}
impl IndexMut<usize> for Program {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.code.index_mut(index)
    }
}

impl From<Program> for Vec<Instruction> {
    fn from(value: Program) -> Self {
        value.code.into_vec()
    }
}
impl TryFrom<Vec<Instruction>> for Program {
    type Error = UnmatchedParentheses;
    fn try_from(value: Vec<Instruction>) -> Result<Self, Self::Error> {
        Self::from_instrs(value)
    }
}
impl From<Program> for Box<[Instruction]> {
    fn from(value: Program) -> Self {
        value.code
    }
}
impl TryFrom<Box<[Instruction]>> for Program {
    type Error = UnmatchedParentheses;
    fn try_from(value: Box<[Instruction]>) -> Result<Self, Self::Error> {
        Self::from_instrs(value.into_vec())
    }
}

impl FromStr for Program {
    type Err = UnmatchedParentheses;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_instrs(s.chars().filter_map(|ch| Instruction::try_from(ch).ok()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
#[error("The brainfuck program has unmatched parentheses")]
pub struct UnmatchedParentheses;

#[cfg(test)]
mod tests {
    use super::Program;

    #[test]
    fn empty() {
        let _: Program = "".parse().unwrap();
    }
    #[test]
    fn parentheses() {
        let _: Program = "[]".parse().unwrap();
    }
}
