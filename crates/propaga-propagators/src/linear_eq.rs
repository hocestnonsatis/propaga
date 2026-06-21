use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left + right == result` using bound consistency.
pub struct LinearEqPropagator {
    watched: [VariableId; 3],
}

impl LinearEqPropagator {
    /// Creates a propagator for `left + right == result`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, result: VariableId) -> Self {
        Self {
            watched: [left, right, result],
        }
    }
}

impl Propagator for LinearEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right, result] = self.watched;
        let mut changed = false;

        changed |= propagate_sum_bounds(ctx, left, right, result);
        changed |= propagate_from_result(ctx, left, right, result);

        if [left, right, result]
            .into_iter()
            .any(|var| ctx.domain(var).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn propagate_sum_bounds(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
    result: VariableId,
) -> bool {
    let mut changed = false;

    if let (Some(lmin), Some(lmax), Some(rmin), Some(rmax)) = (
        ctx.domain(left).min(),
        ctx.domain(left).max(),
        ctx.domain(right).min(),
        ctx.domain(right).max(),
    ) {
        if ctx.remove_below(result, lmin + rmin) {
            changed = true;
        }
        if ctx.remove_above(result, lmax + rmax) {
            changed = true;
        }
    }

    if let (Some(rmin), Some(rmin2), Some(smin), Some(smax)) = (
        ctx.domain(right).min(),
        ctx.domain(right).max(),
        ctx.domain(result).min(),
        ctx.domain(result).max(),
    ) {
        if ctx.remove_below(left, smin - rmin2) {
            changed = true;
        }
        if ctx.remove_above(left, smax - rmin) {
            changed = true;
        }
    }

    if let (Some(lmin), Some(lmax), Some(smin), Some(smax)) = (
        ctx.domain(left).min(),
        ctx.domain(left).max(),
        ctx.domain(result).min(),
        ctx.domain(result).max(),
    ) {
        if ctx.remove_below(right, smin - lmax) {
            changed = true;
        }
        if ctx.remove_above(right, smax - lmin) {
            changed = true;
        }
    }

    changed
}

fn propagate_from_result(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
    result: VariableId,
) -> bool {
    let mut changed = false;

    if let (Some(sum), Some(rfixed)) = (ctx.fixed_value(result), ctx.fixed_value(right)) {
        let target = sum - rfixed;
        if ctx.remove_below(left, target) {
            changed = true;
        }
        if ctx.remove_above(left, target) {
            changed = true;
        }
    }

    if let (Some(sum), Some(lfixed)) = (ctx.fixed_value(result), ctx.fixed_value(left)) {
        let target = sum - lfixed;
        if ctx.remove_below(right, target) {
            changed = true;
        }
        if ctx.remove_above(right, target) {
            changed = true;
        }
    }

    if let (Some(lfixed), Some(rfixed)) = (ctx.fixed_value(left), ctx.fixed_value(right)) {
        let target = lfixed + rfixed;
        if ctx.remove_below(result, target) {
            changed = true;
        }
        if ctx.remove_above(result, target) {
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_operands_fix_result() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::fix(4));
        let result = engine.new_variable(IntervalDomain::new(0, 20));
        engine.add_propagator(Box::new(LinearEqPropagator::new(left, right, result)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(result).fixed_value(), Some(7));
    }

    #[test]
    fn bounds_propagate_to_result() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(2, 4));
        let result = engine.new_variable(IntervalDomain::new(0, 20));
        engine.add_propagator(Box::new(LinearEqPropagator::new(left, right, result)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(result).min(), Some(3));
        assert_eq!(engine.domain(result).max(), Some(7));
    }
}
