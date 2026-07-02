//! Domain implementations for Propaga.
//!
//! Provides [`IntervalDomain`], [`BitsetDomain`], and [`HybridDomain`] for
//! representing integer variable domains with efficient intersection and pruning.

mod bitset;
mod float;
mod hybrid;
mod interval;
mod set;

pub use bitset::BitsetDomain;
pub use float::FloatDomain;
pub use hybrid::{BITSET_SPAN_THRESHOLD, HybridDomain};
pub use interval::IntervalDomain;
pub use set::SetDomain;
