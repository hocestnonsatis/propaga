use propaga_core::{NogoodLiteral, PropagationContext, PropagationStatus, Propagator, VariableId};

/// Forbids the conjunction of branch literals in a learned nogood.
pub struct NogoodPropagator {
    watched: Vec<VariableId>,
    literals: Vec<NogoodLiteral>,
}

impl NogoodPropagator {
    /// Creates a propagator for a learned nogood.
    #[must_use]
    pub fn new(literals: impl Into<Vec<NogoodLiteral>>) -> Self {
        let literals = literals.into();
        let mut watched = Vec::with_capacity(literals.len());
        for literal in &literals {
            if !watched.contains(&literal.variable) {
                watched.push(literal.variable);
            }
        }
        Self { watched, literals }
    }
}

impl Propagator for NogoodPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        1
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut matched = 0usize;
        let mut pending: Option<NogoodLiteral> = None;

        for literal in &self.literals {
            match ctx.fixed_value(literal.variable) {
                Some(value) if value == literal.value => matched += 1,
                Some(_) => return PropagationStatus::OkNoChange,
                None => {
                    if pending.is_some() {
                        return PropagationStatus::OkNoChange;
                    }
                    pending = Some(*literal);
                }
            }
        }

        if matched == self.literals.len() {
            return PropagationStatus::Failure;
        }

        if matched + 1 == self.literals.len()
            && let Some(literal) = pending
            && ctx.remove_value(literal.variable, literal.value)
        {
            if ctx.domain(literal.variable).is_empty() {
                return PropagationStatus::Failure;
            }
            return PropagationStatus::OkChanged;
        }

        PropagationStatus::OkNoChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn detects_fully_matching_nogood() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(NogoodPropagator::new(vec![
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
        engine.add_propagator(Box::new(NogoodPropagator::new(vec![
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
        assert!(!engine.domain(b).contains(2));
    }
}
