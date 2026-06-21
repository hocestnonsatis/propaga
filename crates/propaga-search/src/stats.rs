use propaga_core::VariableId;

/// Statistics collected during search.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchStats {
    /// Number of branch attempts.
    pub nodes: u64,
    /// Number of backtracks performed.
    pub backtracks: u64,
    /// Number of conflicts encountered.
    pub conflicts: u64,
    /// Number of nogoods learned.
    pub nogoods_learned: u64,
    /// Number of restarts performed.
    pub restarts: u64,
    /// Whether search stopped because the time limit was reached.
    pub timed_out: bool,
}

impl SearchStats {
    /// Records a branch attempt.
    pub fn record_node(&mut self) {
        self.nodes += 1;
    }

    /// Records a backtrack.
    pub fn record_backtrack(&mut self) {
        self.backtracks += 1;
    }

    /// Records a conflict.
    pub fn record_conflict(&mut self) {
        self.conflicts += 1;
    }

    /// Records a learned nogood.
    pub fn record_nogood(&mut self) {
        self.nogoods_learned += 1;
    }

    /// Records a restart.
    pub fn record_restart(&mut self) {
        self.restarts += 1;
    }
}

/// Returns the current branch assignments from the explanation log.
#[must_use]
pub fn branch_assignments_from_explanation(
    explanation: &propaga_core::Explanation,
) -> Vec<(VariableId, i32)> {
    explanation
        .unique_branch_literals()
        .into_iter()
        .map(|literal| (literal.variable, literal.value))
        .collect()
}
