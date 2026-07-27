use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Posts `c = min(a, b)` or `c = max(a, b)` for integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntMinMaxOp {
    Min,
    Max,
}

/// Bound-consistent binary integer min/max.
#[derive(Clone, Debug)]
pub struct IntMinMaxPropagator {
    watched: [VariableId; 3],
    op: IntMinMaxOp,
}

impl IntMinMaxPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId, op: IntMinMaxOp) -> Self {
        Self {
            watched: [a, b, c],
            op,
        }
    }
}

fn sync_equal(ctx: &mut dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    let mut changed = false;
    if let Some(rmin) = ctx.domain(right).min() {
        changed |= ctx.remove_below(left, rmin);
    }
    if let Some(rmax) = ctx.domain(right).max() {
        changed |= ctx.remove_above(left, rmax);
    }
    if let Some(lmin) = ctx.domain(left).min() {
        changed |= ctx.remove_below(right, lmin);
    }
    if let Some(lmax) = ctx.domain(left).max() {
        changed |= ctx.remove_above(right, lmax);
    }
    changed
}

impl Propagator for IntMinMaxPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [a, b, c] = self.watched;
        let (Some(amin), Some(amax), Some(bmin), Some(bmax), Some(cmin), Some(cmax)) = (
            ctx.domain(a).min(),
            ctx.domain(a).max(),
            ctx.domain(b).min(),
            ctx.domain(b).max(),
            ctx.domain(c).min(),
            ctx.domain(c).max(),
        ) else {
            return PropagationStatus::Failure;
        };

        let mut changed = false;
        match self.op {
            IntMinMaxOp::Min => {
                let image_min = amin.min(bmin);
                let image_max = amax.min(bmax);
                if image_min > image_max {
                    return PropagationStatus::Failure;
                }
                changed |= ctx.remove_below(c, image_min);
                changed |= ctx.remove_above(c, image_max);
                // c ≤ a, c ≤ b
                changed |= ctx.remove_below(a, cmin);
                changed |= ctx.remove_below(b, cmin);
                changed |= ctx.remove_above(c, amax);
                changed |= ctx.remove_above(c, bmax);
                if amin >= bmax {
                    changed |= sync_equal(ctx, b, c);
                } else if bmin >= amax {
                    changed |= sync_equal(ctx, a, c);
                }
            }
            IntMinMaxOp::Max => {
                let image_min = amin.max(bmin);
                let image_max = amax.max(bmax);
                if image_min > image_max {
                    return PropagationStatus::Failure;
                }
                changed |= ctx.remove_below(c, image_min);
                changed |= ctx.remove_above(c, image_max);
                // a ≤ c, b ≤ c
                changed |= ctx.remove_above(a, cmax);
                changed |= ctx.remove_above(b, cmax);
                changed |= ctx.remove_below(c, amin);
                changed |= ctx.remove_below(c, bmin);
                if amax <= bmin {
                    changed |= sync_equal(ctx, b, c);
                } else if bmax <= amin {
                    changed |= sync_equal(ctx, a, c);
                }
            }
        }

        if ctx.domain(a).is_empty() || ctx.domain(b).is_empty() || ctx.domain(c).is_empty() {
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
    fn min_tightens_result_upper_bound() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        let c = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(IntMinMaxPropagator::new(
            a,
            b,
            c,
            IntMinMaxOp::Min,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).max(), Some(3));
    }

    #[test]
    fn max_tightens_result_lower_bound() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 5));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        let c = engine.new_variable(IntervalDomain::new(-10, 10));
        engine.add_propagator(Box::new(IntMinMaxPropagator::new(
            a,
            b,
            c,
            IntMinMaxOp::Max,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).min(), Some(1));
    }

    #[test]
    fn min_equates_when_one_operand_dominates() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(5, 6));
        let b = engine.new_variable(IntervalDomain::new(0, 2));
        let c = engine.new_variable(IntervalDomain::new(-10, 10));
        engine.add_propagator(Box::new(IntMinMaxPropagator::new(
            a,
            b,
            c,
            IntMinMaxOp::Min,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).min(), Some(0));
        assert_eq!(engine.hybrid_domain(c).max(), Some(2));
    }
}
