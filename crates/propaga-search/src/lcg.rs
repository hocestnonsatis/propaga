//! Lazy clause generation spike: accumulate nogoods as reusable clauses.

use propaga_core::{Nogood, NogoodLiteral, VariableKey};
use std::collections::HashSet;

/// A clause learned during search and eligible for future propagation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedClause {
    /// Literals in the clause.
    pub literals: Vec<NogoodLiteral>,
}

/// Store of lazily generated clauses.
#[derive(Clone, Debug, Default)]
pub struct ClauseStore {
    clauses: Vec<LearnedClause>,
    signatures: HashSet<Vec<(VariableKey, i32)>>,
}

impl ClauseStore {
    /// Creates an empty clause store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a nogood as a clause when it is new.
    pub fn learn_from_nogood(&mut self, nogood: &Nogood) -> bool {
        let literals = nogood.literals().to_vec();
        let signature = literals
            .iter()
            .map(|literal| (literal.variable.key(), literal.value))
            .collect::<Vec<_>>();
        if !self.signatures.insert(signature) {
            return false;
        }
        self.clauses.push(LearnedClause { literals });
        true
    }

    /// Returns learned clauses.
    #[must_use]
    pub fn clauses(&self) -> &[LearnedClause] {
        &self.clauses
    }

    /// Returns `true` when assignment violates any stored clause.
    #[must_use]
    pub fn is_violated(&self, assignment: &[(propaga_core::VariableId, i32)]) -> bool {
        self.clauses.iter().any(|clause| {
            clause.literals.iter().all(|literal| {
                assignment
                    .iter()
                    .any(|(var, value)| literal.variable == *var && literal.value == *value)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::{Nogood, NogoodLiteral, VariableId, VariableKey};
    use slotmap::SlotMap;

    #[test]
    fn stores_unique_clauses() {
        let mut sm: SlotMap<VariableKey, ()> = SlotMap::with_key();
        let var = VariableId::from_key(sm.insert(()));
        let mut store = ClauseStore::new();
        let nogood = Nogood::new(vec![NogoodLiteral {
            variable: var,
            value: 1,
        }]);
        assert!(store.learn_from_nogood(&nogood));
        assert!(!store.learn_from_nogood(&nogood));
        assert_eq!(store.clauses().len(), 1);
    }
}
