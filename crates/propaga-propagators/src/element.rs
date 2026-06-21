use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `value == array[index]` with bound consistency.
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
    let mut changed = false;
    if let Some(value) = ctx.fixed_value(left) {
        if ctx.remove_below(right, value) {
            changed = true;
        }
        if ctx.remove_above(right, value) {
            changed = true;
        }
        for candidate in domain_values(ctx, right) {
            if candidate != value && ctx.remove_value(right, candidate) {
                changed = true;
            }
        }
    }
    if let Some(value) = ctx.fixed_value(right) {
        if ctx.remove_below(left, value) {
            changed = true;
        }
        if ctx.remove_above(left, value) {
            changed = true;
        }
        for candidate in domain_values(ctx, left) {
            if candidate != value && ctx.remove_value(left, candidate) {
                changed = true;
            }
        }
    }

    if let (Some(l_min), Some(l_max)) = (ctx.domain(left).min(), ctx.domain(left).max()) {
        if ctx.remove_below(right, l_min) {
            changed = true;
        }
        if ctx.remove_above(right, l_max) {
            changed = true;
        }
    }
    if let (Some(r_min), Some(r_max)) = (ctx.domain(right).min(), ctx.domain(right).max()) {
        if ctx.remove_below(left, r_min) {
            changed = true;
        }
        if ctx.remove_above(left, r_max) {
            changed = true;
        }
    }

    changed
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
        assert_eq!(engine.domain(value).min(), Some(10));
        assert_eq!(engine.domain(value).max(), Some(20));
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
        assert_eq!(engine.domain(index).fixed_value(), Some(2));
    }
}
