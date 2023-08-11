//! Memory of a Brainfuck engine

use std::{
    hash::Hash,
    iter::{repeat, zip},
};

#[derive(Debug, Clone)]
pub struct Memory {
    mem: Vec<u8>,
}

impl Memory {
    pub fn get(&self, pos: usize) -> &u8 {
        self.mem.get(pos).unwrap_or(&0)
    }
    pub fn get_mut(&mut self, pos: usize) -> &mut u8 {
        self.mem
            .extend(repeat(0).take((pos + 1).saturating_sub(self.mem.len())));
        &mut self.mem[pos]
    }
    pub fn set(&mut self, pos: usize, value: u8) {
        if pos < self.mem.len() {
            self.mem[pos] = value
        } else if value != 0 {
            *self.get_mut(pos) = value;
        } else {
            // Nothing to do. The memory over the limit is taken to be 0
        }
    }
    pub fn filled_len(&self) -> usize {
        let mut len = self.mem.len();
        while len > 0 && self.mem[len - 1] == 0 {
            len -= 1
        }
        len
    }
    pub fn shrink_to_fit(&mut self) {
        self.mem.truncate(self.filled_len());
        self.mem.shrink_to_fit();
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.mem[..self.filled_len()]
    }

    pub fn new() -> Memory {
        Memory { mem: vec![] }
    }
}

impl PartialEq for Memory {
    fn eq(&self, other: &Self) -> bool {
        let [s1, s2] = if self.mem.len() >= other.mem.len() {
            [&self.mem, &other.mem]
        } else {
            [&other.mem, &self.mem]
        }
        .map(Vec::as_slice);
        let (s1, diff) = s1.split_at(s2.len());
        zip(s1, s2).all(|(a, b)| a == b) && diff.into_iter().all(|x| *x == 0)
    }
}
impl Eq for Memory {}

impl PartialOrd for Memory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Memory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let common = usize::min(self.mem.len(), other.mem.len());
        let (sc, sd) = self.mem.split_at(common);
        let (oc, od) = other.mem.split_at(common);
        match sc.cmp(oc) {
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Equal => {
                if sd.iter().any(|x| *x != 0) {
                    std::cmp::Ordering::Greater
                } else if od.iter().any(|x| *x != 0) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        }
    }
}
impl Hash for Memory {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state)
    }
}
