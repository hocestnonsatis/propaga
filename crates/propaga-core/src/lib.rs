//! Core types and traits for the Propaga constraint solver.
//!
//! This crate defines variables, domains, propagators, explanations, nogoods,
//! and the propagation context used by the engine and search layers.

mod context;
mod domain;
mod error;
pub mod id;
mod nogood;
mod propagator;
mod reason;
mod status;

pub use context::PropagationContext;
pub use domain::{Domain, DomainView};
pub use error::PropagaError;
pub use id::{PropagatorId, PropagatorKey, VariableId, VariableKey};
pub use nogood::{Nogood, NogoodLiteral};
pub use propagator::Propagator;
pub use reason::{BoundKind, ChangeReason, Explanation};
pub use status::PropagationStatus;
