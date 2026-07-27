use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `c = a mod b` (Rust / FlatZinc truncating remainder), with `b ≠ 0`.
#[derive(Clone, Debug)]
pub struct IntModPropagator {
    watched: [VariableId; 3],
}

impl IntModPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId) -> Self {
        Self { watched: [a, b, c] }
    }
}

fn rem_bounds_fixed_divisor(amin: i32, amax: i32, b: i32) -> (i32, i32) {
    let abs_b = b.unsigned_abs();
    let span = (amax as i64 - amin as i64).saturating_add(1);
    if span >= abs_b as i64 {
        if amin >= 0 {
            (0, abs_b as i32 - 1)
        } else if amax <= 0 {
            (1 - abs_b as i32, 0)
        } else {
            (1 - abs_b as i32, abs_b as i32 - 1)
        }
    } else if span <= 10_000 {
        let mut lo = i32::MAX;
        let mut hi = i32::MIN;
        for value in amin..=amax {
            let r = value % b;
            lo = lo.min(r);
            hi = hi.max(r);
        }
        (lo, hi)
    } else if amin >= 0 {
        (0, abs_b as i32 - 1)
    } else if amax <= 0 {
        (1 - abs_b as i32, 0)
    } else {
        (1 - abs_b as i32, abs_b as i32 - 1)
    }
}

/// Smallest `x >= start` with `x % b == rem`, or `None` if none exists before overflowing search.
fn first_congruent(start: i32, b: i32, rem: i32) -> Option<i32> {
    let abs_b = b.unsigned_abs() as i64;
    for offset in 0..=abs_b {
        let candidate = start as i64 + offset;
        if candidate > i32::MAX as i64 {
            return None;
        }
        let candidate = candidate as i32;
        if candidate % b == rem {
            return Some(candidate);
        }
    }
    None
}

/// Largest `x <= end` with `x % b == rem`.
fn last_congruent(end: i32, b: i32, rem: i32) -> Option<i32> {
    let abs_b = b.unsigned_abs() as i64;
    for offset in 0..=abs_b {
        let candidate = end as i64 - offset;
        if candidate < i32::MIN as i64 {
            return None;
        }
        let candidate = candidate as i32;
        if candidate % b == rem {
            return Some(candidate);
        }
    }
    None
}

impl Propagator for IntModPropagator {
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
        changed |= ctx.remove_value(b, 0);
        let (Some(bmin), Some(bmax)) = (ctx.domain(b).min(), ctx.domain(b).max()) else {
            return PropagationStatus::Failure;
        };
        if bmin > bmax {
            return PropagationStatus::Failure;
        }

        // |c| < |b| once |b| has a useful lower bound on magnitude.
        if bmin > 0 {
            changed |= ctx.remove_below(c, 0);
            changed |= ctx.remove_above(c, bmin - 1);
        } else if bmax < 0 {
            let abs_lo = (-bmax) as i32;
            changed |= ctx.remove_above(c, 0);
            changed |= ctx.remove_below(c, 1 - abs_lo);
        } else if bmin < 0 && bmax > 0 {
            let mag = (-bmin).max(bmax);
            changed |= ctx.remove_below(c, 1 - mag);
            changed |= ctx.remove_above(c, mag - 1);
        }

        let (Some(amin), Some(amax)) = (ctx.domain(a).min(), ctx.domain(a).max()) else {
            return PropagationStatus::Failure;
        };
        if amin >= 0 {
            changed |= ctx.remove_below(c, 0);
        }
        if amax <= 0 {
            changed |= ctx.remove_above(c, 0);
        }

        // Forward with fixed divisor.
        if let Some(bfixed) = ctx.fixed_value(b) {
            let (rmin, rmax) = rem_bounds_fixed_divisor(amin, amax, bfixed);
            changed |= ctx.remove_below(c, rmin);
            changed |= ctx.remove_above(c, rmax);

            if let Some(cfixed) = ctx.fixed_value(c) {
                let (Some(amin), Some(amax)) = (ctx.domain(a).min(), ctx.domain(a).max()) else {
                    return PropagationStatus::Failure;
                };
                let Some(first) = first_congruent(amin, bfixed, cfixed) else {
                    return PropagationStatus::Failure;
                };
                let Some(last) = last_congruent(amax, bfixed, cfixed) else {
                    return PropagationStatus::Failure;
                };
                if first > last {
                    return PropagationStatus::Failure;
                }
                changed |= ctx.remove_below(a, first);
                changed |= ctx.remove_above(a, last);
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
    fn remainder_bounds_with_fixed_divisor() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 100));
        let b = engine.new_variable(IntervalDomain::fix(7));
        let c = engine.new_variable(IntervalDomain::new(-20, 20));
        engine.add_propagator(Box::new(IntModPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).min(), Some(0));
        assert_eq!(engine.hybrid_domain(c).max(), Some(6));
    }

    #[test]
    fn excludes_zero_divisor() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let b = engine.new_variable(IntervalDomain::new(-1, 1));
        let c = engine.new_variable(IntervalDomain::new(-5, 5));
        engine.add_propagator(Box::new(IntModPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(b).contains(0));
    }

    #[test]
    fn fixed_mod_tightens_dividend() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(0, 30));
        let b = engine.new_variable(IntervalDomain::fix(5));
        let c = engine.new_variable(IntervalDomain::fix(2));
        engine.add_propagator(Box::new(IntModPropagator::new(a, b, c)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(a).min(), Some(2));
        assert_eq!(engine.hybrid_domain(a).max(), Some(27));
    }
}
