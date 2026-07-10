//! Portfolio search over multiple search configurations.

use crate::config::{RestartPolicy, SearchConfig, ValueOrdering, VariableOrdering};
use crate::dfs::DepthFirstSearch;
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
        }
    }

    /// Searches for the first solution using the configured portfolio.
    pub fn solve(&self, engine: &mut Engine) -> (Option<Solution>, SearchStats) {
        if !self.propagate_root(engine) {
            return (None, SearchStats::default());
        }

        let checkpoint = engine.checkpoint();
        let worker_count = if self.portfolio.deterministic {
            1
        } else {
            self.portfolio.workers.max(1)
        };
        let configs = worker_configs(self.base_config, worker_count);

        if worker_count <= 1 {
            return self.solve_sequential(engine, &checkpoint, &configs);
        }

        self.solve_parallel(engine, &checkpoint, &configs)
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
            let mut search = DepthFirstSearch::with_config(self.variables.clone(), *config);
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

                let mut search = DepthFirstSearch::with_config(self.variables.clone(), *config);
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
    use propaga_propagators::AllDifferentPropagator;

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
}
