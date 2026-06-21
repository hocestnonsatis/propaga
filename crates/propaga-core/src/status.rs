/// Result of a single propagator invocation or propagation round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropagationStatus {
    /// Propagation ran but no domain changed.
    OkNoChange,
    /// At least one domain was tightened.
    OkChanged,
    /// A domain became empty, indicating a conflict.
    Failure,
}

impl PropagationStatus {
    /// Returns `true` when propagation detected a conflict.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Failure)
    }

    /// Returns `true` when propagation changed at least one domain.
    #[must_use]
    pub const fn changed(self) -> bool {
        matches!(self, Self::OkChanged)
    }

    /// Combines two statuses, preferring failure, then change.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Failure, _) | (_, Self::Failure) => Self::Failure,
            (Self::OkChanged, _) | (_, Self::OkChanged) => Self::OkChanged,
            _ => Self::OkNoChange,
        }
    }
}
