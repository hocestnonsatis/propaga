use crate::reified::reif_literal;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `sum(coeffs[i] * vars[i]) <= rhs` with bound consistency.
#[derive(Clone)]
pub struct LinearScalarLePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<i32>,
    rhs: i32,
}

impl LinearScalarLePropagator {
    /// Creates a propagator for a weighted sum upper bound.
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<i32>>, vars: impl Into<Vec<VariableId>>, rhs: i32) -> Self {
        let coeffs = coeffs.into();
        let vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        Self {
            watched: vars,
            coeffs,
            rhs,
        }
    }
}

impl Propagator for LinearScalarLePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_scalar_le(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `sum(coeffs[i] * vars[i]) >= rhs` with bound consistency.
#[derive(Clone)]
pub struct LinearScalarGePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<i32>,
    rhs: i32,
}

impl LinearScalarGePropagator {
    /// Creates a propagator for a weighted sum lower bound.
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<i32>>, vars: impl Into<Vec<VariableId>>, rhs: i32) -> Self {
        let coeffs = coeffs.into();
        let vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        Self {
            watched: vars,
            coeffs,
            rhs,
        }
    }
}

impl Propagator for LinearScalarGePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_scalar_ge(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) <= rhs`.
#[derive(Clone)]
pub struct ReifiedScalarLePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<i32>,
    rhs: i32,
    reif: VariableId,
}

impl ReifiedScalarLePropagator {
    /// Creates a reified weighted sum upper-bound propagator.
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        let reif_var = reif;
        vars.push(reif_var);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif: reif_var,
        }
    }
}

impl Propagator for ReifiedScalarLePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        13
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let vars: Vec<VariableId> = self
            .watched
            .iter()
            .copied()
            .filter(|&var| var != self.reif)
            .collect();
        let mut changed = false;

        match reif_literal(ctx, self.reif) {
            Some(1) => {
                let status = propagate_scalar_le(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            Some(0) if self.rhs < i32::MAX => {
                let status = propagate_scalar_ge(ctx, &self.coeffs, &vars, self.rhs + 1);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            _ => {}
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if max_total <= self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if min_total > self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        finish_reified_scalar(ctx, &self.watched, changed)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) >= rhs`.
#[derive(Clone)]
pub struct ReifiedScalarGePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<i32>,
    rhs: i32,
    reif: VariableId,
}

impl ReifiedScalarGePropagator {
    /// Creates a reified weighted sum lower-bound propagator.
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        let reif_var = reif;
        vars.push(reif_var);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif: reif_var,
        }
    }
}

