use crate::config::SearchConfig;
use crate::dfs::DepthFirstSearch;
use crate::stats::SearchStats;
use crate::value::{AssignmentValue, Solution};
use propaga_core::{DomainView, VariableId};
use propaga_domains::{AnyDomain, FloatDomain};
use propaga_engine::Engine;
use propaga_propagators::{FloatLePropagator, LessEqualPropagator, SetCardPropagator};

/// Optimization direction for branch-and-bound search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveDirection {
    /// Minimize the objective variable.
    Minimize,
    /// Maximize the objective variable.
    Maximize,
}

/// Typed objective value produced by optimization.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectiveValue {
    /// Integer objective value.
    Int(i32),
    /// Floating-point objective value.
    Float(f64),
    /// Set cardinality objective value.
    SetCardinality(usize),
}

impl ObjectiveValue {
    /// Returns the integer value when the objective is integral.
    #[must_use]
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the floating-point value when the objective is real-valued.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }
}

/// Branch-and-bound optimization target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizationTarget {
    /// Integer decision variable.
    Int(VariableId),
    /// Floating-point decision variable.
    Float(VariableId),
    /// Set variable optimized by cardinality.
    SetCardinality(VariableId),
}

/// Result of an optimization search.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationResult {
    /// Best solution found.
    pub solution: Option<Solution>,
    /// Best objective value.
    pub objective_value: Option<ObjectiveValue>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
    /// Number of feasible solutions encountered.
    pub solutions_found: u32,
    /// Whether search stopped because the time limit was reached.
    pub timed_out: bool,
}

/// Branch-and-bound optimization over a single objective.
pub struct OptimizationSearch {
    variables: Vec<VariableId>,
    target: OptimizationTarget,
    direction: ObjectiveDirection,
    config: SearchConfig,
}

impl OptimizationSearch {
    /// Creates an optimization search over integer `variables` and `objective`.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objective: VariableId,
        direction: ObjectiveDirection,
        config: SearchConfig,
    ) -> Self {
        Self::with_target(
            variables,
            OptimizationTarget::Int(objective),
            direction,
            config,
        )
    }

    /// Creates an optimization search for a typed objective target.
    #[must_use]
    pub fn with_target(
        variables: impl Into<Vec<VariableId>>,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            target,
            direction,
            config,
        }
    }

    /// Runs branch-and-bound until no improving solution remains.
    pub fn optimize(&mut self, engine: &mut Engine) -> OptimizationResult {
        let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config);
        let mut best_solution = None;
        let mut best_value = None;
        let mut total_stats = SearchStats::default();
        let mut solutions_found = 0;
        let mut timed_out = false;

        loop {
            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }

            let solution = dfs.solve(engine);
            merge_stats(&mut total_stats, dfs.stats());
            if dfs.stats().timed_out {
                timed_out = true;
            }

            let Some(solution) = solution else {
                break;
            };

            solutions_found += 1;
            let objective_value = objective_value_from_solution(engine, self.target, &solution);
            let Some(value) = objective_value else {
                break;
            };

            let is_improvement = match &best_value {
                None => true,
                Some(best) => is_better(self.direction, &value, best),
            };

            if is_improvement {
                best_value = Some(value);
                best_solution = Some(solution);
            }

            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }

            let Some(best) = best_value.as_ref() else {
                break;
            };
            if !self.post_pruning_bound(engine, best) {
                break;
            }
        }

        total_stats.timed_out = timed_out;

        OptimizationResult {
            solution: best_solution,
            objective_value: best_value,
            stats: total_stats,
            solutions_found,
            timed_out,
        }
    }

    fn post_pruning_bound(&mut self, engine: &mut Engine, best: &ObjectiveValue) -> bool {
        match (self.target, best) {
            (OptimizationTarget::Int(objective), ObjectiveValue::Int(best)) => {
                post_int_pruning_bound(engine, objective, self.direction, *best)
            }
            (OptimizationTarget::Float(objective), ObjectiveValue::Float(best)) => {
                post_float_pruning_bound(engine, objective, self.direction, *best)
            }
            (OptimizationTarget::SetCardinality(set), ObjectiveValue::SetCardinality(best)) => {
                post_set_pruning_bound(engine, set, self.direction, *best)
            }
            _ => false,
        }
    }
}

