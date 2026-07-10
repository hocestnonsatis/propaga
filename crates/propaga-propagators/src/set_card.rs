use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator, VariableId,
};

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
}
