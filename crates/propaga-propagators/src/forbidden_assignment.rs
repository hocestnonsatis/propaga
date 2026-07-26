use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};
use propaga_domains::{AnyDomain, FloatDomain, HybridDomain};
use propaga_engine::Engine;

use crate::float_ops::FloatLeReifPropagator;

/// Typed value forbidden for a variable in a blocked assignment.
#[derive(Clone, Debug, PartialEq)]
pub enum ForbiddenValue {
    /// Forbidden integer assignment.
    Int(i32),
    /// Forbidden floating-point assignment (exact fixed point / bound endpoint).
    Float(f64),
    /// Forbidden exact set membership (sorted unique elements).
    Set(Vec<i32>),
}

/// Forbids rediscovering one complete variable assignment.
///
/// Integer components unit-propagate like a nogood. Continuous float values should
/// usually be encoded via [`encode_forbidden_float`] so search can branch on the
/// reified disjunction that excludes a single IEEE point. Set components fail or
/// branch away from the forbidden set.
#[derive(Clone, Debug)]
pub struct ForbiddenAssignmentPropagator {
    watched: Vec<VariableId>,
    forbidden: Vec<(VariableId, ForbiddenValue)>,
}

impl ForbiddenAssignmentPropagator {
    /// Creates a propagator that forbids the given assignment.
    #[must_use]
    pub fn new(forbidden: Vec<(VariableId, ForbiddenValue)>) -> Self {
        let mut watched = Vec::with_capacity(forbidden.len());
        for (var, _) in &forbidden {
            if !watched.contains(var) {
                watched.push(*var);
            }
        }
        Self { watched, forbidden }
    }
}

/// Result of encoding a forbidden float point as a reified disjunction.
#[derive(Clone, Debug)]
pub struct EncodedForbiddenFloat {
    /// Literals to include in a [`ForbiddenAssignmentPropagator`].
    pub forbidden: Vec<(VariableId, ForbiddenValue)>,
    /// Boolean decision variables that should be searchable (reif_le, reif_ge).
    pub decision_vars: Vec<VariableId>,
}

/// Encodes “`var` equals `value`” as two boolean literals that are both zero iff
/// the float is trapped at that IEEE point (`next_down < x < next_up`).
///
/// Posts `FloatLeReif` propagators so assigning either reif to `1` forces the float
/// strictly below or above `value`. Include the returned forbidden pairs in a
/// [`ForbiddenAssignmentPropagator`] and add [`EncodedForbiddenFloat::decision_vars`]
/// to the search variable list so DFS can branch on the disjunction.
#[must_use]
pub fn encode_forbidden_float(
    engine: &mut Engine,
    var: VariableId,
    value: f64,
) -> EncodedForbiddenFloat {
    let down = next_float_down(value);
    let up = next_float_up(value);
    if !down.is_finite() || !up.is_finite() || !(down < value && value < up) {
        return EncodedForbiddenFloat {
            forbidden: vec![(var, ForbiddenValue::Float(value))],
            decision_vars: Vec::new(),
        };
    }

    let bound_lo = engine.new_variable(AnyDomain::Float(FloatDomain::fix(down)));
    let bound_hi = engine.new_variable(AnyDomain::Float(FloatDomain::fix(up)));
    let reif_le = engine.new_variable(HybridDomain::new(0, 1));
    let reif_ge = engine.new_variable(HybridDomain::new(0, 1));
    engine.add_propagator(Box::new(FloatLeReifPropagator::new(var, bound_lo, reif_le)));
    engine.add_propagator(Box::new(FloatLeReifPropagator::new(bound_hi, var, reif_ge)));
    EncodedForbiddenFloat {
        forbidden: vec![
            (reif_le, ForbiddenValue::Int(0)),
            (reif_ge, ForbiddenValue::Int(0)),
        ],
        decision_vars: vec![reif_le, reif_ge],
    }
}

fn next_float_up(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_positive() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_add(1))
    }
}

