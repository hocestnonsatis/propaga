use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left <= right` using bound consistency.
#[derive(Clone)]
pub struct LessEqualPropagator {
    watched: [VariableId; 2],
}

impl LessEqualPropagator {
    /// Creates a propagator for `left <= right`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for LessEqualPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right] = self.watched;
        let mut changed = false;

        if let (Some(lmin), Some(rmax)) = (ctx.domain(left).min(), ctx.domain(right).max())
            && lmin > rmax
        {
            return PropagationStatus::Failure;
        }

        if let Some(rmax) = ctx.domain(right).max()
            && ctx.remove_above(left, rmax)
        {
            changed = true;
        }

        if let Some(lmin) = ctx.domain(left).min()
            && ctx.remove_below(right, lmin)
        {
            changed = true;
        }

        if let Some(lfixed) = ctx.fixed_value(left)
            && ctx.remove_below(right, lfixed)
        {
            changed = true;
        }

        if let Some(rfixed) = ctx.fixed_value(right)
            && ctx.remove_above(left, rfixed)
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
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).max(), Some(4));
    }

    #[test]
    fn fixed_left_tightens_right_lower_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(6));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).min(), Some(6));
    }

    #[test]
    fn fixed_left_tightens_right_when_bounds_already_satisfied() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(6, 6));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
        assert_eq!(engine.hybrid_domain(right).min(), Some(6));
    }

    #[test]
    fn fixed_right_tightens_left_when_bounds_already_satisfied() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::new(4, 4));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
        assert_eq!(engine.hybrid_domain(left).max(), Some(4));
    }

    #[test]
    fn empty_domain_returns_failure() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 0));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn fixed_literals_mark_changed_via_engine() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
        assert_eq!(engine.hybrid_domain(left).max(), Some(1));
    }

    #[test]
    fn propagation_empties_domain_returns_failure() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(5, 6));
        let right = engine.new_variable(IntervalDomain::fix(2));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn disjoint_bounds_fail() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(8, 10));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn fixed_left_successfully_tightens_right() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(2));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let mut prop = LessEqualPropagator::new(left, right);
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(engine.hybrid_domain(right).min().unwrap() >= 2);
    }

    #[test]
    fn fixed_left_no_change_when_right_already_tight() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::new(5, 10));
        let mut prop = LessEqualPropagator::new(left, right);
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn fixed_right_no_change_when_left_already_tight() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(5));
        let mut prop = LessEqualPropagator::new(left, right);
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn direct_propagation_empties_domain_fails() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(5, 6));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let mut prop = LessEqualPropagator::new(left, right);
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::fix(5));
        engine.add_propagator(Box::new(LessEqualPropagator::new(left, right)));

        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn mock_fixed_left_marks_changed() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(left, 2);
        let mut prop = LessEqualPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(ctx.domains[&right].values.borrow().iter().all(|&v| v >= 2));
    }

    #[test]
    fn mock_fixed_right_marks_changed() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![1, 2, 3])
            .with_fixed(right, 4);
        let mut prop = LessEqualPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(!ctx.domains[&left].values.borrow().contains(&5));
    }

    #[test]
    fn mock_fixed_left_literal_path_marks_changed() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4])
            .with_domain(right, vec![5, 6, 7])
            .with_fixed(left, 6);
        let mut prop = LessEqualPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(!ctx.domains[&right].values.borrow().contains(&5));
    }

    #[test]
    fn mock_fixed_right_literal_path_marks_changed() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(right, 3);
        let mut prop = LessEqualPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(!ctx.domains[&left].values.borrow().contains(&5));
    }

    #[test]
    fn mock_fixed_left_empties_right_domain_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![2])
            .with_fixed(left, 3);
        let mut prop = LessEqualPropagator::new(left, right);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }
}
