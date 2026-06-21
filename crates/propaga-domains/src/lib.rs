//! Domain implementations for Propaga.

mod bitset;
mod hybrid;
mod interval;

pub use bitset::BitsetDomain;
pub use hybrid::{HybridDomain, BITSET_SPAN_THRESHOLD};
pub use interval::IntervalDomain;
