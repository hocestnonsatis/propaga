//! Portfolio search over multiple search configurations.

use crate::config::{RestartPolicy, SearchConfig, SearchPhase, ValueOrdering, VariableOrdering};
use crate::dfs::DepthFirstSearch;
use crate::lexicographic::{LexicographicOptimization, LexicographicResult, Objective};
use crate::optimize::{
    ObjectiveDirection, ObjectiveValue, OptimizationResult, OptimizationSearch, OptimizationTarget,
    is_better,
};
use crate::pareto::{ParetoOptimization, ParetoResult, ParetoSolution, dominates};
use crate::stats::SearchStats;
use crate::value::Solution;
use propaga_core::VariableId;
use propaga_engine::{Engine, EngineCheckpoint};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Portfolio search configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortfolioConfig {
    /// Number of search configurations to try.
    pub workers: usize,
    /// When `true`, only the base configuration is used.
    pub deterministic: bool,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            workers: 1,
            deterministic: false,
        }
    }
}

/// Portfolio search that tries multiple configured DFS workers.
pub struct PortfolioSearch {
    variables: Vec<VariableId>,
    base_config: SearchConfig,
    portfolio: PortfolioConfig,
    search_phases: Vec<SearchPhase>,
}

impl PortfolioSearch {
    /// Creates a portfolio search over `variables`.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        base_config: SearchConfig,
        portfolio: PortfolioConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            base_config,
            portfolio,
            search_phases: Vec::new(),
        }
    }

    /// Attaches sequenced search phases shared by every portfolio worker.
    ///
    /// When phases are non-empty, each worker's DFS follows them (active-group
    /// orderings override diversified portfolio heuristics for those variables).
    #[must_use]
    pub fn with_search_phases(mut self, search_phases: impl Into<Vec<SearchPhase>>) -> Self {
        self.search_phases = search_phases.into();
        self
    }

    /// Searches for the first solution using the configured portfolio.
    pub fn solve(&self, engine: &mut Engine) -> (Option<Solution>, SearchStats) {
        if !self.propagate_root(engine) {
            return (None, SearchStats::default());
        }

        let checkpoint = engine.checkpoint();
        let configs = self.worker_configs();

        if configs.len() <= 1 {
            return self.solve_sequential(engine, &checkpoint, &configs);
        }

        self.solve_parallel(engine, &checkpoint, &configs)
    }

    /// Branch-and-bound with diversified workers; returns the best objective found.
    pub fn optimize(
        &self,
        engine: &mut Engine,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
    ) -> OptimizationResult {
        if !self.propagate_root(engine) {
            return OptimizationResult {
                solution: None,
                objective_value: None,
                stats: SearchStats::default(),
                solutions_found: 0,
                timed_out: false,
            };
        }

        let checkpoint = engine.checkpoint();
        let configs = self.worker_configs();
        if configs.len() <= 1 {
            engine.restore_checkpoint(&checkpoint);
            return OptimizationSearch::with_target(
                self.variables.clone(),
                target,
                direction,
                configs[0],
            )
            .with_search_phases(self.search_phases.clone())
            .optimize(engine);
        }

        let workers: Vec<_> = configs
            .iter()
            .map(|_| engine.fork_at_checkpoint(&checkpoint))
            .collect();

        let results: Vec<OptimizationResult> = workers
            .into_par_iter()
            .zip(configs.par_iter())
            .map(|(mut worker_engine, config)| {
                OptimizationSearch::with_target(self.variables.clone(), target, direction, *config)
                    .with_search_phases(self.search_phases.clone())
                    .optimize(&mut worker_engine)
            })
            .collect();

        merge_optimization_results(direction, results)
    }

    /// Lexicographic optimization across diversified workers; best lex vector wins.
    pub fn optimize_lexicographic(
        &self,
        engine: &mut Engine,
        objectives: Vec<Objective>,
    ) -> LexicographicResult {
        if !self.propagate_root(engine) {
            return LexicographicResult {
                solution: None,
                objective_values: Vec::new(),
                stats: SearchStats::default(),
            };
        }

        let checkpoint = engine.checkpoint();
        let configs = self.worker_configs();
        if configs.len() <= 1 {
            engine.restore_checkpoint(&checkpoint);
            return LexicographicOptimization::new(self.variables.clone(), objectives, configs[0])
                .with_search_phases(self.search_phases.clone())
                .optimize(engine);
        }

        let workers: Vec<_> = configs
            .iter()
            .map(|_| engine.fork_at_checkpoint(&checkpoint))
            .collect();
        let objectives_ref = &objectives;

        let results: Vec<LexicographicResult> = workers
            .into_par_iter()
            .zip(configs.par_iter())
            .map(|(mut worker_engine, config)| {
                LexicographicOptimization::new(
                    self.variables.clone(),
                    objectives_ref.clone(),
                    *config,
                )
                .with_search_phases(self.search_phases.clone())
                .optimize(&mut worker_engine)
            })
            .collect();

        merge_lexicographic_results(&objectives, results)
    }

    /// Pareto enumeration across diversified workers; fronts are merged with dominance filtering.
    pub fn optimize_pareto(
        &self,
        engine: &mut Engine,
        objectives: Vec<(OptimizationTarget, ObjectiveDirection)>,
    ) -> ParetoResult {
        if !self.propagate_root(engine) {
            return ParetoResult {
                front: Vec::new(),
                stats: SearchStats::default(),
            };
        }

        let checkpoint = engine.checkpoint();
        let configs = self.worker_configs();
        if configs.len() <= 1 {
            engine.restore_checkpoint(&checkpoint);
            return ParetoOptimization::new(self.variables.clone(), objectives, configs[0])
                .with_search_phases(self.search_phases.clone())
                .optimize(engine);
        }

        let workers: Vec<_> = configs
            .iter()
            .map(|_| engine.fork_at_checkpoint(&checkpoint))
            .collect();
        let objectives_ref = &objectives;

        let results: Vec<ParetoResult> = workers
            .into_par_iter()
            .zip(configs.par_iter())
            .map(|(mut worker_engine, config)| {
                ParetoOptimization::new(self.variables.clone(), objectives_ref.clone(), *config)
                    .with_search_phases(self.search_phases.clone())
                    .optimize(&mut worker_engine)
            })
            .collect();

        merge_pareto_results(&objectives, results)
    }

    fn worker_configs(&self) -> Vec<SearchConfig> {
        let worker_count = if self.portfolio.deterministic {
            1
        } else {
            self.portfolio.workers.max(1)
        };
        worker_configs(self.base_config, worker_count)
    }

    fn dfs_for_config(&self, config: SearchConfig) -> DepthFirstSearch {
        DepthFirstSearch::with_config(self.variables.clone(), config)
            .with_search_phases(self.search_phases.clone())
    }

    fn propagate_root(&self, engine: &mut Engine) -> bool {
        match engine.commit_initial_propagation() {
            Ok(status) => !status.is_failure(),
            Err(_) => false,
        }
    }

    fn solve_sequential(
        &self,
        engine: &mut Engine,
        checkpoint: &EngineCheckpoint,
        configs: &[SearchConfig],
    ) -> (Option<Solution>, SearchStats) {
        let mut total_stats = SearchStats::default();
        for config in configs {
            engine.restore_checkpoint(checkpoint);
            let mut search = self.dfs_for_config(*config);
            if let Some(solution) = search.solve_without_root_propagation(engine) {
                merge_stats(&mut total_stats, search.stats());
                return (Some(solution), total_stats);
            }
            merge_stats(&mut total_stats, search.stats());
            if search.stats().timed_out {
                break;
            }
        }
        (None, total_stats)
    }

    fn solve_parallel(
        &self,
        engine: &Engine,
        checkpoint: &EngineCheckpoint,
        configs: &[SearchConfig],
    ) -> (Option<Solution>, SearchStats) {
        let found = Arc::new(AtomicBool::new(false));
        let solution = Arc::new(Mutex::new(None::<Solution>));
        let stats = Arc::new(Mutex::new(SearchStats::default()));

        let workers: Vec<_> = configs
            .iter()
            .map(|_| engine.fork_at_checkpoint(checkpoint))
            .collect();

        workers
            .into_par_iter()
            .zip(configs.par_iter())
            .for_each(|(mut worker_engine, config)| {
                if found.load(Ordering::Relaxed) {
                    return;
                }

                let mut search = self.dfs_for_config(*config);
                if let Some(worker_solution) =
                    search.solve_without_root_propagation(&mut worker_engine)
                    && !found.swap(true, Ordering::Relaxed)
                    && let Ok(mut guard) = solution.lock()
                {
                    *guard = Some(worker_solution);
                }

                if let Ok(mut guard) = stats.lock() {
                    merge_stats(&mut guard, search.stats());
                }
            });

        (
            solution.lock().ok().and_then(|guard| guard.clone()),
            stats.lock().map(|guard| *guard).unwrap_or_default(),
        )
    }
}

