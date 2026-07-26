use crate::config::SearchConfig;
use crate::dfs::DepthFirstSearch;
use crate::optimize::{ObjectiveDirection, ObjectiveValue, OptimizationTarget, is_better};
use crate::stats::SearchStats;
use crate::value::{AssignmentValue, Solution};
use propaga_core::{NogoodLiteral, VariableId};
use propaga_engine::Engine;
use propaga_propagators::{DominanceCutDirection, IntDominanceCutPropagator, NogoodPropagator};

/// Returns `true` when `a` dominates `b` under the given directions.
#[must_use]
pub fn dominates(
    a: &[ObjectiveValue],
    b: &[ObjectiveValue],
    directions: &[ObjectiveDirection],
) -> bool {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), directions.len());
    let mut strictly_better = false;
    for ((av, bv), direction) in a.iter().zip(b.iter()).zip(directions.iter()) {
        if !weakly_better(*direction, av, bv) {
            return false;
        }
        strictly_better |= is_better(*direction, av, bv);
    }
    strictly_better
}

fn weakly_better(direction: ObjectiveDirection, a: &ObjectiveValue, b: &ObjectiveValue) -> bool {
    match (direction, a, b) {
        (ObjectiveDirection::Minimize, ObjectiveValue::Int(av), ObjectiveValue::Int(bv)) => {
            av <= bv
        }
        (ObjectiveDirection::Maximize, ObjectiveValue::Int(av), ObjectiveValue::Int(bv)) => {
            av >= bv
        }
        (ObjectiveDirection::Minimize, ObjectiveValue::Float(av), ObjectiveValue::Float(bv)) => {
            av <= bv
        }
        (ObjectiveDirection::Maximize, ObjectiveValue::Float(av), ObjectiveValue::Float(bv)) => {
            av >= bv
        }
        (
            ObjectiveDirection::Minimize,
            ObjectiveValue::SetCardinality(av),
            ObjectiveValue::SetCardinality(bv),
        ) => av <= bv,
        (
            ObjectiveDirection::Maximize,
            ObjectiveValue::SetCardinality(av),
            ObjectiveValue::SetCardinality(bv),
        ) => av >= bv,
        _ => false,
    }
}

/// One non-dominated solution with objective vector.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoSolution {
    /// Variable assignment.
    pub assignment: Solution,
    /// Objective values in search order.
    pub objective_values: Vec<ObjectiveValue>,
}

/// Result of Pareto front enumeration.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoResult {
    /// Non-dominated solutions discovered.
    pub front: Vec<ParetoSolution>,
    /// Aggregated search statistics.
    pub stats: SearchStats,
}

/// Enumerates non-dominated solutions for multiple objectives.
pub struct ParetoOptimization {
    variables: Vec<VariableId>,
    objectives: Vec<(OptimizationTarget, ObjectiveDirection)>,
    config: SearchConfig,
}

