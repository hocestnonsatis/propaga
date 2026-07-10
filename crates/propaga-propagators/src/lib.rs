//! Built-in propagators for Propaga.
//!
//! Constraint propagators implement bound consistency, GAC, and scheduling-specific
//! inference for equality, linear, ordering, reified, all-different, GCC, table,
//! element, cumulative, and disjunctive constraints.

mod all_different;
mod circuit;
mod clause;
mod cumulative;
mod diffn;
mod disjunctive;
mod element;
mod equality;
mod float_eq;
mod float_le;
mod gcc;
mod inverse;
mod less_equal;
mod less_than;
mod linear_eq;
mod linear_scalar;
mod matching;
mod nogood;
mod not_equal_offset;
mod regular;
mod reified;
mod scheduling;
mod set_card;
mod set_subset;
mod table;

pub use all_different::AllDifferentPropagator;
pub use circuit::CircuitPropagator;
pub use clause::ClausePropagator;
pub use cumulative::CumulativePropagator;
pub use diffn::{DiffnPropagator, RectangleSpec};
pub use disjunctive::{DisjunctivePropagator, DisjunctiveTask};
pub use element::ElementPropagator;
pub use equality::EqualityPropagator;
pub use float_eq::FloatEqPropagator;
pub use float_le::FloatLePropagator;
pub use gcc::{CardinalityBound, GlobalCardinalityPropagator};
pub use inverse::InversePropagator;
pub use less_equal::LessEqualPropagator;
pub use less_than::LessThanPropagator;
pub use linear_eq::LinearEqPropagator;
pub use linear_scalar::{
    LinearScalarGePropagator, LinearScalarLePropagator, ReifiedScalarEqPropagator,
    ReifiedScalarGePropagator, ReifiedScalarLePropagator,
};
pub use nogood::NogoodPropagator;
pub use not_equal_offset::NotEqualOffsetPropagator;
pub use regular::RegularPropagator;
pub use reified::{
    ReifiedEqualityPropagator, ReifiedLessEqualPropagator, ReifiedLessThanPropagator,
    ReifiedNotEqualPropagator,
};
pub use scheduling::TaskSpec;
pub use set_card::SetCardPropagator;
pub use set_subset::SetSubsetPropagator;
pub use table::TablePropagator;
