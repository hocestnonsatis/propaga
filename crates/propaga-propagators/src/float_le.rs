use propaga_core::{
    FloatDomainSnapshot, PropagationContext, PropagationStatus, Propagator, VariableId,
};

fn next_up(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_positive() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_add(1))
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_negative() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_sub(1))
    }
}

fn min_admissible(snap: &FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut v = snap.min;
    loop {
        if snap.contains(v) {
            return Some(v);
        }
        if v >= snap.max {
            break;
        }
        let next = next_up(v);
        if next <= v || next > snap.max {
            break;
        }
        v = next;
    }
    None
}

fn max_admissible(snap: &FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut v = snap.max;
    loop {
        if snap.contains(v) {
            return Some(v);
        }
        if v <= snap.min {
            break;
        }
        let prev = next_down(v);
        if prev >= v || prev < snap.min {
            break;
        }
        v = prev;
    }
    None
}

#[derive(Clone, Debug)]
pub struct FloatLePropagator {
    watched: [VariableId; 2],
}

impl FloatLePropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

impl Propagator for FloatLePropagator {
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
        // left ≤ right: prune against admissible supports, not raw hull endpoints.
        let max_r = max_admissible(&right).unwrap_or(right.max);
        changed |= ext.tighten_float_above(self.watched[0], max_r);
        let min_l = min_admissible(&left).unwrap_or(left.min);
        changed |= ext.tighten_float_below(self.watched[1], min_l);
        let left_after = ext
            .float_domain(self.watched[0])
            .unwrap_or_else(|| left.clone());
        let right_after = ext
            .float_domain(self.watched[1])
            .unwrap_or_else(|| right.clone());
        if left_after.is_empty() || right_after.is_empty() {
            return PropagationStatus::Failure;
        }

        // When ≤ collapses both sides to the same fixed value, share holes like
        // float_eq (including pre-tighten holes that landed on the new bound).
        if left_after.is_fixed()
            && right_after.is_fixed()
            && (left_after.min - right_after.min).abs() <= f64::EPSILON
        {
            let mut left_holes = left.holes.clone();
            for hole in &left_after.holes {
                if !left_holes
                    .iter()
                    .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
                {
                    left_holes.push(*hole);
                }
            }
            let mut right_holes = right.holes.clone();
            for hole in &right_after.holes {
                if !right_holes
                    .iter()
                    .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
                {
                    right_holes.push(*hole);
                }
            }
            for hole in &left_holes {
                changed |= ext.exclude_float_point(self.watched[1], *hole);
            }
            for hole in &right_holes {
                changed |= ext.exclude_float_point(self.watched[0], *hole);
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
    fn propagates_float_le_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 10.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 4.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        let status = engine.propagate_all().unwrap();
        assert!(status.is_failure());
    }

    #[test]
    fn propagates_successfully() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 15.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(3.0, 10.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
        let left_domain = engine.domain(left).as_float().unwrap();
        let right_domain = engine.domain(right).as_float().unwrap();
        assert!((left_domain.upper_bound() - 10.0).abs() < f64::EPSILON);
        assert!((right_domain.lower_bound() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 5.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 10.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn no_extended_context_returns_ok_no_change() {
        use crate::test_support::NoExtendedCtx;
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let _ = engine.new_variable(IntervalDomain::new(1, 5));
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let mut prop = FloatLePropagator::new(left, right);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn shares_holes_when_le_collapses_to_fixed_equality() {
        let mut engine = Engine::new();
        // a ≤ b with a ≥ 5 and b ≤ 5 forces a = b = 5; hole 5 on a must empty both.
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 10.0).exclude(5.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn propagates_le_using_admissible_endpoints() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        let right =
            engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0).exclude(10.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let left_domain = engine.domain(left).as_float().unwrap();
        assert!(left_domain.upper_bound() < 10.0);

        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0).exclude(0.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let right_domain = engine.domain(right).as_float().unwrap();
        assert!(right_domain.lower_bound() > 0.0);
    }

    #[test]
    fn dual_le_advances_past_shared_equality_hole() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0).exclude(3.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(3.0, 5.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(left, right)));
        engine.add_propagator(Box::new(FloatLePropagator::new(right, left)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(left).as_float().unwrap().contains(3.0));
        assert!(!engine.domain(right).as_float().unwrap().contains(3.0));
    }
}
