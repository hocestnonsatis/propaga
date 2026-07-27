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
        // Conflict only when a forced member is impossible in the other set.
        let definitely_ne = left.glb.iter().any(|v| !right.lub.contains(v))
            || right.glb.iter().any(|v| !left.lub.contains(v));

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
        let subset = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let Some(subset) = ext.set_domain(subset_id) else {
                return PropagationStatus::Failure;
            };
            subset.clone()
        };

        let mut changed = false;
        if ctx.fixed_value(reif_id) == Some(1)
            && let Some(ext) = ctx.as_extended()
        {
            for &value in &subset.glb {
                changed |= ext.force_set_in(superset_id, value);
            }
            if let Some(superset) = ext.set_domain(superset_id) {
                for value in subset.lub.clone() {
                    if !superset.lub.contains(&value) {
                        changed |= ext.force_set_out(subset_id, value);
                    }
                }
            }
        }

        if let Some(ext) = ctx.as_extended()
            && let Some(superset) = ext.set_domain(superset_id)
        {
            let violated = subset.glb.iter().any(|v| !superset.lub.contains(v));
            if violated && ctx.fixed_value(reif_id) == Some(1) {
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
        if let Some(value) = ctx.fixed_value(value_id) {
            if !set.lub.contains(&value) {
                return PropagationStatus::Failure;
            }
            if ctx.fixed_value(reif_id) == Some(0) {
                return PropagationStatus::Failure;
            }
            if let Some(ext) = ctx.as_extended() {
                changed |= ext.force_set_in(set_id, value);
            }
        }

        if ctx.fixed_value(reif_id) == Some(1)
            && let Some(value) = ctx.fixed_value(value_id)
            && let Some(ext) = ctx.as_extended()
        {
            changed |= ext.force_set_in(set_id, value);
        }

        if ctx.fixed_value(reif_id) == Some(0)
            && let Some(value) = ctx.fixed_value(value_id)
            && set.glb.contains(&value)
        {
            return PropagationStatus::Failure;
        }

        if let (Some(min), Some(max)) = (ctx.domain(value_id).min(), ctx.domain(value_id).max()) {
            for value in min..=max {
                if ctx.domain(value_id).contains(value) && !set.lub.contains(&value) {
                    if ctx.fixed_value(reif_id) == Some(1) {
                        return PropagationStatus::Failure;
                    }
                    changed |= ctx.remove_value(value_id, value);
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
}
