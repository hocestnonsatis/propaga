use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left == right` for set variables.
#[derive(Clone, Debug)]
pub struct SetEqPropagator {
    watched: [VariableId; 2],
}

impl SetEqPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for SetEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id) = (self.watched[0], self.watched[1]);
        let (left, right) = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id))
            else {
                return PropagationStatus::Failure;
            };
            if left.is_empty() || right.is_empty() {
                return PropagationStatus::Failure;
            }
            (left.clone(), right.clone())
        };

        let mut changed = false;
        {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            for &value in &left.glb {
                changed |= ext.force_set_in(right_id, value);
            }
            for &value in &right.glb {
                changed |= ext.force_set_in(left_id, value);
            }
            for value in left.lub.clone() {
                if !right.lub.contains(&value) {
                    changed |= ext.force_set_out(left_id, value);
                }
            }
            for value in right.lub.clone() {
                if !left.lub.contains(&value) {
                    changed |= ext.force_set_out(right_id, value);
                }
            }

            let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id))
            else {
                return PropagationStatus::Failure;
            };
            if left.is_empty() || right.is_empty() {
                return PropagationStatus::Failure;
            }

            let card_min = left.card_min.max(right.card_min).max(left.glb.len());
            let card_max = left
                .card_max
                .min(right.card_max)
                .min(left.lub.len())
                .min(right.lub.len());
            if card_min > card_max {
                return PropagationStatus::Failure;
            }
            if card_min != left.card_min || card_max != left.card_max {
                changed |= ext.tighten_set_cardinality(left_id, card_min, card_max);
            }
            if card_min != right.card_min || card_max != right.card_max {
                changed |= ext.tighten_set_cardinality(right_id, card_min, card_max);
            }
        }

        let (left_after, right_after) = {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            (ext.set_domain(left_id), ext.set_domain(right_id))
        };
        if left_after.as_ref().is_none_or(|d| d.is_empty())
            || right_after.as_ref().is_none_or(|d| d.is_empty())
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
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn syncs_glb_both_ways() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 3)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetEqPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(b).as_set().unwrap().glb().contains(&1));
    }

    #[test]
    fn syncs_cardinality_bounds() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let right = SetIntervalDomain::universe(1..=4).with_cardinality(1, 2);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetEqPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(a).as_set().unwrap().card_min(), 2);
        assert_eq!(engine.domain(a).as_set().unwrap().card_max(), 2);
        assert_eq!(engine.domain(b).as_set().unwrap().card_min(), 2);
        assert_eq!(engine.domain(b).as_set().unwrap().card_max(), 2);
    }

    #[test]
    fn prunes_lub_to_intersection() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let right = SetIntervalDomain::universe(2..=4).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetEqPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(a).as_set().unwrap().lub().contains(&1));
        assert!(!engine.domain(b).as_set().unwrap().lub().contains(&4));
    }
}
