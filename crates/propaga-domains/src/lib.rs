//! Domain implementations for Propaga.
//!
//! Provides [`IntervalDomain`], [`BitsetDomain`], and [`HybridDomain`] for
//! representing integer variable domains with efficient intersection and pruning.

mod any;
mod bitset;
mod float;
mod hybrid;
mod interval;
mod set;
mod set_interval;

pub use any::{AnyDomain, DomainKind};
pub use bitset::BitsetDomain;
pub use float::{
    FloatDomain, cos_monotonic_on, sin_monotonic_on, unique_cos_preimage, unique_sin_preimage,
};
pub use hybrid::{BITSET_SPAN_THRESHOLD, HybridDomain};
pub use interval::IntervalDomain;
pub use set::SetDomain;
pub use set_interval::SetIntervalDomain;
