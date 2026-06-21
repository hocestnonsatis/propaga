/// Read-only interface for inspecting a variable domain.
pub trait DomainView {
    /// Value type stored in the domain.
    type Value: Copy + Eq + Ord + std::fmt::Debug;

    /// Returns `true` when the domain contains no values.
    fn is_empty(&self) -> bool;

    /// Returns `true` when the domain contains exactly one value.
    fn is_fixed(&self) -> bool;

    /// Returns the number of values in the domain.
    fn size(&self) -> usize;

    /// Returns the smallest value in the domain, if any.
    fn min(&self) -> Option<Self::Value>;

    /// Returns the largest value in the domain, if any.
    fn max(&self) -> Option<Self::Value>;

    /// Returns `true` when the domain contains `value`.
    fn contains(&self, value: Self::Value) -> bool;
}

/// Domain that can be cloned and stored in the engine.
pub trait Domain: DomainView + Clone {}
