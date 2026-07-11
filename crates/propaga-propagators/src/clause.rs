use propaga_core::{NogoodLiteral, PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagator for a learned clause (same semantics as a nogood).
#[derive(Clone)]
pub struct ClausePropagator {
    watched: Vec<VariableId>,
    literals: Vec<NogoodLiteral>,
}

impl ClausePropagator {
    /// Creates a propagator for a learned clause.
    #[must_use]
    pub fn new(literals: impl Into<Vec<NogoodLiteral>>) -> Self {
        let literals = literals.into();
        let mut watched = Vec::new();
        for literal in &literals {
            if !watched.contains(&literal.variable) {
                watched.push(literal.variable);
            }
        }
        Self { watched, literals }
    }
}

impl Propagator for ClausePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        1
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        crate::nogood::propagate_nogood_literals(&self.literals, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn watched_variables_are_deduplicated() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let propagator = ClausePropagator::new(vec![
            NogoodLiteral {
                variable: a,
                value: 1,
            },
            NogoodLiteral {
                variable: a,
                value: 2,
            },
        ]);
        assert_eq!(propagator.watched_variables(), &[a]);
        assert_eq!(propagator.priority(), 1);
    }

    #[test]
    fn detects_fully_matching_clause() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(ClausePropagator::new(vec![
            NogoodLiteral {
                variable: a,
                value: 1,
            },
            NogoodLiteral {
                variable: b,
                value: 2,
            },
        ])));

        engine.fix_variable(a, 1).unwrap();
        let status = engine.fix_variable(b, 2).unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn prunes_last_open_literal() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(ClausePropagator::new(vec![
            NogoodLiteral {
                variable: a,
                value: 1,
            },
            NogoodLiteral {
                variable: b,
                value: 2,
            },
        ])));

        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(b).contains(2));
    }
}
