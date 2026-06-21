//! Propagation engine for Propaga.

mod context;
mod engine;
mod event_queue;
mod trail;

pub use engine::{ConflictInfo, Engine};
