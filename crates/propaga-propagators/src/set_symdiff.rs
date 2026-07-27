use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `result = left △ right` (symmetric difference).
#[derive(Clone, Debug)]
pub struct SetSymDiffPropagator {
    watched: [VariableId; 3],
}

impl SetSymDiffPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, result: VariableId) -> Self {
        Self {
            watched: [left, right, result],
        }
    }
}

impl Propagator for SetSymDiffPropagator {
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

        // x ∈ R ⇒ x in exactly one of A, B.
        for value in result.glb.clone() {
            if left.glb.contains(&value) {
                changed |= ext.force_set_out(right_id, value);
            } else if right.glb.contains(&value) {
                changed |= ext.force_set_out(left_id, value);
            } else if !left.lub.contains(&value) {
                changed |= ext.force_set_in(right_id, value);
            } else if !right.lub.contains(&value) {
                changed |= ext.force_set_in(left_id, value);
            }
        }

        // x ∈ A ∩ B ⇒ x ∉ R; exclusive forced members enter R.
        for value in left.glb.clone() {
            if right.glb.contains(&value) {
                changed |= ext.force_set_out(result_id, value);
            } else if !right.lub.contains(&value) {
                changed |= ext.force_set_in(result_id, value);
            }
        }
        for value in right.glb.clone() {
            if left.glb.contains(&value) {
                changed |= ext.force_set_out(result_id, value);
            } else if !left.lub.contains(&value) {
                changed |= ext.force_set_in(result_id, value);
            }
        }

        // x ∉ A ∪ B ⇒ x ∉ R.
        for value in result.lub.clone() {
            if !left.lub.contains(&value) && !right.lub.contains(&value) {
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

        // x ∉ R ⇒ A and B agree on x (both in or both out).
        for value in left.glb.clone() {
            if !result.lub.contains(&value) {
                changed |= ext.force_set_in(right_id, value);
            }
        }
        for value in right.glb.clone() {
            if !result.lub.contains(&value) {
                changed |= ext.force_set_in(left_id, value);
            }
        }
        for value in left.lub.clone() {
            if !result.lub.contains(&value) && !right.lub.contains(&value) {
                changed |= ext.force_set_out(left_id, value);
            }
        }
        for value in right.lub.clone() {
            if !result.lub.contains(&value) && !left.lub.contains(&value) {
                changed |= ext.force_set_out(right_id, value);
            }
        }

        // Re-apply exclusive membership after agreement rules.
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
        for value in result.glb.clone() {
            if left.glb.contains(&value) {
                changed |= ext.force_set_out(right_id, value);
            } else if right.glb.contains(&value) {
                changed |= ext.force_set_out(left_id, value);
            } else if !left.lub.contains(&value) {
                changed |= ext.force_set_in(right_id, value);
            } else if !right.lub.contains(&value) {
                changed |= ext.force_set_in(left_id, value);
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

        // |R| ≥ |glb(A)\lub(B)| + |glb(B)\lub(A)|;
        // |R| ≤ |lub(A)\glb(B)| + |lub(B)\glb(A)|;
        // |R| ≥ ||A|−|B|| and |R| ≤ |A|+|B|.
        let exclusive_glb = {
            let mut count = 0usize;
            for value in &left.glb {
                if !right.lub.contains(value) {
                    count += 1;
                }
            }
            for value in &right.glb {
                if !left.lub.contains(value) {
                    count += 1;
                }
            }
            count
        };
        let exclusive_lub = {
            let mut count = left
                .lub
                .iter()
                .filter(|value| !right.glb.contains(value))
                .count();
            count += right
                .lub
                .iter()
                .filter(|value| !left.glb.contains(value) && !left.lub.contains(value))
                .count();
            count
        };

        let result_card_min = result
            .card_min
            .max(result.glb.len())
            .max(exclusive_glb)
            .max(left.card_min.saturating_sub(right.card_max))
            .max(right.card_min.saturating_sub(left.card_max));
        let result_card_max = result
            .card_max
            .min(left.card_max.saturating_add(right.card_max))
            .min(exclusive_lub)
            .min(result.lub.len());
        if result_card_min > result_card_max {
            return PropagationStatus::Failure;
        }
        if result_card_min != result.card_min || result_card_max != result.card_max {
            changed |= ext.tighten_set_cardinality(result_id, result_card_min, result_card_max);
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
    fn forces_exclusive_glb_into_result() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
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
        engine.add_propagator(Box::new(SetSymDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().glb().contains(&1));
    }

    #[test]
    fn forces_shared_glb_out_of_result() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetSymDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(r).as_set().unwrap().lub().contains(&2));
    }

    #[test]
    fn raises_result_card_min_from_exclusive_glbs() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap()
            .force_out(3)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=4)
            .with_cardinality(1, 2)
            .force_in(3)
            .unwrap()
            .force_out(1)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=4).with_cardinality(0, 4);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetSymDiffPropagator::new(a, b, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().card_min() >= 2);
    }
}