fn merge_optimization_results(
    direction: ObjectiveDirection,
    results: Vec<OptimizationResult>,
) -> OptimizationResult {
    let mut best: Option<OptimizationResult> = None;
    let mut total_stats = SearchStats::default();
    let mut solutions_found = 0;
    let mut timed_out = false;

    for result in results {
        merge_stats(&mut total_stats, result.stats);
        solutions_found += result.solutions_found;
        timed_out |= result.timed_out;
        let take = match (&best, &result.objective_value) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(current), Some(candidate)) => match &current.objective_value {
                Some(best_value) => is_better(direction, candidate, best_value),
                None => true,
            },
        };
        if take {
            best = Some(result);
        }
    }

    match best {
        Some(mut winner) => {
            winner.stats = total_stats;
            winner.solutions_found = solutions_found;
            winner.timed_out = timed_out;
            winner
        }
        None => OptimizationResult {
            solution: None,
            objective_value: None,
            stats: total_stats,
            solutions_found,
            timed_out,
        },
    }
}

fn lex_better(
    objectives: &[Objective],
    candidate: &[ObjectiveValue],
    best: &[ObjectiveValue],
) -> bool {
    for (index, objective) in objectives.iter().enumerate() {
        let (Some(c), Some(b)) = (candidate.get(index), best.get(index)) else {
            return false;
        };
        if is_better(objective.direction, c, b) {
            return true;
        }
        if is_better(objective.direction, b, c) {
            return false;
        }
    }
    false
}

