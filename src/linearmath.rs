use std::{
    collections::{hash_map::Entry, HashMap},
    convert::identity,
    iter::{once, once_with, repeat, repeat_with},
    mem,
    num::{NonZeroU8, Wrapping},
    ops::{Add, AddAssign, Mul},
};

use either::Either::{Left, Right};

use crate::IRState;

/// An infinite matrix
#[derive(Debug, Clone, PartialEq, Eq)]
struct BareMatrix(HashMap<(isize, isize), NonZeroU8>);
impl BareMatrix {
    fn zero() -> Self {
        Self(HashMap::new())
    }
    fn single_entry(i: isize, j: isize, v: u8) -> Self {
        match NonZeroU8::new(v) {
            Some(v) => Self(HashMap::from([((i, j), v)])),
            None => Self::zero(),
        }
    }

    fn sum(&self, rhs: &Self) -> Self {
        if self.is_zero() {
            return rhs.clone();
        }
        if rhs.is_zero() {
            return self.clone();
        }

        let mut res = HashMap::new();
        for (key, v) in self.0.iter().chain(rhs.0.iter()) {
            let v = res
                .remove(key)
                .map_or(0, NonZeroU8::get)
                .wrapping_add(v.get());
            if let Some(v) = NonZeroU8::new(v) {
                res.insert(*key, v);
            }
        }
        Self(res)
    }

    fn chain(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() {
            return Self::zero();
        }
        let mut res = HashMap::new();
        for ((i, j1), a) in self.0.iter() {
            for ((j2, k), b) in rhs.0.iter() {
                if j1 == j2 {
                    let v = res
                        .remove(&(*i, *k))
                        .map_or(0, NonZeroU8::get)
                        .wrapping_add(u8::wrapping_mul(a.get(), b.get()));
                    if let Some(v) = NonZeroU8::new(v) {
                        res.insert((*i, *k), v);
                    }
                }
            }
        }
        Self(res)
    }

    fn apply(&self, rhs: &Vector) -> Vector {
        if self.is_zero() || rhs.is_zero() {
            return Vector::zero();
        }
        let mut res = HashMap::new();
        for ((i, j), a) in self.0.iter() {
            if let Some(b) = rhs.get(j) {
                let v = a.get().wrapping_mul(b.get());
                if let Some(v) = NonZeroU8::new(v) {
                    res.insert(*i, v);
                }
            }
        }
        Vector(res)
    }
    fn right_apply(&self, rhs: &Vector) -> Vector {
        if self.is_zero() || rhs.is_zero() {
            return Vector::zero();
        }
        let mut res = HashMap::new();
        for ((i, j), a) in self.0.iter() {
            if let Some(b) = rhs.get(i) {
                let v = a.get().wrapping_mul(b.get());
                if let Some(v) = NonZeroU8::new(v) {
                    res.insert(*j, v);
                }
            }
        }
        Vector(res)
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// Return all the non null rows
    fn rows(&self) -> impl Iterator<Item = (isize, impl Iterator<Item = (isize, NonZeroU8)>)> {
        let mut table = HashMap::new();
        for ((i, j), v) in self.0.iter() {
            table.entry(*i).or_insert_with(HashMap::new).insert(*j, *v);
        }
        table.into_iter().map(|(i, r)| (i, r.into_iter()))
    }

    fn from_rows(mat: impl Iterator<Item = ((isize, isize), u8)>) -> Self {
        Self(
            mat.filter_map(|(i, v)| NonZeroU8::new(v).map(|v| (i, v)))
                .collect(),
        )
    }
}

/// An infinite matrix, saved as the difference from the identity
/// This permit to encode identity in finite space
#[derive(Debug, Clone, PartialEq, Eq)]
struct IdPlusMatrix(BareMatrix);
impl IdPlusMatrix {
    /// Identity matrix
    fn identity() -> Self {
        Self(BareMatrix::zero())
    }
    /// Matrix that set a single value to one
    fn null_one(pos: isize) -> Self {
        Self(BareMatrix::single_entry(pos, pos, 255))
    }

    fn chain(&self, rhs: &Self) -> Self {
        let prod = BareMatrix::chain(&self.0, &rhs.0);
        Self(self.0.chain(&rhs.0.chain(&prod)))
    }

