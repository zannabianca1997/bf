//! Raw brainfuck

use std::{
    fmt::{Display, Write},
    slice,
    str::FromStr,
    vec,
};

#[derive(Debug, Clone)]
pub struct Program(Box<[Instruction]>);

impl FromStr for Program {
    type Err = !;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Program(
            s.chars()
                // Convert into raw instruction and filter out failing ones
                .filter_map(|ch| ch.try_into().ok())
                .collect(),
        ))
    }
}
impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instr in self {
            f.write_char((*instr).into())?
        }
        Ok(())
    }
}

impl IntoIterator for Program {
    type Item = Instruction;

    type IntoIter = vec::IntoIter<Instruction>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}
impl<'a> IntoIterator for &'a Program {
    type Item = &'a Instruction;

    type IntoIter = slice::Iter<'a, Instruction>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Left,
    Right,
    OpenLoop,
    CloseLoop,
    Input,
    Output,
    Increase,
    Decrease,
}
impl TryFrom<char> for Instruction {
    type Error = char;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use Instruction::*;
        match value {
            '+' => Ok(Increase),
            '-' => Ok(Decrease),
            '.' => Ok(Output),
            ',' => Ok(Input),
            '>' => Ok(Right),
            '<' => Ok(Left),
            '[' => Ok(OpenLoop),
            ']' => Ok(CloseLoop),
            ch => Err(ch), // ignore all other chars
        }
    }
}
impl From<Instruction> for char {
    fn from(value: Instruction) -> Self {
        use Instruction::*;
        match value {
            Left => '<',
            Right => '>',
            OpenLoop => '[',
            CloseLoop => ']',
            Input => ',',
            Output => '.',
            Increase => '+',
            Decrease => '-',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Instruction;

    #[test]
    fn round_trips_single_char() {
        for ch in ".,-+><[]".chars() {
            let instr = Instruction::try_from(ch)
                .expect("All the chars in the string should be valid BF instructions");
            assert_eq!(ch, char::from(instr))
        }
    }
}
