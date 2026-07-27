//! Search strategies for Propaga.
//!
//! [`DepthFirstSearch`] provides MRV-based DFS with nogood learning, restarts,
//! and phase saving. [`OptimizationSearch`] adds branch-and-bound for typed
//! objectives (int, float, set cardinality). Lexicographic and Pareto search
//! reuse the same typed targets. Configure behavior via [`SearchConfig`].

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

pub use config::{
    RestartPolicy, SearchConfig, SearchPhase, ValueOrdering, VariableOrdering, luby_sequence,
};
pub use conflict::{ConflictAnalyzer, NogoodStore};
pub use dfs::DepthFirstSearch;
pub use lcg::{ClauseStore, LearnedClause};
pub use lexicographic::{LexicographicOptimization, LexicographicResult, Objective};
pub use optimize::{
    ObjectiveDirection, ObjectiveValue, OptimizationResult, OptimizationSearch, OptimizationTarget,
    is_better, objective_value_from_solution,
};
pub use pareto::{ParetoOptimization, ParetoResult, ParetoSolution, dominates};
pub use portfolio::{PortfolioConfig, PortfolioSearch};
pub use stats::SearchStats;
pub use value::{AssignmentValue, Solution, assignment_int, solution_int_map, solution_int_values};
