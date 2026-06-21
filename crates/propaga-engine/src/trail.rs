use propaga_core::{ChangeReason, Explanation, VariableId};
use propaga_domains::HybridDomain;

struct TrailEntry {
    variable: VariableId,
    old_domain: HybridDomain,
}

/// Stack of domain snapshots for backtracking.
pub struct Trail {
    marks: Vec<usize>,
    explanation_marks: Vec<usize>,
    entries: Vec<TrailEntry>,
}

impl Trail {
    /// Creates an empty trail.
    #[must_use]
    pub fn new() -> Self {
        Self {
            marks: Vec::new(),
            explanation_marks: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// Records a choice point. Returns the level index used by [`Self::backtrack`].
    pub fn mark(&mut self, explanation_len: usize) -> usize {
        self.marks.push(self.entries.len());
        self.explanation_marks.push(explanation_len);
        self.marks.len() - 1
    }

    /// Records a domain change for `variable`.
    pub fn push(
        &mut self,
        variable: VariableId,
        old_domain: HybridDomain,
        reason: Option<ChangeReason>,
        explanation: &mut Explanation,
    ) {
        if let Some(reason) = reason {
            explanation.record(reason);
        }
        self.entries.push(TrailEntry {
            variable,
            old_domain,
        });
    }

    /// Returns domain snapshots recorded after `level` and truncates the trail to that level.
    pub fn backtrack(&mut self, level: usize) -> (Vec<(VariableId, HybridDomain)>, usize) {
        if self.marks.is_empty() {
            return (Vec::new(), 0);
        }
        let level = level.min(self.marks.len() - 1);
        let start = self.marks[level];
        let explanation_len = self.explanation_marks[level];
        let drained: Vec<_> = self
            .entries
            .drain(start..)
            .map(|entry| (entry.variable, entry.old_domain))
            .collect();
        self.marks.truncate(level);
        self.explanation_marks.truncate(level);
        (drained, explanation_len)
    }

    /// Returns the number of active decision levels.
    #[must_use]
    pub fn decision_levels(&self) -> usize {
        self.marks.len()
    }

    /// Returns `true` when at least one backtrack choice point exists.
    pub(crate) fn has_choice_point(&self) -> bool {
        !self.marks.is_empty()
    }

    /// Discards all recorded changes without restoring domains or clearing explanations.
    pub fn commit_base_level(&mut self) {
        self.marks.clear();
        self.explanation_marks.clear();
        self.entries.clear();
    }

    /// Discards all recorded changes and choice points.
    pub fn clear(&mut self, explanation: &mut Explanation) {
        self.commit_base_level();
        explanation.reset();
    }
}

impl Default for Trail {
    fn default() -> Self {
        Self::new()
    }
}