fn merge_lexicographic_results(
    objectives: &[Objective],
    results: Vec<LexicographicResult>,
) -> LexicographicResult {
    let mut best: Option<LexicographicResult> = None;
    let mut total_stats = SearchStats::default();

    for result in results {
        merge_stats(&mut total_stats, result.stats);
        let take = match &best {
            None => result.solution.is_some(),
            Some(current) => {
                result.solution.is_some()
                    && lex_better(
                        objectives,
                        &result.objective_values,
                        &current.objective_values,
                    )
            }
        };
        if take {
            best = Some(result);
        }
    }

    match best {
        Some(mut winner) => {
            winner.stats = total_stats;
            winner
        }
        None => LexicographicResult {
            solution: None,
            objective_values: Vec::new(),
            stats: total_stats,
        },
    }
}

fn merge_pareto_results(
    objectives: &[(OptimizationTarget, ObjectiveDirection)],
    results: Vec<ParetoResult>,
) -> ParetoResult {
    let directions: Vec<_> = objectives.iter().map(|(_, direction)| *direction).collect();
    let mut front: Vec<ParetoSolution> = Vec::new();
    let mut total_stats = SearchStats::default();

    for result in results {
        merge_stats(&mut total_stats, result.stats);
        for candidate in result.front {
            front.retain(|existing| {
                !dominates(
                    &candidate.objective_values,
                    &existing.objective_values,
                    &directions,
                )
            });
            let dominated = front.iter().any(|existing| {
                dominates(
                    &existing.objective_values,
                    &candidate.objective_values,
                    &directions,
                )
            });
            if !dominated {
                front.push(candidate);
            }
        }
    }

    ParetoResult {
        front,
        stats: total_stats,
    }
}

