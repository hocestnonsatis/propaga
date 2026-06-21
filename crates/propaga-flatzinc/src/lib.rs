//! FlatZinc subset parser and compiler for Propaga.

mod compile;
mod error;
mod parse;

pub use compile::{CompiledInstance, ObjectiveSpec, compile};
pub use error::FlatZincError;
pub use parse::{FlatZincProgram, OutputDirective, OutputSegment, parse};
