use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `sum(coeffs[i] * vars[i]) <= rhs` with bound consistency.
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

        match ctx.fixed_value(self.reif) {
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

        if ctx.domain(self.reif).size() == 1 {
            match ctx.domain(self.reif).min() {
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
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if max_total <= self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if min_total > self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        if self.watched.iter().any(|var| ctx.domain(*var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) >= rhs`.
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

        match ctx.fixed_value(self.reif) {
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

        if ctx.domain(self.reif).size() == 1 {
            match ctx.domain(self.reif).min() {
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
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if min_total >= self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if max_total < self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        if self.watched.iter().any(|var| ctx.domain(*var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif == 1 <=> sum(coeffs[i] * vars[i]) == rhs`.
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

        match ctx.fixed_value(self.reif) {
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
                let status = propagate_scalar_not_eq(ctx, &self.coeffs, &vars, self.rhs);
                if status.is_failure() {
                    return status;
                }
                changed |= status == PropagationStatus::OkChanged;
            }
            _ => {}
        }

        if ctx.domain(self.reif).size() == 1 {
            match ctx.domain(self.reif).min() {
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
                    let status = propagate_scalar_not_eq(ctx, &self.coeffs, &vars, self.rhs);
                    if status.is_failure() {
                        return status;
                    }
                    changed |= status == PropagationStatus::OkChanged;
                }
                _ => {}
            }
        }

        let min_total = min_sum(ctx, &self.coeffs, &vars);
        let max_total = max_sum(ctx, &self.coeffs, &vars);
        if min_total == max_total && min_total == self.rhs {
            changed |= tighten_reif(ctx, self.reif, 1);
        } else if min_total > self.rhs || max_total < self.rhs {
            changed |= tighten_reif(ctx, self.reif, 0);
        }

        if self.watched.iter().any(|var| ctx.domain(*var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
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
        assert!(engine.domain(x).max().unwrap() <= 3);
        assert!(engine.domain(y).max().unwrap() <= 6);
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
        assert_eq!(engine.domain(reif).min(), Some(1));
        assert_eq!(engine.domain(reif).max(), Some(1));
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
        assert!(engine.domain(x).max().unwrap() <= 3);
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
        assert!(!engine.domain(x).contains(2));
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
        assert_eq!(engine.domain(x).max(), Some(4));
        assert_eq!(engine.domain(y).max(), Some(4));
    }
}
