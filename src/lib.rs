#![feature(never_type)]

pub mod ir;
mod linear;
mod optimize;
pub mod raw;
pub mod runtime;

pub use ir::Program as IRProgram;
pub use raw::Program as RawProgram;