/// Returns `true` when `candidate` is strictly better than `best` under `direction`.
#[must_use]
pub fn is_better(
    direction: ObjectiveDirection,
    candidate: &ObjectiveValue,
    best: &ObjectiveValue,
) -> bool {
    match (direction, candidate, best) {
        (ObjectiveDirection::Minimize, ObjectiveValue::Int(a), ObjectiveValue::Int(b)) => a < b,
        (ObjectiveDirection::Maximize, ObjectiveValue::Int(a), ObjectiveValue::Int(b)) => a > b,
        (ObjectiveDirection::Minimize, ObjectiveValue::Float(a), ObjectiveValue::Float(b)) => a < b,
        (ObjectiveDirection::Maximize, ObjectiveValue::Float(a), ObjectiveValue::Float(b)) => a > b,
        (
            ObjectiveDirection::Minimize,
            ObjectiveValue::SetCardinality(a),
            ObjectiveValue::SetCardinality(b),
        ) => a < b,
        (
            ObjectiveDirection::Maximize,
            ObjectiveValue::SetCardinality(a),
            ObjectiveValue::SetCardinality(b),
        ) => a > b,
        _ => false,
    }
}

fn post_int_pruning_bound(
    engine: &mut Engine,
    objective: VariableId,
    direction: ObjectiveDirection,
    best: i32,
) -> bool {
    let bound = match direction {
        ObjectiveDirection::Minimize => best.saturating_sub(1),
        ObjectiveDirection::Maximize => best.saturating_add(1),
    };

    if !int_bound_is_feasible(engine, objective, direction, bound) {
        return false;
    }

    let bound_var = engine.new_variable(propaga_domains::HybridDomain::fix(bound));
    match direction {
        ObjectiveDirection::Minimize => {
            engine.add_propagator(Box::new(LessEqualPropagator::new(objective, bound_var)));
        }
        ObjectiveDirection::Maximize => {
            engine.add_propagator(Box::new(LessEqualPropagator::new(bound_var, objective)));
        }
    }

    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
    }
}

fn post_float_pruning_bound(
    engine: &mut Engine,
    objective: VariableId,
    direction: ObjectiveDirection,
    best: f64,
) -> bool {
    let Some(bound) = float_prune_bound(direction, best) else {
        return false;
    };

    if !float_bound_is_feasible(engine, objective, direction, bound) {
        return false;
    }

    let bound_var = engine.new_variable(AnyDomain::Float(FloatDomain::fix(bound)));
    match direction {
        ObjectiveDirection::Minimize => {
            engine.add_propagator(Box::new(FloatLePropagator::new(objective, bound_var)));
        }
        ObjectiveDirection::Maximize => {
            engine.add_propagator(Box::new(FloatLePropagator::new(bound_var, objective)));
        }
    }

    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
    }
}

fn float_prune_bound(direction: ObjectiveDirection, best: f64) -> Option<f64> {
    match direction {
        ObjectiveDirection::Minimize => {
            let bound = next_float_down(best);
            (bound < best).then_some(bound)
        }
        ObjectiveDirection::Maximize => {
            let bound = next_float_up(best);
            (bound > best).then_some(bound)
        }
    }
}

fn post_set_pruning_bound(
    engine: &mut Engine,
    set: VariableId,
    direction: ObjectiveDirection,
    best: usize,
) -> bool {
    let Some(domain) = engine.domain(set).as_set().cloned() else {
        return false;
    };

    let tightened = match direction {
        ObjectiveDirection::Minimize => {
            if best == 0 {
                return false;
            }
            let card_max = best - 1;
            if domain.card_min() > card_max {
                return false;
            }
            let card_min = domain.card_min();
            domain.with_cardinality(card_min, card_max)
        }
        ObjectiveDirection::Maximize => {
            let card_min = best + 1;
            if domain.card_max() < card_min {
                return false;
            }
            let card_max = domain.card_max();
            domain.with_cardinality(card_min, card_max)
        }
    };

    if tightened.is_empty() {
        return false;
    }

    engine.set_domain(set, AnyDomain::Set(tightened));
    engine.add_propagator(Box::new(SetCardPropagator::new(set)));

    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
    }
}

