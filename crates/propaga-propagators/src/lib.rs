//! Built-in propagators for Propaga.
//!
//! Constraint propagators implement bound consistency, GAC, and scheduling-specific
//! inference for equality, linear, ordering, reified, all-different, GCC, table,
//! element, cumulative, and disjunctive constraints.

mod all_different;
mod cumulative;
mod disjunctive;
mod element;
mod equality;
mod gcc;
mod less_equal;
mod less_than;
mod linear_eq;
mod linear_scalar;
mod matching;
mod nogood;
mod not_equal_offset;
mod reified;
mod scheduling;
mod table;

pub use all_different::AllDifferentPropagator;
pub use cumulative::CumulativePropagator;
pub use disjunctive::{DisjunctivePropagator, DisjunctiveTask};
pub use element::ElementPropagator;
pub use equality::EqualityPropagator;
pub use gcc::{CardinalityBound, GlobalCardinalityPropagator};
pub use less_equal::LessEqualPropagator;
pub use less_than::LessThanPropagator;
pub use linear_eq::LinearEqPropagator;
pub use linear_scalar::{
    LinearScalarGePropagator, LinearScalarLePropagator, ReifiedScalarEqPropagator,
    ReifiedScalarGePropagator, ReifiedScalarLePropagator,
};
pub use nogood::NogoodPropagator;
pub use not_equal_offset::NotEqualOffsetPropagator;
pub use reified::{
    ReifiedEqualityPropagator, ReifiedLessEqualPropagator, ReifiedLessThanPropagator,
    ReifiedNotEqualPropagator,
};
pub use scheduling::TaskSpec;
pub use table::TablePropagator;
