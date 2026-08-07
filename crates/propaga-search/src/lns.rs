//! Large-neighborhood search for integer optimization.

use crate::config::{SearchConfig, SearchPhase};
use crate::optimize::{
    ObjectiveDirection, ObjectiveValue, OptimizationResult, OptimizationSearch, OptimizationTarget,
    is_better, objective_value_from_solution,
};
use crate::stats::SearchStats;
use crate::value::{AssignmentValue, Solution, assignment_int};
use propaga_core::{PropagationStatus, VariableId};
use propaga_engine::Engine;

/// Configuration for large-neighborhood search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LnsConfig {
    /// Number of destroy/repair iterations after the initial solution.
    pub iterations: u32,
    /// Fraction of decision variables to free each iteration (`0.0..=1.0`).
    pub destroy_fraction: f64,
    /// Deterministic seed for destroy selection.
    pub seed: u64,
}

impl Default for LnsConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            destroy_fraction: 0.3,
            seed: 1,
        }
    }
}

/// Large-neighborhood search over an integer (or typed) objective.
pub struct LargeNeighborhoodSearch {
    variables: Vec<VariableId>,
    target: OptimizationTarget,
    direction: ObjectiveDirection,
    config: SearchConfig,
    search_phases: Vec<SearchPhase>,
    lns: LnsConfig,
    hint: Option<Solution>,
}

