use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};
use propaga_domains::FloatDomain;

#[derive(Clone, Debug)]
pub struct FloatTimesPropagator {
    watched: [VariableId; 3],
}

impl FloatTimesPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId) -> Self {
        Self { watched: [a, b, c] }
    }
}

impl Propagator for FloatTimesPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(a), Some(b), Some(c)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
            ext.float_domain(self.watched[2]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;

        let a_dom = FloatDomain::from_bounds_with_holes(a.min, a.max, &a.holes);
        let b_dom = FloatDomain::from_bounds_with_holes(b.min, b.max, &b.holes);
        let product = a_dom.times(&b_dom);
        changed |= ext.tighten_float_below(self.watched[2], product.lower_bound());
        changed |= ext.tighten_float_above(self.watched[2], product.upper_bound());
        for hole in product.holes() {
            changed |= ext.exclude_float_point(self.watched[2], *hole);
        }

        let c_snap = ext
            .float_domain(self.watched[2])
            .unwrap_or_else(|| c.clone());
        let b_snap = ext
            .float_domain(self.watched[1])
            .unwrap_or_else(|| b.clone());
        let a_snap = ext
            .float_domain(self.watched[0])
            .unwrap_or_else(|| a.clone());
        // Prefer holes recorded before bound sync on the product.
        let mut reverse_holes = c.holes.clone();
        for hole in &c_snap.holes {
            if !reverse_holes
                .iter()
                .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
            {
                reverse_holes.push(*hole);
            }
        }

        let a_from_c =
            FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &reverse_holes).divide(
                &FloatDomain::from_bounds_with_holes(b_snap.min, b_snap.max, &b_snap.holes),
            );
        if a_from_c.lower_bound().is_finite() {
            changed |= ext.tighten_float_below(self.watched[0], a_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[0], a_from_c.upper_bound());
            for hole in a_from_c.holes() {
                changed |= ext.exclude_float_point(self.watched[0], *hole);
            }
        }

        let b_from_c =
            FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &reverse_holes).divide(
                &FloatDomain::from_bounds_with_holes(a_snap.min, a_snap.max, &a_snap.holes),
            );
        if b_from_c.lower_bound().is_finite() {
            changed |= ext.tighten_float_below(self.watched[1], b_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[1], b_from_c.upper_bound());
            for hole in b_from_c.holes() {
                changed |= ext.exclude_float_point(self.watched[1], *hole);
            }
        }

        let a_after = ext
            .float_domain(self.watched[0])
            .unwrap_or_else(|| a.clone());
        let b_after = ext
            .float_domain(self.watched[1])
            .unwrap_or_else(|| b.clone());
        let c_after = ext
            .float_domain(self.watched[2])
            .unwrap_or_else(|| c.clone());
        if a_after.is_empty() || b_after.is_empty() || c_after.is_empty() {
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
    fn tightens_product_bounds() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 3.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(4.0, 5.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 100.0)));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        let c_domain = engine.domain(c).as_float().unwrap();
        assert!((c_domain.lower_bound() - 8.0).abs() < f64::EPSILON);
        assert!((c_domain.upper_bound() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn projects_holes_when_factor_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(c).as_float().unwrap().contains(2.0));
    }

    #[test]
    fn reverse_projects_product_hole_when_factor_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0).exclude(4.0)));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(a).as_float().unwrap().contains(2.0));
    }

    #[test]
    fn already_satisfied_no_change() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::fix(3.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::fix(6.0)));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
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
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0)));
        let mut prop = FloatTimesPropagator::new(a, b, c);
        let mut ctx = NoExtendedCtx::new(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn empty_float_domain_after_propagation_fails() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 0.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 3.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 100.0)));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn integer_variables_fail() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        let c = engine.new_variable(IntervalDomain::new(1, 9));
        engine.add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }
}
