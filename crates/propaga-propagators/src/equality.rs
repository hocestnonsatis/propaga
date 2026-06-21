use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `left == right` using bound consistency.
pub struct EqualityPropagator {
    watched: [VariableId; 2],
}

impl EqualityPropagator {
    /// Creates an equality propagator for `left == right`.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for EqualityPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right] = self.watched;
        let mut changed = false;

        if let Some(value) = ctx.fixed_value(left) {
            if tighten_to_point(ctx, right, value) {
                changed = true;
            }
        }

        if let Some(value) = ctx.fixed_value(right) {
            if tighten_to_point(ctx, left, value) {
                changed = true;
            }
        }

        if sync_bounds(ctx, left, right) {
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

fn tighten_to_point(
    ctx: &mut dyn PropagationContext,
    var: VariableId,
    value: i32,
) -> bool {
    let mut changed = false;
    if ctx.remove_below(var, value) {
        changed = true;
    }
    if ctx.remove_above(var, value) {
        changed = true;
    }
    changed
}

fn sync_bounds(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let (Some(min), Some(max)) = (ctx.domain(left).min(), ctx.domain(left).max()) {
        if ctx.remove_below(right, min) {
            changed = true;
        }
        if ctx.remove_above(right, max) {
            changed = true;
        }
    }

    if let (Some(min), Some(max)) = (ctx.domain(right).min(), ctx.domain(right).max()) {
        if ctx.remove_below(left, min) {
            changed = true;
        }
        if ctx.remove_above(left, max) {
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_left_fixes_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(5));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(right).fixed_value(), Some(5));
    }

    #[test]
    fn bounds_are_synchronized() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 7));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(EqualityPropagator::new(left, right)));

        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(left).min(), Some(3));
        assert_eq!(engine.domain(left).max(), Some(7));
        assert_eq!(engine.domain(right).min(), Some(3));
        assert_eq!(engine.domain(right).max(), Some(7));
    }
}
