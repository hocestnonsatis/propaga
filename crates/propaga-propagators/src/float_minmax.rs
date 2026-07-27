use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator, VariableId,
};
use propaga_domains::FloatDomain;

/// Posts `c = min(a, b)` or `c = max(a, b)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatMinMaxOp {
    Min,
    Max,
}

/// Propagates binary float min/max.
#[derive(Clone, Debug)]
pub struct FloatMinMaxPropagator {
    watched: [VariableId; 3],
    op: FloatMinMaxOp,
}

impl FloatMinMaxPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId, op: FloatMinMaxOp) -> Self {
        Self {
            watched: [a, b, c],
            op,
        }
    }
}

fn read_float(ext: &dyn ExtendedPropagationContext, var: VariableId) -> Option<FloatDomain> {
    let snap = ext.float_domain(var)?;
    Some(FloatDomain::from_bounds_with_holes(
        snap.min,
        snap.max,
        &snap.holes,
    ))
}

fn sync_equal(
    ext: &mut dyn ExtendedPropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let (Some(left_dom), Some(right_dom)) = (read_float(ext, left), read_float(ext, right)) else {
        return false;
    };
    let mut changed = false;
    changed |= ext.tighten_float_below(left, right_dom.lower_bound());
    changed |= ext.tighten_float_above(left, right_dom.upper_bound());
    changed |= ext.tighten_float_below(right, left_dom.lower_bound());
    changed |= ext.tighten_float_above(right, left_dom.upper_bound());
    for hole in left_dom.holes() {
        changed |= ext.exclude_float_point(right, *hole);
    }
    for hole in right_dom.holes() {
        changed |= ext.exclude_float_point(left, *hole);
    }
    changed
}

impl Propagator for FloatMinMaxPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (a_id, b_id, c_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let (Some(a_dom), Some(b_dom), Some(c_dom)) = (
            read_float(ext, a_id),
            read_float(ext, b_id),
            read_float(ext, c_id),
        ) else {
            return PropagationStatus::Failure;
        };
        if a_dom.is_empty() || b_dom.is_empty() || c_dom.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        let image = match self.op {
            FloatMinMaxOp::Min => a_dom.min_with(&b_dom),
            FloatMinMaxOp::Max => a_dom.max_with(&b_dom),
        };
        if image.is_empty() {
            return PropagationStatus::Failure;
        }
        changed |= ext.tighten_float_below(c_id, image.lower_bound());
        changed |= ext.tighten_float_above(c_id, image.upper_bound());
        for hole in image.holes() {
            changed |= ext.exclude_float_point(c_id, *hole);
        }

        match self.op {
            FloatMinMaxOp::Min => {
                // c ≤ a and c ≤ b
                changed |= ext.tighten_float_below(a_id, c_dom.lower_bound());
                changed |= ext.tighten_float_below(b_id, c_dom.lower_bound());
                changed |= ext.tighten_float_above(c_id, a_dom.upper_bound());
                changed |= ext.tighten_float_above(c_id, b_dom.upper_bound());
                if a_dom.lower_bound() >= b_dom.upper_bound() {
                    changed |= sync_equal(ext, b_id, c_id);
                } else if b_dom.lower_bound() >= a_dom.upper_bound() {
                    changed |= sync_equal(ext, a_id, c_id);
                }
            }
            FloatMinMaxOp::Max => {
                // a ≤ c and b ≤ c
                changed |= ext.tighten_float_above(a_id, c_dom.upper_bound());
                changed |= ext.tighten_float_above(b_id, c_dom.upper_bound());
                changed |= ext.tighten_float_below(c_id, a_dom.lower_bound());
                changed |= ext.tighten_float_below(c_id, b_dom.lower_bound());
                if a_dom.upper_bound() <= b_dom.lower_bound() {
                    changed |= sync_equal(ext, b_id, c_id);
                } else if b_dom.upper_bound() <= a_dom.lower_bound() {
                    changed |= sync_equal(ext, a_id, c_id);
                }
            }
        }

        if ext.float_domain(a_id).is_some_and(|d| d.is_empty())
            || ext.float_domain(b_id).is_some_and(|d| d.is_empty())
            || ext.float_domain(c_id).is_some_and(|d| d.is_empty())
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
    fn min_tightens_result_upper_bound() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        engine.add_propagator(Box::new(FloatMinMaxPropagator::new(
            a,
            b,
            c,
            FloatMinMaxOp::Min,
        )));
        engine.propagate_all().unwrap();
        assert!((engine.domain(c).as_float().unwrap().upper_bound() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn max_tightens_result_lower_bound() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(-10.0, 10.0)));
        engine.add_propagator(Box::new(FloatMinMaxPropagator::new(
            a,
            b,
            c,
            FloatMinMaxOp::Max,
        )));
        engine.propagate_all().unwrap();
        assert!((engine.domain(c).as_float().unwrap().lower_bound() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn min_equates_when_one_operand_dominates() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(5.0, 6.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(-10.0, 10.0)));
        engine.add_propagator(Box::new(FloatMinMaxPropagator::new(
            a,
            b,
            c,
            FloatMinMaxOp::Min,
        )));
        engine.propagate_all().unwrap();
        assert!((engine.domain(c).as_float().unwrap().upper_bound() - 2.0).abs() < 1e-9);
        assert!((engine.domain(c).as_float().unwrap().lower_bound() - 0.0).abs() < 1e-9);
    }
}
