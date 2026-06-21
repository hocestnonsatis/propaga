//! Domain implementations for Propaga.

mod bitset;
mod hybrid;
mod interval;

pub use bitset::BitsetDomain;
pub use hybrid::{BITSET_SPAN_THRESHOLD, HybridDomain};
pub use interval::IntervalDomain;
