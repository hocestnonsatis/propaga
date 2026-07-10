//! Search strategies for Propaga.
//!
//! [`DepthFirstSearch`] provides MRV-based DFS with nogood learning, restarts,
//! and phase saving. [`OptimizationSearch`] adds branch-and-bound for a single
//! integer objective. Configure behavior via [`SearchConfig`].

mod config;
mod conflict;
mod dfs;
mod lcg;
mod lexicographic;
mod optimize;
mod pareto;
mod portfolio;
mod stats;
mod value;

pub use config::{RestartPolicy, SearchConfig, ValueOrdering, VariableOrdering, luby_sequence};
pub use conflict::{ConflictAnalyzer, NogoodStore};
pub use dfs::DepthFirstSearch;
pub use lcg::{ClauseStore, LearnedClause};
pub use lexicographic::{LexicographicOptimization, LexicographicResult, Objective};
pub use optimize::{ObjectiveDirection, OptimizationResult, OptimizationSearch};
pub use pareto::{ParetoOptimization, ParetoResult, ParetoSolution, dominates};
pub use portfolio::{PortfolioConfig, PortfolioSearch};
pub use stats::SearchStats;
pub use value::{AssignmentValue, Solution, assignment_int, solution_int_map, solution_int_values};
