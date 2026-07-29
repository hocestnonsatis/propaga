use propaga_core::{
    FloatDomainSnapshot, PropagationContext, PropagationStatus, Propagator, VariableId,
};

use super::float_eq::FloatEqPropagator;

/// Propagates `value == array[index]` for float array elements, including holes.
#[derive(Clone, Debug)]
pub struct FloatElementPropagator {
    watched: Vec<VariableId>,
    index: VariableId,
    array: Vec<VariableId>,
    value: VariableId,
}

impl FloatElementPropagator {
    /// Creates a float element propagator for `value == array[index]` (0-based index).
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

impl Propagator for FloatElementPropagator {
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
        let max_index = self.array.len() as i32 - 1;
        changed |= ctx.remove_below(self.index, 0);
        changed |= ctx.remove_above(self.index, max_index);

        if let Some(idx) = ctx.fixed_value(self.index) {
            if !(0..self.array.len()).contains(&(idx as usize)) {
                return PropagationStatus::Failure;
            }
            let mut eq = FloatEqPropagator::new(self.value, self.array[idx as usize]);
            let status = eq.propagate(ctx);
            if status.is_failure() {
                return status;
            }
            changed |= status == PropagationStatus::OkChanged;
            return finish(ctx, self.index, &self.array, self.value, changed);
        }

        changed |= prune_unsupported_indices(ctx, self.index, &self.array, self.value);
        changed |= tighten_value_from_candidates(ctx, self.index, &self.array, self.value);
        changed |= project_common_absent_holes(ctx, self.index, &self.array, self.value);

        if let Some(idx) = ctx.fixed_value(self.index)
            && (0..self.array.len()).contains(&(idx as usize))
        {
            let mut eq = FloatEqPropagator::new(self.value, self.array[idx as usize]);
            let status = eq.propagate(ctx);
            if status.is_failure() {
                return status;
            }
            changed |= status == PropagationStatus::OkChanged;
        }

        finish(ctx, self.index, &self.array, self.value, changed)
    }
}

fn finish(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
    changed: bool,
) -> PropagationStatus {
    if ctx.domain(index).is_empty() {
        return PropagationStatus::Failure;
    }
    let value_empty = ctx
        .as_extended()
        .and_then(|ext| ext.float_domain(value))
        .is_none_or(|domain| domain.is_empty());
    if value_empty {
        return PropagationStatus::Failure;
    }
    for &var in array {
        let empty = ctx
            .as_extended()
            .and_then(|ext| ext.float_domain(var))
            .is_none_or(|domain| domain.is_empty());
        if empty {
            return PropagationStatus::Failure;
        }
    }
    if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn float_snap(ctx: &mut dyn PropagationContext, var: VariableId) -> Option<FloatDomainSnapshot> {
    ctx.as_extended()
        .and_then(|ext| ext.float_domain(var))
        .filter(|domain| !domain.is_empty())
}

fn domains_overlap(left: &FloatDomainSnapshot, right: &FloatDomainSnapshot) -> bool {
    if left.max < right.min || right.max < left.min {
        return false;
    }
    let lo = left.min.max(right.min);
    let hi = left.max.min(right.max);
    // Singleton intersection: must be admissible in both domains (holes count).
    if (hi - lo).abs() <= f64::EPSILON {
        return left.contains(lo) && right.contains(lo);
    }
    if left.is_fixed() && !right.contains(left.min) {
        return false;
    }
    if right.is_fixed() && !left.contains(right.min) {
        return false;
    }
    true
}

fn prune_unsupported_indices(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
) -> bool {
    let Some(value_dom) = float_snap(ctx, value) else {
        return false;
    };
    let mut changed = false;
    for idx in index_values(ctx, index) {
        let Some(elem) = float_snap(ctx, array[idx as usize]) else {
            changed |= ctx.remove_value(index, idx);
            continue;
        };
        if !domains_overlap(&elem, &value_dom) {
            changed |= ctx.remove_value(index, idx);
        }
    }
    changed
}

fn tighten_value_from_candidates(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
) -> bool {
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    let mut any = false;
    for idx in index_values(ctx, index) {
        let Some(elem) = float_snap(ctx, array[idx as usize]) else {
            continue;
        };
        min_value = min_value.min(elem.min);
        max_value = max_value.max(elem.max);
        any = true;
    }
    if !any {
        return false;
    }
    let Some(ext) = ctx.as_extended() else {
        return false;
    };
    let mut changed = false;
    changed |= ext.tighten_float_below(value, min_value);
    changed |= ext.tighten_float_above(value, max_value);
    changed
}

fn project_common_absent_holes(
    ctx: &mut dyn PropagationContext,
    index: VariableId,
    array: &[VariableId],
    value: VariableId,
) -> bool {
    let candidates: Vec<FloatDomainSnapshot> = index_values(ctx, index)
        .into_iter()
        .filter_map(|idx| float_snap(ctx, array[idx as usize]))
        .collect();
    if candidates.is_empty() {
        return false;
    }
    let mut hole_candidates = Vec::new();
    for domain in &candidates {
        for &hole in &domain.holes {
            if !hole_candidates
                .iter()
                .any(|existing: &f64| (*existing - hole).abs() <= f64::EPSILON)
            {
                hole_candidates.push(hole);
            }
        }
    }
    let Some(ext) = ctx.as_extended() else {
        return false;
    };
    let mut changed = false;
    for hole in hole_candidates {
        if candidates.iter().all(|domain| !domain.contains(hole)) {
            changed |= ext.exclude_float_point(value, hole);
        }
    }
    changed
}

fn index_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
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
    use propaga_domains::{AnyDomain, FloatDomain, IntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn fixed_index_shares_holes_with_value() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::fix(1));
        let a0 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let a1 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let value = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatElementPropagator::new(
            index,
            vec![a0, a1],
            value,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(value).as_float().unwrap().contains(1.0));
    }

    #[test]
    fn projects_hole_common_to_all_candidates() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 1));
        let a0 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let a1 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let value = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatElementPropagator::new(
            index,
            vec![a0, a1],
            value,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(value).as_float().unwrap().contains(1.0));
    }

    #[test]
    fn does_not_project_hole_when_one_candidate_contains_it() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 1));
        let a0 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let a1 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let value = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatElementPropagator::new(
            index,
            vec![a0, a1],
            value,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(engine.domain(value).as_float().unwrap().contains(1.0));
    }

    #[test]
    fn removes_index_when_element_fixed_outside_value() {
        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 1));
        let a0 = engine.new_variable(AnyDomain::Float(FloatDomain::fix(5.0)));
        let a1 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let value = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatElementPropagator::new(
            index,
            vec![a0, a1],
            value,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(index).fixed_value(), Some(1));
    }

    #[test]
    fn removes_index_when_singleton_overlap_is_a_hole() {
        use propaga_core::DomainView;

        let mut engine = Engine::new();
        let index = engine.new_variable(IntervalDomain::new(0, 1));
        // Overlap with value is only {2}, which is a hole on a0.
        let a0 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(2.0)));
        let a1 = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 4.0)));
        let value = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 4.0)));
        engine.add_propagator(Box::new(FloatElementPropagator::new(
            index,
            vec![a0, a1],
            value,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.hybrid_domain(index).contains(0));
        assert!(engine.hybrid_domain(index).contains(1));
    }
}