    fn apply(&self, rhs: &Vector) -> Vector {
        rhs.sum(&self.0.apply(rhs))
    }
    fn right_apply(&self, rhs: &Vector) -> Vector {
        rhs.sum(&self.0.right_apply(rhs))
    }

    fn is_identity(&self) -> bool {
        self.0.is_zero()
    }

    /// Return all the non trivial rows
    fn rows(&self) -> impl Iterator<Item = (isize, impl Iterator<Item = (isize, NonZeroU8)>)> {
        self.0.rows().into_iter().map(|(i, row)| {
            // Adding the identity element
            (
                i,
                row.map(Some)
                    .chain(once(None))
                    .scan(false, move |identity_found, v| {
                        if let Some((j, v)) = v {
                            if !*identity_found && i == j {
                                *identity_found = true;
                                if let Some(v) = NonZeroU8::new(v.get().wrapping_add(1)) {
                                    // pass new value
                                    Some(Some((j, v)))
                                } else {
                                    // the term was cancelled
                                    Some(None)
                                }
                            } else {
                                // term is not modified
                                Some(Some((j, v)))
                            }
                        } else {
                            // iterator ended
                            if !*identity_found {
                                // the identity was not found
                                Some(Some((i, NonZeroU8::new(1).unwrap())))
                            } else {
                                // identity was already parsed
                                None
                            }
                        }
                    })
                    .filter_map(identity),
            )
        })
    }

    fn from_rows(mat: impl Iterator<Item = ((isize, isize), u8)>) -> Self {
        Self(BareMatrix::from_rows(mat.map(|(i, v)| {
            (i, v.wrapping_sub(if i.0 == i.1 { 1 } else { 0 }))
        })))
    }
}

/// An infinite vector
#[derive(Debug, Clone, PartialEq, Eq)]
struct Vector(HashMap<isize, NonZeroU8>);
impl Vector {
    fn zero() -> Self {
        Self(HashMap::new())
    }

    fn single_entry(pos: isize, v: u8) -> Vector {
        match NonZeroU8::new(v) {
            Some(v) => Self(HashMap::from([(pos, v)])),
            None => Self::zero(),
        }
    }

    fn get(&self, k: &isize) -> Option<NonZeroU8> {
        self.0.get(k).map(|v| *v)
    }
    fn get_u8(&self, k: &isize) -> u8 {
        self.0.get(k).map(|v| *v).map_or(0, NonZeroU8::get)
    }

    fn sum(&self, rhs: &Vector) -> Vector {
        if self.is_zero() {
            return rhs.clone();
        }
        if rhs.is_zero() {
            return self.clone();
        }
        let mut res = HashMap::new();
        for (key, v) in self.0.iter().chain(rhs.0.iter()) {
            let v = res
                .remove(key)
                .map_or(0, NonZeroU8::get)
                .wrapping_add(v.get());
            if let Some(v) = NonZeroU8::new(v) {
                res.insert(*key, v);
            }
        }
        Self(res)
    }

    fn mul(&self, n: u8) -> Vector {
        if n == 0 {
            return Vector::zero();
        }

        Self(
            self.0
                .iter()
                .filter_map(|(k, v)| NonZeroU8::new(v.get().wrapping_mul(n)).map(|v| (*k, v)))
                .collect(),
        )
    }
    fn dot(&self, rhs: &Vector) -> u8 {
        if self.is_zero() || rhs.is_zero() {
            return 0;
        }
        self.0
            .iter()
            .filter_map(|(k, a)| rhs.get(k).map(|b| u8::wrapping_mul(a.get(), b.get())))
            .reduce(u8::wrapping_add)
            .unwrap_or(0)
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn elements(&self) -> impl Iterator<Item = (isize, NonZeroU8)> + '_ {
        self.0.iter().map(|(a, b)| (*a, *b))
    }

