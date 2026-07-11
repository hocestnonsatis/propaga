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

        let product = FloatDomain::new(a.min, a.max).times(FloatDomain::new(b.min, b.max));
        changed |= ext.tighten_float_below(self.watched[2], product.lower_bound());
        changed |= ext.tighten_float_above(self.watched[2], product.upper_bound());

        let c_snap = ext.float_domain(self.watched[2]).unwrap_or(c);
        let b_snap = ext.float_domain(self.watched[1]).unwrap_or(b);
        let a_snap = ext.float_domain(self.watched[0]).unwrap_or(a);

        let a_from_c = FloatDomain::new(c_snap.min, c_snap.max)
            .divide(FloatDomain::new(b_snap.min, b_snap.max));
        if a_from_c.lower_bound().is_finite() {
            changed |= ext.tighten_float_below(self.watched[0], a_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[0], a_from_c.upper_bound());
        }

        let b_from_c = FloatDomain::new(c_snap.min, c_snap.max)
            .divide(FloatDomain::new(a_snap.min, a_snap.max));
        if b_from_c.lower_bound().is_finite() {
            changed |= ext.tighten_float_below(self.watched[1], b_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[1], b_from_c.upper_bound());
        }

        let a_after = ext.float_domain(self.watched[0]).unwrap_or(a);
        let b_after = ext.float_domain(self.watched[1]).unwrap_or(b);
        let c_after = ext.float_domain(self.watched[2]).unwrap_or(c);
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