impl ParetoOptimization {
    /// Creates a Pareto optimizer over `variables` and `objectives`.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(OptimizationTarget, ObjectiveDirection)>,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            objectives,
            config,
        }
    }

    /// Enumerates the Pareto front incrementally.
    ///
    /// All-integer objectives use repeated search with dominance cuts that prune
    /// weakly-dominated orthants. Other objective mixes stream solutions via DFS
    /// and maintain the front online without buffering the full feasible set.
    pub fn optimize(&mut self, engine: &mut Engine) -> ParetoResult {
        if self
            .objectives
            .iter()
            .all(|(target, _)| matches!(target, OptimizationTarget::Int(_)))
        {
            self.optimize_with_dominance_cuts(engine)
        } else {
            self.optimize_streaming(engine)
        }
    }

    fn optimize_streaming(&mut self, engine: &mut Engine) -> ParetoResult {
        let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config);
        let directions: Vec<_> = self.objectives.iter().map(|(_, d)| *d).collect();
        let mut front: Vec<ParetoSolution> = Vec::new();
        let objectives = self.objectives.clone();

        dfs.solve_each(engine, |solution| {
            let Some(objective_values) = objective_values_from_assignment(&objectives, solution)
            else {
                return true;
            };
            if is_dominated_by_front(&objective_values, &front, &directions) {
                return true;
            }
            front.retain(|entry| {
                !dominates(&objective_values, &entry.objective_values, &directions)
            });
            front.push(ParetoSolution {
                assignment: solution.clone(),
                objective_values,
            });
            true
        });

        ParetoResult {
            front,
            stats: dfs.stats(),
        }
    }

    fn optimize_with_dominance_cuts(&mut self, engine: &mut Engine) -> ParetoResult {
        let directions: Vec<_> = self.objectives.iter().map(|(_, d)| *d).collect();
        let mut front: Vec<ParetoSolution> = Vec::new();
        let mut total_stats = SearchStats::default();

        loop {
            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }

            let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config);
            let Some(solution) = dfs.solve(engine) else {
                merge_stats(&mut total_stats, dfs.stats());
                break;
            };
            merge_stats(&mut total_stats, dfs.stats());

            let Some(objective_values) =
                objective_values_from_assignment(&self.objectives, &solution)
            else {
                break;
            };

            if is_dominated_by_front(&objective_values, &front, &directions) {
                if engine.trail_depth() > 0 {
                    engine.trail_backtrack(0);
                }
                if !block_int_assignment(engine, &solution) {
                    break;
                }
                continue;
            }

            front.retain(|entry| {
                !dominates(&objective_values, &entry.objective_values, &directions)
            });
            front.push(ParetoSolution {
                assignment: solution.clone(),
                objective_values: objective_values.clone(),
            });

            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }
            if !post_int_dominance_cut(engine, &self.objectives, &objective_values)
                && !block_int_assignment(engine, &solution)
            {
                break;
            }
        }

        ParetoResult {
            front,
            stats: total_stats,
        }
    }
}

fn post_int_dominance_cut(
    engine: &mut Engine,
    objectives: &[(OptimizationTarget, ObjectiveDirection)],
    values: &[ObjectiveValue],
) -> bool {
    let mut cuts = Vec::with_capacity(objectives.len());
    for ((target, direction), value) in objectives.iter().zip(values.iter()) {
        let (OptimizationTarget::Int(var), ObjectiveValue::Int(threshold)) = (target, value) else {
            return false;
        };
        let cut_direction = match direction {
            ObjectiveDirection::Minimize => DominanceCutDirection::Minimize,
            ObjectiveDirection::Maximize => DominanceCutDirection::Maximize,
        };
        cuts.push((*var, cut_direction, *threshold));
    }
    if cuts.is_empty() {
        return false;
    }
    engine.add_propagator(Box::new(IntDominanceCutPropagator::new(cuts)));
    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
    }
}

