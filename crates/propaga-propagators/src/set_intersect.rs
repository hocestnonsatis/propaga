use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `result = left ∩ right`.
#[derive(Clone, Debug)]
pub struct SetIntersectPropagator {
    watched: [VariableId; 3],
}

impl SetIntersectPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, result: VariableId) -> Self {
        Self {
            watched: [left, right, result],
        }
    }
}

impl Propagator for SetIntersectPropagator {
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

        for value in left.glb.iter().copied() {
            if right.lub.contains(&value) {
                changed |= ext.force_set_in(result_id, value);
            }
        }
        for value in right.glb.iter().copied() {
            if left.lub.contains(&value) {
                changed |= ext.force_set_in(result_id, value);
            }
        }

        for value in result.glb.clone() {
            changed |= ext.force_set_in(left_id, value);
            changed |= ext.force_set_in(right_id, value);
        }

        for value in result.lub.clone() {
            if !left.lub.contains(&value) || !right.lub.contains(&value) {
                changed |= ext.force_set_out(result_id, value);
            }
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
    fn fixes_intersection_when_operands_are_fixed() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 2)
            .force_in(2)
            .unwrap()
            .force_in(3)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(1, 2);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetIntersectPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        assert_eq!(
            engine.domain(r).as_set().unwrap().fixed_values(),
            Some(vec![2])
        );
    }
}
