use crate::{PropagatorId, VariableId};

/// Kind of bound tightening applied to a domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundKind {
    /// Values strictly below the bound were removed.
    Below,
    /// Values strictly above the bound were removed.
    Above,
}

/// Why a domain change occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeReason {
    /// Search assigned a value during branching.
    Branch { variable: VariableId, value: i32 },
    /// A propagator removed values or tightened bounds.
    Propagator {
        propagator: PropagatorId,
        variable: VariableId,
        removed_value: Option<i32>,
        bound: Option<(BoundKind, i32)>,
    },
    /// A propagator detected infeasibility and recorded contributing assignments.
    PropagatorConflict {
        /// Branch literals that jointly caused the conflict.
        literals: Vec<(VariableId, i32)>,
    },
}

/// Collected explanation for the latest conflict or propagation round.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Explanation {
    entries: Vec<ChangeReason>,
}

impl Explanation {
    /// Creates an empty explanation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Returns recorded change reasons.
    #[must_use]
    pub fn entries(&self) -> &[ChangeReason] {
        &self.entries
    }

    /// Records a change reason.
    pub fn record(&mut self, reason: ChangeReason) {
        self.entries.push(reason);
    }

    /// Clears all recorded reasons.
    pub fn reset(&mut self) {
        self.entries.clear();
    }

    /// Truncates the log to the first `len` entries.
    pub fn truncate(&mut self, len: usize) {
        self.entries.truncate(len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VariableId;
    use slotmap::SlotMap;

    fn make_var() -> VariableId {
        let mut sm: SlotMap<crate::VariableKey, ()> = SlotMap::with_key();
        VariableId::from_key(sm.insert(()))
    }

    #[test]
    fn records_and_resets_entries() {
        let mut explanation = Explanation::new();
        assert!(explanation.entries().is_empty());
        explanation.record(ChangeReason::Branch {
            variable: make_var(),
            value: 1,
        });
        assert_eq!(explanation.entries().len(), 1);
        explanation.reset();
        assert!(explanation.entries().is_empty());
    }

    #[test]
    fn truncates_to_prefix() {
        let mut explanation = Explanation::new();
        for value in 1..=3 {
            explanation.record(ChangeReason::Branch {
                variable: make_var(),
                value,
            });
        }
        explanation.truncate(2);
        assert_eq!(explanation.entries().len(), 2);
    }
}
