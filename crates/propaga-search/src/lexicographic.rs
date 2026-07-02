//! Lexicographic multi-objective optimization.

use crate::config::SearchConfig;
use crate::dfs::Solution;
use crate::optimize::{ObjectiveDirection, OptimizationSearch};
use crate::stats::SearchStats;
use propaga_core::VariableId;
use propaga_domains::HybridDomain;
use propaga_engine::Engine;
use propaga_propagators::LessEqualPropagator;

/// One objective in a lexicographic optimization problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Objective {
    /// Objective variable.
    pub var: VariableId,
    /// Optimization direction.
    pub direction: ObjectiveDirection,
}

/// Result of lexicographic optimization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexicographicResult {
    /// Best solution found.
    pub solution: Option<Solution>,
    /// Objective values in priority order.
    pub objective_values: Vec<i32>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
}

/// Lexicographic branch-and-bound over multiple objectives.
pub struct LexicographicOptimization {
    variables: Vec<VariableId>,
    objectives: Vec<Objective>,
    config: SearchConfig,
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
        }
    }

    /// Optimizes objectives in order, fixing each optimal value before the next.
    pub fn optimize(&mut self, engine: &mut Engine) -> LexicographicResult {
        let mut total_stats = SearchStats::default();
        let mut objective_values = Vec::new();
        let mut best_solution = None;

        for (index, objective) in self.objectives.clone().into_iter().enumerate() {
            let mut search = OptimizationSearch::new(
                self.variables.clone(),
                objective.var,
                objective.direction,
                self.config,
            );
            let result = search.optimize(engine);
            merge_stats(&mut total_stats, result.stats);
            let Some(value) = result.objective_value else {
                return LexicographicResult {
                    solution: None,
                    objective_values,
                    stats: total_stats,
                };
            };
            objective_values.push(value);
            best_solution = result.solution.or(best_solution);

            if index + 1 < self.objectives.len() {
                tighten_objective(engine, objective, value);
            }
        }

        LexicographicResult {
            solution: best_solution,
            objective_values,
            stats: total_stats,
        }
    }
}

fn tighten_objective(engine: &mut Engine, objective: Objective, value: i32) {
    let bound = engine.new_variable(HybridDomain::fix(value));
    match objective.direction {
        ObjectiveDirection::Minimize => {
            engine.add_propagator(Box::new(LessEqualPropagator::new(objective.var, bound)));
        }
        ObjectiveDirection::Maximize => {
            engine.add_propagator(Box::new(LessEqualPropagator::new(bound, objective.var)));
        }
    }
    let _ = engine.propagate_all();
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

    #[test]
    fn lexicographic_minimizes_in_order() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let mut search = LexicographicOptimization::new(
            vec![x, y],
            vec![
                Objective {
                    var: x,
                    direction: ObjectiveDirection::Minimize,
                },
                Objective {
                    var: y,
                    direction: ObjectiveDirection::Minimize,
                },
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert_eq!(result.objective_values, vec![1, 1]);
        assert!(result.solution.is_some());
    }
}
