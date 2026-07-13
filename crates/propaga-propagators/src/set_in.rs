use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `value ∈ set` for an integer variable and a set variable.
#[derive(Clone, Debug)]
pub struct SetInPropagator {
    watched: [VariableId; 2],
}

impl SetInPropagator {
    #[must_use]
    pub fn new(value: VariableId, set: VariableId) -> Self {
        Self {
            watched: [value, set],
        }
    }
}

impl Propagator for SetInPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let set = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let Some(set) = ext.set_domain(self.watched[1]) else {
                return PropagationStatus::Failure;
            };
            set.clone()
        };

        let value_id = self.watched[0];
        let set_id = self.watched[1];
        let fixed_value = ctx.fixed_value(value_id);
        let (value_min, value_max) = {
            let domain = ctx.domain(value_id);
            (domain.min(), domain.max())
        };
        let mut changed = false;

        if let Some(value) = fixed_value {
            if !set.lub.contains(&value) {
                return PropagationStatus::Failure;
            }
            if let Some(ext) = ctx.as_extended() {
                changed |= ext.force_set_in(set_id, value);
            }
        }

        if let (Some(min), Some(max)) = (value_min, value_max) {
            for value in min..=max {
                let in_lub = set.lub.contains(&value);
                let in_domain = ctx.domain(value_id).contains(value);
                if in_domain && !in_lub {
                    changed |= ctx.remove_value(value_id, value);
                }
            }
        }

        let set_is_fixed = set.glb.len() == set.lub.len();
        if !set_is_fixed {
            for &value in &set.glb {
                if !ctx.domain(value_id).contains(value)
                    && let Some(ext) = ctx.as_extended()
                {
                    changed |= ext.force_set_out(set_id, value);
                }
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
    use propaga_domains::{AnyDomain, HybridDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn fixed_value_forced_into_set() {
        let mut engine = Engine::new();
        let value = engine.new_variable(AnyDomain::Int(HybridDomain::Interval(
            IntervalDomain::new(1, 3),
        )));
        let set = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=5).with_cardinality(1, 3),
        ));
        engine.add_propagator(Box::new(SetInPropagator::new(value, set)));
        engine.fix_variable(value, 2).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.domain(set).as_set().unwrap().glb().contains(&2));
    }

    #[test]
    fn fixed_set_param_membership_allows_glb_elements() {
        let mut engine = Engine::new();
        let value = engine.new_variable(AnyDomain::Int(HybridDomain::Interval(
            IntervalDomain::new(1, 3),
        )));
        let mut set_domain = SetIntervalDomain::universe(1..=3).with_cardinality(2, 2);
        set_domain = set_domain.force_in(1).unwrap().force_in(3).unwrap();
        let set = engine.new_variable(AnyDomain::Set(set_domain));
        engine.add_propagator(Box::new(SetInPropagator::new(value, set)));
        engine.propagate_all().unwrap();
        engine.fix_variable(value, 1).unwrap();
        let status = engine.propagate_all().unwrap();
        assert!(!status.is_failure());
    }
}
