use crate::{BitsetDomain, IntervalDomain};
use propaga_core::{Domain, DomainView};

/// Maximum contiguous span kept as an interval before preferring a bitset.
pub const BITSET_SPAN_THRESHOLD: i32 = 64;

/// Domain that uses an interval or bitset representation depending on shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HybridDomain {
    Interval(IntervalDomain),
    Bitset(BitsetDomain),
}

impl HybridDomain {
    /// Creates a hybrid domain over the inclusive range `[min, max]`.
    #[must_use]
    pub fn new(min: i32, max: i32) -> Self {
        Self::Interval(IntervalDomain::new(min, max))
    }

    /// Creates a singleton domain.
    #[must_use]
    pub fn fix(value: i32) -> Self {
        Self::Interval(IntervalDomain::fix(value))
    }

    /// Creates a hybrid domain from an interval.
    #[must_use]
    pub fn from_interval(domain: IntervalDomain) -> Self {
        Self::Interval(domain).normalized()
    }

    /// Removes values strictly below `bound`.
    #[must_use]
    pub fn remove_below(&self, bound: i32) -> Self {
        let next = match self {
            Self::Interval(domain) => Self::Interval(domain.remove_below(bound)),
            Self::Bitset(domain) => Self::Bitset(domain.remove_below(bound)),
        };
        next.normalized()
    }

    /// Removes values strictly above `bound`.
    #[must_use]
    pub fn remove_above(&self, bound: i32) -> Self {
        let next = match self {
            Self::Interval(domain) => Self::Interval(domain.remove_above(bound)),
            Self::Bitset(domain) => Self::Bitset(domain.remove_above(bound)),
        };
        next.normalized()
    }

    /// Removes `value` from the domain.
    #[must_use]
    pub fn remove(&self, value: i32) -> Self {
        let next = match self {
            Self::Interval(domain) => Self::Interval(domain.remove(value)),
            Self::Bitset(domain) => Self::Bitset(domain.remove(value)),
        };
        next.normalized()
    }

    /// Returns the fixed value when the domain is a singleton.
    #[must_use]
    pub fn fixed_value(&self) -> Option<i32> {
        self.is_fixed()
            .then(|| self.min().expect("fixed domain must have min"))
    }

    /// Invokes `f` for each value in the domain.
    pub fn for_each_value(&self, mut f: impl FnMut(i32)) {
        match self {
            Self::Interval(domain) => {
                if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
                    for value in min..=max {
                        if domain.contains(value) {
                            f(value);
                        }
                    }
                }
            }
            Self::Bitset(domain) => {
                for value in domain.values() {
                    f(value);
                }
            }
        }
    }

    /// Collects all values in the domain.
    #[must_use]
    pub fn collect_values(&self) -> Vec<i32> {
        let mut values = Vec::new();
        self.for_each_value(|value| values.push(value));
        values
    }

    fn normalized(self) -> Self {
        let mut domain = self;
        domain.maybe_promote_to_bitset();
        domain.maybe_demote_to_interval();
        domain
    }

    fn maybe_promote_to_bitset(&mut self) {
        let Self::Interval(interval) = self else {
            return;
        };

        if interval.is_empty() {
            return;
        }

        let min = interval.lower_bound();
        let max = interval.upper_bound();
        let span = max - min + 1;
        let has_holes = interval.size() as i32 != span;

        if has_holes || span <= BITSET_SPAN_THRESHOLD {
            let values = interval.collect_values();
            *self = Self::Bitset(BitsetDomain::from_values(min, max, &values));
        }
    }

    fn maybe_demote_to_interval(&mut self) {
        let Self::Bitset(bitset) = self else {
            return;
        };

        if bitset.is_empty() {
            *self = Self::Interval(IntervalDomain::new(1, 0));
            return;
        }

        let min = bitset.min().expect("non-empty bitset");
        let max = bitset.max().expect("non-empty bitset");
        let span = max - min + 1;
        if span <= BITSET_SPAN_THRESHOLD && bitset.size() as i32 == span {
            *self = Self::Interval(IntervalDomain::new(min, max));
        }
    }
}

impl DomainView for HybridDomain {
    type Value = i32;

    fn is_empty(&self) -> bool {
        match self {
            Self::Interval(domain) => domain.is_empty(),
            Self::Bitset(domain) => domain.is_empty(),
        }
    }

    fn is_fixed(&self) -> bool {
        self.size() == 1
    }

    fn size(&self) -> usize {
        match self {
            Self::Interval(domain) => domain.size(),
            Self::Bitset(domain) => domain.size(),
        }
    }

    fn min(&self) -> Option<Self::Value> {
        match self {
            Self::Interval(domain) => domain.min(),
            Self::Bitset(domain) => domain.min(),
        }
    }

    fn max(&self) -> Option<Self::Value> {
        match self {
            Self::Interval(domain) => domain.max(),
            Self::Bitset(domain) => domain.max(),
        }
    }

    fn contains(&self, value: Self::Value) -> bool {
        match self {
            Self::Interval(domain) => domain.contains(value),
            Self::Bitset(domain) => domain.contains(value),
        }
    }
}

impl Domain for HybridDomain {}

impl From<IntervalDomain> for HybridDomain {
    fn from(domain: IntervalDomain) -> Self {
        Self::from_interval(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interior_removal_promotes_to_bitset() {
        let domain = HybridDomain::new(1, 5).remove(3);
        assert!(matches!(domain, HybridDomain::Bitset(_)));
        assert!(!domain.contains(3));
    }

    #[test]
    fn contiguous_bitset_demotes_to_interval() {
        let domain = HybridDomain::Bitset(BitsetDomain::new(2, 4)).normalized();
        assert!(matches!(domain, HybridDomain::Interval(_)));
    }
}
