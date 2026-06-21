//! Propagation engine for Propaga.
//!
//! The [`Engine`] maintains variables, propagators, a trail for backtracking,
//! and an event queue that schedules propagators until a fixpoint or conflict.

mod context;
mod engine;
mod event_queue;
mod trail;

pub use engine::{ConflictInfo, Engine};
