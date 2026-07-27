use crate::config::{SearchConfig, SearchPhase};
use crate::dfs::DepthFirstSearch;
use crate::optimize::{ObjectiveDirection, ObjectiveValue, OptimizationTarget, is_better};
use crate::stats::SearchStats;
use crate::value::{AssignmentValue, Solution};
use propaga_core::VariableId;
use propaga_engine::Engine;
use propaga_propagators::{
    DominanceCutDirection, DominanceCutPropagator, DominanceCutTarget,
    ForbiddenAssignmentPropagator, ForbiddenValue, encode_forbidden_float,
};
use std::collections::{HashMap, HashSet};

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
    float_holes: HashMap<VariableId, Vec<f64>>,
    search_phases: Vec<SearchPhase>,
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
            float_holes: HashMap::new(),
            search_phases: Vec::new(),
        }
    }

    /// Attaches sequenced search phases to each Pareto DFS restart.
    #[must_use]
    pub fn with_search_phases(mut self, search_phases: impl Into<Vec<SearchPhase>>) -> Self {
        self.search_phases = search_phases.into();
        self
    }

    /// Enumerates the Pareto front incrementally with dominance-cut pruning.
    ///
    /// After each non-dominated solution, a typed dominance cut prunes the
    /// weakly-dominated orthant and the exact assignment is blocked so search
    /// cannot rediscover the same point.
    pub fn optimize(&mut self, engine: &mut Engine) -> ParetoResult {
        let directions: Vec<_> = self.objectives.iter().map(|(_, d)| *d).collect();
        let mut front: Vec<ParetoSolution> = Vec::new();
        let mut total_stats = SearchStats::default();
        let mut seen = HashSet::new();

        loop {
            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }

            let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config)
                .with_search_phases(self.search_phases.clone())
                .with_float_holes(self.float_holes.clone());
            let Some(solution) = dfs.solve(engine) else {
                merge_stats(&mut total_stats, dfs.stats());
                break;
            };
            merge_stats(&mut total_stats, dfs.stats());

            if !seen.insert(assignment_fingerprint(&solution)) {
                break;
            }

            let Some(objective_values) =
                objective_values_from_assignment(&self.objectives, &solution)
            else {
                break;
            };

            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }

            if is_dominated_by_front(&objective_values, &front, &directions) {
                let cut_ok = post_dominance_cut(engine, &self.objectives, &objective_values);
                let block_ok = block_assignment(
                    engine,
                    &mut self.variables,
                    &mut self.float_holes,
                    &solution,
                );
                if !cut_ok && !block_ok {
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

            let cut_ok = post_dominance_cut(engine, &self.objectives, &objective_values);
            let block_ok = block_assignment(
                engine,
                &mut self.variables,
                &mut self.float_holes,
                &solution,
            );
            if !cut_ok && !block_ok {
                break;
            }
        }

        ParetoResult {
            front,
            stats: total_stats,
        }
    }
}

fn assignment_fingerprint(solution: &Solution) -> String {
    let mut parts: Vec<String> = solution
        .iter()
        .map(|(var, value)| {
            let key = format!("{var:?}");
            match value {
                AssignmentValue::Int(value) => format!("{key}=i{value}"),
                AssignmentValue::Float(value) => format!("{key}=f{value}"),
                AssignmentValue::Set(values) => format!("{key}=s{values:?}"),
            }
        })
        .collect();
    parts.sort();
    parts.join("|")
}

fn post_dominance_cut(
    engine: &mut Engine,
    objectives: &[(OptimizationTarget, ObjectiveDirection)],
    values: &[ObjectiveValue],
) -> bool {
    let mut cuts = Vec::with_capacity(objectives.len());
    for ((target, direction), value) in objectives.iter().zip(values.iter()) {
        let cut_direction = match direction {
            ObjectiveDirection::Minimize => DominanceCutDirection::Minimize,
            ObjectiveDirection::Maximize => DominanceCutDirection::Maximize,
        };
        let cut = match (target, value) {
            (OptimizationTarget::Int(var), ObjectiveValue::Int(threshold)) => {
                DominanceCutTarget::Int {
                    var: *var,
                    direction: cut_direction,
                    value: *threshold,
                }
            }
            (OptimizationTarget::Float(var), ObjectiveValue::Float(threshold)) => {
                DominanceCutTarget::Float {
                    var: *var,
                    direction: cut_direction,
                    value: *threshold,
                }
            }
            (
                OptimizationTarget::SetCardinality(var),
                ObjectiveValue::SetCardinality(threshold),
            ) => DominanceCutTarget::SetCardinality {
                var: *var,
                direction: cut_direction,
                value: *threshold,
            },
            _ => return false,
        };
        cuts.push(cut);
    }
    if cuts.is_empty() {
        return false;
    }
    engine.add_propagator(Box::new(DominanceCutPropagator::new(cuts)));
    match engine.commit_initial_propagation() {
        Ok(status) => !status.is_failure(),
        Err(_) => false,
    }
}

fn block_assignment(
    engine: &mut Engine,
    decision_vars: &mut Vec<VariableId>,
    float_holes: &mut HashMap<VariableId, Vec<f64>>,
    solution: &Solution,
) -> bool {
    if solution.is_empty() {
        return false;
    }
    let mut forbidden = Vec::with_capacity(solution.len());
    for (var, value) in solution {
        match value {
            AssignmentValue::Int(value) => {
                forbidden.push((*var, ForbiddenValue::Int(*value)));
            }
            AssignmentValue::Float(value) => {
                let holes = float_holes.entry(*var).or_default();
                if !holes
                    .iter()
                    .any(|hole| (*hole - *value).abs() <= f64::EPSILON)
                {
                    holes.push(*value);
                }
                let encoded = encode_forbidden_float(engine, *var, *value);
                for reif in &encoded.decision_vars {
                    if !decision_vars.contains(reif) {
                        decision_vars.push(*reif);
                    }
                }
                forbidden.extend(encoded.forbidden);
            }
            AssignmentValue::Set(values) => {
                forbidden.push((*var, ForbiddenValue::Set(values.clone())));
            }
        }
    }
    engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(forbidden)));
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
            result.front.iter().any(|entry| {
                entry.objective_values
                    == vec![
                        ObjectiveValue::SetCardinality(1),
                        ObjectiveValue::SetCardinality(1),
                    ]
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
