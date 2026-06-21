use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left != right + offset`.
pub struct NotEqualOffsetPropagator {
    watched: [VariableId; 2],
    offset: i32,
}

impl NotEqualOffsetPropagator {
    /// Creates a propagator for `left != right + offset`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, offset: i32) -> Self {
        Self {
            watched: [left, right],
            offset,
        }
    }
}

impl Propagator for NotEqualOffsetPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right] = self.watched;
        let mut changed = false;

        if let Some(value) = ctx.fixed_value(right)
            && ctx.remove_value(left, value + self.offset)
        {
            changed = true;
        }

        if let Some(value) = ctx.fixed_value(left)
            && ctx.remove_value(right, value - self.offset)
        {
            changed = true;
        }

        if ctx.domain(left).is_empty() || ctx.domain(right).is_empty() {
            PropagationStatus::Failure
        } else if changed {
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
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_right_removes_conflicting_left_value() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 3));
        let right = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(NotEqualOffsetPropagator::new(left, right, 1)));

        engine.propagate_all().unwrap();
        assert!(!engine.domain(left).contains(2));
    }
}
