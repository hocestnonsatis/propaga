use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};
use std::collections::HashMap;

/// Cardinality bounds for a single value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardinalityBound {
    /// Minimum occurrences of the value.
    pub min: i32,
    /// Maximum occurrences of the value.
    pub max: i32,
}

impl CardinalityBound {
    /// Creates bounds with the same minimum and maximum.
    #[must_use]
    pub const fn exact(count: i32) -> Self {
        Self {
            min: count,
            max: count,
        }
    }

    /// Creates inclusive bounds.
    #[must_use]
    pub const fn range(min: i32, max: i32) -> Self {
        Self { min, max }
    }
}

/// Propagates global cardinality with bounds consistency.
#[derive(Clone)]
pub struct GlobalCardinalityPropagator {
    variables: Vec<VariableId>,
    cards: HashMap<i32, CardinalityBound>,
}

impl GlobalCardinalityPropagator {
    /// Creates a GCC propagator over `variables` and per-value bounds.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        cards: impl IntoIterator<Item = (i32, CardinalityBound)>,
    ) -> Self {
        Self {
            variables: variables.into(),
            cards: cards.into_iter().collect(),
        }
    }
}

impl Propagator for GlobalCardinalityPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.variables
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;

        loop {
            match propagate_bounds(ctx, &self.variables, &self.cards) {
                Ok(round_changed) => {
                    changed |= round_changed;
                    if !round_changed {
                        break;
                    }
                }
                Err(()) => return PropagationStatus::Failure,
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

fn propagate_bounds(
    ctx: &mut dyn PropagationContext,
    variables: &[VariableId],
    cards: &HashMap<i32, CardinalityBound>,
) -> Result<bool, ()> {
    let mut changed = false;
    let values = collect_relevant_values(ctx, variables, cards);

    for value in values {
        let bounds = cards.get(&value).copied().unwrap_or(CardinalityBound {
            min: 0,
            max: variables.len() as i32,
        });

        let mut fixed = Vec::new();
        let mut open = Vec::new();
        for &var in variables {
            match ctx.fixed_value(var) {
                Some(fixed_value) if fixed_value == value => fixed.push(var),
                Some(_) => {}
                None if ctx.domain(var).contains(value) => open.push(var),
                None => {}
            }
        }

        let mut fixed_count = fixed.len() as i32;
        let possible_count = fixed_count + open.len() as i32;

        if fixed_count > bounds.max || possible_count < bounds.min {
            return Err(());
        }

        if fixed_count == bounds.max {
            for &var in &open {
                if ctx.remove_value(var, value) {
                    changed = true;
                }
            }
        }

        if possible_count == bounds.min {
            for &var in &open {
                if ctx.fixed_value(var) != Some(value) {
                    changed |= remove_all_except(ctx, var, value);
                }
            }
            fixed.clear();
            open.clear();
            for &var in variables {
                match ctx.fixed_value(var) {
                    Some(fixed_value) if fixed_value == value => fixed.push(var),
                    Some(_) => {}
                    None if ctx.domain(var).contains(value) => open.push(var),
                    None => {}
                }
            }
            fixed_count = fixed.len() as i32;
        }
    }

    Ok(changed)
}

fn collect_relevant_values(
    ctx: &dyn PropagationContext,
    variables: &[VariableId],
    cards: &HashMap<i32, CardinalityBound>,
) -> Vec<i32> {
    let mut values: Vec<i32> = cards.keys().copied().collect();
    for &var in variables {
        if let (Some(min), Some(max)) = (ctx.domain(var).min(), ctx.domain(var).max()) {
            for value in min..=max {
                if ctx.domain(var).contains(value) && !values.contains(&value) {
                    values.push(value);
                }
            }
        }
    }
    values.sort_unstable();
    values
}

fn remove_all_except(ctx: &mut dyn PropagationContext, var: VariableId, keep: i32) -> bool {
    let values = collect_values(ctx, var);
    let mut changed = false;
    for value in values {
        if value != keep && ctx.remove_value(var, value) {
            changed = true;
        }
    }
    changed
}

fn collect_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
    let domain = ctx.domain(var);
    match (domain.min(), domain.max()) {
        (Some(min), Some(max)) => (min..=max)
            .filter(|&value| domain.contains(value))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn cardinality_bound_range_constructor() {
        let bounds = CardinalityBound::range(1, 3);
        assert_eq!(bounds.min, 1);
        assert_eq!(bounds.max, 3);
    }

    #[test]
    fn empty_variable_domain_fails_after_propagation() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 0));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(vec![a, b], [])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn collects_values_not_in_cards_map() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(1, CardinalityBound::exact(1))],
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn prunes_value_when_insufficient_support_for_min() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(2, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(c).contains(1));
    }

    #[test]
    fn forces_min_with_holey_domain() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 4).remove(2).remove(3));
        let b = engine.new_variable(IntervalDomain::new(1, 4).remove(2).remove(3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(1));
    }

    #[test]
    fn max_cardinality_removes_value_from_open_vars() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(c).contains(1));
    }

    #[test]
    fn exceeds_max_cardinality_fails() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn min_cardinality_forces_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(1, CardinalityBound::exact(2))],
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(1));
    }

    #[test]
    fn prunes_unsupported_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));

        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(c).contains(1));
        assert!(engine.hybrid_domain(c).contains(2));
        assert!(engine.hybrid_domain(c).contains(3));
    }

    #[test]
    fn prunes_value_exceeding_max() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(1))],
        )));

        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(b).contains(1));
        assert!(!engine.hybrid_domain(c).contains(1));
    }

    #[test]
    fn default_bounds_apply() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(5));
        let b = engine.new_variable(IntervalDomain::fix(5));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(1, CardinalityBound::exact(0))],
        )));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkNoChange);
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(5));
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(5));
    }

    #[test]
    fn gcc_already_satisfied_no_change() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(2));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [
                (1, CardinalityBound::exact(1)),
                (2, CardinalityBound::exact(1)),
            ],
        )));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkNoChange);
    }

    #[test]
    fn empty_domain_after_bounds_loop_fails() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 0));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(1, CardinalityBound::exact(1))],
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn collects_values_from_domains_outside_cards_map() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(5, 7));
        let b = engine.new_variable(IntervalDomain::new(5, 7));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(5, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(5));
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(5));
    }

    #[test]
    fn range_bounds_used_in_propagation() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b],
            [(2, CardinalityBound::range(0, 0))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(a).contains(2));
        assert!(!engine.hybrid_domain(b).contains(2));
    }

    #[test]
    fn prunes_value_with_insufficient_support() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(2, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(1));
    }

    #[test]
    fn prunes_value_when_max_exceeded_for_var() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(c).contains(1));
    }

    #[test]
    fn remove_all_except_forces_open_var() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 4).remove(2).remove(3));
        let b = engine.new_variable(IntervalDomain::new(1, 4).remove(2).remove(3));
        let vars = vec![a, b];
        let cards = [(1, CardinalityBound::exact(2))].into_iter().collect();
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_bounds(&mut ctx, &vars, &cards).unwrap());
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
        assert_eq!(engine.hybrid_domain(b).fixed_value(), Some(1));
    }

    #[test]
    fn prunes_insufficient_per_value_support() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::fix(2));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::range(2, 2))],
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
    }

    #[test]
    fn prunes_value_when_var_would_exceed_max_alone() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(a).contains(1));
    }

    #[test]
    fn empty_domain_after_bounds_loop_returns_failure() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(a, vec![1, 2])
            .with_domain(b, vec![]);
        let mut prop = GlobalCardinalityPropagator::new(vec![a, b], []);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_propagate_bounds_per_var_support_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let c = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(a, vec![1, 2])
            .with_domain(b, vec![1])
            .with_domain(c, vec![1])
            .with_fixed(b, 1)
            .with_fixed(c, 1);
        let cards = [(1, CardinalityBound::exact(2))].into_iter().collect();
        assert!(propagate_bounds(&mut ctx, &[a, b, c], &cards).unwrap());
        assert!(!ctx.domains[&a].values.borrow().contains(&1));
    }

    #[test]
    fn collect_values_visits_holey_domain_members() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let ctx = MockIntCtx::new().with_domain(a, vec![1, 3, 5]);
        assert_eq!(collect_values(&ctx, a), vec![1, 3, 5]);
        let empty = MockIntCtx::new().with_domain(b, vec![]);
        assert!(collect_values(&empty, b).is_empty());
    }

    #[test]
    fn mock_propagate_bounds_max_exceeded_for_open_var() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let c = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(a, vec![1])
            .with_domain(b, vec![1])
            .with_domain(c, vec![1, 2])
            .with_fixed(a, 1)
            .with_fixed(b, 1);
        let cards = [(1, CardinalityBound::exact(2))].into_iter().collect();
        assert!(propagate_bounds(&mut ctx, &[a, b, c], &cards).unwrap());
        assert!(!ctx.domains[&c].values.borrow().contains(&1));
    }

    #[test]
    fn mock_propagate_bounds_min_support_removes_value() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let c = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(a, vec![1, 2])
            .with_domain(b, vec![1])
            .with_domain(c, vec![2])
            .with_fixed(b, 1)
            .with_fixed(c, 2);
        let cards = [(1, CardinalityBound::range(2, 2))].into_iter().collect();
        assert!(propagate_bounds(&mut ctx, &[a, b, c], &cards).unwrap());
        assert_eq!(ctx.domains[&a].values.borrow().as_slice(), &[1]);
    }

    #[test]
    fn collect_values_iterates_holey_domain_range() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let ctx = MockIntCtx::new().with_domain(a, vec![2, 4, 6]);
        assert_eq!(collect_values(&ctx, a), vec![2, 4, 6]);
    }
}
