use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `reif <=> left == right` for set variables.
#[derive(Clone, Debug)]
pub struct SetEqReifPropagator {
    watched: [VariableId; 3],
}

impl SetEqReifPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for SetEqReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let (left, right) = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id))
            else {
                return PropagationStatus::Failure;
            };
            (left.clone(), right.clone())
        };

        let mut changed = false;

        let definitely_equal = left.glb == right.glb
            && left.lub == right.lub
            && left.glb.len() == left.lub.len()
            && left.glb.len() == right.lub.len();
        // Conflict when a forced member is impossible in the other set, or when
        // cardinality bounds are already disjoint.
        let definitely_ne = left.glb.iter().any(|v| !right.lub.contains(v))
            || right.glb.iter().any(|v| !left.lub.contains(v))
            || left.card_max < right.card_min
            || right.card_max < left.card_min;

        if definitely_equal {
            changed |= tighten_reif(ctx, reif_id, 1);
        }
        if definitely_ne {
            changed |= tighten_reif(ctx, reif_id, 0);
        }

        if ctx.fixed_value(reif_id) == Some(1)
            && let Some(ext) = ctx.as_extended()
        {
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

        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn tighten_reif(ctx: &mut dyn PropagationContext, reif: VariableId, value: i32) -> bool {
    let mut changed = false;
    if ctx.remove_below(reif, value) {
        changed = true;
    }
    if ctx.remove_above(reif, value) {
        changed = true;
    }
    changed
}

/// Propagates `reif <=> subset ⊆ superset`.
#[derive(Clone, Debug)]
pub struct SetSubsetReifPropagator {
    watched: [VariableId; 3],
}

impl SetSubsetReifPropagator {
    #[must_use]
    pub fn new(subset: VariableId, superset: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [subset, superset, reif],
        }
    }
}

