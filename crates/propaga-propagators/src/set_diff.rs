use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `result = left \\ right`.
#[derive(Clone, Debug)]
pub struct SetDiffPropagator {
    watched: [VariableId; 3],
}

impl SetDiffPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, result: VariableId) -> Self {
        Self {
            watched: [left, right, result],
        }
    }
}

impl Propagator for SetDiffPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(left), Some(right), Some(result)) = (
            ext.set_domain(self.watched[0]),
            ext.set_domain(self.watched[1]),
            ext.set_domain(self.watched[2]),
        ) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() || result.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        let left_id = self.watched[0];
        let right_id = self.watched[1];
        let result_id = self.watched[2];

        // R ⊆ A: forced members of R enter A and leave B.
        for value in result.glb.clone() {
            changed |= ext.force_set_in(left_id, value);
            changed |= ext.force_set_out(right_id, value);
        }

        // R ∩ B = ∅: forced members of B leave R.
        for value in right.glb.clone() {
            changed |= ext.force_set_out(result_id, value);
        }

        // x ∈ A ∧ x ∉ B ⇒ x ∈ R.
        for value in left.glb.clone() {
            if !right.lub.contains(&value) {
                changed |= ext.force_set_in(result_id, value);
            }
        }

        // R ⊆ A: values outside lub(A) leave R.
        for value in result.lub.clone() {
            if !left.lub.contains(&value) {
                changed |= ext.force_set_out(result_id, value);
            }
        }

        let (Some(left), Some(right), Some(result)) = (
            ext.set_domain(left_id),
            ext.set_domain(right_id),
            ext.set_domain(result_id),
        ) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() || result.is_empty() {
            return PropagationStatus::Failure;
        }

        // A ⊆ R ∪ B: x ∈ A ∧ x ∉ R ⇒ x ∈ B; x ∉ R ∧ x ∉ B ⇒ x ∉ A.
        for value in left.glb.clone() {
            if !result.lub.contains(&value) {
                changed |= ext.force_set_in(right_id, value);
            }
        }
        for value in left.lub.clone() {
            if !result.lub.contains(&value) && !right.lub.contains(&value) {
                changed |= ext.force_set_out(left_id, value);
            }
        }

        let (Some(left), Some(right), Some(result)) = (
            ext.set_domain(left_id),
            ext.set_domain(right_id),
            ext.set_domain(result_id),
        ) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() || result.is_empty() {
            return PropagationStatus::Failure;
        }

        // |R| ≤ |lub(A) \ glb(B)|; |R| ≥ |A| − |B| and |glb(A) \ lub(B)|;
        // A ⊆ R ∪ B ⇒ |A| ≤ |R| + |B|; R ⊆ A ⇒ |A| ≥ |R| + |glb(A) \ lub(R)|.
        let left_minus_right_max = left
            .lub
            .iter()
            .filter(|value| !right.glb.contains(value))
            .count();
        let glb_diff = left
            .glb
            .iter()
            .filter(|value| !right.lub.contains(value))
            .count();
        let left_forced_outside_result = left
            .glb
            .iter()
            .filter(|value| !result.lub.contains(value))
            .count();

        let result_card_min = result
            .card_min
            .max(result.glb.len())
            .max(glb_diff)
            .max(left.card_min.saturating_sub(right.card_max));
        let result_card_max = result
            .card_max
            .min(left.card_max)
            .min(left_minus_right_max)
            .min(result.lub.len());
        if result_card_min > result_card_max {
            return PropagationStatus::Failure;
        }
        if result_card_min != result.card_min || result_card_max != result.card_max {
            changed |= ext.tighten_set_cardinality(result_id, result_card_min, result_card_max);
        }

        let left_card_min = left
            .card_min
            .max(result_card_min.saturating_add(left_forced_outside_result))
            .max(left.glb.len());
        let left_card_max = left
            .card_max
            .min(result_card_max.saturating_add(right.card_max))
            .min(left.lub.len());
        if left_card_min > left_card_max {
            return PropagationStatus::Failure;
        }
        if left_card_min != left.card_min || left_card_max != left.card_max {
            changed |= ext.tighten_set_cardinality(left_id, left_card_min, left_card_max);
        }

        let right_card_min = right
            .card_min
            .max(left_forced_outside_result)
            .max(right.glb.len());
        let right_card_max = right.card_max.min(right.lub.len());
        if right_card_min > right_card_max {
            return PropagationStatus::Failure;
        }
        if right_card_min != right.card_min || right_card_max != right.card_max {
            changed |= ext.tighten_set_cardinality(right_id, right_card_min, right_card_max);
        }

        let left_after = ext.set_domain(left_id);
        let right_after = ext.set_domain(right_id);
        let result_after = ext.set_domain(result_id);
        if left_after.as_ref().is_none_or(|d| d.is_empty())
            || right_after.as_ref().is_none_or(|d| d.is_empty())
            || result_after.as_ref().is_none_or(|d| d.is_empty())
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
    fn forces_left_only_members_into_result() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 3)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(0, 2)
            .force_out(1)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().glb().contains(&1));
    }

    #[test]
    fn forces_right_glb_out_of_result() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(r).as_set().unwrap().lub().contains(&2));
    }

    #[test]
    fn raises_result_card_min_from_glb_diff() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4)
            .with_cardinality(2, 3)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=4)
            .with_cardinality(0, 2)
            .force_out(1)
            .unwrap()
            .force_out(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=4).with_cardinality(0, 4);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().card_min() >= 2);
    }

    #[test]
    fn lowers_result_card_max_from_left_minus_right() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        // lub(A)\glb(B) ⊆ {3}
        assert!(engine.domain(r).as_set().unwrap().card_max() <= 1);
    }
}
