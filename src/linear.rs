//! Linear algebra to agglomerate memory operations

use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    num::NonZeroU8,
};

/// An affine transformation from memory to a single value
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AffineToScalar {
    /// Memory to read, coupled with multipliers
    addends: BTreeMap<isize, NonZeroU8>,
    /// Bias to add at the end
    bias: u8,
}

impl AffineToScalar {
    /// Extract a single memory value
    pub(crate) fn extract(pos: isize) -> Self {
        Self {
            addends: BTreeMap::from([(pos, NonZeroU8::new(1).unwrap())]),
            bias: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BiasPropToScalar {
    coeffs: BTreeMap<isize, NonZeroU8>,
}
impl BiasPropToScalar {
    /// Add the scalar to a single memory location
    pub(crate) fn to_single(pos: isize) -> BiasPropToScalar {
        Self {
            coeffs: BTreeMap::from([(pos, NonZeroU8::new(1).unwrap())]),
        }
    }
}

/// Affine memory transform
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Affine {
    /// Matrix to apply first to the memory
    ///
    /// Stored as difference from identity
    matrix: BTreeMap<(isize, isize), NonZeroU8>,

    /// Bias to add at the end
    bias: BTreeMap<isize, NonZeroU8>,
}

impl Affine {
    /// A transformation that set a value
    pub(crate) fn set(pos: isize, value: u8) -> Self {
        let matrix = BTreeMap::from([((pos, pos), NonZeroU8::new(u8::MAX).unwrap())]);
        if let Some(value) = NonZeroU8::new(value) {
            Self {
                matrix,
                bias: BTreeMap::from([(pos, value)]),
            }
        } else {
            Self {
                matrix,
                bias: BTreeMap::new(),
            }
        }
    }
    /// A transformation that add a constant to a memory position
    pub(crate) fn add(pos: isize, value: u8) -> Self {
        let matrix = BTreeMap::new();
        if let Some(value) = NonZeroU8::new(value) {
            Self {
                matrix,
                bias: BTreeMap::from([(pos, value)]),
            }
        } else {
            Self {
                matrix,
                bias: BTreeMap::new(),
            }
        }
    }

    /// Combine two memory transformations
    ///
    /// `self.combine(other).apply(&mut mem);` is equivalent to `self.apply(&mut mem); other.apply(&mem);`
    pub(crate) fn combine(self, other: Self) -> Self {
        // calculate bias
        let bias = {
            let mut bias = other.bias;
            for (j2, m) in self.bias {
                for ((i, j1), n) in other.matrix.iter() {
                    if *j1 == j2 {
                        if let Some(mn) = NonZeroU8::new(m.get().wrapping_mul(n.get())) {
                            match bias.entry(*i) {
                                Vacant(ve) => {
                                    ve.insert(mn);
                                }
                                Occupied(mut oe) => {
                                    // Calculate if entry is still present, even after adding
                                    if let Some(entry) =
                                        NonZeroU8::new(oe.get().get().wrapping_add(mn.get()))
                                    {
                                        oe.insert(entry);
                                    } else {
                                        oe.remove();
                                    }
                                }
                            }
                        }
                    }
                }
                // adding bias itself
                match bias.entry(j2) {
                    Vacant(ve) => {
                        ve.insert(m);
                    }
                    Occupied(mut oe) => {
                        // Calculate if entry is still present, even after adding
                        if let Some(entry) = NonZeroU8::new(oe.get().get().wrapping_add(m.get())) {
                            oe.insert(entry);
                        } else {
                            oe.remove();
                        }
                    }
                }
            }
            bias
        };

        // calculate combined matrix
        let matrix = {
            let mut matrix = BTreeMap::new();
            for ((i, j1), n) in self.matrix.iter() {
                for ((j2, k), m) in other.matrix.iter() {
                    if j1 == j2 {
                        if let Some(mn) = NonZeroU8::new(m.get().wrapping_mul(n.get())) {
                            match matrix.entry((*i, *k)) {
                                Vacant(ve) => {
                                    ve.insert(mn);
                                }
                                Occupied(mut oe) => {
                                    // Calculate if entry is still present, even after adding
                                    if let Some(entry) =
                                        NonZeroU8::new(oe.get().get().wrapping_add(mn.get()))
                                    {
                                        oe.insert(entry);
                                    } else {
                                        oe.remove();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for ((i, k), n) in self.matrix.into_iter().chain(other.matrix) {
                match matrix.entry((i, k)) {
                    Vacant(ve) => {
                        ve.insert(n);
                    }
                    Occupied(mut oe) => {
                        // Calculate if entry is still present, even after adding
                        if let Some(entry) = NonZeroU8::new(oe.get().get().wrapping_add(n.get())) {
                            oe.insert(entry);
                        } else {
                            oe.remove();
                        }
                    }
                }
            }
            matrix
        };

        Self { matrix, bias }
    }
}

#[cfg(test)]
mod test_affine {
    use super::Affine;

    #[test]
    fn double_set_comm() {
        let set1 = Affine::set(-1, 5);
        let set2 = Affine::set(-3, 42);
        assert_eq!(
            Affine::combine(set1.clone(), set2.clone()),
            Affine::combine(set2, set1)
        )
    }
    #[test]
    fn double_set() {
        let set1 = Affine::set(-1, 5);
        let set2 = Affine::set(-1, 42);
        assert_eq!(Affine::combine(set1, set2.clone()), set2)
    }
    #[test]
    fn double_add_comm() {
        let add1 = Affine::add(-1, 5);
        let add2 = Affine::add(-3, 42);
        assert_eq!(
            Affine::combine(add1.clone(), add2.clone()),
            Affine::combine(add2, add1)
        )
    }
    #[test]
    fn double_add() {
        let add1 = Affine::add(-1, 5);
        let add2 = Affine::add(-1, 42);
        assert_eq!(Affine::combine(add1, add2), Affine::add(-1, 5 + 42))
    }
}