fn block_int_assignment(engine: &mut Engine, solution: &Solution) -> bool {
    let literals: Vec<_> = solution
        .iter()
        .filter_map(|(var, value)| match value {
            AssignmentValue::Int(value) => Some(NogoodLiteral {
                variable: *var,
                value: *value,
            }),
            _ => None,
        })
        .collect();
    if literals.is_empty() {
        return false;
    }
    engine.add_propagator(Box::new(NogoodPropagator::new(literals)));
    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
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

fn objective_values_from_assignment(
    objectives: &[(OptimizationTarget, ObjectiveDirection)],
    solution: &Solution,
) -> Option<Vec<ObjectiveValue>> {
    objectives
        .iter()
        .map(|(target, _)| match target {
            OptimizationTarget::Int(var) => solution
                .iter()
                .find(|(candidate, _)| candidate == var)
                .and_then(|(_, value)| match value {
                    AssignmentValue::Int(value) => Some(ObjectiveValue::Int(*value)),
                    _ => None,
                }),
            OptimizationTarget::Float(var) => solution
                .iter()
                .find(|(candidate, _)| candidate == var)
                .and_then(|(_, value)| match value {
                    AssignmentValue::Float(value) => Some(ObjectiveValue::Float(*value)),
                    _ => None,
                }),
            OptimizationTarget::SetCardinality(var) => solution
                .iter()
                .find(|(candidate, _)| candidate == var)
                .and_then(|(_, value)| match value {
                    AssignmentValue::Set(values) => {
                        Some(ObjectiveValue::SetCardinality(values.len()))
                    }
                    _ => None,
                }),
        })
        .collect()
}

fn is_dominated_by_front(
    values: &[ObjectiveValue],
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
    use propaga_domains::{AnyDomain, IntervalDomain};

    #[test]
    fn dominates_minimize_pair() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Minimize];
        assert!(dominates(
            &[ObjectiveValue::Int(1), ObjectiveValue::Int(2)],
            &[ObjectiveValue::Int(2), ObjectiveValue::Int(3)],
            &dirs
        ));
        assert!(!dominates(
            &[ObjectiveValue::Int(2), ObjectiveValue::Int(1)],
            &[ObjectiveValue::Int(1), ObjectiveValue::Int(2)],
            &dirs
        ));
    }

    #[test]
    fn mixed_directions() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Maximize];
        assert!(dominates(
            &[ObjectiveValue::Int(1), ObjectiveValue::Int(5)],
            &[ObjectiveValue::Int(2), ObjectiveValue::Int(4)],
            &dirs
        ));
    }

    #[test]
    fn dominates_float_pair() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Minimize];
        assert!(dominates(
            &[ObjectiveValue::Float(0.5), ObjectiveValue::Float(1.0)],
            &[ObjectiveValue::Float(1.0), ObjectiveValue::Float(2.0)],
            &dirs
        ));
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
        let mut search = ParetoOptimization::new(
            vec![x, y],
            vec![
                (OptimizationTarget::Int(x), ObjectiveDirection::Minimize),
                (OptimizationTarget::Int(y), ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert!(result.front.len() >= 2);
        assert!(result.front.iter().any(|entry| {
            entry.objective_values == vec![ObjectiveValue::Int(1), ObjectiveValue::Int(3)]
        }));
        assert!(result.front.iter().any(|entry| {
            entry.objective_values == vec![ObjectiveValue::Int(3), ObjectiveValue::Int(1)]
        }));
    }

    #[test]
    fn dominance_cuts_find_full_int_front() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 4));
        let y = engine.new_variable(IntervalDomain::new(1, 4));
        let sum = engine.new_variable(IntervalDomain::fix(5));
        engine.add_propagator(Box::new(propaga_propagators::LinearEqPropagator::new(
            x, y, sum,
        )));
        let _ = engine.commit_initial_propagation();
        let mut search = ParetoOptimization::new(
            vec![x, y],
            vec![
                (OptimizationTarget::Int(x), ObjectiveDirection::Minimize),
                (OptimizationTarget::Int(y), ObjectiveDirection::Minimize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        // All points on x+y=5 are mutually non-dominated for bi-minimize.
        assert_eq!(result.front.len(), 4);
    }

    #[test]
    fn finds_pareto_front_for_set_cardinality_objectives() {
        let mut engine = Engine::new();
        let s1 = engine.new_variable(AnyDomain::Set(
            propaga_domains::SetIntervalDomain::universe(1..=2).with_cardinality(1, 2),
        ));
        let s2 = engine.new_variable(AnyDomain::Set(
            propaga_domains::SetIntervalDomain::universe(1..=2).with_cardinality(1, 2),
        ));
        engine.add_propagator(Box::new(propaga_propagators::SetCardPropagator::new(s1)));
        engine.add_propagator(Box::new(propaga_propagators::SetCardPropagator::new(s2)));
        let _ = engine.commit_initial_propagation();

        let mut search = ParetoOptimization::new(
            vec![s1, s2],
            vec![
                (
                    OptimizationTarget::SetCardinality(s1),
                    ObjectiveDirection::Minimize,
                ),
                (
                    OptimizationTarget::SetCardinality(s2),
                    ObjectiveDirection::Minimize,
                ),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert!(
            !result.front.is_empty(),
            "expected a non-empty set-cardinality Pareto front"
        );
        assert!(
            result
                .front
                .iter()
                .all(|entry| entry.objective_values.len() == 2)
        );
        assert!(
            result.front.iter().all(|entry| {
                entry
                    .objective_values
                    .iter()
                    .all(|value| matches!(value, ObjectiveValue::SetCardinality(_)))
            }),
            "front={:?}",
            result
                .front
                .iter()
                .map(|e| &e.objective_values)
                .collect::<Vec<_>>()
        );
    }
}
