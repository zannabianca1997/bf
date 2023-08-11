//! Various ir optimizations

use std::{
    mem,
    num::{NonZeroIsize, NonZeroU8},
};

use either::Either::{self, Left, Right};

use super::{Add, Loop, Node, Shift};

const OPTIMIZATIONS_1: &[fn([Node; 1]) -> Either<[Node; 1], Vec<Node>>] = &[recurse, remove_noops];
const OPTIMIZATIONS_2: &[fn([Node; 2]) -> Either<[Node; 2], Vec<Node>>] = &[collate];

fn recurse(node: [Node; 1]) -> Either<[Node; 1], Vec<Node>> {
    match node {
        [Node::Loop(Loop { mut body })] => {
            if body.optimize() {
                Right(vec![Node::Loop(Loop { body })])
            } else {
                Left([Node::Loop(Loop { body })])
            }
        }
        node => Left(node),
    }
}
fn remove_noops(node: [Node; 1]) -> Either<[Node; 1], Vec<Node>> {
    match node {
        [Node::Noop] => Right(vec![]),
        node => Left(node),
    }
}

fn collate(nodes: [Node; 2]) -> Either<[Node; 2], Vec<Node>> {
    match nodes {
        [Node::Shift(Shift { amount: a1 }), Node::Shift(Shift { amount: a2 })] => {
            Right(match NonZeroIsize::new(a1.get() + a2.get()) {
                Some(amount) => vec![Node::Shift(Shift { amount })],
                None => vec![],
            })
        }
        [Node::Add(Add { amount: a1 }), Node::Add(Add { amount: a2 })] => {
            Right(match NonZeroU8::new(u8::wrapping_add(a1.get(), a2.get())) {
                Some(amount) => vec![Node::Add(Add { amount })],
                None => vec![],
            })
        }
        nodes => Left(nodes),
    }
}

pub(super) fn optimize(nodes: Vec<Node>, changed: &mut bool) -> Vec<Node> {
    let nodes = optimize_n(nodes, changed, OPTIMIZATIONS_1);
    let nodes = optimize_n(nodes, changed, OPTIMIZATIONS_2);
    nodes
}
fn optimize_n<const N: usize>(
    mut nodes: Vec<Node>,
    changed: &mut bool,
    optimizations: &'static [fn([Node; N]) -> Either<[Node; N], Vec<Node>>],
) -> Vec<Node> {
    if nodes.len() < N {
        return nodes;
    }
    for i in 0..N {
        let (prefix, postfix) = nodes.split_at_mut(i);
        let (chunks, postfix) = postfix.as_chunks_mut::<N>();
        if chunks.is_empty() {
            continue;
        }

        let mut optimizing: Vec<_> = chunks
            .into_iter()
            .map(|ch| Left(mem::replace(ch, [(); N].map(|_| Default::default()))))
            .collect();
        for opt in optimizations {
            optimizing = optimizing
                .into_iter()
                .map(|ch| match ch {
                    Left(node) => opt(node),
                    Right(nodes) => Right(nodes),
                })
                .collect()
        }

        // recollecting
        let mut optimized: Vec<_> = prefix.into_iter().map(mem::take).collect();
        for ch in optimizing {
            match ch {
                Left(nodes) => optimized.extend(nodes.into_iter()),
                Right(nodes) => {
                    *changed = true;
                    optimized.extend(nodes.into_iter())
                }
            }
        }
        optimized.extend(postfix.into_iter().map(mem::take));
        nodes = optimized
    }

    nodes
}