impl Propagator for ReifiedScalarGePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        13
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let vars: Vec<VariableId> = self
            .watched
            .iter()
            .copied()
            .filter(|&var| var != self.reif)
            .collect();
        let mut changed = false;

        match reif_literal(ctx, self.reif) {
            Some(1) => {
                let status = propagate_scalar_ge(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            Some(0) if self.rhs > i32::MIN => {
                let status = propagate_scalar_le(ctx, &self.coeffs, &vars, self.rhs - 1);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            _ => {}
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if min_total >= self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if max_total < self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        finish_reified_scalar(ctx, &self.watched, changed)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) == rhs`.
#[derive(Clone)]
pub struct ReifiedScalarEqPropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<i32>,
    rhs: i32,
    reif: VariableId,
}

impl ReifiedScalarEqPropagator {
    /// Creates a reified weighted sum equality propagator.
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        let reif_var = reif;
        vars.push(reif_var);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif: reif_var,
        }
    }
}

impl Propagator for ReifiedScalarEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        13
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let vars: Vec<VariableId> = self
            .watched
            .iter()
            .copied()
            .filter(|&var| var != self.reif)
            .collect();
        let mut changed = false;

        match reif_literal(ctx, self.reif) {
            Some(1) => {
                for status in [
                    propagate_scalar_le(ctx, &self.coeffs, &vars, self.rhs),
                    propagate_scalar_ge(ctx, &self.coeffs, &vars, self.rhs),
                ] {
                    if status.is_failure() {
                        return status;
                    }
                    changed |= status == PropagationStatus::OkChanged;
                }
            }
            Some(0) => {
                let min_total = min_sum(ctx, &self.coeffs, &vars);
                let max_total = max_sum(ctx, &self.coeffs, &vars);
                if min_total == max_total && min_total == self.rhs {
                    return PropagationStatus::Failure;
                }
                changed |= propagate_scalar_not_eq(ctx, &self.coeffs, &vars, self.rhs)
                    == PropagationStatus::OkChanged;
            }
            _ => {}
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if min_total == max_total && min_total == self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if min_total > self.rhs || max_total < self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        finish_reified_scalar(ctx, &self.watched, changed)
    }
}

fn propagate_scalar_le(
    ctx: &mut dyn PropagationContext,
    coeffs: &[i32],
    vars: &[VariableId],
    rhs: i32,
) -> PropagationStatus {
    let mut changed = false;

    if min_sum(ctx, coeffs, vars) > rhs {
        return PropagationStatus::Failure;
    }

    for (index, &var) in vars.iter().enumerate() {
        let coeff = coeffs[index];
        if coeff == 0 {
            continue;
        }

        let other_min = min_sum_excluding(ctx, coeffs, vars, index);
        let slack = rhs - other_min;

        if coeff > 0 {
            let max_allowed = slack / coeff;
            if ctx.remove_above(var, max_allowed) {
                changed = true;
            }
        } else {
            let min_allowed = div_ceil(slack, coeff);
            if ctx.remove_below(var, min_allowed) {
                changed = true;
            }
        }
    }

    if vars.iter().any(|var| ctx.domain(*var).is_empty()) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn propagate_scalar_ge(
    ctx: &mut dyn PropagationContext,
    coeffs: &[i32],
    vars: &[VariableId],
    rhs: i32,
) -> PropagationStatus {
    let mut changed = false;

    if max_sum(ctx, coeffs, vars) < rhs {
        return PropagationStatus::Failure;
    }

    for (index, &var) in vars.iter().enumerate() {
        let coeff = coeffs[index];
        if coeff == 0 {
            continue;
        }

        let other_max = max_sum_excluding(ctx, coeffs, vars, index);
        let slack = rhs - other_max;

        if coeff > 0 {
            let min_allowed = div_ceil(slack, coeff);
            if ctx.remove_below(var, min_allowed) {
                changed = true;
            }
        } else {
            let max_allowed = slack / coeff;
            if ctx.remove_above(var, max_allowed) {
                changed = true;
            }
        }
    }

    if vars.iter().any(|var| ctx.domain(*var).is_empty()) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn propagate_scalar_not_eq(
    ctx: &mut dyn PropagationContext,
    coeffs: &[i32],
    vars: &[VariableId],
    rhs: i32,
) -> PropagationStatus {
    let mut changed = false;

    for (index, &var) in vars.iter().enumerate() {
        for value in domain_values(ctx, var) {
            let contribution = coeffs[index].saturating_mul(value);
            let min_total = contribution + min_sum_excluding(ctx, coeffs, vars, index);
            let max_total = contribution + max_sum_excluding(ctx, coeffs, vars, index);
            if min_total == max_total && min_total == rhs && ctx.remove_value(var, value) {
                changed = true;
            }
        }
    }

    if vars.iter().any(|var| ctx.domain(*var).is_empty()) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn domain_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
    let domain = ctx.domain(var);
    let mut values = Vec::new();
    if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
        for value in min..=max {
            if domain.contains(value) {
                values.push(value);
            }
        }
    }
    values
}

fn finish_reified_scalar(
    ctx: &dyn PropagationContext,
    watched: &[VariableId],
    changed: bool,
) -> PropagationStatus {
    if watched.iter().any(|var| ctx.domain(*var).is_empty()) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn tighten_reif(ctx: &mut dyn PropagationContext, reif: VariableId, value: i32) -> bool {
    let mut changed = false;
    if ctx.remove_below(reif, value) {
        changed = true;
    }
    if ctx.remove_above(reif, value) {
        changed = true;
    }
    changed
}

fn min_sum(ctx: &dyn PropagationContext, coeffs: &[i32], vars: &[VariableId]) -> i32 {
    coeffs
        .iter()
        .zip(vars)
        .map(|(&coeff, &var)| contribution_min(ctx, var, coeff))
        .sum()
}

fn max_sum(ctx: &dyn PropagationContext, coeffs: &[i32], vars: &[VariableId]) -> i32 {
    coeffs
        .iter()
        .zip(vars)
        .map(|(&coeff, &var)| contribution_max(ctx, var, coeff))
        .sum()
}

fn min_sum_excluding(
    ctx: &dyn PropagationContext,
    coeffs: &[i32],
    vars: &[VariableId],
    skip: usize,
) -> i32 {
    coeffs
        .iter()
        .zip(vars)
        .enumerate()
        .filter(|(index, _)| *index != skip)
        .map(|(_, (&coeff, &var))| contribution_min(ctx, var, coeff))
        .sum()
}

fn max_sum_excluding(
    ctx: &dyn PropagationContext,
    coeffs: &[i32],
    vars: &[VariableId],
    skip: usize,
) -> i32 {
    coeffs
        .iter()
        .zip(vars)
        .enumerate()
        .filter(|(index, _)| *index != skip)
        .map(|(_, (&coeff, &var))| contribution_max(ctx, var, coeff))
        .sum()
}

fn contribution_min(ctx: &dyn PropagationContext, var: VariableId, coeff: i32) -> i32 {
    if coeff > 0 {
        coeff.saturating_mul(ctx.domain(var).min().unwrap_or(0))
    } else {
        coeff.saturating_mul(ctx.domain(var).max().unwrap_or(0))
    }
}

fn contribution_max(ctx: &dyn PropagationContext, var: VariableId, coeff: i32) -> i32 {
    if coeff > 0 {
        coeff.saturating_mul(ctx.domain(var).max().unwrap_or(0))
    } else {
        coeff.saturating_mul(ctx.domain(var).min().unwrap_or(0))
    }
}

fn div_ceil(numerator: i32, denominator: i32) -> i32 {
    if denominator == 0 {
        return if numerator >= 0 { i32::MAX } else { i32::MIN };
    }
    if denominator < 0 {
        return div_ceil(-numerator, -denominator);
    }
    if numerator >= 0 {
        (numerator + denominator - 1) / denominator
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn div_ceil_handles_zero_and_negative_denominator() {
        assert_eq!(div_ceil(5, 0), i32::MAX);
        assert_eq!(div_ceil(-5, 0), i32::MIN);
        assert_eq!(div_ceil(7, -3), div_ceil(-7, 3));
        assert_eq!(div_ceil(-7, 3), -2);
        assert_eq!(div_ceil(7, 3), 3);
    }

    #[test]
    fn reified_scalar_le_singleton_true_propagates() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
    }

    #[test]
    fn reified_scalar_le_fixed_true_infeasible_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(5, 10));
        let y = engine.new_variable(IntervalDomain::new(5, 10));
        let reif = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            3,
            reif,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_scalar_le_singleton_false_infeasible_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
            reif,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_scalar_ge_singleton_true_propagates() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![2, 1],
            vec![x, y],
            15,
            reif,
        )));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
    }

    #[test]
    fn reified_scalar_ge_fixed_false_infeasible_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::fix(3));
        let y = engine.new_variable(IntervalDomain::fix(3));
        let reif = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![1, 1],
            vec![x, y],
            5,
            reif,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_scalar_ge_singleton_false_propagates() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
    }

    #[test]
    fn reified_scalar_eq_singleton_true_propagates() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
    }

    #[test]
    fn reified_scalar_eq_singleton_false_propagates() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            4,
            reif,
        )));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkChanged
        );
    }

    #[test]
    fn reified_scalar_eq_empty_domain_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 0));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            4,
            reif,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn scalar_le_empty_domain_after_propagation_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 0));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn scalar_not_eq_prunes_with_holey_domain() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 5).remove(2).remove(4));
        let y = engine.new_variable(IntervalDomain::fix(3));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            6,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(x).contains(3));
    }

    #[test]
    fn weighted_sum_upper_bound_tightens_domains() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).max().unwrap() <= 3);
        assert!(engine.hybrid_domain(y).max().unwrap() <= 6);
    }

    #[test]
    fn reified_scalar_le_tightens_reif_when_sum_always_below() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).min(), Some(1));
        assert_eq!(engine.hybrid_domain(reif).max(), Some(1));
    }

    #[test]
    fn reified_scalar_le_propagates_when_reif_fixed() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).max().unwrap() <= 3);
    }

    #[test]
    fn reified_scalar_eq_reif_zero_prunes_forcing_values() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            4,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(x).contains(2));
    }

    #[test]
    fn reified_scalar_eq_fails_when_sum_fixed_and_reif_zero() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(2, 2));
        let y = engine.new_variable(IntervalDomain::new(3, 3));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            5,
            reif,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn weighted_sum_equality_fixes_total() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 4));
        let y = engine.new_variable(IntervalDomain::new(1, 4));
        let z = engine.new_variable(IntervalDomain::new(1, 4));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1, 1],
            vec![x, y, z],
            6,
        )));
        engine.add_propagator(Box::new(LinearScalarGePropagator::new(
            vec![1, 1, 1],
            vec![x, y, z],
            6,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(x).max(), Some(4));
        assert_eq!(engine.hybrid_domain(y).max(), Some(4));
    }

    #[test]
    fn weighted_sum_lower_bound_tightens_domains() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarGePropagator::new(
            vec![2, 1],
            vec![x, y],
            15,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).min().unwrap() >= 3);
        assert!(engine.hybrid_domain(y).min().unwrap() >= 0);
    }

    #[test]
    fn scalar_ge_infeasible_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(LinearScalarGePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn scalar_le_infeasible_fails() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(5, 10));
        let y = engine.new_variable(IntervalDomain::new(5, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            3,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn negative_coeff_le_tightens_lower_bound() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 5));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![-1, 1],
            vec![x, y],
            3,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(y).max(), Some(8));
    }

    #[test]
    fn negative_coeff_ge_tightens_upper_bound() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarGePropagator::new(
            vec![1, -1],
            vec![x, y],
            5,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(y).max(), Some(5));
    }

    #[test]
    fn zero_coeff_ignored_in_propagation() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 100));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![0, 1],
            vec![x, y],
            5,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(x).min(), Some(0));
        assert_eq!(engine.hybrid_domain(x).max(), Some(100));
        assert_eq!(engine.hybrid_domain(y).max(), Some(5));
    }

    #[test]
    fn reified_scalar_le_false_forces_ge() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![2, 1],
            vec![x, y],
            14,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).min().unwrap() >= 3);
    }

    #[test]
    fn reified_scalar_le_infers_false() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(5, 10));
        let y = engine.new_variable(IntervalDomain::new(5, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            3,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_scalar_ge_tightens_reif_when_sum_always_above() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(5, 10));
        let y = engine.new_variable(IntervalDomain::new(5, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![1, 1],
            vec![x, y],
            3,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_scalar_ge_propagates_when_reif_fixed() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![2, 1],
            vec![x, y],
            15,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).min().unwrap() >= 3);
    }

    #[test]
    fn reified_scalar_ge_false_forces_le() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).max().unwrap() <= 2);
    }

    #[test]
    fn reified_scalar_ge_infers_false() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarGePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_scalar_eq_propagates_when_reif_fixed() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![2, 1],
            vec![x, y],
            6,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).max().unwrap() <= 3);
        assert!(engine.hybrid_domain(y).max().unwrap() <= 6);
    }

    #[test]
    fn reified_scalar_eq_infers_true() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(2, 2));
        let y = engine.new_variable(IntervalDomain::new(3, 3));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            5,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_scalar_eq_infers_false() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn negative_coeff_div_ceil_tightens_ge_lower_bound() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(-10, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(LinearScalarGePropagator::new(
            vec![-3, 1],
            vec![x, y],
            1,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).max().unwrap() <= 0);
    }

    #[test]
    fn negative_coeff_div_ceil_tightens_le_lower_bound() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(-10, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 20));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![-3, 1],
            vec![x, y],
            7,
        )));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(x).min().unwrap() >= -2);
    }

    #[test]
    fn scalar_not_eq_prunes_forcing_value() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 5));
        let y = engine.new_variable(IntervalDomain::fix(3));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedScalarEqPropagator::new(
            vec![1, 1],
            vec![x, y],
            6,
            reif,
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(x).contains(3));
    }

    #[test]
    fn scalar_le_failure_on_empty_domain() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 0));
        let y = engine.new_variable(IntervalDomain::new(1, 5));
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(
            propagate_scalar_le(&mut ctx, &[1, 1], &[x, y], 10),
            PropagationStatus::Failure
        );
    }

    #[test]
    fn scalar_ge_failure_on_empty_domain() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 0));
        let y = engine.new_variable(IntervalDomain::new(1, 5));
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(
            propagate_scalar_ge(&mut ctx, &[1, 1], &[x, y], 1),
            PropagationStatus::Failure
        );
    }

    #[test]
    fn scalar_not_eq_failure_on_empty_domain() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 0));
        let y = engine.new_variable(IntervalDomain::new(1, 5));
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(
            propagate_scalar_not_eq(&mut ctx, &[1, 1], &[x, y], 6),
            PropagationStatus::Failure
        );
    }

    #[test]
    fn mock_reified_scalar_le_singleton_true_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4])
            .with_domain(y, vec![0, 1, 2, 3, 4])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_le_singleton_false_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2])
            .with_domain(y, vec![0, 1, 2])
            .with_domain(reif, vec![0]);
        let mut prop = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 2, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_ge_singleton_true_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4, 5])
            .with_domain(y, vec![0, 1, 2, 3, 4, 5])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 3, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_ge_singleton_false_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![4, 5, 6])
            .with_domain(y, vec![4, 5, 6])
            .with_domain(reif, vec![0]);
        let mut prop = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 10, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_eq_singleton_true_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4, 5])
            .with_domain(y, vec![0, 1, 2, 3, 4, 5])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 6, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_eq_singleton_false_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![1, 2, 3])
            .with_domain(y, vec![3])
            .with_domain(reif, vec![0]);
        let mut prop = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 6, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
        assert!(!ctx.domains[&x].values.borrow().contains(&3));
    }

    #[test]
    fn mock_reified_scalar_singleton_paths() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4])
            .with_domain(y, vec![0, 1, 2, 3, 4])
            .with_domain(reif, vec![1]);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_ne!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 3, reif);
        assert_ne!(ge.propagate(&mut ctx), PropagationStatus::Failure);

        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 6, reif);
        assert_ne!(eq.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_eq_singleton_false_failure_paths() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![3])
            .with_domain(y, vec![3])
            .with_domain(reif, vec![0]);
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 6, reif);
        assert_eq!(eq.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_eq_singleton_true_both_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![1, 2, 3])
            .with_domain(y, vec![1, 2, 3])
            .with_domain(reif, vec![1]);
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 4, reif);
        assert_ne!(eq.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_empty_operand_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![1, 2, 3])
            .with_domain(reif, vec![1]);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_eq!(ge.propagate(&mut ctx), PropagationStatus::Failure);

        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_eq!(eq.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_reif_scalar_propagators() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4])
            .with_domain(y, vec![0, 1, 2, 3, 4])
            .with_open_singleton(reif, 1);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_ne!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 3, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2, 3, 4])
            .with_domain(y, vec![0, 1, 2, 3, 4])
            .with_open_singleton(reif2, 1);
        assert_ne!(ge.propagate(&mut ctx2), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 4, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![1, 2, 3])
            .with_domain(y, vec![1, 2, 3])
            .with_open_singleton(reif3, 0);
        assert_ne!(eq.propagate(&mut ctx3), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_reif_scalar_false_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![10, 11, 12])
            .with_domain(y, vec![10, 11, 12])
            .with_open_singleton(reif, 0);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_ne!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 30, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(x, vec![10, 11, 12])
            .with_domain(y, vec![10, 11, 12])
            .with_open_singleton(reif2, 0);
        assert_ne!(ge.propagate(&mut ctx2), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_scalar_singleton_failure_and_empty_paths() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![10, 11])
            .with_domain(y, vec![10, 11])
            .with_open_singleton(reif, 1);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 50, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(x, vec![10, 11])
            .with_domain(y, vec![10, 11])
            .with_open_singleton(reif2, 1);
        assert_eq!(ge.propagate(&mut ctx2), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 20, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![10])
            .with_domain(y, vec![10])
            .with_open_singleton(reif3, 0);
        assert_eq!(eq.propagate(&mut ctx3), PropagationStatus::Failure);

        let reif4 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx4 = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif4, 1);
        let mut le_empty = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif4);
        assert_eq!(le_empty.propagate(&mut ctx4), PropagationStatus::Failure);
    }

    #[test]
    fn propagate_scalar_ge_skips_zero_coefficient() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![1, 2, 3])
            .with_domain(y, vec![4, 5, 6]);
        assert_eq!(
            propagate_scalar_ge(&mut ctx, &[0, 1], &[x, y], 5),
            PropagationStatus::OkChanged
        );
        assert_eq!(ctx.domain_values(y), vec![5, 6]);
    }

    #[test]
    fn mock_open_singleton_reif_scalar_failure_returns() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));

        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif);
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![1, 2])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif, 0);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 10, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(x, vec![1, 2])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif2, 1);
        assert_eq!(ge.propagate(&mut ctx2), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut le3 = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 5, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif3, 1);
        assert_eq!(le3.propagate(&mut ctx3), PropagationStatus::Failure);

        let reif4 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge4 = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 3, reif4);
        let mut ctx4 = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif4, 0);
        assert_eq!(ge4.propagate(&mut ctx4), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_invalid_reif_value_ignored() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2])
            .with_domain(y, vec![0, 1, 2])
            .with_open_singleton(reif, 2);
        let mut le = ReifiedScalarLePropagator::new(vec![1, 1], vec![x, y], 100, reif);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 3, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(x, vec![0, 1, 2])
            .with_domain(y, vec![0, 1, 2])
            .with_open_singleton(reif2, 2);
        assert_eq!(ge.propagate(&mut ctx2), PropagationStatus::OkNoChange);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 4, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![1, 2, 3])
            .with_domain(y, vec![1, 2, 3])
            .with_open_singleton(reif3, 2);
        assert_eq!(eq.propagate(&mut ctx3), PropagationStatus::OkNoChange);
    }

    #[test]
    fn mock_open_singleton_reif_eq_failure_paths() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 10, reif);
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![1, 2])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif, 1);
        assert_eq!(eq.propagate(&mut ctx), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq3 = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 6, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![3])
            .with_open_singleton(reif3, 1);
        assert_eq!(eq3.propagate(&mut ctx3), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_reif_ge_and_eq_failure_returns() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 0));
        let y = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ge = ReifiedScalarGePropagator::new(vec![1, 1], vec![x, y], 50, reif);
        let mut ctx = MockIntCtx::new()
            .with_domain(x, vec![10, 11])
            .with_domain(y, vec![10, 11])
            .with_open_singleton(reif, 1);
        assert_eq!(ge.propagate(&mut ctx), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq3 = ReifiedScalarEqPropagator::new(vec![1, 1], vec![x, y], 4, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(x, vec![])
            .with_domain(y, vec![1, 2])
            .with_open_singleton(reif3, 1);
        assert_eq!(eq3.propagate(&mut ctx3), PropagationStatus::Failure);
    }
}
