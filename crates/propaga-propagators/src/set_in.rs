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

        // value ∈ S ⇒ value ∈ lub(S); when S is fixed, value ∈ glb(S).
        let allowed = if set.glb.len() == set.lub.len() {
            &set.glb
        } else {
            &set.lub
        };
        if let (Some(min), Some(max)) = (value_min, value_max) {
            let mut any_allowed = false;
            for value in min..=max {
                if !ctx.domain(value_id).contains(value) {
                    continue;
                }
                if allowed.contains(&value) {
                    any_allowed = true;
                } else {
                    changed |= ctx.remove_value(value_id, value);
                }
            }
            if !any_allowed {
                return PropagationStatus::Failure;
            }
        }

        if let Some(value) = ctx.fixed_value(value_id)
            && let Some(ext) = ctx.as_extended()
        {
            changed |= ext.force_set_in(set_id, value);
        }

        let set_after = {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            ext.set_domain(set_id)
        };
        if set_after.as_ref().is_none_or(|d| d.is_empty()) {
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
    use propaga_core::DomainView;
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

    #[test]
    fn forced_set_members_outside_value_domain_are_allowed() {
        let mut engine = Engine::new();
        let value = engine.new_variable(AnyDomain::Int(HybridDomain::Interval(
            IntervalDomain::new(1, 2),
        )));
        let set_domain = SetIntervalDomain::universe(1..=5)
            .with_cardinality(2, 3)
            .force_in(5)
            .unwrap();
        let set = engine.new_variable(AnyDomain::Set(set_domain));
        engine.add_propagator(Box::new(SetInPropagator::new(value, set)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(set).as_set().unwrap().glb().contains(&5));
        engine.fix_variable(value, 1).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.domain(set).as_set().unwrap().glb().contains(&5));
        assert!(engine.domain(set).as_set().unwrap().glb().contains(&1));
    }

    #[test]
    fn fixed_set_prunes_value_to_members() {
        let mut engine = Engine::new();
        let value = engine.new_variable(AnyDomain::Int(HybridDomain::Interval(
            IntervalDomain::new(1, 5),
        )));
        let set_domain = SetIntervalDomain::universe(2..=4)
            .with_cardinality(2, 2)
            .force_in(2)
            .unwrap()
            .force_in(4)
            .unwrap();
        let set = engine.new_variable(AnyDomain::Set(set_domain));
        engine.add_propagator(Box::new(SetInPropagator::new(value, set)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(value).contains(1));
        assert!(!engine.hybrid_domain(value).contains(3));
        assert!(!engine.hybrid_domain(value).contains(5));
        assert!(engine.hybrid_domain(value).contains(2));
        assert!(engine.hybrid_domain(value).contains(4));
    }
}
