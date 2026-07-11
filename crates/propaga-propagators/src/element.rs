use crate::reified::propagate_equal;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `value == array[index]` with bound consistency.
#[derive(Clone)]
pub struct ElementPropagator {
    watched: Vec<VariableId>,
    index: VariableId,
    array: Vec<VariableId>,
    value: VariableId,
}

impl ElementPropagator {
    /// Creates an element propagator for `value == array[index]`.
    #[must_use]
    pub fn new(index: VariableId, array: impl Into<Vec<VariableId>>, value: VariableId) -> Self {
        let array = array.into();
        let mut watched = Vec::with_capacity(array.len() + 2);
        watched.push(index);
        watched.extend(&array);
        watched.push(value);
        Self {
            watched,
            index,
            array,
            value,
        }
    }
}

impl Propagator for ElementPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        15
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        if self.array.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        loop {
            let mut round_changed = false;
            round_changed |= propagate_index(ctx, self.index, &self.array, self.value);
            round_changed |= propagate_value_bounds(ctx, self.index, &self.array, self.value);
            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if ctx.domain(self.index).is_empty()
            || ctx.domain(self.value).is_empty()
            || self.array.iter().any(|&var| ctx.domain(var).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn propagate_index(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
) -> bool {
    let mut changed = false;
    let max_index = array.len() as i32 - 1;

    if ctx.remove_below(index, 0) {
        changed = true;
    }
    if ctx.remove_above(index, max_index) {
        changed = true;
    }

    if let Some(idx) = ctx.fixed_value(index) {
        if !(0..array.len()).contains(&(idx as usize)) {
            return changed;
        }
        let element = array[idx as usize];
        changed |= tighten_equal(ctx, value, element);
    }

    if let Some(val) = ctx.fixed_value(value) {
        let mut supported = Vec::new();
        for (position, &element) in array.iter().enumerate() {
            if ctx.domain(element).contains(val) {
                supported.push(position as i32);
            }
        }
        for idx in domain_values(ctx, index) {
            if !supported.contains(&idx) && ctx.remove_value(index, idx) {
                changed = true;
            }
        }
    }

    changed
}

fn propagate_value_bounds(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(idx) = ctx.fixed_value(index) {
        let element = array[idx as usize];
        return tighten_equal(ctx, value, element);
    }

    let mut min_value = i32::MAX;
    let mut max_value = i32::MIN;
    let mut any = false;

    for idx in domain_values(ctx, index) {
        let element = array[idx as usize];
        if let (Some(min), Some(max)) = (ctx.domain(element).min(), ctx.domain(element).max()) {
            min_value = min_value.min(min);
            max_value = max_value.max(max);
            any = true;
        }
    }

    if any {
        if ctx.remove_below(value, min_value) {
            changed = true;
        }
        if ctx.remove_above(value, max_value) {
            changed = true;
        }
    }

    for idx in domain_values(ctx, index) {
        let element = array[idx as usize];
        changed |= propagate_element_to_value(ctx, element, value);
    }

    changed
}

fn propagate_element_to_value(
    ctx: &mut dyn PropagationContext,
    element: VariableId,
    value: VariableId,
) -> bool {
    let mut changed = false;
    if let (Some(v_min), Some(v_max)) = (ctx.domain(value).min(), ctx.domain(value).max()) {
        if ctx.remove_below(element, v_min) {
            changed = true;
        }
        if ctx.remove_above(element, v_max) {
            changed = true;
        }
    }
    if let (Some(e_min), Some(e_max)) = (ctx.domain(element).min(), ctx.domain(element).max()) {
        if ctx.remove_below(value, e_min) {
            changed = true;
        }
        if ctx.remove_above(value, e_max) {
            changed = true;
        }
    }
    changed
}

fn tighten_equal(ctx: &mut dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    propagate_equal(ctx, left, right)
}

fn domain_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
    let domain = ctx.domain(var);
    let mut values = Vec::new();
    if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
        for value in min..=max {
            if domain.contains(value) {
                values.push(value);
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_left_equalizes_holey_right_domain() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let a = engine.new_variable(IntervalDomain::new(1, 5).remove(2).remove(4));
        let value = engine.new_variable(IntervalDomain::fix(3));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        let _ = engine.propagate_all();
    }

    #[test]
    fn fixed_right_equalizes_holey_left_domain() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let a = engine.new_variable(IntervalDomain::new(1, 5).remove(2));
        let value = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
    }

    #[test]
    fn bounds_sync_between_value_and_element() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let a = engine.new_variable(IntervalDomain::new(3, 7));
        let value = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(value).min(), Some(3));
        assert_eq!(engine.hybrid_domain(value).max(), Some(7));
        assert_eq!(engine.hybrid_domain(a).min(), Some(3));
        assert_eq!(engine.hybrid_domain(a).max(), Some(7));
    }

    #[test]
    fn invalid_fixed_index_returns_without_equalizing() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(5));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let value = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        let _ = engine.propagate_all();
    }

    #[test]
    fn fixed_value_prunes_unsupported_indices_with_holes() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 2).remove(1));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 5));
        let c = engine.new_variable(IntervalDomain::fix(9));
        let value = engine.new_variable(IntervalDomain::fix(9));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(index).fixed_value(), Some(2));
    }

    #[test]
    fn tighten_equal_with_holey_domains() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let a = engine.new_variable(IntervalDomain::new(1, 5).remove(2).remove(4));
        let value = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
    }

    #[test]
    fn fixed_value_equalizes_element_with_holes() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let a = engine.new_variable(IntervalDomain::new(1, 5).remove(2));
        let value = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).fixed_value(), Some(1));
    }

    #[test]
    fn element_bounds_sync_to_value() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 0));
        let a = engine.new_variable(IntervalDomain::new(10, 20));
        let b = engine.new_variable(IntervalDomain::new(30, 40));
        let value = engine.new_variable(IntervalDomain::new(1, 100));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a, b], value)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(value).min(), Some(10));
        assert_eq!(engine.hybrid_domain(value).max(), Some(20));
    }

    #[test]
    fn empty_index_domain_fails() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(1, 0));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let value = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn fixed_index_propagates_to_value() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(2));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 5));
        let c = engine.new_variable(IntervalDomain::new(10, 20));
        let value = engine.new_variable(IntervalDomain::new(1, 20));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(value).min(), Some(10));
        assert_eq!(engine.hybrid_domain(value).max(), Some(20));
    }

    #[test]
    fn fixed_value_prunes_index() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 2));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 5));
        let c = engine.new_variable(IntervalDomain::fix(9));
        let value = engine.new_variable(IntervalDomain::fix(9));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(index).fixed_value(), Some(2));
    }

    #[test]
    fn empty_array_fails() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(0));
        let value = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![], value)));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn index_clamped_to_array_bounds() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(-1, 5));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 5));
        let c = engine.new_variable(IntervalDomain::new(1, 5));
        let value = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(index).min(), Some(0));
        assert_eq!(engine.hybrid_domain(index).max(), Some(2));
    }

    #[test]
    fn variable_index_tightens_value_bounds() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 1));
        let a = engine.new_variable(IntervalDomain::new(10, 20));
        let b = engine.new_variable(IntervalDomain::new(10, 20));
        let c = engine.new_variable(IntervalDomain::new(30, 40));
        let value = engine.new_variable(IntervalDomain::new(1, 100));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(value).min(), Some(10));
        assert_eq!(engine.hybrid_domain(value).max(), Some(20));
    }

    #[test]
    fn fixed_index_equalizes_value_and_element() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(1));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(10, 20));
        let c = engine.new_variable(IntervalDomain::new(30, 40));
        let value = engine.new_variable(IntervalDomain::new(1, 100));
        engine.add_propagator(Box::new(ElementPropagator::new(
            index,
            vec![a, b, c],
            value,
        )));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(value).min(), Some(10));
        assert_eq!(engine.hybrid_domain(value).max(), Some(20));
        assert_eq!(engine.hybrid_domain(b).min(), Some(10));
        assert_eq!(engine.hybrid_domain(b).max(), Some(20));
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(1));
        let a = engine.new_variable(IntervalDomain::fix(5));
        let b = engine.new_variable(IntervalDomain::fix(10));
        let value = engine.new_variable(IntervalDomain::fix(10));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a, b], value)));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkNoChange);
    }

    #[test]
    fn tighten_equal_syncs_bounds_between_operands() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(2, 4));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let mut ctx = MutEngine(&mut engine);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(engine.hybrid_domain(right).min(), Some(2));
        assert_eq!(engine.hybrid_domain(right).max(), Some(4));
    }

    #[test]
    fn propagate_element_to_value_syncs_both_ways() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let element = engine.new_variable(IntervalDomain::new(3, 7));
        let value = engine.new_variable(IntervalDomain::new(1, 10));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_element_to_value(&mut ctx, element, value));
        assert_eq!(engine.hybrid_domain(value).min(), Some(3));
        assert_eq!(engine.hybrid_domain(value).max(), Some(7));
    }

    #[test]
    fn invalid_fixed_index_skips_equalization() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(5));
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let value = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(ElementPropagator::new(index, vec![a], value)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn mock_invalid_fixed_index_returns_early() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 0));
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(index, vec![5])
            .with_domain(a, vec![1, 2, 3])
            .with_domain(value, vec![1, 2, 3])
            .with_fixed(index, 5);
        let changed = propagate_index(&mut ctx, index, &[a], value);
        assert!(!changed || ctx.domains[&value].values.borrow().len() == 3);
    }

    #[test]
    fn mock_tighten_equal_prunes_holey_domain() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(left, 3);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[3]);
    }

    #[test]
    fn mock_propagate_element_to_value_syncs_both_directions() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let element = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(element, vec![2, 3, 4, 5])
            .with_domain(value, vec![3, 4]);
        assert!(propagate_element_to_value(&mut ctx, element, value));
        assert_eq!(ctx.domains[&element].values.borrow().as_slice(), &[3, 4]);
        assert_eq!(ctx.domains[&value].values.borrow().as_slice(), &[3, 4]);
    }

    #[test]
    fn mock_tighten_equal_syncs_from_right() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6, 10])
            .with_domain(right, vec![4, 5, 6]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn mock_propagate_element_bounds_from_value() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 0));
        let a = engine.new_variable(IntervalDomain::new(0, 0));
        let b = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(index, vec![0, 1])
            .with_domain(a, vec![10, 20])
            .with_domain(b, vec![30, 40])
            .with_domain(value, vec![12, 18]);
        assert!(propagate_value_bounds(&mut ctx, index, &[a, b], value));
    }

    #[test]
    fn mock_tighten_equal_fixed_right_prunes_holey_left() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![5])
            .with_fixed(right, 5);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[5]);
    }

    #[test]
    fn mock_tighten_equal_syncs_left_bounds_to_right() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6])
            .with_domain(right, vec![4, 5, 6, 7]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[5, 6]);
    }

    #[test]
    fn mock_propagate_element_to_value_tightens_element_from_value() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let element = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(element, vec![1, 2, 3, 4, 5])
            .with_domain(value, vec![4]);
        assert!(propagate_element_to_value(&mut ctx, element, value));
        assert_eq!(ctx.domains[&element].values.borrow().as_slice(), &[4]);
    }

    #[test]
    fn mock_propagate_element_to_value_syncs_value_bounds_to_element() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let element = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(element, vec![1, 2, 3, 4, 5, 6])
            .with_domain(value, vec![3, 4]);
        assert!(propagate_element_to_value(&mut ctx, element, value));
        assert_eq!(ctx.domains[&element].values.borrow().as_slice(), &[3, 4]);
    }

    #[test]
    fn mock_tighten_equal_fixed_left_prunes_holey_right() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![7])
            .with_domain(right, vec![5, 6, 7, 8, 9])
            .with_fixed(left, 7);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn mock_tighten_equal_fixed_right_prunes_holey_left_values() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![3])
            .with_fixed(right, 3);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[3]);
    }

    #[test]
    fn mock_tighten_equal_syncs_left_bounds_to_right_domain() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5])
            .with_domain(right, vec![1, 2, 3, 4, 5, 6]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[4, 5]);
    }

    #[test]
    fn mock_tighten_equal_syncs_right_bounds_to_left_domain() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5, 6])
            .with_domain(right, vec![4, 5]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[4, 5]);
    }

    #[test]
    fn mock_tighten_equal_fixed_left_prunes_extra_right_values() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(left, 3);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[3]);
    }

    #[test]
    fn mock_tighten_equal_bound_sync_from_right_fixed() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5, 6])
            .with_domain(right, vec![4, 5]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[4, 5]);
    }

    #[test]
    fn mock_tighten_equal_prunes_extra_values_both_fixed_sides() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![9])
            .with_domain(right, vec![6, 7, 8, 9, 10])
            .with_fixed(left, 9);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[9]);

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![6, 7, 8, 9, 10])
            .with_domain(right, vec![9])
            .with_fixed(right, 9);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[9]);

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5])
            .with_domain(right, vec![2, 3, 4, 5, 6]);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[4, 5]);
    }

    #[test]
    fn mock_propagate_element_to_value_element_only_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let element = engine.new_variable(IntervalDomain::new(0, 0));
        let value = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(element, vec![1, 2, 3, 4, 5, 6])
            .with_domain(value, vec![]);
        assert!(!propagate_element_to_value(&mut ctx, element, value));
    }

    #[test]
    fn mock_tighten_equal_fixed_left_removes_holey_right_values() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![8])
            .with_domain(right, vec![5, 6, 7, 8, 9])
            .with_fixed(left, 8);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&right].values.borrow().as_slice(), &[8]);
    }

    #[test]
    fn mock_tighten_equal_fixed_right_removes_holey_left_values() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7, 8, 9])
            .with_domain(right, vec![8])
            .with_fixed(right, 8);
        assert!(tighten_equal(&mut ctx, left, right));
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[8]);
    }
}
