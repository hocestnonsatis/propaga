use crate::reified::reif_literal;
use propaga_core::{
    FloatDomainSnapshot, PropagationContext, PropagationStatus, Propagator, VariableId,
};

/// Propagates `sum(coeffs[i] * vars[i]) == rhs` with interval bounds and hole projection.
#[derive(Clone, Debug)]
pub struct FloatLinearEqPropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
}

impl FloatLinearEqPropagator {
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<f64>>, vars: impl Into<Vec<VariableId>>, rhs: f64) -> Self {
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

impl Propagator for FloatLinearEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_float_eq(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `sum(coeffs[i] * vars[i]) != rhs` with interval bound consistency.
#[derive(Clone, Debug)]
pub struct FloatLinearNePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
}

impl FloatLinearNePropagator {
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<f64>>, vars: impl Into<Vec<VariableId>>, rhs: f64) -> Self {
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

impl Propagator for FloatLinearNePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        13
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_float_ne(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `sum(coeffs[i] * vars[i]) <= rhs` with interval bound consistency.
#[derive(Clone, Debug)]
pub struct FloatLinearLePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
}

impl FloatLinearLePropagator {
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<f64>>, vars: impl Into<Vec<VariableId>>, rhs: f64) -> Self {
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

impl Propagator for FloatLinearLePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_float_le(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `sum(coeffs[i] * vars[i]) >= rhs` with interval bound consistency.
#[derive(Clone, Debug)]
pub struct FloatLinearGePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
}

impl FloatLinearGePropagator {
    #[must_use]
    pub fn new(coeffs: impl Into<Vec<f64>>, vars: impl Into<Vec<VariableId>>, rhs: f64) -> Self {
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

impl Propagator for FloatLinearGePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        propagate_float_ge(ctx, &self.coeffs, &self.watched, self.rhs)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) <= rhs`.
#[derive(Clone, Debug)]
pub struct ReifiedFloatLinearLePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
    reif: VariableId,
}

impl ReifiedFloatLinearLePropagator {
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        vars.push(reif);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif,
        }
    }
}

impl Propagator for ReifiedFloatLinearLePropagator {
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
                let status = propagate_float_le(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            Some(0) => {
                let status = propagate_float_ge(ctx, &self.coeffs, &vars, next_up(self.rhs));
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

        finish_reified_float_scalar(ctx, self.reif, &vars, changed)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) == rhs`.
#[derive(Clone, Debug)]
pub struct ReifiedFloatLinearEqPropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
    reif: VariableId,
}

impl ReifiedFloatLinearEqPropagator {
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        vars.push(reif);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif,
        }
    }
}

impl Propagator for ReifiedFloatLinearEqPropagator {
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
                let status = propagate_float_eq(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            Some(0) => {
                let status = propagate_float_ne(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            _ => {}
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if (min_total - self.rhs).abs() < f64::EPSILON
            && (max_total - self.rhs).abs() < f64::EPSILON
        {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if min_total > self.rhs || max_total < self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        finish_reified_float_scalar(ctx, self.reif, &vars, changed)
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) >= rhs`.
#[derive(Clone, Debug)]
pub struct ReifiedFloatLinearGePropagator {
    watched: Vec<VariableId>,
    coeffs: Vec<f64>,
    rhs: f64,
    reif: VariableId,
}

impl ReifiedFloatLinearGePropagator {
    #[must_use]
    pub fn new(
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) -> Self {
        let coeffs = coeffs.into();
        let mut vars = vars.into();
        assert_eq!(coeffs.len(), vars.len());
        vars.push(reif);
        Self {
            watched: vars,
            coeffs,
            rhs,
            reif,
        }
    }
}

impl Propagator for ReifiedFloatLinearGePropagator {
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
                let status = propagate_float_ge(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            Some(0) => {
                let status = propagate_float_le(ctx, &self.coeffs, &vars, next_down(self.rhs));
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

        finish_reified_float_scalar(ctx, self.reif, &vars, changed)
    }
}

fn finish_reified_float_scalar(
    ctx: &mut dyn PropagationContext,
    reif: VariableId,
    float_vars: &[VariableId],
    changed: bool,
) -> PropagationStatus {
    if ctx.domain(reif).is_empty() || any_empty(ctx, float_vars) {
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

fn min_term(coeff: f64, domain: &FloatDomainSnapshot) -> f64 {
    if coeff >= 0.0 {
        coeff * domain.min
    } else {
        coeff * domain.max
    }
}

fn max_term(coeff: f64, domain: &FloatDomainSnapshot) -> f64 {
    if coeff >= 0.0 {
        coeff * domain.max
    } else {
        coeff * domain.min
    }
}

fn float_domain(ctx: &mut dyn PropagationContext, var: VariableId) -> Option<FloatDomainSnapshot> {
    ctx.as_extended()
        .and_then(|ext| ext.float_domain(var))
        .filter(|domain| !domain.is_empty())
}

fn snapshot_domains(
    ctx: &mut dyn PropagationContext,
    vars: &[VariableId],
) -> Option<Vec<FloatDomainSnapshot>> {
    let mut domains = Vec::with_capacity(vars.len());
    for &var in vars {
        domains.push(float_domain(ctx, var)?);
    }
    Some(domains)
}

fn min_sum_domains(coeffs: &[f64], domains: &[FloatDomainSnapshot]) -> f64 {
    coeffs
        .iter()
        .zip(domains)
        .map(|(&coeff, domain)| min_term(coeff, domain))
        .sum()
}

fn max_sum_domains(coeffs: &[f64], domains: &[FloatDomainSnapshot]) -> f64 {
    coeffs
        .iter()
        .zip(domains)
        .map(|(&coeff, domain)| max_term(coeff, domain))
        .sum()
}

fn min_sum_excluding_domains(coeffs: &[f64], domains: &[FloatDomainSnapshot], skip: usize) -> f64 {
    coeffs
        .iter()
        .zip(domains)
        .enumerate()
        .filter(|(index, _)| *index != skip)
        .map(|(_, (&coeff, domain))| min_term(coeff, domain))
        .sum()
}

fn max_sum_excluding_domains(coeffs: &[f64], domains: &[FloatDomainSnapshot], skip: usize) -> f64 {
    coeffs
        .iter()
        .zip(domains)
        .enumerate()
        .filter(|(index, _)| *index != skip)
        .map(|(_, (&coeff, domain))| max_term(coeff, domain))
        .sum()
}

fn min_sum(ctx: &mut dyn PropagationContext, coeffs: &[f64], vars: &[VariableId]) -> f64 {
    snapshot_domains(ctx, vars)
        .map(|domains| min_sum_domains(coeffs, &domains))
        .unwrap_or(f64::INFINITY)
}

fn max_sum(ctx: &mut dyn PropagationContext, coeffs: &[f64], vars: &[VariableId]) -> f64 {
    snapshot_domains(ctx, vars)
        .map(|domains| max_sum_domains(coeffs, &domains))
        .unwrap_or(f64::NEG_INFINITY)
}

fn any_empty(ctx: &mut dyn PropagationContext, vars: &[VariableId]) -> bool {
    vars.iter().any(|&var| {
        ctx.as_extended()
            .and_then(|ext| ext.float_domain(var))
            .is_none_or(|domain| domain.is_empty())
    })
}

fn propagate_float_le(
    ctx: &mut dyn PropagationContext,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
) -> PropagationStatus {
    let Some(domains) = snapshot_domains(ctx, vars) else {
        return PropagationStatus::Failure;
    };

    if min_sum_domains(coeffs, &domains) > rhs {
        return PropagationStatus::Failure;
    }

    let Some(ext) = ctx.as_extended() else {
        return PropagationStatus::OkNoChange;
    };

    let mut changed = false;
    for (index, &var) in vars.iter().enumerate() {
        let coeff = coeffs[index];
        if coeff == 0.0 {
            continue;
        }
        let other_min = min_sum_excluding_domains(coeffs, &domains, index);
        let slack = rhs - other_min;
        if coeff > 0.0 {
            let max_allowed = slack / coeff;
            changed |= ext.tighten_float_above(var, max_allowed);
        } else {
            let min_allowed = slack / coeff;
            changed |= ext.tighten_float_below(var, min_allowed);
        }
    }

    if any_empty(ctx, vars) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn propagate_float_ge(
    ctx: &mut dyn PropagationContext,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
) -> PropagationStatus {
    let Some(domains) = snapshot_domains(ctx, vars) else {
        return PropagationStatus::Failure;
    };

    if max_sum_domains(coeffs, &domains) < rhs {
        return PropagationStatus::Failure;
    }

    let Some(ext) = ctx.as_extended() else {
        return PropagationStatus::OkNoChange;
    };

    let mut changed = false;
    for (index, &var) in vars.iter().enumerate() {
        let coeff = coeffs[index];
        if coeff == 0.0 {
            continue;
        }
        let other_max = max_sum_excluding_domains(coeffs, &domains, index);
        let slack = rhs - other_max;
        if coeff > 0.0 {
            let min_allowed = slack / coeff;
            changed |= ext.tighten_float_below(var, min_allowed);
        } else {
            let max_allowed = slack / coeff;
            changed |= ext.tighten_float_above(var, max_allowed);
        }
    }

    if any_empty(ctx, vars) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

fn propagate_float_ne(
    ctx: &mut dyn PropagationContext,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
) -> PropagationStatus {
    let Some(domains) = snapshot_domains(ctx, vars) else {
        return PropagationStatus::Failure;
    };

    let min_total = min_sum_domains(coeffs, &domains);
    let max_total = max_sum_domains(coeffs, &domains);

    if min_total > rhs || max_total < rhs {
        return PropagationStatus::OkNoChange;
    }
    if (min_total - max_total).abs() <= f64::EPSILON && (min_total - rhs).abs() <= f64::EPSILON {
        return PropagationStatus::Failure;
    }

    // Only one side of sum ≠ rhs remains feasible.
    if (min_total - rhs).abs() <= f64::EPSILON {
        return propagate_float_ge(ctx, coeffs, vars, next_up(rhs));
    }
    if (max_total - rhs).abs() <= f64::EPSILON {
        return propagate_float_le(ctx, coeffs, vars, next_down(rhs));
    }

    // When all but one term are fixed, exclude the unique equality-forcing value
    // (endpoint shrink or interior hole).
    let Some(ext) = ctx.as_extended() else {
        return PropagationStatus::OkNoChange;
    };
    let mut changed = false;
    for (index, &var) in vars.iter().enumerate() {
        let coeff = coeffs[index];
        if coeff == 0.0 {
            continue;
        }
        let other_min = min_sum_excluding_domains(coeffs, &domains, index);
        let other_max = max_sum_excluding_domains(coeffs, &domains, index);
        if (other_min - other_max).abs() > f64::EPSILON {
            continue;
        }
        let required = (rhs - other_min) / coeff;
        if required.is_finite() {
            changed |= ext.exclude_float_point(var, required);
        }
    }

    if any_empty(ctx, vars) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

/// Propagates `sum(coeffs[i] * vars[i]) == rhs` with bounds and hole projection.
fn propagate_float_eq(
    ctx: &mut dyn PropagationContext,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
) -> PropagationStatus {
    let le = propagate_float_le(ctx, coeffs, vars, rhs);
    if le.is_failure() {
        return le;
    }
    let ge = propagate_float_ge(ctx, coeffs, vars, rhs);
    if ge.is_failure() {
        return ge;
    }
    let mut changed = le == PropagationStatus::OkChanged || ge == PropagationStatus::OkChanged;
    changed |= project_float_lin_eq_holes(ctx, coeffs, vars, rhs);

    if any_empty(ctx, vars) {
        PropagationStatus::Failure
    } else if changed {
        PropagationStatus::OkChanged
    } else {
        PropagationStatus::OkNoChange
    }
}

/// When all but two terms are fixed under equality, map holes through the affine link.
fn project_float_lin_eq_holes(
    ctx: &mut dyn PropagationContext,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
) -> bool {
    let Some(domains) = snapshot_domains(ctx, vars) else {
        return false;
    };
    let free: Vec<usize> = domains
        .iter()
        .enumerate()
        .filter(|(_, domain)| !domain.is_fixed())
        .map(|(index, _)| index)
        .collect();
    if free.len() != 2 {
        return false;
    }
    let i = free[0];
    let j = free[1];
    let ci = coeffs[i];
    let cj = coeffs[j];
    if ci == 0.0 || cj == 0.0 {
        return false;
    }
    let fixed_sum: f64 = coeffs
        .iter()
        .zip(&domains)
        .enumerate()
        .filter(|(index, _)| *index != i && *index != j)
        .map(|(_, (&coeff, domain))| coeff * domain.min)
        .sum();
    let rhs_prime = rhs - fixed_sum;
    let Some(ext) = ctx.as_extended() else {
        return false;
    };
    let mut changed = false;
    for hole in &domains[j].holes {
        let mapped = (rhs_prime - cj * hole) / ci;
        if mapped.is_finite() {
            changed |= ext.exclude_float_point(vars[i], mapped);
        }
    }
    for hole in &domains[i].holes {
        let mapped = (rhs_prime - ci * hole) / cj;
        if mapped.is_finite() {
            changed |= ext.exclude_float_point(vars[j], mapped);
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::{AnyDomain, FloatDomain, HybridDomain};
    use propaga_engine::Engine;

    #[test]
    fn float_lin_ne_fails_when_sum_fixed_at_rhs() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(3.0)));
        engine.add_propagator(Box::new(FloatLinearNePropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn float_lin_ne_forces_above_when_sum_min_equals_rhs() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(4.0)));
        engine.add_propagator(Box::new(FloatLinearNePropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn float_lin_ne_prunes_unit_endpoint() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatLinearNePropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            3.0,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn float_lin_ne_excludes_interior_when_others_fixed() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatLinearNePropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(x).as_float().unwrap().contains(3.0));
    }

    #[test]
    fn float_lin_eq_shares_holes_across_affine_link() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0).exclude(2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        engine.add_propagator(Box::new(FloatLinearEqPropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(y).as_float().unwrap().contains(3.0));
    }

    #[test]
    fn float_lin_eq_fails_when_forced_value_is_a_hole() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0).exclude(2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(3.0)));
        engine.add_propagator(Box::new(FloatLinearEqPropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_float_eq_false_uses_ne_not_both_bounds() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 4.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 4.0)));
        let reif = engine.new_variable(HybridDomain::fix(0));
        engine.add_propagator(Box::new(ReifiedFloatLinearEqPropagator::new(
            vec![1.0, 1.0],
            vec![x, y],
            5.0,
            reif,
        )));
        let status = engine.propagate_all().unwrap();
        assert_ne!(status, PropagationStatus::Failure);
        assert!(engine.domain(x).as_float().unwrap().lower_bound() <= 4.0);
        assert!(engine.domain(y).as_float().unwrap().upper_bound() >= 0.0);
    }
}
