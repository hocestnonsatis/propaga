//! Portfolio search over multiple search configurations.

use crate::config::{RestartPolicy, SearchConfig, ValueOrdering, VariableOrdering};
use crate::dfs::{DepthFirstSearch, Solution};
use crate::stats::SearchStats;
use propaga_core::VariableId;
use propaga_engine::Engine;

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

/// Portfolio search that tries multiple configured DFS workers in sequence.
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
        let worker_count = if self.portfolio.deterministic {
            1
        } else {
            self.portfolio.workers.max(1)
        };

        let mut total_stats = SearchStats::default();
        for config in worker_configs(self.base_config, worker_count) {
            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }
            let mut search = DepthFirstSearch::with_config(self.variables.clone(), config);
            if let Some(solution) = search.solve(engine) {
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

    #[test]
    fn portfolio_finds_solution() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 3)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

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
}
