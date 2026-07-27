//! Lexicographic multi-objective optimization.

use crate::config::{SearchConfig, SearchPhase};
use crate::optimize::{ObjectiveDirection, ObjectiveValue, OptimizationSearch, OptimizationTarget};
use crate::stats::SearchStats;
use crate::value::Solution;
use propaga_core::VariableId;
use propaga_domains::{AnyDomain, FloatDomain, HybridDomain};
use propaga_engine::Engine;
use propaga_propagators::SetCardPropagator;

/// One objective in a lexicographic optimization problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Objective {
    /// Typed optimization target.
    pub target: OptimizationTarget,
    /// Optimization direction.
    pub direction: ObjectiveDirection,
}

impl Objective {
    /// Creates an integer lexicographic objective.
    #[must_use]
    pub fn int(var: VariableId, direction: ObjectiveDirection) -> Self {
        Self {
            target: OptimizationTarget::Int(var),
            direction,
        }
    }

    /// Creates a floating-point lexicographic objective.
    #[must_use]
    pub fn float(var: VariableId, direction: ObjectiveDirection) -> Self {
        Self {
            target: OptimizationTarget::Float(var),
            direction,
        }
    }

    /// Creates a set-cardinality lexicographic objective.
    #[must_use]
    pub fn set_cardinality(var: VariableId, direction: ObjectiveDirection) -> Self {
        Self {
            target: OptimizationTarget::SetCardinality(var),
            direction,
        }
    }
}

/// Result of lexicographic optimization.
#[derive(Clone, Debug, PartialEq)]
pub struct LexicographicResult {
    /// Best solution found.
    pub solution: Option<Solution>,
    /// Objective values in priority order.
    pub objective_values: Vec<ObjectiveValue>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
}

/// Lexicographic branch-and-bound over multiple objectives.
pub struct LexicographicOptimization {
    variables: Vec<VariableId>,
    objectives: Vec<Objective>,
    config: SearchConfig,
    search_phases: Vec<SearchPhase>,
}

impl LexicographicOptimization {
    /// Creates a lexicographic optimizer.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<Objective>,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            objectives,
            config,
            search_phases: Vec::new(),
        }
    }

    /// Attaches sequenced search phases shared by every BnB phase.
    #[must_use]
    pub fn with_search_phases(mut self, search_phases: impl Into<Vec<SearchPhase>>) -> Self {
        self.search_phases = search_phases.into();
        self
    }

    /// Optimizes objectives in order, fixing each optimal value before the next.
    pub fn optimize(&mut self, engine: &mut Engine) -> LexicographicResult {
        let mut total_stats = SearchStats::default();
        let mut objective_values = Vec::new();
        let mut best_solution = None;

        for (index, objective) in self.objectives.clone().into_iter().enumerate() {
            // BnB posts improving bounds permanently; restore afterward so we can
            // pin the objective exactly to its optimum for the next priority.
            let checkpoint = engine.checkpoint();
            let mut search = OptimizationSearch::with_target(
                self.variables.clone(),
                objective.target,
                objective.direction,
                self.config,
            )
            .with_search_phases(self.search_phases.clone());
            let result = search.optimize(engine);
            merge_stats(&mut total_stats, result.stats);

            let Some(value) = result.objective_value else {
                engine.restore_checkpoint(&checkpoint);
                return LexicographicResult {
                    solution: None,
                    objective_values,
                    stats: total_stats,
                };
            };

            engine.restore_checkpoint(&checkpoint);
            objective_values.push(value.clone());
            best_solution = result.solution.or(best_solution);

            if index + 1 < self.objectives.len() {
                fix_objective(engine, objective.target, &value);
            }
        }

        LexicographicResult {
            solution: best_solution,
            objective_values,
            stats: total_stats,
        }
    }
}

fn fix_objective(engine: &mut Engine, target: OptimizationTarget, value: &ObjectiveValue) {
    match (target, value) {
        (OptimizationTarget::Int(var), ObjectiveValue::Int(value)) => {
            engine.set_domain(var, AnyDomain::Int(HybridDomain::fix(*value)));
        }
        (OptimizationTarget::Float(var), ObjectiveValue::Float(value)) => {
            engine.set_domain(var, AnyDomain::Float(FloatDomain::fix(*value)));
        }
        (OptimizationTarget::SetCardinality(var), ObjectiveValue::SetCardinality(card)) => {
            let Some(domain) = engine.domain(var).as_set().cloned() else {
                return;
            };
            let tightened = domain.with_cardinality(*card, *card);
            if tightened.is_empty() {
                return;
            }
            engine.set_domain(var, AnyDomain::Set(tightened));
            engine.add_propagator(Box::new(SetCardPropagator::new(var)));
        }
        _ => return,
    }
    let _ = engine.commit_initial_propagation();
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
    use propaga_propagators::FloatLePropagator;

    #[test]
    fn lexicographic_minimizes_in_order() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let mut search = LexicographicOptimization::new(
            vec![x, y],
            vec![
                Objective::int(x, ObjectiveDirection::Minimize),
                Objective::int(y, ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(
            result.objective_values,
            vec![ObjectiveValue::Int(1), ObjectiveValue::Int(1)]
        );
        assert!(result.solution.is_some());
    }

    #[test]
    fn lexicographic_minimizes_float_then_int() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let bound = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        engine.add_propagator(Box::new(FloatLePropagator::new(bound, x)));
        let _ = engine.commit_initial_propagation();

        let mut search = LexicographicOptimization::new(
            vec![x, y],
            vec![
                Objective::float(x, ObjectiveDirection::Minimize),
                Objective::int(y, ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(result.objective_values[0], ObjectiveValue::Float(1.0));
        assert_eq!(result.objective_values[1], ObjectiveValue::Int(1));
        assert!(result.solution.is_some());
    }

    #[test]
    fn lexicographic_fixes_non_boundary_int_optimum() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 5));
        let y = engine.new_variable(IntervalDomain::new(1, 5));
        // Force x >= 3 so the minimize optimum is interior to the original domain.
        let lower = engine.new_variable(HybridDomain::fix(3));
        engine.add_propagator(Box::new(propaga_propagators::LessEqualPropagator::new(
            lower, x,
        )));
        let _ = engine.commit_initial_propagation();

        let mut search = LexicographicOptimization::new(
            vec![x, y],
            vec![
                Objective::int(x, ObjectiveDirection::Minimize),
                Objective::int(y, ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(
            result.objective_values,
            vec![ObjectiveValue::Int(3), ObjectiveValue::Int(1)]
        );
        assert!(result.solution.is_some());
    }
}
