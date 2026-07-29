use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Strengthens `subset ⊂ superset` (proper) via cardinality:
/// `|superset| ≥ |subset| + 1`.
///
/// Pair with [`SetSubsetPropagator`](crate::SetSubsetPropagator); equality is ruled out
/// because equal sets share a cardinality.
#[derive(Clone, Debug)]
pub struct SetLtPropagator {
    watched: [VariableId; 2],
}

impl SetLtPropagator {
    #[must_use]
    pub fn new(subset: VariableId, superset: VariableId) -> Self {
        Self {
            watched: [subset, superset],
        }
    }
}

impl Propagator for SetLtPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (sub_id, sup_id) = (self.watched[0], self.watched[1]);
        let (Some(sub), Some(sup)) = (ext.set_domain(sub_id), ext.set_domain(sup_id)) else {
            return PropagationStatus::Failure;
        };
        if sub.is_empty() || sup.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        // |B| ≥ |A| + 1 ⇒ card_min(B) ≥ card_min(A) + 1 and card_max(A) ≤ card_max(B) - 1
        let sub_card_min = sub.card_min.max(sub.glb.len());
        let sup_card_max = sup.card_max.min(sup.lub.len());
        let sub_card_max = sub
            .card_max
            .min(sub.lub.len())
            .min(sup_card_max.saturating_sub(1));
        let sup_card_min = sup
            .card_min
            .max(sup.glb.len())
            .max(sub_card_min.saturating_add(1));

        if sub_card_min > sub_card_max || sup_card_min > sup_card_max {
            return PropagationStatus::Failure;
        }
        if sub_card_min != sub.card_min || sub_card_max != sub.card_max {
            changed |= ext.tighten_set_cardinality(sub_id, sub_card_min, sub_card_max);
        }
        if let Some(sup) = ext.set_domain(sup_id) {
            let sup_card_min = sup
                .card_min
                .max(sup.glb.len())
                .max(sub_card_min.saturating_add(1));
            let sup_card_max = sup.card_max.min(sup.lub.len());
            if sup_card_min > sup_card_max {
                return PropagationStatus::Failure;
            }
            if sup_card_min != sup.card_min || sup_card_max != sup.card_max {
                changed |= ext.tighten_set_cardinality(sup_id, sup_card_min, sup_card_max);
            }
        } else {
            return PropagationStatus::Failure;
        }

        // Fixed subset and superset must grow by exactly one LUB candidate ⇒ force it in.
        if let (Some(sub), Some(sup)) = (ext.set_domain(sub_id), ext.set_domain(sup_id))
            && sub.glb.len() == sub.lub.len()
            && sup.card_min == sub.glb.len() + 1
            && sup.card_max == sub.glb.len() + 1
        {
            let extra: Vec<i32> = sup
                .lub
                .iter()
                .copied()
                .filter(|value| !sub.glb.contains(value))
                .collect();
            if extra.len() == 1 {
                changed |= ext.force_set_in(sup_id, extra[0]);
            }
        }

        let sub_after = ext.set_domain(sub_id);
        let sup_after = ext.set_domain(sup_id);
        if sub_after.as_ref().is_none_or(|d| d.is_empty())
            || sup_after.as_ref().is_none_or(|d| d.is_empty())
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
    use crate::SetSubsetPropagator;
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn raises_superset_card_min() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3).with_cardinality(1, 2);
        let superset = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.add_propagator(Box::new(SetLtPropagator::new(sub, sup)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(sup).as_set().unwrap().card_min() >= 2);
    }

    #[test]
    fn fails_when_equal_fixed_sets() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let superset = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.add_propagator(Box::new(SetLtPropagator::new(sub, sup)));
        let status = engine.propagate_all().unwrap();
        assert!(status.is_failure());
    }

    #[test]
    fn lowers_subset_card_max() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let superset = SetIntervalDomain::universe(1..=3).with_cardinality(0, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.add_propagator(Box::new(SetLtPropagator::new(sub, sup)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(sub).as_set().unwrap().card_max() <= 1);
    }

    #[test]
    fn forces_singleton_extra_into_superset_when_subset_fixed() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let superset = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap();
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        engine.add_propagator(Box::new(SetSubsetPropagator::new(sub, sup)));
        engine.add_propagator(Box::new(SetLtPropagator::new(sub, sup)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(engine.domain(sup).as_set().unwrap().glb().contains(&2));
    }
}
