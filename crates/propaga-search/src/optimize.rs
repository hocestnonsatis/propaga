use crate::config::SearchConfig;
use crate::dfs::{DepthFirstSearch, Solution};
use crate::stats::SearchStats;
use propaga_core::VariableId;
use propaga_engine::Engine;
use propaga_propagators::LessEqualPropagator;

/// Optimization direction for branch-and-bound search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveDirection {
    /// Minimize the objective variable.
    Minimize,
    /// Maximize the objective variable.
    Maximize,
}

/// Result of an optimization search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptimizationResult {
    /// Best solution found.
    pub solution: Option<Solution>,
    /// Best objective value.
    pub objective_value: Option<i32>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
    /// Number of feasible solutions encountered.
    pub solutions_found: u32,
}

/// Branch-and-bound optimization over a single integer objective.
pub struct OptimizationSearch {
    variables: Vec<VariableId>,
    objective: VariableId,
    direction: ObjectiveDirection,
    config: SearchConfig,
}

impl OptimizationSearch {
    /// Creates an optimization search over `variables` and `objective`.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objective: VariableId,
        direction: ObjectiveDirection,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            objective,
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

        loop {
            let solution = dfs.solve(engine);
            merge_stats(&mut total_stats, dfs.stats());

            let Some(solution) = solution else {
                break;
            };

            solutions_found += 1;
            let objective_value = objective_value_from_solution(engine, self.objective, &solution);
            let Some(value) = objective_value else {
                break;
            };

            let is_improvement = match best_value {
                None => true,
                Some(best) => match self.direction {
                    ObjectiveDirection::Minimize => value < best,
                    ObjectiveDirection::Maximize => value > best,
                },
            };

            if is_improvement {
                best_value = Some(value);
                best_solution = Some(solution);
            }

            if !self.post_pruning_bound(engine, best_value.unwrap()) {
                break;
            }
        }

        OptimizationResult {
            solution: best_solution,
            objective_value: best_value,
            stats: total_stats,
            solutions_found,
        }
    }

    fn post_pruning_bound(&mut self, engine: &mut Engine, best: i32) -> bool {
        let bound = match self.direction {
            ObjectiveDirection::Minimize => best.saturating_sub(1),
            ObjectiveDirection::Maximize => best.saturating_add(1),
        };

        let bound_var = engine.new_variable(propaga_domains::HybridDomain::fix(bound));
        match self.direction {
            ObjectiveDirection::Minimize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(
                    self.objective,
                    bound_var,
                )));
            }
            ObjectiveDirection::Maximize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(
                    bound_var,
                    self.objective,
                )));
            }
        }

        match engine.commit_initial_propagation() {
            Ok(status) => !status.is_failure(),
            Err(_) => false,
        }
    }
}

fn objective_value_from_solution(
    engine: &Engine,
    objective: VariableId,
    solution: &Solution,
) -> Option<i32> {
    solution
        .iter()
        .find(|(var, _)| *var == objective)
        .map(|(_, value)| *value)
        .or_else(|| engine.domain(objective).fixed_value())
}

fn merge_stats(total: &mut SearchStats, partial: SearchStats) {
    total.nodes += partial.nodes;
    total.backtracks += partial.backtracks;
    total.conflicts += partial.conflicts;
    total.nogoods_learned += partial.nogoods_learned;
    total.restarts += partial.restarts;
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
        assert!(result.objective_value.unwrap() >= 5);
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
        assert_eq!(result.objective_value, Some(5));
    }
}
