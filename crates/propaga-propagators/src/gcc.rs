use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};
use std::collections::HashMap;

/// Cardinality bounds for a single value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardinalityBound {
    /// Minimum occurrences of the value.
    pub min: i32,
    /// Maximum occurrences of the value.
    pub max: i32,
}

impl CardinalityBound {
    /// Creates bounds with the same minimum and maximum.
    #[must_use]
    pub const fn exact(count: i32) -> Self {
        Self {
            min: count,
            max: count,
        }
    }

    /// Creates inclusive bounds.
    #[must_use]
    pub const fn range(min: i32, max: i32) -> Self {
        Self { min, max }
    }
}

/// Propagates global cardinality with bounds consistency.
pub struct GlobalCardinalityPropagator {
    variables: Vec<VariableId>,
    cards: HashMap<i32, CardinalityBound>,
}

impl GlobalCardinalityPropagator {
    /// Creates a GCC propagator over `variables` and per-value bounds.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        cards: impl IntoIterator<Item = (i32, CardinalityBound)>,
    ) -> Self {
        Self {
            variables: variables.into(),
            cards: cards.into_iter().collect(),
        }
    }
}

impl Propagator for GlobalCardinalityPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.variables
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;

        loop {
            match propagate_bounds(ctx, &self.variables, &self.cards) {
                Ok(round_changed) => {
                    changed |= round_changed;
                    if !round_changed {
                        break;
                    }
                }
                Err(()) => return PropagationStatus::Failure,
            }
        }

        if self
            .variables
            .iter()
            .any(|var| ctx.domain(*var).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn propagate_bounds(
    ctx: &mut dyn PropagationContext,
    variables: &[VariableId],
    cards: &HashMap<i32, CardinalityBound>,
) -> Result<bool, ()> {
    let mut changed = false;
    let values = collect_relevant_values(ctx, variables, cards);

    for value in values {
        let bounds = cards
            .get(&value)
            .copied()
            .unwrap_or(CardinalityBound {
                min: 0,
                max: variables.len() as i32,
            });

        let mut fixed = Vec::new();
        let mut open = Vec::new();
        for &var in variables {
            match ctx.fixed_value(var) {
                Some(fixed_value) if fixed_value == value => fixed.push(var),
                Some(_) => {}
                None if ctx.domain(var).contains(value) => open.push(var),
                None => {}
            }
        }

        let fixed_count = fixed.len() as i32;
        let possible_count = fixed_count + open.len() as i32;

        if fixed_count > bounds.max || possible_count < bounds.min {
            return Err(());
        }

        if fixed_count == bounds.max {
            for &var in &open {
                if ctx.remove_value(var, value) {
                    changed = true;
                }
            }
        }

        if possible_count == bounds.min {
            for &var in &open {
                if ctx.fixed_value(var) != Some(value) {
                    changed |= remove_all_except(ctx, var, value);
                }
            }
        }

        for &var in variables {
            if !ctx.domain(var).contains(value) {
                continue;
            }
            let others_open = open
                .iter()
                .filter(|&&candidate| candidate != var)
                .count() as i32;
            let others_fixed = fixed_count - i32::from(ctx.fixed_value(var) == Some(value));
            if others_open + others_fixed < bounds.min - 1
                && ctx.remove_value(var, value)
            {
                changed = true;
            }
            if others_fixed + 1 > bounds.max && ctx.remove_value(var, value) {
                changed = true;
            }
        }
    }

    Ok(changed)
}

fn collect_relevant_values(
    ctx: &dyn PropagationContext,
    variables: &[VariableId],
    cards: &HashMap<i32, CardinalityBound>,
) -> Vec<i32> {
    let mut values: Vec<i32> = cards.keys().copied().collect();
    for &var in variables {
        if let (Some(min), Some(max)) = (ctx.domain(var).min(), ctx.domain(var).max()) {
            for value in min..=max {
                if ctx.domain(var).contains(value) && !values.contains(&value) {
                    values.push(value);
                }
            }
        }
    }
    values.sort_unstable();
    values
}

fn remove_all_except(ctx: &mut dyn PropagationContext, var: VariableId, keep: i32) -> bool {
    let values = collect_values(ctx, var);
    let mut changed = false;
    for value in values {
        if value != keep && ctx.remove_value(var, value) {
            changed = true;
        }
    }
    changed
}

fn collect_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn max_cardinality_removes_value_from_open_vars() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(GlobalCardinalityPropagator::new(
            vec![a, b, c],
            [(1, CardinalityBound::exact(2))],
        )));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(c).contains(1));
    }
}