    fn from_elements(con: impl Iterator<Item = (isize, u8)>) -> Vector {
        Self(
            con.filter_map(|(i, v)| NonZeroU8::new(v).map(|v| (i, v)))
                .collect(),
        )
    }
}

/// A combined operation of a matrix and a constant
#[derive(Debug, Clone, PartialEq, Eq)]
struct AffineTransform {
    m: IdPlusMatrix,
    c: Vector,
}
impl AffineTransform {
    fn identity() -> Self {
        Self {
            m: IdPlusMatrix::identity(),
            c: Vector::zero(),
        }
    }
    fn set_one(pos: isize, v: u8) -> Self {
        Self {
            m: IdPlusMatrix::null_one(pos),
            c: Vector::single_entry(pos, v),
        }
    }

    fn chain(&self, rhs: &Self) -> Self {
        Self {
            m: self.m.chain(&rhs.m),
            c: self.m.apply(&rhs.c).sum(&self.c),
        }
    }

    fn apply(&self, rhs: &Vector) -> Vector {
        self.m.apply(&rhs).sum(&self.c)
    }

    fn is_identity(&self) -> bool {
        self.m.is_identity() && self.c.is_zero()
    }
    fn is_linear(&self) -> bool {
        self.c.is_zero()
    }
    fn is_translation(&self) -> bool {
        self.m.is_identity()
    }
}
impl From<IdPlusMatrix> for AffineTransform {
    fn from(value: IdPlusMatrix) -> Self {
        Self {
            m: value,
            c: Vector::zero(),
        }
    }
}

/// A combined operation of multiplication by a vector and summing a constant
#[derive(Debug, Clone, PartialEq, Eq)]
struct AffineTransformTo1D {
    m: Vector,
    c: u8,
}
impl AffineTransformTo1D {
    fn get_one(pos: isize) -> Self {
        Self {
            m: Vector::single_entry(pos, 1),
            c: 0,
        }
    }
    fn constant(c: u8) -> Self {
        Self {
            m: Vector::zero(),
            c,
        }
    }

    fn chain(&self, rhs: &AffineTransform) -> Self {
        Self {
            m: rhs.m.right_apply(&self.m),
            c: self.m.dot(&rhs.c).wrapping_add(self.c),
        }
    }

    fn apply(&self, rhs: &Vector) -> u8 {
        self.m.dot(&rhs).wrapping_add(self.c)
    }

    fn is_linear(&self) -> bool {
        self.c == 0
    }
    fn is_constant(&self) -> bool {
        self.m.is_zero()
    }
}

/// Add from a 1D space
#[derive(Debug, Clone, PartialEq, Eq)]
struct AddFrom1D {
    m: Vector,
}
impl AddFrom1D {
    fn to_one(pos: isize) -> Self {
        Self {
            m: Vector::single_entry(pos, 1),
        }
    }

    /// Find the trasformation to apply after the affine transform to get the same result
    fn postpone(&self, rhs: &AffineTransform) -> Self {
        Self {
            m: rhs.m.apply(&self.m),
        }
    }

    fn apply(&self, rhs: &Vector, n: u8) -> Vector {
        rhs.sum(&self.m.mul(n))
    }

