use crate::reified::propagate_equal;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left == right` using bound consistency.
#[derive(Clone)]
pub struct EqualityPropagator {
    watched: [VariableId; 2],
}

impl EqualityPropagator {
    /// Creates an equality propagator for `left == right`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for EqualityPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right] = self.watched;
        let changed = propagate_equal(ctx, left, right);

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
    use crate::reified::propagate_equal;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_left_fixes_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(5));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(5));
    }

    #[test]
    fn bounds_are_synchronized() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 7));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).min(), Some(3));
        assert_eq!(engine.hybrid_domain(left).max(), Some(7));
        assert_eq!(engine.hybrid_domain(right).min(), Some(3));
        assert_eq!(engine.hybrid_domain(right).max(), Some(7));
    }

    #[test]
    fn fixed_right_fixes_left() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::fix(5));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).fixed_value(), Some(5));
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(4));
        let right = engine.new_variable(IntervalDomain::fix(4));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn sync_bounds_from_right_tightens_left() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 10));
        let right = engine.new_variable(IntervalDomain::new(4, 6));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).min(), Some(4));
        assert_eq!(engine.hybrid_domain(left).max(), Some(6));
    }

    #[test]
    fn sync_bounds_right_to_left_via_mock_ctx() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6, 10])
            .with_domain(right, vec![4, 5, 6]);
        let mut prop = EqualityPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domains[&left].values.borrow().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn propagate_equal_direct_helper_tightens_both_operands() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6])
            .with_domain(right, vec![4, 5, 6, 10]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![4, 5, 6]);
        assert_eq!(ctx.domain_values(right), vec![4, 5, 6]);
    }

    #[test]
    fn empty_domain_returns_failure_status() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![])
            .with_domain(right, vec![1, 2, 3]);
        let mut prop = EqualityPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn propagate_equal_no_change_returns_false() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6])
            .with_domain(right, vec![4, 5, 6]);
        assert!(!propagate_equal(&mut ctx, left, right));
    }

    #[test]
    fn mock_propagate_equal_right_only_tightens_left() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5, 6])
            .with_domain(right, vec![4, 5]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![4, 5]);
    }

    #[test]
    fn mock_propagate_equal_remove_above_left_from_right_max() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6, 9])
            .with_domain(right, vec![4, 5, 6]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![4, 5, 6]);
    }
}
