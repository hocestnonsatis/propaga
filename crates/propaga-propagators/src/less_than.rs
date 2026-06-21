use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left < right` using bound consistency.
pub struct LessThanPropagator {
    watched: [VariableId; 2],
}

impl LessThanPropagator {
    /// Creates a propagator for `left < right`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for LessThanPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right] = self.watched;
        let mut changed = false;

        if let (Some(lmin), Some(rmax)) = (ctx.domain(left).min(), ctx.domain(right).max())
            && lmin >= rmax
        {
            return PropagationStatus::Failure;
        }

        if let Some(rmax) = ctx.domain(right).max()
            && ctx.remove_above(left, rmax - 1)
        {
            changed = true;
        }

        if let Some(lmin) = ctx.domain(left).min()
            && ctx.remove_below(right, lmin + 1)
        {
            changed = true;
        }

        if let Some(lfixed) = ctx.fixed_value(left)
            && ctx.remove_below(right, lfixed + 1)
        {
            changed = true;
        }

        if let Some(rfixed) = ctx.fixed_value(right)
            && ctx.remove_above(left, rfixed - 1)
        {
            changed = true;
        }

        if ctx.domain(left).is_empty() || ctx.domain(right).is_empty() {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_right_tightens_left_upper_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::fix(4));
        engine.add_propagator(Box::new(LessThanPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(left).max(), Some(3));
    }

    #[test]
    fn fixed_left_tightens_right_lower_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(6));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(LessThanPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(right).min(), Some(7));
    }

    #[test]
    fn disjoint_bounds_fail() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(8, 10));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(LessThanPropagator::new(left, right)));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }
}
