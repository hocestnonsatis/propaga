//! Search strategies for Propaga.
//!
//! [`DepthFirstSearch`] provides MRV-based DFS with nogood learning, restarts,
//! and phase saving. [`OptimizationSearch`] adds branch-and-bound for a single
//! integer objective. Configure behavior via [`SearchConfig`].

mod config;
mod conflict;
mod dfs;
mod optimize;
mod stats;

pub use config::{RestartPolicy, SearchConfig, ValueOrdering, VariableOrdering, luby_sequence};
pub use conflict::{ConflictAnalyzer, NogoodStore};
pub use dfs::{DepthFirstSearch, Solution};
pub use optimize::{ObjectiveDirection, OptimizationResult, OptimizationSearch};
pub use stats::SearchStats;