fn int_bound_is_feasible(
    engine: &Engine,
    objective: VariableId,
    direction: ObjectiveDirection,
    bound: i32,
) -> bool {
    let domain = engine.int_domain(objective).expect("int objective");
    match direction {
        ObjectiveDirection::Minimize => domain.min().is_some_and(|min| bound >= min),
        ObjectiveDirection::Maximize => domain.max().is_some_and(|max| bound <= max),
    }
}

fn float_bound_is_feasible(
    engine: &Engine,
    objective: VariableId,
    direction: ObjectiveDirection,
    bound: f64,
) -> bool {
    let Some(domain) = engine.domain(objective).as_float().copied() else {
        return false;
    };
    match direction {
        ObjectiveDirection::Minimize => domain.lower_bound() <= bound,
        ObjectiveDirection::Maximize => domain.upper_bound() >= bound,
    }
}

/// Reads the typed objective value for `target` from a solution (or fixed domain).
#[must_use]
pub fn objective_value_from_solution(
    engine: &Engine,
    target: OptimizationTarget,
    solution: &Solution,
) -> Option<ObjectiveValue> {
    match target {
        OptimizationTarget::Int(objective) => solution
            .iter()
            .find(|(var, _)| *var == objective)
            .and_then(|(_, value)| match value {
                AssignmentValue::Int(value) => Some(*value),
                _ => None,
            })
            .map(ObjectiveValue::Int)
            .or_else(|| {
                engine
                    .int_domain(objective)
                    .and_then(|domain| domain.fixed_value())
                    .map(ObjectiveValue::Int)
            }),
        OptimizationTarget::Float(objective) => solution
            .iter()
            .find(|(var, _)| *var == objective)
            .and_then(|(_, value)| match value {
                AssignmentValue::Float(value) => Some(*value),
                _ => None,
            })
            .map(ObjectiveValue::Float)
            .or_else(|| {
                engine.domain(objective).as_float().and_then(|domain| {
                    domain
                        .is_fixed()
                        .then_some(domain.lower_bound())
                        .map(ObjectiveValue::Float)
                })
            }),
        OptimizationTarget::SetCardinality(set) => solution
            .iter()
            .find(|(var, _)| *var == set)
            .and_then(|(_, value)| match value {
                AssignmentValue::Set(values) => Some(values.len()),
                _ => None,
            })
            .map(ObjectiveValue::SetCardinality)
            .or_else(|| {
                engine
                    .domain(set)
                    .as_set()
                    .and_then(|domain| {
                        domain
                            .fixed_values()
                            .map(|values| values.len())
                            .or_else(|| domain.is_fixed().then_some(domain.glb().len()))
                    })
                    .map(ObjectiveValue::SetCardinality)
            }),
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

fn merge_stats(total: &mut SearchStats, partial: SearchStats) {
    total.nodes += partial.nodes;
    total.backtracks += partial.backtracks;
    total.conflicts += partial.conflicts;
    total.nogoods_learned += partial.nogoods_learned;
    total.restarts += partial.restarts;
    total.timed_out |= partial.timed_out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::IntervalDomain;
    use propaga_propagators::{LessEqualPropagator, LinearScalarLePropagator};

    #[test]
    fn maximizes_single_variable() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
        )));

        let mut search = OptimizationSearch::new(
            vec![x, y],
            x,
            ObjectiveDirection::Maximize,
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert!(
            result
                .objective_value
                .as_ref()
                .and_then(|v| v.as_int())
                .unwrap()
                >= 5
        );
    }

    #[test]
    fn minimizes_single_variable() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let lower_bound = engine.new_variable(IntervalDomain::fix(5));
        engine.add_propagator(Box::new(LessEqualPropagator::new(lower_bound, x)));

        let mut search = OptimizationSearch::new(
            vec![x],
            x,
            ObjectiveDirection::Minimize,
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(result.objective_value, Some(ObjectiveValue::Int(5)));
    }

    #[test]
    fn minimizes_float_variable() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        let mut search = OptimizationSearch::with_target(
            vec![x],
            OptimizationTarget::Float(x),
            ObjectiveDirection::Minimize,
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(result.objective_value, Some(ObjectiveValue::Float(0.0)));
    }
}
