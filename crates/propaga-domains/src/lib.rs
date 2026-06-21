//! Domain implementations for Propaga.
//!
//! Provides [`IntervalDomain`], [`BitsetDomain`], and [`HybridDomain`] for
//! representing integer variable domains with efficient intersection and pruning.

mod bitset;
mod hybrid;
mod interval;

pub use bitset::BitsetDomain;
pub use hybrid::{BITSET_SPAN_THRESHOLD, HybridDomain};
pub use interval::IntervalDomain;
