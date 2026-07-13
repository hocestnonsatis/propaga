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

        let reif_domain = ctx.domain(reif_id);
        let mut changed = false;

        let definitely_equal = left.glb == right.glb
            && left.lub == right.lub
            && left.glb.len() == left.lub.len()
            && left.glb.len() == right.lub.len();
        let definitely_ne = left.glb != right.glb
            || left.lub != right.lub
            || left.glb.iter().any(|v| !right.lub.contains(v))
            || right.glb.iter().any(|v| !left.lub.contains(v));

        if definitely_equal {
            if reif_domain.is_fixed() && ctx.fixed_value(reif_id) != Some(1) {
                return PropagationStatus::Failure;
            }
            if !reif_domain.contains(1) {
                return PropagationStatus::Failure;
            }
        }
        if definitely_ne {
            if reif_domain.is_fixed() && ctx.fixed_value(reif_id) == Some(1) {
                return PropagationStatus::Failure;
            }
            if reif_domain.is_fixed() && !reif_domain.contains(0) {
                return PropagationStatus::Failure;
            }
        }

        if ctx.fixed_value(reif_id) == Some(1) {
            if let Some(ext) = ctx.as_extended() {
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
            }
        }

        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
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
        if ctx.fixed_value(reif_id) == Some(1) {
            if let Some(ext) = ctx.as_extended() {
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
        }

        if let Some(ext) = ctx.as_extended() {
            if let Some(superset) = ext.set_domain(superset_id) {
                let violated = subset.glb.iter().any(|v| !superset.lub.contains(v));
                if violated && ctx.fixed_value(reif_id) == Some(1) {
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

        if ctx.fixed_value(reif_id) == Some(1) {
            if let Some(value) = ctx.fixed_value(value_id) {
                if let Some(ext) = ctx.as_extended() {
                    changed |= ext.force_set_in(set_id, value);
                }
            }
        }

        if ctx.fixed_value(reif_id) == Some(0) {
            if let Some(value) = ctx.fixed_value(value_id) {
                if set.glb.contains(&value) {
                    return PropagationStatus::Failure;
                }
            }
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