    fn is_adsorbing(&self) -> bool {
        self.m.is_zero()
    }
}

/// An affine transform, in compacted form to be speedly applied to a memory, or code generated

#[derive(Debug, Clone, PartialEq, Eq)]
struct AffineMemoryTransform(Box<[AffineMemoryTransformRow]>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct AffineMemoryTransformRow {
    dest: isize,
    c: Wrapping<u8>,
    addends: Box<[(isize, Wrapping<u8>)]>,
}

impl AffineMemoryTransform {
    fn apply(&self, mem: &mut IRState) -> Result<(), super::MemoryPointerUnderflowError> {
        self.0
            .iter()
            .map(|AffineMemoryTransformRow { dest, c, addends }| {
                (
                    dest,
                    addends.iter().map(|(src, mult)| {
                        mem[usize::try_from(src + mem.mp)?]
                    }),
                )
            })
    }
}

impl From<AffineTransform> for AffineMemoryTransform {
    fn from(value: AffineTransform) -> Self {
        let mut rows: Box<_> = value
            .m
            .rows()
            .map(Some)
            .chain(once(None))
            .scan(
                value.c.elements().collect::<HashMap<_, _>>(),
                |cons, value| {
                    if let Some((dest, row)) = value {
                        let mut addends: Box<_> =
                            row.map(|(j, v)| (j, Wrapping(v.get()))).collect();
                        // sort to ensure equivalence
                        addends.sort_unstable_by_key(|v| v.0);

                        Some(Left(once(AffineMemoryTransformRow {
                            dest,
                            c: Wrapping(cons.remove(&dest).map_or(0, NonZeroU8::get)),
                            addends,
                        })))
                    } else {
                        Some(Right(mem::take(cons).into_iter().map(|(dest, c)| {
                            // drain the remaining consts
                            AffineMemoryTransformRow {
                                dest,
                                c: Wrapping(c.get()),
                                addends: Default::default(),
                            }
                        })))
                    }
                },
            )
            .flatten()
            .collect();

        // Sorting to make comparation order-indipendent
        rows.sort_unstable_by_key(|v| v.dest);

        Self(rows)
    }
}
impl From<AffineMemoryTransform> for AffineTransform {
    fn from(value: AffineMemoryTransform) -> Self {
        let (mat, con) = value.0.into_vec().into_iter().fold(
            (vec![], vec![]),
            |(mut mat, mut con),
             AffineMemoryTransformRow {
                 dest: i,
                 c,
                 addends,
             }| {
                mat.extend(addends.into_vec().into_iter().map(|(j, v)| ((i, j), v.0)));
                con.push((i, c.0));
                (mat, con)
            },
        );
        AffineTransform {
            m: IdPlusMatrix::from_rows(mat.into_iter()),
            c: Vector::from_elements(con.into_iter()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod vector {
        use super::Vector;
        use std::num::NonZeroU8;

        const INDICES: [isize; 7] = [-100, -50, -1, 0, 1, 50, 100];

        #[test]
        fn zero_is_all_zeros() {
            let vec = Vector::zero();
            for idx in INDICES {
                assert_eq!(vec.get(&idx), None);
                assert_eq!(vec.get_u8(&idx), 0);
            }
        }

        #[test]
        fn single_entry() {
            for i in INDICES {
                let vec = Vector::single_entry(i, 1);
                for idx in INDICES {
                    if idx != i {
                        assert_eq!(vec.get(&idx), None);
                        assert_eq!(vec.get_u8(&idx), 0);
                    } else {
                        assert_eq!(vec.get(&idx), NonZeroU8::new(1));
                        assert_eq!(vec.get_u8(&idx), 1);
                    }
                }
            }
        }

        #[test]
        fn sum_two_entries() {
            for i in INDICES {
                for j in INDICES {
                    let vec = Vector::sum(&Vector::single_entry(i, 1), &Vector::single_entry(j, 1));
                    for idx in INDICES {
                        if idx != i && idx != j {
                            assert_eq!(vec.get(&idx), None);
                            assert_eq!(vec.get_u8(&idx), 0);
                        } else if idx == i && idx == j {
                            assert_eq!(vec.get(&idx), NonZeroU8::new(2));
                            assert_eq!(vec.get_u8(&idx), 2);
                        } else {
                            assert_eq!(vec.get(&idx), NonZeroU8::new(1));
                            assert_eq!(vec.get_u8(&idx), 1);
                        }
                    }
                }
            }
        }

        #[test]
        fn sum_two_opposite_entries() {
            for i in INDICES {
                for j in INDICES {
                    let vec =
                        Vector::sum(&Vector::single_entry(i, 1), &Vector::single_entry(j, 255));
                    for idx in INDICES {
                        if (idx != i && idx != j) || (idx == i && idx == j) {
                            assert_eq!(vec.get(&idx), None);
                            assert_eq!(vec.get_u8(&idx), 0);
                        } else if idx == i {
                            assert_eq!(vec.get(&idx), NonZeroU8::new(1));
                            assert_eq!(vec.get_u8(&idx), 1);
                        } else {
                            assert_eq!(vec.get(&idx), NonZeroU8::new(255));
                            assert_eq!(vec.get_u8(&idx), 255);
                        }
                    }
                }
            }
        }
    }
    mod bare_matrix {
        use super::{BareMatrix, Vector};
    }
}
