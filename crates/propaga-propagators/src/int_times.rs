use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `c = a * b` with bound consistency.
#[derive(Clone, Debug)]
pub struct IntTimesPropagator {
    watched: [VariableId; 3],
}

impl IntTimesPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId) -> Self {
        Self { watched: [a, b, c] }
    }
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Floor division toward −∞ (`n / d` for nonzero `d`).
fn floor_div(n: i64, d: i64) -> i64 {
    let q = n / d;
    let r = n % d;
    if r != 0 && (n < 0) != (d < 0) {
        q - 1
    } else {
        q
    }
}

/// Ceiling division toward +∞ (`n / d` for nonzero `d`).
fn ceil_div(n: i64, d: i64) -> i64 {
    let q = n / d;
    let r = n % d;
    if r != 0 && (n < 0) == (d < 0) {
        q + 1
    } else {
        q
    }
}

fn mul_bounds(amin: i32, amax: i32, bmin: i32, bmax: i32) -> (i32, i32) {
    let corners = [
        amin as i64 * bmin as i64,
        amin as i64 * bmax as i64,
        amax as i64 * bmin as i64,
        amax as i64 * bmax as i64,
    ];
    let min = corners.iter().copied().min().unwrap();
    let max = corners.iter().copied().max().unwrap();
    (clamp_i32(min), clamp_i32(max))
}

/// Bounds on `factor` such that `factor * fixed` can land in `[cmin, cmax]`.
fn factor_bounds_from_product(cmin: i32, cmax: i32, fixed: i32) -> Option<(i32, i32)> {
    if fixed == 0 {
        return None;
    }
    let (cmin, cmax, fixed) = (cmin as i64, cmax as i64, fixed as i64);
    let (lo, hi) = if fixed > 0 {
        (ceil_div(cmin, fixed), floor_div(cmax, fixed))
    } else {
        (ceil_div(cmax, fixed), floor_div(cmin, fixed))
    };
    if lo > hi {
        None
    } else {
        Some((clamp_i32(lo), clamp_i32(hi)))
    }
}

impl Propagator for IntTimesPropagator {
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

        // Zero factor ⇒ product is zero.
        if ctx.domain(a).min() == Some(0) && ctx.domain(a).max() == Some(0) {
            changed |= ctx.remove_below(c, 0);
            changed |= ctx.remove_above(c, 0);
        }
        if ctx.domain(b).min() == Some(0) && ctx.domain(b).max() == Some(0) {
            changed |= ctx.remove_below(c, 0);
            changed |= ctx.remove_above(c, 0);
        }

        let (Some(amin), Some(amax), Some(bmin), Some(bmax)) = (
            ctx.domain(a).min(),
            ctx.domain(a).max(),
            ctx.domain(b).min(),
            ctx.domain(b).max(),
        ) else {
            return PropagationStatus::Failure;
        };

        let (pmin, pmax) = mul_bounds(amin, amax, bmin, bmax);
        if pmin > pmax {
            return PropagationStatus::Failure;
        }
        changed |= ctx.remove_below(c, pmin);
        changed |= ctx.remove_above(c, pmax);

        let (Some(cmin), Some(cmax)) = (ctx.domain(c).min(), ctx.domain(c).max()) else {
            return PropagationStatus::Failure;
        };

        // Nonzero product ⇒ neither factor can be fixed at zero.
        if cmin > 0 || cmax < 0 {
            if amin == 0 && amax == 0 {
                return PropagationStatus::Failure;
            }
            if bmin == 0 && bmax == 0 {
                return PropagationStatus::Failure;
            }
            if amin == 0 {
                changed |= ctx.remove_value(a, 0);
            }
            if bmin == 0 {
                changed |= ctx.remove_value(b, 0);
            }
            // remove_value above may leave holes; re-check fixed zeros after.
            if ctx.domain(a).min() == Some(0) && ctx.domain(a).max() == Some(0) {
                return PropagationStatus::Failure;
            }
            if ctx.domain(b).min() == Some(0) && ctx.domain(b).max() == Some(0) {
                return PropagationStatus::Failure;
            }
        }

        // Reverse when a factor is fixed and nonzero.
        if let Some(afixed) = ctx.fixed_value(a)
            && afixed != 0
        {
            match factor_bounds_from_product(cmin, cmax, afixed) {
                Some((lo, hi)) => {
                    changed |= ctx.remove_below(b, lo);
                    changed |= ctx.remove_above(b, hi);
                }
                None => return PropagationStatus::Failure,
            }
        }
        if let Some(bfixed) = ctx.fixed_value(b)
            && bfixed != 0
        {
            match factor_bounds_from_product(
                ctx.domain(c).min().unwrap_or(cmin),
                ctx.domain(c).max().unwrap_or(cmax),
                bfixed,
            ) {
                Some((lo, hi)) => {
                    changed |= ctx.remove_below(a, lo);
                    changed |= ctx.remove_above(a, hi);
                }
                None => return PropagationStatus::Failure,
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
    fn floor_ceil_div_helpers() {
        assert_eq!(floor_div(7, 3), 2);
        assert_eq!(ceil_div(7, 3), 3);
        assert_eq!(floor_div(-7, 3), -3);
        assert_eq!(ceil_div(-7, 3), -2);
        assert_eq!(floor_div(7, -3), -3);
        assert_eq!(ceil_div(7, -3), -2);
    }

    #[test]
    fn forward_tightens_product_bounds() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(2, 4));
        let b = engine.new_variable(IntervalDomain::new(3, 5));
        let c = engine.new_variable(IntervalDomain::new(0, 100));
        engine.add_propagator(Box::new(IntTimesPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).min(), Some(6));
        assert_eq!(engine.hybrid_domain(c).max(), Some(20));
    }

    #[test]
    fn reverse_when_factor_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(3));
        let b = engine.new_variable(IntervalDomain::new(1, 10));
        let c = engine.new_variable(IntervalDomain::new(12, 15));
        engine.add_propagator(Box::new(IntTimesPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(b).min(), Some(4));
        assert_eq!(engine.hybrid_domain(b).max(), Some(5));
    }

    #[test]
    fn zero_factor_forces_zero_product() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(0));
        let b = engine.new_variable(IntervalDomain::new(-5, 5));
        let c = engine.new_variable(IntervalDomain::new(-10, 10));
        engine.add_propagator(Box::new(IntTimesPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).fixed_value(), Some(0));
    }
}