fn next_float_down(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_negative() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_sub(1))
    }
}

impl Propagator for ForbiddenAssignmentPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        2
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut matched = 0usize;
        let mut pending: Option<(VariableId, ForbiddenValue)> = None;

        for (var, value) in &self.forbidden {
            match assignment_status(ctx, *var, value) {
                AssignmentStatus::Matches => matched += 1,
                AssignmentStatus::Conflicts => return PropagationStatus::OkNoChange,
                AssignmentStatus::Open => {
                    if pending.is_some() {
                        return PropagationStatus::OkNoChange;
                    }
                    pending = Some((*var, value.clone()));
                }
            }
        }

        if matched == self.forbidden.len() {
            return PropagationStatus::Failure;
        }

        if matched + 1 == self.forbidden.len()
            && let Some((var, value)) = pending
        {
            return forbid_value(ctx, var, &value);
        }

        PropagationStatus::OkNoChange
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssignmentStatus {
    Matches,
    Conflicts,
    Open,
}

fn assignment_status(
    ctx: &mut dyn PropagationContext,
    var: VariableId,
    value: &ForbiddenValue,
) -> AssignmentStatus {
    match value {
        ForbiddenValue::Int(expected) => match ctx.fixed_value(var) {
            Some(actual) if actual == *expected => AssignmentStatus::Matches,
            Some(_) => AssignmentStatus::Conflicts,
            None => {
                if ctx.domain(var).contains(*expected) {
                    AssignmentStatus::Open
                } else {
                    AssignmentStatus::Conflicts
                }
            }
        },
        ForbiddenValue::Float(expected) => {
            let Some(ext) = ctx.as_extended() else {
                return AssignmentStatus::Conflicts;
            };
            let Some(domain) = ext.float_domain(var) else {
                return AssignmentStatus::Conflicts;
            };
            if domain.is_empty() {
                return AssignmentStatus::Conflicts;
            }
            if (domain.min - domain.max).abs() <= f64::EPSILON
                && (domain.min - expected).abs() <= f64::EPSILON
            {
                AssignmentStatus::Matches
            } else if domain.contains(*expected) {
                AssignmentStatus::Open
            } else {
                AssignmentStatus::Conflicts
            }
        }
        ForbiddenValue::Set(expected) => {
            let Some(ext) = ctx.as_extended() else {
                return AssignmentStatus::Conflicts;
            };
            let Some(domain) = ext.set_domain(var) else {
                return AssignmentStatus::Conflicts;
            };
            if domain.is_empty() {
                return AssignmentStatus::Conflicts;
            }
            let mut expected = expected.clone();
            expected.sort_unstable();
            expected.dedup();
            let glb_sorted = {
                let mut glb = domain.glb.clone();
                glb.sort_unstable();
                glb
            };
            let lub_sorted = {
                let mut lub = domain.lub.clone();
                lub.sort_unstable();
                lub
            };
            if glb_sorted == expected && lub_sorted == expected {
                AssignmentStatus::Matches
            } else if expected.iter().all(|value| lub_sorted.contains(value))
                && glb_sorted.iter().all(|value| expected.contains(value))
            {
                AssignmentStatus::Open
            } else {
                AssignmentStatus::Conflicts
            }
        }
    }
}

