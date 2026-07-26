use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

#[derive(Clone, Debug)]
pub struct FloatEqPropagator {
    watched: [VariableId; 2],
}

impl FloatEqPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for FloatEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(left), Some(right)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        changed |= ext.tighten_float_below(self.watched[0], right.min);
        changed |= ext.tighten_float_above(self.watched[0], right.max);
        changed |= ext.tighten_float_below(self.watched[1], left.min);
        changed |= ext.tighten_float_above(self.watched[1], left.max);
        let left_after = ext
            .float_domain(self.watched[0])
            .unwrap_or_else(|| left.clone());
        let right_after = ext
            .float_domain(self.watched[1])
            .unwrap_or_else(|| right.clone());
        if left_after.is_empty() || right_after.is_empty() {
            return PropagationStatus::Failure;
        }
        // Equality shares excluded IEEE points inside the common interval.
        for hole in &left_after.holes {
            changed |= ext.exclude_float_point(self.watched[1], *hole);
        }
        for hole in &right_after.holes {
            changed |= ext.exclude_float_point(self.watched[0], *hole);
        }
        if ext
            .float_domain(self.watched[0])
            .is_some_and(|domain| domain.is_empty())
            || ext
                .float_domain(self.watched[1])
                .is_some_and(|domain| domain.is_empty())
        {
            return PropagationStatus::Failure;
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::{AnyDomain, FloatDomain};
    use propaga_engine::Engine;

    #[test]
    fn propagates_float_eq_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(3.0, 7.0)));
        engine.add_propagator(Box::new(FloatEqPropagator::new(left, right)));
        engine.propagate_all().unwrap();
        let left_domain = engine.domain(left).as_float().unwrap();
        let right_domain = engine.domain(right).as_float().unwrap();
        assert_eq!(left_domain.lower_bound(), 3.0);
        assert_eq!(left_domain.upper_bound(), 7.0);
        assert_eq!(right_domain.lower_bound(), 3.0);
        assert_eq!(right_domain.upper_bound(), 7.0);
    }

    #[test]
    fn already_equal_no_change() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(4.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(4.0)));
        engine.add_propagator(Box::new(FloatEqPropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn disjoint_float_intervals_fail() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 10.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 4.0)));
        engine.add_propagator(Box::new(FloatEqPropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn shares_interior_holes_under_equality() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatEqPropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(right).as_float().unwrap().contains(1.0));
        assert_eq!(engine.domain(right).as_float().unwrap().holes(), &[1.0]);
    }

    #[test]
    fn no_extended_context_returns_ok_no_change() {
        use crate::test_support::NoExtendedCtx;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let _ = engine.new_variable(IntervalDomain::new(1, 5));
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let mut prop = FloatEqPropagator::new(left, right);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(FloatEqPropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }
}
