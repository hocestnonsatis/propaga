use propaga_core::{Domain, DomainView};

/// Inclusive integer interval domain with optional excluded interior values.
///
/// The domain is empty when `min > max` or every value in the span is excluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalDomain {
    min: i32,
    max: i32,
    excluded: Vec<i32>,
}

impl IntervalDomain {
    /// Creates an inclusive interval domain.
    #[must_use]
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            min,
            max,
            excluded: Vec::new(),
        }
        .normalized()
    }

    /// Creates a singleton domain fixed to `value`.
    #[must_use]
    pub fn fix(value: i32) -> Self {
        Self {
            min: value,
            max: value,
            excluded: Vec::new(),
        }
    }

    /// Creates the full domain for the given inclusive bounds.
    #[must_use]
    pub fn full(min: i32, max: i32) -> Self {
        Self::new(min, max)
    }

    /// Returns the current lower bound of the span.
    #[must_use]
    pub const fn lower_bound(&self) -> i32 {
        self.min
    }

    /// Returns the current upper bound of the span.
    #[must_use]
    pub const fn upper_bound(&self) -> i32 {
        self.max
    }

    /// Collects all values in the domain.
    #[must_use]
    pub fn collect_values(&self) -> Vec<i32> {
        let mut values = Vec::with_capacity(self.size());
        if let (Some(min), Some(max)) = (self.min(), self.max()) {
            for value in min..=max {
                if self.contains(value) {
                    values.push(value);
                }
            }
        }
        values
    }

    /// Removes values strictly below `bound`.
    #[must_use]
    pub fn remove_below(&self, bound: i32) -> Self {
        if self.is_empty() {
            return self.clone();
        }

        let mut next = self.clone();
        next.min = next.min.max(bound);
        next.excluded.retain(|value| *value >= next.min);
        next.normalized()
    }

    /// Removes values strictly above `bound`.
    #[must_use]
    pub fn remove_above(&self, bound: i32) -> Self {
        if self.is_empty() {
            return self.clone();
        }

        let mut next = self.clone();
        next.max = next.max.min(bound);
        next.excluded.retain(|value| *value <= next.max);
        next.normalized()
    }

    /// Removes `value` from the domain.
    #[must_use]
    pub fn remove(&self, value: i32) -> Self {
        if self.is_empty() || !self.contains(value) {
            return self.clone();
        }

        let mut next = self.clone();

        if next.is_fixed() {
            return Self::empty();
        }

        if value == next.min {
            next.min = next.min.saturating_add(1);
        } else if value == next.max {
            next.max = next.max.saturating_sub(1);
        } else if !next.excluded.contains(&value) {
            next.excluded.push(value);
            next.excluded.sort_unstable();
        }

        next.normalized()
    }

    /// Tightens this domain to the intersection with `other`.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut next = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
            excluded: self
                .excluded
                .iter()
                .chain(other.excluded.iter())
                .copied()
                .collect(),
        };
        next.excluded.sort_unstable();
        next.excluded.dedup();
        next.normalized()
    }

    fn empty() -> Self {
        Self {
            min: 1,
            max: 0,
            excluded: Vec::new(),
        }
    }

    fn normalized(mut self) -> Self {
        if self.min > self.max {
            return Self::empty();
        }

        self.excluded
            .retain(|value| *value >= self.min && *value <= self.max);
        self.excluded.sort_unstable();
        self.excluded.dedup();

        while self.min <= self.max && self.excluded.contains(&self.min) {
            self.min += 1;
        }

        while self.max >= self.min && self.excluded.contains(&self.max) {
            self.max -= 1;
        }

        self.excluded
            .retain(|value| *value >= self.min && *value <= self.max);

        if self.min > self.max {
            return Self::empty();
        }

        self
    }
}

impl DomainView for IntervalDomain {
    type Value = i32;

    fn is_empty(&self) -> bool {
        self.min > self.max
    }

    fn is_fixed(&self) -> bool {
        self.size() == 1
    }

    fn size(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        let span = (self.max - self.min + 1) as usize;
        span.saturating_sub(self.excluded.len())
    }

    fn min(&self) -> Option<Self::Value> {
        if self.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }

    fn max(&self) -> Option<Self::Value> {
        if self.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }

    fn contains(&self, value: Self::Value) -> bool {
        !self.is_empty()
            && value >= self.min
            && value <= self.max
            && !self.excluded.contains(&value)
    }
}

impl Domain for IntervalDomain {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_when_min_greater_than_max() {
        let domain = IntervalDomain::new(5, 3);
        assert!(domain.is_empty());
        assert_eq!(domain.size(), 0);
    }

    #[test]
    fn fixed_domain_has_size_one() {
        let domain = IntervalDomain::fix(7);
        assert!(domain.is_fixed());
        assert_eq!(domain.size(), 1);
        assert_eq!(domain.min(), Some(7));
        assert_eq!(domain.max(), Some(7));
    }

    #[test]
    fn remove_below_tightens_lower_bound() {
        let domain = IntervalDomain::new(1, 10).remove_below(4);
        assert_eq!(domain.lower_bound(), 4);
        assert_eq!(domain.upper_bound(), 10);
        assert_eq!(domain.size(), 7);
    }

    #[test]
    fn remove_above_tightens_upper_bound() {
        let domain = IntervalDomain::new(1, 10).remove_above(6);
        assert_eq!(domain.lower_bound(), 1);
        assert_eq!(domain.upper_bound(), 6);
    }

    #[test]
    fn remove_boundary_values() {
        let domain = IntervalDomain::new(1, 3).remove(1);
        assert_eq!(domain, IntervalDomain::new(2, 3));

        let domain = IntervalDomain::new(1, 3).remove(3);
        assert_eq!(domain, IntervalDomain::new(1, 2));
    }

    #[test]
    fn remove_interior_value() {
        let domain = IntervalDomain::new(1, 3).remove(2);
        assert_eq!(domain.size(), 2);
        assert!(domain.contains(1));
        assert!(!domain.contains(2));
        assert!(domain.contains(3));
    }

    #[test]
    fn remove_only_value_becomes_empty() {
        let domain = IntervalDomain::fix(5).remove(5);
        assert!(domain.is_empty());
    }

    #[test]
    fn intersect_overlapping_intervals() {
        let a = IntervalDomain::new(1, 10);
        let b = IntervalDomain::new(4, 15);
        assert_eq!(a.intersect(&b), IntervalDomain::new(4, 10));
    }

    #[test]
    fn intersect_disjoint_intervals_is_empty() {
        let a = IntervalDomain::new(1, 3);
        let b = IntervalDomain::new(5, 7);
        assert!(a.intersect(&b).is_empty());
    }
}
