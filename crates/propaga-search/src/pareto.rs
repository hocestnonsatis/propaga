use crate::config::SearchConfig;
use crate::dfs::{DepthFirstSearch, Solution};
use crate::optimize::ObjectiveDirection;
use crate::stats::SearchStats;
use propaga_core::VariableId;
use propaga_engine::Engine;

/// Returns `true` when `a` dominates `b` under the given directions.
pub fn dominates(a: &[i32], b: &[i32], directions: &[ObjectiveDirection]) -> bool {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), directions.len());
    let mut strictly_better = false;
    for ((&av, &bv), direction) in a.iter().zip(b.iter()).zip(directions.iter()) {
        let better = match direction {
            ObjectiveDirection::Minimize => av <= bv,
            ObjectiveDirection::Maximize => av >= bv,
        };
        if !better {
            return false;
        }
        let strictly = match direction {
            ObjectiveDirection::Minimize => av < bv,
            ObjectiveDirection::Maximize => av > bv,
        };
        strictly_better |= strictly;
    }
    strictly_better
}

/// One non-dominated solution with objective vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParetoSolution {
    /// Variable assignment.
    pub assignment: Solution,
    /// Objective values in search order.
    pub objective_values: Vec<i32>,
}

/// Result of Pareto front enumeration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParetoResult {
    /// Non-dominated solutions discovered.
    pub front: Vec<ParetoSolution>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
}

/// Enumerates non-dominated solutions for multiple objectives.
pub struct ParetoOptimization {
    variables: Vec<VariableId>,
    objectives: Vec<(VariableId, ObjectiveDirection)>,
    config: SearchConfig,
}

impl ParetoOptimization {
    /// Creates a Pareto optimizer over `variables` and `objectives`.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(VariableId, ObjectiveDirection)>,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            objectives,
            config,
        }
    }

    /// Enumerates the Pareto front by collecting feasible solutions.
    pub fn optimize(&mut self, engine: &mut Engine) -> ParetoResult {
        let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config);
        let all_solutions = dfs.solve_all(engine);
        let total_stats = dfs.stats();
        let directions: Vec<_> = self.objectives.iter().map(|(_, d)| *d).collect();

        let mut front: Vec<ParetoSolution> = Vec::new();
        for solution in all_solutions {
            let objective_values = objective_values(engine, &self.objectives, &solution);
            if is_dominated_by_front(&objective_values, &front, &directions) {
                continue;
            }
            front.retain(|entry| {
                !dominates(&objective_values, &entry.objective_values, &directions)
            });
            front.push(ParetoSolution {
                assignment: solution,
                objective_values,
            });
        }

        ParetoResult {
            front,
            stats: total_stats,
        }
    }
}

fn objective_values(
    engine: &Engine,
    objectives: &[(VariableId, ObjectiveDirection)],
    solution: &Solution,
) -> Vec<i32> {
    let map: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    objectives
        .iter()
        .map(|(var, _)| {
            map.get(var)
                .copied()
                .or_else(|| {
                    engine
                        .int_domain(*var)
                        .and_then(|domain| domain.fixed_value())
                })
                .unwrap_or(0)
        })
        .collect()
}

fn is_dominated_by_front(
    values: &[i32],
    front: &[ParetoSolution],
    directions: &[ObjectiveDirection],
) -> bool {
    front
        .iter()
        .any(|entry| dominates(&entry.objective_values, values, directions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::IntervalDomain;

    #[test]
    fn dominates_minimize_pair() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Minimize];
        assert!(dominates(&[1, 2], &[2, 3], &dirs));
        assert!(!dominates(&[2, 1], &[1, 2], &dirs));
    }

    #[test]
    fn mixed_directions() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Maximize];
        assert!(dominates(&[1, 5], &[2, 4], &dirs));
    }

    #[test]
    fn finds_pareto_front_for_two_objectives() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let sum = engine.new_variable(IntervalDomain::fix(4));
        engine.add_propagator(Box::new(propaga_propagators::LinearEqPropagator::new(
            x, y, sum,
        )));
        let _ = engine.propagate_all();
        let mut dfs = DepthFirstSearch::with_config(vec![x, y], SearchConfig::without_learning());
        let all = dfs.solve_all(&mut engine);
        assert_eq!(all.len(), 3, "expected three feasible assignments");
        let mut search = ParetoOptimization::new(
            vec![x, y],
            vec![
                (x, ObjectiveDirection::Minimize),
                (y, ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert!(result.front.len() >= 2);
        assert!(
            result
                .front
                .iter()
                .any(|entry| entry.objective_values == vec![1, 3])
        );
        assert!(
            result
                .front
                .iter()
                .any(|entry| entry.objective_values == vec![3, 1])
        );
    }
}