impl LargeNeighborhoodSearch {
    /// Creates an LNS optimizer.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
        config: SearchConfig,
        lns: LnsConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            target,
            direction,
            config,
            search_phases: Vec::new(),
            lns,
            hint: None,
        }
    }

    /// Attaches sequenced search phases shared by every repair DFS.
    #[must_use]
    pub fn with_search_phases(mut self, search_phases: impl Into<Vec<SearchPhase>>) -> Self {
        self.search_phases = search_phases.into();
        self
    }

    /// Seeds LNS with a warm-start assignment (int components used).
    #[must_use]
    pub fn with_hint(mut self, hint: Solution) -> Self {
        self.hint = Some(hint);
        self
    }

    /// Runs LNS: initial solution (hint or BnB), then destroy/repair iterations.
    pub fn optimize(&mut self, engine: &mut Engine) -> OptimizationResult {
        let mut total_stats = SearchStats::default();
        let mut solutions_found = 0;
        let mut timed_out = false;

        let (mut best_solution, mut best_value) = self.initial_solution(
            engine,
            &mut total_stats,
            &mut solutions_found,
            &mut timed_out,
        );

        if best_solution.is_none() {
            return OptimizationResult {
                solution: None,
                objective_value: None,
                stats: total_stats,
                solutions_found,
                timed_out,
            };
        }

        let mut rng = self.lns.seed;
        for _ in 0..self.lns.iterations {
            if timed_out {
                break;
            }
            let Some(current) = best_solution.clone() else {
                break;
            };
            let Some(current_value) = best_value.clone() else {
                break;
            };

            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }
            let checkpoint = engine.checkpoint();
            let mut worker = engine.fork_at_checkpoint(&checkpoint);

            let freed = destroy_set(&self.variables, self.lns.destroy_fraction, &mut rng);
            if !fix_kept(&mut worker, &current, &freed) {
                continue;
            }
            if !post_bound(&mut worker, self.target, self.direction, &current_value) {
                continue;
            }

            let mut repair_config = self.config;
            repair_config.incomplete = true;
            // Prefer objective-improving values on the first incomplete repair hit.
            repair_config.value_ordering = match self.direction {
                ObjectiveDirection::Maximize => crate::config::ValueOrdering::Descending,
                ObjectiveDirection::Minimize => crate::config::ValueOrdering::Ascending,
            };
            let mut repair = OptimizationSearch::with_target(
                self.variables.clone(),
                self.target,
                self.direction,
                repair_config,
            )
            .with_search_phases(self.search_phases.clone());
            let repaired = repair.optimize(&mut worker);
            merge_stats(&mut total_stats, repaired.stats);
            solutions_found += repaired.solutions_found;
            timed_out |= repaired.timed_out;

            if let (Some(solution), Some(value)) = (repaired.solution, repaired.objective_value)
                && is_better(self.direction, &value, &current_value)
            {
                best_solution = Some(solution);
                best_value = Some(value);
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

    fn initial_solution(
        &self,
        engine: &mut Engine,
        total_stats: &mut SearchStats,
        solutions_found: &mut u32,
        timed_out: &mut bool,
    ) -> (Option<Solution>, Option<ObjectiveValue>) {
        if let Some(hint) = &self.hint
            && let Some((solution, value)) = try_accept_hint(engine, self.target, hint)
        {
            *solutions_found += 1;
            return (Some(solution), Some(value));
        }

        let mut search = OptimizationSearch::with_target(
            self.variables.clone(),
            self.target,
            self.direction,
            self.config,
        )
        .with_search_phases(self.search_phases.clone());
        let result = search.optimize(engine);
        merge_stats(total_stats, result.stats);
        *solutions_found += result.solutions_found;
        *timed_out |= result.timed_out;
        (result.solution, result.objective_value)
    }
}

/// Tries to apply an integer hint and returns it when feasible under the current domains.
#[must_use]
pub fn try_accept_hint(
    engine: &mut Engine,
    target: OptimizationTarget,
    hint: &Solution,
) -> Option<(Solution, ObjectiveValue)> {
    if engine.trail_depth() > 0 {
        engine.trail_backtrack(0);
    }
    let level = engine.trail_mark();
    if !assign_int_hint(engine, hint) {
        engine.trail_backtrack(level);
        return None;
    }
    let solution = hint.clone();
    let value = objective_value_from_solution(engine, target, &solution)?;
    engine.trail_backtrack(level);
    Some((solution, value))
}

/// Applies integer components of `hint` as root decisions; returns false on conflict.
pub fn assign_int_hint(engine: &mut Engine, hint: &Solution) -> bool {
    for (var, value) in hint {
        let AssignmentValue::Int(int_value) = value else {
            continue;
        };
        match engine.fix_variable(*var, *int_value) {
            Ok(PropagationStatus::Failure) | Err(_) => return false,
            Ok(_) => {}
        }
    }
    true
}

fn fix_kept(engine: &mut Engine, best: &Solution, freed: &[VariableId]) -> bool {
    for (var, value) in best {
        if freed.contains(var) {
            continue;
        }
        let AssignmentValue::Int(int_value) = value else {
            continue;
        };
        match engine.fix_variable(*var, *int_value) {
            Ok(PropagationStatus::Failure) | Err(_) => return false,
            Ok(_) => {}
        }
    }
    true
}

fn post_bound(
    engine: &mut Engine,
    target: OptimizationTarget,
    direction: ObjectiveDirection,
    best: &ObjectiveValue,
) -> bool {
    crate::optimize::post_objective_pruning_bound(engine, target, direction, best)
}

/// Deterministic destroy set: free about `destroy_fraction` of `variables`.
fn destroy_set(variables: &[VariableId], destroy_fraction: f64, rng: &mut u64) -> Vec<VariableId> {
    if variables.is_empty() {
        return Vec::new();
    }
    let fraction = destroy_fraction.clamp(0.0, 1.0);
    let mut count = ((variables.len() as f64) * fraction).round() as usize;
    count = count.clamp(1, variables.len());
    let mut indices: Vec<usize> = (0..variables.len()).collect();
    // Fisher–Yates with LCG
    for i in (1..indices.len()).rev() {
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (*rng as usize) % (i + 1);
        indices.swap(i, j);
    }
    indices
        .into_iter()
        .take(count)
        .map(|index| variables[index])
        .collect()
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
    use propaga_propagators::LinearScalarLePropagator;

    #[test]
    fn lns_improves_or_matches_hint_maximize() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
        )));

        let hint = vec![(x, AssignmentValue::Int(3)), (y, AssignmentValue::Int(0))];
        let mut search = LargeNeighborhoodSearch::new(
            vec![x, y],
            OptimizationTarget::Int(x),
            ObjectiveDirection::Maximize,
            SearchConfig {
                learning: false,
                restart_policy: crate::config::RestartPolicy::None,
                ..SearchConfig::default()
            },
            LnsConfig {
                iterations: 8,
                destroy_fraction: 0.5,
                seed: 7,
            },
        )
        .with_hint(hint);

        let result = search.optimize(&mut engine);
        assert_eq!(result.objective_value, Some(ObjectiveValue::Int(10)));
        assert_eq!(
            assignment_int(result.solution.as_ref().unwrap(), x),
            Some(10)
        );
    }

    #[test]
    fn try_accept_hint_rejects_infeasible() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 5));
        let hint = vec![(x, AssignmentValue::Int(9))];
        assert!(try_accept_hint(&mut engine, OptimizationTarget::Int(x), &hint).is_none());
    }
}
