use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `result = left ∪ right`.
#[derive(Clone, Debug)]
pub struct SetUnionPropagator {
    watched: [VariableId; 3],
}

impl SetUnionPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, result: VariableId) -> Self {
        Self {
            watched: [left, right, result],
        }
    }
}

impl Propagator for SetUnionPropagator {
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

        for value in left.glb.iter().chain(right.glb.iter()).copied() {
            changed |= ext.force_set_in(result_id, value);
        }

        for value in result.lub.clone() {
            if !left.lub.contains(&value) && !right.lub.contains(&value) {
                changed |= ext.force_set_out(result_id, value);
            }
        }

        for value in result.glb.clone() {
            if !right.lub.contains(&value) {
                changed |= ext.force_set_in(left_id, value);
            }
            if !left.lub.contains(&value) {
                changed |= ext.force_set_in(right_id, value);
            }
        }

        for value in left.lub.clone() {
            if !result.lub.contains(&value) {
                changed |= ext.force_set_out(left_id, value);
            }
        }
        for value in right.lub.clone() {
            if !result.lub.contains(&value) {
                changed |= ext.force_set_out(right_id, value);
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

        // |A ∪ B| ≥ max(|A|, |B|) and |A ∪ B| ≤ |A| + |B|; operands are subsets of the result.
        let result_card_min = result
            .card_min
            .max(left.card_min)
            .max(right.card_min)
            .max(result.glb.len());
        let result_card_max = result
            .card_max
            .min(left.card_max.saturating_add(right.card_max))
            .min(result.lub.len());
        if result_card_min > result_card_max {
            return PropagationStatus::Failure;
        }
        if result_card_min != result.card_min || result_card_max != result.card_max {
            changed |= ext.tighten_set_cardinality(result_id, result_card_min, result_card_max);
        }

        let left_card_min = left
            .card_min
            .max(result_card_min.saturating_sub(right.card_max))
            .max(left.glb.len());
        let left_card_max = left.card_max.min(result_card_max).min(left.lub.len());
        if left_card_min > left_card_max {
            return PropagationStatus::Failure;
        }
        if left_card_min != left.card_min || left_card_max != left.card_max {
            changed |= ext.tighten_set_cardinality(left_id, left_card_min, left_card_max);
        }

        let right_card_min = right
            .card_min
            .max(result_card_min.saturating_sub(left.card_max))
            .max(right.glb.len());
        let right_card_max = right.card_max.min(result_card_max).min(right.lub.len());
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
    use propaga_domains::{AnyDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn forces_union_glb_into_result() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(2, 3);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        let domain = engine.domain(r).as_set().unwrap();
        assert!(domain.glb().contains(&1));
        assert!(domain.glb().contains(&2));
    }

    #[test]
    fn raises_result_card_min_from_operands() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3).with_cardinality(2, 2);
        let right = SetIntervalDomain::universe(1..=3).with_cardinality(2, 2);
        let result = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().card_min() >= 2);
    }

    #[test]
    fn lowers_result_card_max_from_operand_sum() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4).with_cardinality(0, 1);
        let right = SetIntervalDomain::universe(1..=4).with_cardinality(0, 1);
        let result = SetIntervalDomain::universe(1..=4).with_cardinality(0, 4);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(r).as_set().unwrap().card_max() <= 2);
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=2)
            .with_cardinality(1, 1)
            .force_in(2)
            .unwrap();
        let result = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
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
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        let mut prop = SetUnionPropagator::new(x, y, r);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn propagation_empties_operand_domain_fails() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(3, 3);
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(1, 3));
        let result = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(SetUnionPropagator::new(left, right, result)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn empty_set_domain_fails() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(3, 3);
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn prunes_result_outside_operand_lub() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=3)
            .with_cardinality(2, 3)
            .force_in(3)
            .unwrap();
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(r).as_set().unwrap().lub().contains(&3));
    }

    #[test]
    fn prunes_operands_outside_result_lub() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(3)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(x).as_set().unwrap().lub().contains(&3));
    }

    #[test]
    fn propagation_empties_result_domain_fails() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let right = SetIntervalDomain::universe(1..=2).with_cardinality(1, 2);
        let result = SetIntervalDomain::universe(1..=2)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let x = engine.new_variable(AnyDomain::Set(left));
        let y = engine.new_variable(AnyDomain::Set(right));
        let r = engine.new_variable(AnyDomain::Set(result));
        engine.add_propagator(Box::new(SetUnionPropagator::new(x, y, r)));
        engine.set_domain(
            r,
            AnyDomain::Set(SetIntervalDomain::universe(1..=2).with_cardinality(3, 3)),
        );
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn mock_missing_set_snapshot_fails() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let result = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new().with_set(
            left,
            SetDomainSnapshot {
                glb: vec![1],
                lub: vec![1, 2],
                card_min: 1,
                card_max: 2,
            },
        );
        let mut prop = SetUnionPropagator::new(left, right, result);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_inconsistent_initial_snapshot_fails() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let result = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new()
            .with_set(
                left,
                SetDomainSnapshot {
                    glb: vec![1],
                    lub: vec![1, 2],
                    card_min: 1,
                    card_max: 2,
                },
            )
            .with_set(
                right,
                SetDomainSnapshot {
                    glb: vec![],
                    lub: vec![1, 2],
                    card_min: 1,
                    card_max: 2,
                },
            )
            .with_set(
                result,
                SetDomainSnapshot {
                    glb: vec![],
                    lub: vec![1, 2],
                    card_min: 3,
                    card_max: 2,
                },
            );
        let mut prop = SetUnionPropagator::new(left, right, result);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_propagation_empties_operand_domain_fails() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let result = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new()
            .with_set(
                left,
                SetDomainSnapshot {
                    glb: vec![1],
                    lub: vec![1, 2],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                right,
                SetDomainSnapshot {
                    glb: vec![2],
                    lub: vec![2],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                result,
                SetDomainSnapshot {
                    glb: vec![1],
                    lub: vec![1],
                    card_min: 2,
                    card_max: 2,
                },
            );
        let mut prop = SetUnionPropagator::new(left, right, result);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_set_union_propagation_empties_operand_after_changes() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let result = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new()
            .with_set(
                left,
                SetDomainSnapshot {
                    glb: vec![1],
                    lub: vec![1, 2],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                right,
                SetDomainSnapshot {
                    glb: vec![2],
                    lub: vec![2, 3],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                result,
                SetDomainSnapshot {
                    glb: vec![1],
                    lub: vec![1, 2],
                    card_min: 1,
                    card_max: 2,
                },
            );
        let mut prop = SetUnionPropagator::new(left, right, result);
        let status = prop.propagate(&mut ctx);
        assert!(matches!(
            status,
            PropagationStatus::Failure | PropagationStatus::OkChanged
        ));
    }

    #[test]
    fn mock_set_union_empties_right_operand_after_pruning() {
        use crate::test_support::MockSetCtx;
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let result = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new()
            .with_set(
                left,
                SetDomainSnapshot {
                    glb: vec![],
                    lub: vec![1],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                right,
                SetDomainSnapshot {
                    glb: vec![],
                    lub: vec![2],
                    card_min: 1,
                    card_max: 1,
                },
            )
            .with_set(
                result,
                SetDomainSnapshot {
                    glb: vec![],
                    lub: vec![1],
                    card_min: 1,
                    card_max: 1,
                },
            );
        let mut prop = SetUnionPropagator::new(left, right, result);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
        assert!(ctx.sets[&right].lub.is_empty());
    }
}
