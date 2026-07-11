use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates set cardinality bounds already stored in the domain.
#[derive(Clone, Debug)]
pub struct SetCardPropagator {
    var: VariableId,
}

impl SetCardPropagator {
    #[must_use]
    pub fn new(var: VariableId) -> Self {
        Self { var }
    }
}

impl Propagator for SetCardPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        std::slice::from_ref(&self.var)
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let Some(domain) = ext.set_domain(self.var) else {
            return PropagationStatus::Failure;
        };
        if domain.is_empty() {
            return PropagationStatus::Failure;
        }
        if domain.glb.len() == domain.card_max {
            let mut changed = false;
            for value in domain.undecided() {
                changed |= ext.force_set_out(self.var, value);
            }
            return if changed {
                PropagationStatus::OkChanged
            } else {
                PropagationStatus::OkNoChange
            };
        }
        if domain.lub.len() == domain.card_min {
            let mut changed = false;
            for value in domain.undecided() {
                changed |= ext.force_set_in(self.var, value);
            }
            return if changed {
                PropagationStatus::OkChanged
            } else {
                PropagationStatus::OkNoChange
            };
        }
        PropagationStatus::OkNoChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn fixes_set_when_card_equals_lub_size() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3).with_cardinality(3, 3);
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        engine.propagate_all().unwrap();
        let fixed = engine.domain(var).as_set().unwrap().fixed_values().unwrap();
        assert_eq!(fixed, vec![1, 2, 3]);
    }

    #[test]
    fn forces_out_when_at_card_max() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        engine.propagate_all().unwrap();
        let domain = engine.domain(var).as_set().unwrap();
        assert!(!domain.lub().contains(&3));
        assert_eq!(domain.fixed_values(), Some(vec![1, 2]));
    }

    #[test]
    fn forces_in_when_at_card_min() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2).with_cardinality(2, 2);
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        engine.propagate_all().unwrap();
        assert_eq!(
            engine.domain(var).as_set().unwrap().fixed_values(),
            Some(vec![1, 2])
        );
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap();
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn no_extended_context_returns_ok_no_change() {
        use crate::test_support::NoExtendedCtx;
        use propaga_domains::{IntervalDomain, SetIntervalDomain};

        let mut engine = Engine::new();
        let _ = engine.new_variable(IntervalDomain::new(1, 5));
        let set = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let var = engine.new_variable(AnyDomain::Set(set));
        let mut prop = SetCardPropagator::new(var);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn forces_out_already_at_max_no_change() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn empty_set_domain_fails_immediately() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2).with_cardinality(3, 2);
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn forces_out_already_satisfied_returns_no_change() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn forces_in_with_all_decided_returns_no_change() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn inconsistent_cardinality_domain_fails() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2).with_cardinality(3, 2);
        let var = engine.new_variable(AnyDomain::Set(set));
        engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn mock_empty_set_snapshot_fails() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut prop = SetCardPropagator::new(var);
        let mut ctx = MockSetCtx::new().with_set(
            var,
            SetDomainSnapshot {
                glb: vec![],
                lub: vec![],
                card_min: 5,
                card_max: 2,
            },
        );
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_at_card_max_forces_undecided_out() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut prop = SetCardPropagator::new(var);
        let mut ctx = MockSetCtx::new().with_set(
            var,
            SetDomainSnapshot {
                glb: vec![1, 2],
                lub: vec![1, 2, 3],
                card_min: 2,
                card_max: 2,
            },
        );
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(!ctx.sets[&var].lub.contains(&3));
    }

    #[test]
    fn mock_at_card_min_forces_undecided_in() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut prop = SetCardPropagator::new(var);
        let mut ctx = MockSetCtx::new().with_set(
            var,
            SetDomainSnapshot {
                glb: vec![],
                lub: vec![1, 2],
                card_min: 2,
                card_max: 2,
            },
        );
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.sets[&var].glb, vec![1, 2]);
    }

    #[test]
    fn mock_at_card_min_all_decided_is_no_change() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut prop = SetCardPropagator::new(var);
        let mut ctx = MockSetCtx::new().with_set(
            var,
            SetDomainSnapshot {
                glb: vec![1, 2],
                lub: vec![1, 2],
                card_min: 2,
                card_max: 3,
            },
        );
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }
}
