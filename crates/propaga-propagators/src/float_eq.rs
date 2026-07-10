use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator, VariableId,
};

#[derive(Clone, Debug)]
pub struct FloatEqPropagator {
    watched: [VariableId; 2],
}

impl FloatEqPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for FloatEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(left), Some(right)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        changed |= ext.tighten_float_below(self.watched[0], right.min);
        changed |= ext.tighten_float_above(self.watched[0], right.max);
        changed |= ext.tighten_float_below(self.watched[1], left.min);
        changed |= ext.tighten_float_above(self.watched[1], left.max);
        let left_after = ext.float_domain(self.watched[0]).unwrap_or(left);
        let right_after = ext.float_domain(self.watched[1]).unwrap_or(right);
        if left_after.is_empty() || right_after.is_empty() {
            return PropagationStatus::Failure;
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}
