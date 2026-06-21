use crate::VariableId;
use thiserror::Error;

/// Errors returned by the propagation engine.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PropagaError {
    /// A variable domain became empty during propagation.
    #[error("domain of variable {0:?} is empty")]
    DomainEmpty(VariableId),

    /// A variable handle does not exist in the engine.
    #[error("unknown variable {0:?}")]
    UnknownVariable(VariableId),

    /// A propagator handle does not exist in the engine.
    #[error("unknown propagator")]
    UnknownPropagator,
}