fn worker_configs(base: SearchConfig, workers: usize) -> Vec<SearchConfig> {
    let presets: &[(VariableOrdering, ValueOrdering, RestartPolicy)] = &[
        (
            VariableOrdering::Mrv,
            ValueOrdering::Ascending,
            RestartPolicy::Luby { base: 512 },
        ),
        (
            VariableOrdering::Dom,
            ValueOrdering::Lcv,
            RestartPolicy::Constant { scale: 256 },
        ),
        (
            VariableOrdering::Activity,
            ValueOrdering::Split,
            RestartPolicy::Geometric {
                base: 1.5,
                scale: 128,
            },
        ),
        (
            VariableOrdering::DomWdeg,
            ValueOrdering::Median,
            RestartPolicy::Linear { scale: 200 },
        ),
    ];

    (0..workers)
        .map(|index| {
            let (variable_ordering, value_ordering, restart_policy) =
                presets[index % presets.len()];
            SearchConfig {
                variable_ordering,
                value_ordering,
                restart_policy,
                ..base
            }
        })
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
    use propaga_propagators::{AllDifferentPropagator, LinearScalarLePropagator};

    fn all_different_engine() -> (Engine, Vec<VariableId>) {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 3)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));
        (engine, vars)
    }

    #[test]
    fn portfolio_finds_solution() {
        let (mut engine, vars) = all_different_engine();
        let search = PortfolioSearch::new(
            vars,
            SearchConfig::default(),
            PortfolioConfig {
                workers: 2,
                deterministic: false,
            },
        );
        let (solution, _) = search.solve(&mut engine);
        assert!(solution.is_some());
    }

    #[test]
    fn parallel_portfolio_finds_solution() {
        let (mut engine, vars) = all_different_engine();
        let search = PortfolioSearch::new(
            vars,
            SearchConfig::default(),
            PortfolioConfig {
                workers: 3,
                deterministic: false,
            },
        );
        let (solution, _) = search.solve(&mut engine);
        assert!(solution.is_some());
    }

    #[test]
    fn portfolio_respects_search_phases() {
        let (mut engine, vars) = all_different_engine();
        let search = PortfolioSearch::new(
            vars.clone(),
            SearchConfig::default(),
            PortfolioConfig {
                workers: 2,
                deterministic: true,
            },
        )
        .with_search_phases(vec![
            SearchPhase::new(
                vec![vars[0]],
                VariableOrdering::InputOrder,
                ValueOrdering::Ascending,
            ),
            SearchPhase::new(
                vars[1..].to_vec(),
                VariableOrdering::Mrv,
                ValueOrdering::Descending,
            ),
        ]);
        let (solution, _) = search.solve(&mut engine);
        assert!(solution.is_some());
    }

    #[test]
    fn portfolio_optimize_maximizes() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(0, 10));
        let y = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(LinearScalarLePropagator::new(
            vec![1, 1],
            vec![x, y],
            10,
        )));
        let search = PortfolioSearch::new(
            vec![x, y],
            SearchConfig::default(),
            PortfolioConfig {
                workers: 2,
                deterministic: false,
            },
        );
        let result = search.optimize(
            &mut engine,
            OptimizationTarget::Int(x),
            ObjectiveDirection::Maximize,
        );
        assert_eq!(result.objective_value, Some(ObjectiveValue::Int(10)));
    }
}
