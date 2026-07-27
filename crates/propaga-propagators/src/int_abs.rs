use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `b = |a|` with bound consistency.
#[derive(Clone, Debug)]
pub struct IntAbsPropagator {
    watched: [VariableId; 2],
}

impl IntAbsPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId) -> Self {
        Self { watched: [a, b] }
    }
}

fn abs_image_bounds(amin: i32, amax: i32) -> (i32, i32) {
    if amin >= 0 {
        (amin, amax)
    } else if amax <= 0 {
        (-amax, -amin)
    } else {
        (0, amin.unsigned_abs().max(amax.unsigned_abs()) as i32)
    }
}

impl Propagator for IntAbsPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [a, b] = self.watched;
        let (Some(amin), Some(amax), Some(bmin), Some(bmax)) = (
            ctx.domain(a).min(),
            ctx.domain(a).max(),
            ctx.domain(b).min(),
            ctx.domain(b).max(),
        ) else {
            return PropagationStatus::Failure;
        };

        let mut changed = false;
        // |a| ≥ 0
        changed |= ctx.remove_below(b, 0);

        let (image_min, image_max) = abs_image_bounds(amin, amax);
        if image_min > image_max {
            return PropagationStatus::Failure;
        }
        changed |= ctx.remove_below(b, image_min);
        changed |= ctx.remove_above(b, image_max);

        // a ∈ [-b.max, b.max]
        let bmax = ctx.domain(b).max().unwrap_or(bmax);
        let bmin = ctx.domain(b).min().unwrap_or(bmin);
        changed |= ctx.remove_below(a, -bmax);
        changed |= ctx.remove_above(a, bmax);

        let (Some(amin), Some(amax)) = (ctx.domain(a).min(), ctx.domain(a).max()) else {
            return PropagationStatus::Failure;
        };

        if amin >= 0 {
            // b = a
            changed |= ctx.remove_below(a, bmin);
            changed |= ctx.remove_above(a, bmax);
            changed |= ctx.remove_below(b, amin);
            changed |= ctx.remove_above(b, amax);
        } else if amax <= 0 {
            // b = -a  ⇒  a = -b
            changed |= ctx.remove_below(a, -bmax);
            changed |= ctx.remove_above(a, -bmin);
            changed |= ctx.remove_below(b, -amax);
            changed |= ctx.remove_above(b, -amin);
        }

        if ctx.domain(a).is_empty() || ctx.domain(b).is_empty() {
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
    fn abs_of_negative_interval() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(-5, -2));
        let b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(IntAbsPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(b).min(), Some(2));
        assert_eq!(engine.hybrid_domain(b).max(), Some(5));
    }

    #[test]
    fn abs_crossing_zero() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(-3, 4));
        let b = engine.new_variable(IntervalDomain::new(-10, 10));
        engine.add_propagator(Box::new(IntAbsPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(b).min(), Some(0));
        assert_eq!(engine.hybrid_domain(b).max(), Some(4));
    }

    #[test]
    fn abs_equates_when_nonnegative() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(2, 5));
        let b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(IntAbsPropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(b).min(), Some(2));
        assert_eq!(engine.hybrid_domain(b).max(), Some(5));
    }
}
