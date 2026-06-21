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
        self.entries()
            .iter()
            .rev()
            .find_map(|entry| match entry {
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
            if let Some(existing) = literals.iter_mut().find(|l: &&mut NogoodLiteral| {
                l.variable == literal.variable
            }) {
                *existing = literal;
            } else {
                literals.push(literal);
            }
        }
        literals
    }
}
