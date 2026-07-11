use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `subset ⊆ superset`.
#[derive(Clone, Debug)]
pub struct SetSubsetPropagator {
    watched: [VariableId; 2],
}

impl SetSubsetPropagator {
    #[must_use]
    pub fn new(subset: VariableId, superset: VariableId) -> Self {
        Self {
            watched: [subset, superset],
        }
    }
}

impl Propagator for SetSubsetPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(sub), Some(sup)) = (
            ext.set_domain(self.watched[0]),
            ext.set_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        for &value in &sub.glb {
            changed |= ext.force_set_in(self.watched[1], value);
        }
        for value in sub.lub.clone() {
            if !sup.lub.contains(&value) {
                changed |= ext.force_set_out(self.watched[0], value);
            }
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
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn glb_forced_into_superset() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap();
        let superset = SetIntervalDomain::universe(1..=3).with_cardinality(2, 3);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(sup).as_set().unwrap().glb().contains(&1));
    }

    #[test]
    fn lub_value_forced_out_of_subset() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3).with_cardinality(1, 2);
        let superset = SetIntervalDomain::universe(2..=3).with_cardinality(1, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(sub).as_set().unwrap().lub().contains(&1));
    }

    #[test]
    fn consistent_subset_no_change() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=4)
            .with_cardinality(1, 2)
            .force_in(2)
            .unwrap();
        let superset = SetIntervalDomain::universe(1..=5)
            .with_cardinality(1, 3)
            .force_in(2)
            .unwrap();
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
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
        let subset = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let superset = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let mut prop = SetSubsetPropagator::new(sub, sup);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let sub = engine.new_variable(IntervalDomain::new(1, 3));
        let sup = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }
}
