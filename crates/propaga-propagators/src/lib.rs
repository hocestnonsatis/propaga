//! Built-in propagators for Propaga.
//!
//! Constraint propagators implement bound consistency, GAC, and scheduling-specific
//! inference for equality, linear, ordering, reified, all-different, GCC, table,
//! element, cumulative, and disjunctive constraints.

#[cfg(test)]
mod test_support;

mod all_different;
mod circuit;
mod clause;
mod cumulative;
mod diffn;
mod disjunctive;
mod dominance_cut;
mod element;
mod equality;
mod float_element;
mod float_eq;
mod float_le;
mod float_lin_scalar;
mod float_ne;
mod float_ops;
mod float_times;
mod forbidden_assignment;
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
mod set_card_eq;
mod set_diff;
mod set_eq;
mod set_in;
mod set_intersect;
mod set_reif;
mod set_subset;
mod set_symdiff;
mod set_union;
mod table;

pub use all_different::AllDifferentPropagator;
pub use circuit::CircuitPropagator;
pub use clause::ClausePropagator;
pub use cumulative::CumulativePropagator;
pub use diffn::{DiffnPropagator, RectangleSpec};
pub use disjunctive::{DisjunctivePropagator, DisjunctiveTask};
pub use dominance_cut::{
    DominanceCutDirection, DominanceCutPropagator, DominanceCutTarget, IntDominanceCutPropagator,
};
pub use element::ElementPropagator;
pub use equality::EqualityPropagator;
pub use float_element::FloatElementPropagator;
pub use float_eq::FloatEqPropagator;
pub use float_le::FloatLePropagator;
pub use float_lin_scalar::{
    FloatLinearEqPropagator, FloatLinearGePropagator, FloatLinearLePropagator,
    FloatLinearNePropagator, ReifiedFloatLinearEqPropagator, ReifiedFloatLinearGePropagator,
    ReifiedFloatLinearLePropagator,
};
pub use float_ne::FloatNePropagator;
pub use float_ops::{
    FloatBinaryOp, FloatBinaryPropagator, FloatEqReifPropagator, FloatLeReifPropagator,
    FloatUnaryOp, FloatUnaryPropagator, Int2FloatPropagator,
};
pub use float_times::FloatTimesPropagator;
pub use forbidden_assignment::{
    EncodedForbiddenFloat, ForbiddenAssignmentPropagator, ForbiddenValue, encode_forbidden_float,
};
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
pub use set_card_eq::SetCardEqPropagator;
pub use set_diff::SetDiffPropagator;
pub use set_eq::SetEqPropagator;
pub use set_in::SetInPropagator;
pub use set_intersect::SetIntersectPropagator;
pub use set_reif::{SetEqReifPropagator, SetInReifPropagator, SetSubsetReifPropagator};
pub use set_subset::SetSubsetPropagator;
pub use set_symdiff::SetSymDiffPropagator;
pub use set_union::SetUnionPropagator;
pub use table::TablePropagator;
