use crate::matching::{has_perfect_matching, remove_unsupported_values};
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates pairwise distinctness to generalized arc consistency when possible.
pub struct AllDifferentPropagator {
    variables: Vec<VariableId>,
}

impl AllDifferentPropagator {
    /// Creates an all-different propagator over `variables`.
    #[must_use]
    pub fn new(variables: impl Into<Vec<VariableId>>) -> Self {
        Self {
            variables: variables.into(),
        }
    }
}

impl Propagator for AllDifferentPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.variables
    }

    fn priority(&self) -> u32 {
        10
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;

        loop {
            let mut round_changed = false;
            round_changed |= remove_fixed_values(ctx, &self.variables);
            match propagate_matching(ctx, &self.variables) {
                Ok(matching_changed) => round_changed |= matching_changed,
                Err(()) => return PropagationStatus::Failure,
            }
            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if self.variables.iter().any(|var| ctx.domain(*var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn remove_fixed_values(ctx: &mut dyn PropagationContext, variables: &[VariableId]) -> bool {
    let fixed: Vec<(VariableId, i32)> = variables
        .iter()
        .filter_map(|&var| ctx.fixed_value(var).map(|value| (var, value)))
        .collect();

    let mut changed = false;
    for (source, value) in fixed {
        for &var in variables {
            if var != source && ctx.remove_value(var, value) {
                changed = true;
            }
        }
    }
    changed
}

fn propagate_matching(ctx: &mut dyn PropagationContext, variables: &[VariableId]) -> Result<bool, ()> {
    if variables.len() <= 1 {
        return Ok(false);
    }

    if !has_perfect_matching(ctx, variables) {
        return Err(());
    }

    remove_unsupported_values(ctx, variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_values_are_removed_from_other_variables() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(2));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(AllDifferentPropagator::new([a, b, c])));

        engine.propagate_all().unwrap();
        assert!(!engine.domain(b).contains(2));
        assert!(!engine.domain(c).contains(2));
    }

    #[test]
    fn gac_prunes_unsupported_values() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vec![a, b, c])));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(c).size(), 1);
        assert_eq!(engine.domain(c).min(), Some(3));
    }
}
