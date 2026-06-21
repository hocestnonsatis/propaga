use crate::{ChangeReason, Explanation, VariableId};

/// Branch literal: `variable` must equal `value` for this nogood to apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NogoodLiteral {
    /// Decision variable.
    pub variable: VariableId,
    /// Assigned value.
    pub value: i32,
}

/// Learned nogood: the conjunction of literals cannot all hold simultaneously.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nogood {
    literals: Vec<NogoodLiteral>,
}

impl Nogood {
    /// Creates a nogood from branch literals.
    #[must_use]
    pub fn new(literals: Vec<NogoodLiteral>) -> Self {
        Self { literals }
    }

    /// Returns the nogood literals.
    #[must_use]
    pub fn literals(&self) -> &[NogoodLiteral] {
        &self.literals
    }

    /// Returns `true` when every literal matches `assignment`.
    #[must_use]
    pub fn is_satisfied_by(&self, assignment: &[(VariableId, i32)]) -> bool {
        self.literals.iter().all(|literal| {
            assignment
                .iter()
                .any(|&(var, value)| var == literal.variable && value == literal.value)
        })
    }

    /// Returns `true` when extending `assignment` with `(var, value)` satisfies every literal.
    #[must_use]
    pub fn would_be_satisfied_by(
        &self,
        assignment: &[(VariableId, i32)],
        var: VariableId,
        value: i32,
    ) -> bool {
        self.literals.iter().all(|literal| {
            if literal.variable == var {
                literal.value == value
            } else {
                assignment
                    .iter()
                    .any(|&(v, val)| v == literal.variable && val == literal.value)
            }
        })
    }
}

impl Explanation {
    /// Returns branch literals in chronological order.
    pub fn branch_literals(&self) -> impl Iterator<Item = NogoodLiteral> + '_ {
        self.entries().iter().filter_map(|entry| match entry {
            ChangeReason::Branch { variable, value } => Some(NogoodLiteral {
                variable: *variable,
                value: *value,
            }),
            ChangeReason::Propagator { .. } | ChangeReason::PropagatorConflict { .. } => None,
        })
    }

    /// Returns literals from the most recent propagator-recorded conflict, if any.
    #[must_use]
    pub fn propagator_conflict_literals(&self) -> Option<Vec<NogoodLiteral>> {
        self.entries().iter().rev().find_map(|entry| match entry {
            ChangeReason::PropagatorConflict { literals } => Some(
                literals
                    .iter()
                    .map(|&(variable, value)| NogoodLiteral { variable, value })
                    .collect(),
            ),
            _ => None,
        })
    }

    /// Returns unique branch literals, keeping the latest assignment per variable.
    #[must_use]
    pub fn unique_branch_literals(&self) -> Vec<NogoodLiteral> {
        let mut literals = Vec::new();
        for literal in self.branch_literals() {
            if let Some(existing) = literals
                .iter_mut()
                .find(|l: &&mut NogoodLiteral| l.variable == literal.variable)
            {
                *existing = literal;
            } else {
                literals.push(literal);
            }
        }
        literals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChangeReason, Explanation, VariableId};
    use slotmap::SlotMap;

    fn make_var() -> VariableId {
        let mut sm: SlotMap<crate::VariableKey, ()> = SlotMap::with_key();
        VariableId::from_key(sm.insert(()))
    }

    #[test]
    fn nogood_satisfied_by_matching_assignment() {
        let v0 = make_var();
        let v1 = make_var();
        let nogood = Nogood::new(vec![
            NogoodLiteral {
                variable: v0,
                value: 1,
            },
            NogoodLiteral {
                variable: v1,
                value: 2,
            },
        ]);
        assert!(nogood.is_satisfied_by(&[(v0, 1), (v1, 2)]));
        assert!(!nogood.is_satisfied_by(&[(v0, 1), (v1, 3)]));
    }

    #[test]
    fn nogood_would_be_satisfied_by_extension() {
        let v0 = make_var();
        let nogood = Nogood::new(vec![NogoodLiteral {
            variable: v0,
            value: 5,
        }]);
        assert!(nogood.would_be_satisfied_by(&[], v0, 5));
        assert!(!nogood.would_be_satisfied_by(&[], v0, 4));
    }

    #[test]
    fn branch_literals_skip_propagator_entries() {
        let v0 = make_var();
        let mut explanation = Explanation::new();
        explanation.record(ChangeReason::Branch {
            variable: v0,
            value: 3,
        });
        explanation.record(ChangeReason::Propagator {
            propagator: crate::PropagatorId::from_key({
                let mut sm: SlotMap<crate::PropagatorKey, ()> = SlotMap::with_key();
                sm.insert(())
            }),
            variable: v0,
            removed_value: Some(2),
            bound: None,
        });
        let literals: Vec<_> = explanation.branch_literals().collect();
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].value, 3);
    }

    #[test]
    fn unique_branch_literals_keep_latest_per_variable() {
        let v0 = make_var();
        let mut explanation = Explanation::new();
        explanation.record(ChangeReason::Branch {
            variable: v0,
            value: 1,
        });
        explanation.record(ChangeReason::Branch {
            variable: v0,
            value: 2,
        });
        let literals = explanation.unique_branch_literals();
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0].value, 2);
    }

    #[test]
    fn propagator_conflict_literals_from_latest_conflict() {
        let v0 = make_var();
        let v1 = make_var();
        let mut explanation = Explanation::new();
        explanation.record(ChangeReason::PropagatorConflict {
            literals: vec![(v0, 1), (v1, 2)],
        });
        let literals = explanation
            .propagator_conflict_literals()
            .expect("conflict literals");
        assert_eq!(literals.len(), 2);
    }
}
