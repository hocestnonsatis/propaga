use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates floating-point disequality `left != right`.
///
/// When one side is fixed, the other excludes that IEEE point (bound tighten at
/// endpoints, interior hole otherwise).
#[derive(Clone, Debug)]
pub struct FloatNePropagator {
    watched: [VariableId; 2],
}

impl FloatNePropagator {
    /// Creates a float disequality propagator.
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for FloatNePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
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
        if left.is_empty() || right.is_empty() {
            return PropagationStatus::Failure;
        }

        let left_fixed = (left.min - left.max).abs() <= f64::EPSILON && left.contains(left.min);
        let right_fixed =
            (right.min - right.max).abs() <= f64::EPSILON && right.contains(right.min);

        if left_fixed && right_fixed {
            return if (left.min - right.min).abs() <= f64::EPSILON {
                PropagationStatus::Failure
            } else {
                PropagationStatus::OkNoChange
            };
        }

        // Already separated by bounds (sound even ignoring holes).
        if left.max < right.min || right.max < left.min {
            return PropagationStatus::OkNoChange;
        }

        // Hole-aware separation when the only overlapping point is excluded on either side.
        let overlap_lo = left.min.max(right.min);
        let overlap_hi = left.max.min(right.max);
        if (overlap_hi - overlap_lo).abs() <= f64::EPSILON
            && (!left.contains(overlap_lo) || !right.contains(overlap_lo))
        {
            return PropagationStatus::OkNoChange;
        }

        let mut changed = false;
        if left_fixed {
            changed |= ext.exclude_float_point(self.watched[1], left.min);
        } else if right_fixed {
            changed |= ext.exclude_float_point(self.watched[0], right.min);
        }

        if ext
            .float_domain(self.watched[0])
            .is_some_and(|domain| domain.is_empty())
            || ext
                .float_domain(self.watched[1])
                .is_some_and(|domain| domain.is_empty())
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
    use propaga_domains::{AnyDomain, FloatDomain};
    use propaga_engine::Engine;

    #[test]
    fn fails_when_both_fixed_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn prunes_forbidden_value_at_lower_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(right).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn records_interior_hole_when_forbidden_value_is_interior() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(right).as_float().unwrap();
        assert!(!domain.contains(1.0));
        assert_eq!(domain.holes(), &[1.0]);
    }

    #[test]
    fn prunes_forbidden_value_at_upper_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(left).as_float().unwrap();
        assert!(domain.upper_bound() < 1.0);
    }

    #[test]
    fn interior_overlap_without_fixed_side_is_noop() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn singleton_overlap_excluded_by_hole_is_already_separated() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        engine.add_propagator(Box::new(FloatNePropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }
}
