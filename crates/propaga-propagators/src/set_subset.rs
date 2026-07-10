use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `subset ⊆ superset`.
#[derive(Clone, Debug)]
pub struct SetSubsetPropagator {
    watched: [VariableId; 2],
}

impl SetSubsetPropagator {
    #[must_use]
    pub fn new(subset: VariableId, superset: VariableId) -> Self {
        Self {
            watched: [subset, superset],
        }
    }
}

impl Propagator for SetSubsetPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(sub), Some(sup)) = (
            ext.set_domain(self.watched[0]),
            ext.set_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        for &value in &sub.glb {
            changed |= ext.force_set_in(self.watched[1], value);
        }
        for value in sub.lub.clone() {
            if !sup.lub.contains(&value) {
                changed |= ext.force_set_out(self.watched[0], value);
            }
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}
