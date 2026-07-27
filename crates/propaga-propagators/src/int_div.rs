use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `c = a / b` (truncating toward zero), with `b ≠ 0`.
#[derive(Clone, Debug)]
pub struct IntDivPropagator {
    watched: [VariableId; 3],
}

impl IntDivPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId) -> Self {
        Self { watched: [a, b, c] }
    }
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn trunc_div(a: i32, b: i32) -> i32 {
    a / b
}

/// Union of trunc-div images for `b` ranging over a nonzero same-sign interval.
fn quot_bounds_divisor_interval(amin: i32, amax: i32, bmin: i32, bmax: i32) -> (i32, i32) {
    let corners = [
        trunc_div(amin, bmin),
        trunc_div(amin, bmax),
        trunc_div(amax, bmin),
        trunc_div(amax, bmax),
    ];
    let mut lo = corners[0];
    let mut hi = corners[0];
    for &q in &corners[1..] {
        lo = lo.min(q);
        hi = hi.max(q);
    }
    (lo, hi)
}

/// Dividend interval whose trunc-div by fixed `b` yields quotient `q`.
fn dividend_interval_for_quotient(q: i32, b: i32) -> (i64, i64) {
    let b = b as i64;
    let q = q as i64;
    let abs_b = b.abs();
    if b > 0 {
        if q > 0 {
            (q * b, q * b + (abs_b - 1))
        } else if q < 0 {
            (q * b - (abs_b - 1), q * b)
        } else {
            (-(abs_b - 1), abs_b - 1)
        }
    } else if q > 0 {
        // b < 0, positive quotient: a is non-positive
        (q * b - (abs_b - 1), q * b)
    } else if q < 0 {
        (q * b, q * b + (abs_b - 1))
    } else {
        (-(abs_b - 1), abs_b - 1)
    }
}

fn dividend_bounds_for_quotients(cmin: i32, cmax: i32, b: i32) -> (i32, i32) {
    let mut lo = i64::MAX;
    let mut hi = i64::MIN;
    // Exact union; for huge quotient ranges fall back to endpoint envelopes.
    if (cmax as i64 - cmin as i64).saturating_abs() > 10_000 {
        for &q in &[cmin, cmax] {
            let (qlo, qhi) = dividend_interval_for_quotient(q, b);
            lo = lo.min(qlo);
            hi = hi.max(qhi);
        }
    } else {
        for q in cmin..=cmax {
            let (qlo, qhi) = dividend_interval_for_quotient(q, b);
            lo = lo.min(qlo);
            hi = hi.max(qhi);
        }
    }
    (clamp_i32(lo), clamp_i32(hi))
}

impl Propagator for IntDivPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [a, b, c] = self.watched;
        if ctx.domain(a).min().is_none()
            || ctx.domain(a).max().is_none()
            || ctx.domain(b).min().is_none()
            || ctx.domain(b).max().is_none()
            || ctx.domain(c).min().is_none()
            || ctx.domain(c).max().is_none()
        {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        // Divisor cannot be zero.
        changed |= ctx.remove_value(b, 0);
        let (Some(bmin), Some(bmax)) = (ctx.domain(b).min(), ctx.domain(b).max()) else {
            return PropagationStatus::Failure;
        };
        if bmin > bmax || (bmin == 0 && bmax == 0) {
            return PropagationStatus::Failure;
        }

        let (Some(amin), Some(amax)) = (ctx.domain(a).min(), ctx.domain(a).max()) else {
            return PropagationStatus::Failure;
        };

        // Forward: tighten quotient bounds.
        let (qmin, qmax) = if bmin > 0 || bmax < 0 {
            quot_bounds_divisor_interval(amin, amax, bmin, bmax)
        } else {
            // Mixed-sign divisor (0 already removed): union pos and neg parts.
            let mut lo = i32::MAX;
            let mut hi = i32::MIN;
            if bmin <= -1 {
                let (l, h) = quot_bounds_divisor_interval(amin, amax, bmin, -1);
                lo = lo.min(l);
                hi = hi.max(h);
            }
            if bmax >= 1 {
                let (l, h) = quot_bounds_divisor_interval(amin, amax, 1, bmax);
                lo = lo.min(l);
                hi = hi.max(h);
            }
            if lo > hi {
                return PropagationStatus::Failure;
            }
            (lo, hi)
        };
        changed |= ctx.remove_below(c, qmin);
        changed |= ctx.remove_above(c, qmax);

        // Reverse when divisor is fixed.
        if let Some(bfixed) = ctx.fixed_value(b) {
            let (Some(cmin), Some(cmax)) = (ctx.domain(c).min(), ctx.domain(c).max()) else {
                return PropagationStatus::Failure;
            };
            let (alo, ahi) = dividend_bounds_for_quotients(cmin, cmax, bfixed);
            if alo > ahi {
                return PropagationStatus::Failure;
            }
            changed |= ctx.remove_below(a, alo);
            changed |= ctx.remove_above(a, ahi);
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
    fn forward_with_fixed_divisor() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(10, 20));
        let b = engine.new_variable(IntervalDomain::fix(3));
        let c = engine.new_variable(IntervalDomain::new(-10, 20));
        engine.add_propagator(Box::new(IntDivPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).min(), Some(3));
        assert_eq!(engine.hybrid_domain(c).max(), Some(6));
    }

    #[test]
    fn excludes_zero_divisor() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(-1, 1));
        let c = engine.new_variable(IntervalDomain::new(-10, 10));
        engine.add_propagator(Box::new(IntDivPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(b).contains(0));
    }

    #[test]
    fn reverse_tightens_dividend() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 100));
        let b = engine.new_variable(IntervalDomain::fix(5));
        let c = engine.new_variable(IntervalDomain::fix(3));
        engine.add_propagator(Box::new(IntDivPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).min(), Some(15));
        assert_eq!(engine.hybrid_domain(a).max(), Some(19));
    }
}