fn forbid_value(
    ctx: &mut dyn PropagationContext,
    var: VariableId,
    value: &ForbiddenValue,
) -> PropagationStatus {
    match value {
        ForbiddenValue::Int(expected) => {
            if ctx.remove_value(var, *expected) {
                if ctx.domain(var).is_empty() {
                    PropagationStatus::Failure
                } else {
                    PropagationStatus::OkChanged
                }
            } else if ctx.fixed_value(var) == Some(*expected) {
                PropagationStatus::Failure
            } else {
                PropagationStatus::OkNoChange
            }
        }
        ForbiddenValue::Float(expected) => {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::Failure;
            };
            let Some(domain) = ext.float_domain(var) else {
                return PropagationStatus::Failure;
            };
            if (domain.min - domain.max).abs() <= f64::EPSILON
                && (domain.min - expected).abs() <= f64::EPSILON
            {
                return PropagationStatus::Failure;
            }
            // Prefer bound tightening when the forbidden point sits on an endpoint.
            if (domain.min - expected).abs() <= f64::EPSILON {
                let up = next_float_up(*expected);
                let _ = ext.tighten_float_below(var, up);
                return if ext
                    .float_domain(var)
                    .is_some_and(|domain| !domain.is_empty() && domain.min >= up)
                {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::Failure
                };
            }
            if (domain.max - expected).abs() <= f64::EPSILON {
                let down = next_float_down(*expected);
                let _ = ext.tighten_float_above(var, down);
                return if ext
                    .float_domain(var)
                    .is_some_and(|domain| !domain.is_empty() && domain.max <= down)
                {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::Failure
                };
            }
            PropagationStatus::OkNoChange
        }
        ForbiddenValue::Set(expected) => {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::Failure;
            };
            let Some(domain) = ext.set_domain(var) else {
                return PropagationStatus::Failure;
            };
            let mut expected = expected.clone();
            expected.sort_unstable();
            expected.dedup();
            let undecided: Vec<i32> = domain
                .lub
                .iter()
                .copied()
                .filter(|value| !domain.glb.contains(value))
                .collect();
            if undecided.is_empty() {
                let mut glb = domain.glb.clone();
                glb.sort_unstable();
                return if glb == expected {
                    PropagationStatus::Failure
                } else {
                    PropagationStatus::OkNoChange
                };
            }
            let pivot = undecided[0];
            let changed = if expected.contains(&pivot) {
                ext.force_set_out(var, pivot)
            } else {
                ext.force_set_in(var, pivot)
            };
            if let Some(domain) = ext.set_domain(var)
                && domain.is_empty()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::{AnyDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn blocks_integer_assignment_like_nogood() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(vec![
            (x, ForbiddenValue::Int(1)),
            (y, ForbiddenValue::Int(2)),
        ])));
        engine.fix_variable(x, 1).unwrap();
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        assert!(!engine.domain(y).as_int().unwrap().contains(2));
    }

    #[test]
    fn blocks_fixed_set_assignment() {
        let mut engine = Engine::new();
        let set = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(vec![(
            set,
            ForbiddenValue::Set(vec![1, 2]),
        )])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(status.is_failure());
    }

    #[test]
    fn forbids_float_endpoint_by_bound_tightening() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::fix(1));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(vec![
            (x, ForbiddenValue::Int(1)),
            (y, ForbiddenValue::Float(1.0)),
        ])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        let domain = engine.domain(y).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn encode_forbidden_float_unit_props_through_reif_branch() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::fix(1));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let encoded = encode_forbidden_float(&mut engine, y, 1.0);
        assert_eq!(encoded.decision_vars.len(), 2);
        let reif_le = encoded.decision_vars[0];
        let mut forbidden = vec![(x, ForbiddenValue::Int(1))];
        forbidden.extend(encoded.forbidden);
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(forbidden)));
        // Choosing “not ≤ next_down(1)” forces the complementary ≥ next_up(1) branch.
        let status = engine.fix_variable(reif_le, 0).unwrap();
        assert!(!status.is_failure());
        let domain = engine.domain(y).as_float().unwrap();
        assert!(
            domain.lower_bound() > 1.0,
            "expected y forced above 1.0, got [{}, {}]",
            domain.lower_bound(),
            domain.upper_bound()
        );
    }

    #[test]
    fn encode_forbidden_float_fails_when_fixed_at_point() {
        let mut engine = Engine::new();
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let encoded = encode_forbidden_float(&mut engine, y, 1.0);
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(
            encoded.forbidden,
        )));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(status.is_failure());
    }
}