impl Propagator for SetSubsetReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (subset_id, superset_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let (subset, superset) = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let (Some(subset), Some(superset)) =
                (ext.set_domain(subset_id), ext.set_domain(superset_id))
            else {
                return PropagationStatus::Failure;
            };
            (subset.clone(), superset.clone())
        };

        let mut changed = false;

        let sub_card_min = subset.card_min.max(subset.glb.len());
        let sup_card_max = superset.card_max.min(superset.lub.len());
        let forced_outside_sub = superset
            .glb
            .iter()
            .filter(|value| !subset.lub.contains(value))
            .count();
        // A ⊆ B ⇒ |B| ≥ |A| + |glb(B)\lub(A)|
        let violated = subset.glb.iter().any(|v| !superset.lub.contains(v))
            || sub_card_min.saturating_add(forced_outside_sub) > sup_card_max;
        let definitely_subset = subset.lub.iter().all(|v| superset.glb.contains(v));

        if violated {
            changed |= tighten_reif(ctx, reif_id, 0);
        }
        if definitely_subset {
            changed |= tighten_reif(ctx, reif_id, 1);
        }
        if ctx.domain(reif_id).is_empty() {
            return PropagationStatus::Failure;
        }

        if ctx.fixed_value(reif_id) == Some(1)
            && let Some(ext) = ctx.as_extended()
        {
            for &value in &subset.glb {
                changed |= ext.force_set_in(superset_id, value);
            }
            for value in subset.lub.clone() {
                if let Some(superset) = ext.set_domain(superset_id)
                    && !superset.lub.contains(&value)
                {
                    changed |= ext.force_set_out(subset_id, value);
                }
            }

            let (Some(subset), Some(superset)) =
                (ext.set_domain(subset_id), ext.set_domain(superset_id))
            else {
                return PropagationStatus::Failure;
            };
            if subset.is_empty() || superset.is_empty() {
                return PropagationStatus::Failure;
            }

            let forced_outside_sub = superset
                .glb
                .iter()
                .filter(|value| !subset.lub.contains(value))
                .count();
            let sub_card_min = subset.card_min.max(subset.glb.len());
            let sub_card_max = subset.card_max.min(superset.card_max).min(subset.lub.len());
            if sub_card_min > sub_card_max {
                return PropagationStatus::Failure;
            }
            if sub_card_min != subset.card_min || sub_card_max != subset.card_max {
                changed |= ext.tighten_set_cardinality(subset_id, sub_card_min, sub_card_max);
            }

            let sup_card_min = superset
                .card_min
                .max(sub_card_min.saturating_add(forced_outside_sub))
                .max(superset.glb.len());
            let sup_card_max = superset.card_max.min(superset.lub.len());
            if sup_card_min > sup_card_max {
                return PropagationStatus::Failure;
            }
            if sup_card_min != superset.card_min || sup_card_max != superset.card_max {
                changed |= ext.tighten_set_cardinality(superset_id, sup_card_min, sup_card_max);
            }
        }

        if ctx.fixed_value(reif_id) == Some(0) {
            // Fixed A fully inside forced B means A ⊆ B is inevitable.
            let subset_fixed = subset.glb.len() == subset.lub.len();
            if subset_fixed && subset.glb.iter().all(|v| superset.glb.contains(v)) {
                return PropagationStatus::Failure;
            }
        }

        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif <=> value ∈ set`.
#[derive(Clone, Debug)]
pub struct SetInReifPropagator {
    watched: [VariableId; 3],
}

impl SetInReifPropagator {
    #[must_use]
    pub fn new(value: VariableId, set: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [value, set, reif],
        }
    }
}

impl Propagator for SetInReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (value_id, set_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let set = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let Some(set) = ext.set_domain(set_id) else {
                return PropagationStatus::Failure;
            };
            set.clone()
        };

        let mut changed = false;
        let (value_min, value_max) = {
            let domain = ctx.domain(value_id);
            (domain.min(), domain.max())
        };

        if let Some(value) = ctx.fixed_value(value_id) {
            if set.glb.contains(&value) {
                if ctx.fixed_value(reif_id) == Some(0) {
                    return PropagationStatus::Failure;
                }
                changed |= tighten_reif(ctx, reif_id, 1);
            } else if !set.lub.contains(&value) {
                if ctx.fixed_value(reif_id) == Some(1) {
                    return PropagationStatus::Failure;
                }
                changed |= tighten_reif(ctx, reif_id, 0);
            } else if ctx.fixed_value(reif_id) == Some(1)
                && let Some(ext) = ctx.as_extended()
            {
                changed |= ext.force_set_in(set_id, value);
            } else if ctx.fixed_value(reif_id) == Some(0)
                && let Some(ext) = ctx.as_extended()
            {
                changed |= ext.force_set_out(set_id, value);
            }
        } else if let (Some(min), Some(max)) = (value_min, value_max) {
            let mut any_in_lub = false;
            let mut all_in_glb = true;
            let mut any_value = false;
            for value in min..=max {
                if !ctx.domain(value_id).contains(value) {
                    continue;
                }
                any_value = true;
                if set.lub.contains(&value) {
                    any_in_lub = true;
                }
                if !set.glb.contains(&value) {
                    all_in_glb = false;
                }
            }
            if !any_value {
                return PropagationStatus::Failure;
            }
            if all_in_glb {
                changed |= tighten_reif(ctx, reif_id, 1);
            } else if !any_in_lub {
                changed |= tighten_reif(ctx, reif_id, 0);
            }
        }

        if ctx.fixed_value(reif_id) == Some(1) {
            if let (Some(min), Some(max)) = (value_min, value_max) {
                for value in min..=max {
                    if ctx.domain(value_id).contains(value) && !set.lub.contains(&value) {
                        changed |= ctx.remove_value(value_id, value);
                    }
                }
            }
            if let Some(value) = ctx.fixed_value(value_id)
                && let Some(ext) = ctx.as_extended()
            {
                changed |= ext.force_set_in(set_id, value);
            }
        }

        if ctx.fixed_value(reif_id) == Some(0) {
            if let Some(value) = ctx.fixed_value(value_id) {
                if set.glb.contains(&value) {
                    return PropagationStatus::Failure;
                }
                if let Some(ext) = ctx.as_extended() {
                    changed |= ext.force_set_out(set_id, value);
                }
            } else if let (Some(min), Some(max)) = (value_min, value_max) {
                // ¬(x ∈ S) ⇒ drop values already forced into S.
                for value in min..=max {
                    if ctx.domain(value_id).contains(value) && set.glb.contains(&value) {
                        changed |= ctx.remove_value(value_id, value);
                    }
                }
                if ctx.domain(value_id).is_empty() {
                    return PropagationStatus::Failure;
                }
                // Membership is inevitable when every remaining value is forced in the set.
                let inevitable = (min..=max)
                    .all(|value| !ctx.domain(value_id).contains(value) || set.glb.contains(&value));
                if inevitable {
                    return PropagationStatus::Failure;
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
    use propaga_core::DomainView;
    use propaga_domains::{AnyDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn differing_lub_alone_is_not_definitely_unequal() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2).with_cardinality(0, 2);
        let right = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetEqReifPropagator::new(a, b, reif)));
        engine.propagate_all().unwrap();
        // Domains can still become equal (e.g. both {1,2}); reif stays unfixed.
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), None);
    }

    #[test]
    fn conflicting_glb_forces_reif_false() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(2..=3).with_cardinality(0, 2);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetEqReifPropagator::new(a, b, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn disjoint_cardinality_forces_reif_false() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4).with_cardinality(0, 1);
        let right = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetEqReifPropagator::new(a, b, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn fixed_equal_sets_force_reif_true() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetEqReifPropagator::new(a, b, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reif_true_syncs_cardinality_bounds() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let right = SetIntervalDomain::universe(1..=4).with_cardinality(1, 2);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        let reif = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(SetEqReifPropagator::new(a, b, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(a).as_set().unwrap().card_min(), 2);
        assert_eq!(engine.domain(a).as_set().unwrap().card_max(), 2);
        assert_eq!(engine.domain(b).as_set().unwrap().card_min(), 2);
        assert_eq!(engine.domain(b).as_set().unwrap().card_max(), 2);
    }

    #[test]
    fn subset_violation_forces_reif_false() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 2)
            .force_in(1)
            .unwrap();
        let superset = SetIntervalDomain::universe(2..=3).with_cardinality(0, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetSubsetReifPropagator::new(sub, sup, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn subset_disjoint_cardinality_forces_reif_false() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=4).with_cardinality(3, 4);
        let superset = SetIntervalDomain::universe(1..=4).with_cardinality(0, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetSubsetReifPropagator::new(sub, sup, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn subset_card_plus_forced_outside_forces_reif_false() {
        let mut engine = Engine::new();
        // |A|≥2 and B forces 4∉lub(A) with |B|≤2 ⇒ |B| ≥ |A|+1 is impossible.
        let subset = SetIntervalDomain::universe(1..=3).with_cardinality(2, 2);
        let superset = SetIntervalDomain::universe(1..=4)
            .with_cardinality(0, 2)
            .force_in(4)
            .unwrap();
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetSubsetReifPropagator::new(sub, sup, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn definite_subset_forces_reif_true() {
        let mut engine = Engine::new();
        // Fixed {1} with 1 ∈ glb(B) ⇒ lub(A) ⊆ glb(B).
        let subset = SetIntervalDomain::universe(1..=1)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let superset = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 3)
            .force_in(1)
            .unwrap();
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetSubsetReifPropagator::new(sub, sup, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn subset_reif_true_syncs_card_bounds() {
        let mut engine = Engine::new();
        let subset = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let superset = SetIntervalDomain::universe(1..=4).with_cardinality(0, 2);
        let sub = engine.new_variable(AnyDomain::Set(subset));
        let sup = engine.new_variable(AnyDomain::Set(superset));
        let reif = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(SetSubsetReifPropagator::new(sub, sup, reif)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(sub).as_set().unwrap().card_max() <= 2);
        assert!(engine.domain(sup).as_set().unwrap().card_min() >= 2);
    }

    #[test]
    fn set_in_reif_value_outside_lub_forces_false() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=2).with_cardinality(0, 2);
        let value = engine.new_variable(IntervalDomain::fix(3));
        let set_var = engine.new_variable(AnyDomain::Set(set));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetInReifPropagator::new(value, set_var, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn set_in_reif_value_in_glb_forces_true() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 3)
            .force_in(2)
            .unwrap();
        let value = engine.new_variable(IntervalDomain::fix(2));
        let set_var = engine.new_variable(AnyDomain::Set(set));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetInReifPropagator::new(value, set_var, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn set_in_reif_false_forces_value_out_of_set() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3).with_cardinality(0, 3);
        let value = engine.new_variable(IntervalDomain::fix(2));
        let set_var = engine.new_variable(AnyDomain::Set(set));
        let reif = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(SetInReifPropagator::new(value, set_var, reif)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(set_var).as_set().unwrap().lub().contains(&2));
    }

    #[test]
    fn set_in_reif_false_prunes_glb_from_value() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 3)
            .force_in(1)
            .unwrap();
        let value = engine.new_variable(IntervalDomain::new(1, 3));
        let set_var = engine.new_variable(AnyDomain::Set(set));
        let reif = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(SetInReifPropagator::new(value, set_var, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.hybrid_domain(value).contains(1));
        assert!(engine.hybrid_domain(value).contains(2));
        assert!(engine.hybrid_domain(value).contains(3));
    }
}
