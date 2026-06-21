//! FlatZinc subset parser and compiler for Propaga.

mod compile;
mod error;
mod parse;

pub use compile::{compile, CompiledInstance, ObjectiveSpec};
pub use error::FlatZincError;
pub use parse::{parse, FlatZincProgram, OutputDirective, OutputSegment};
