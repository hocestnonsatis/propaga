use propaga_core::{Domain, DomainView};
use std::collections::BTreeSet;

/// Finite set domain over integers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetDomain {
    values: BTreeSet<i32>,
}

impl SetDomain {
    /// Creates a set domain from explicit values.
    #[must_use]
    pub fn from_values(values: impl IntoIterator<Item = i32>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    /// Creates a set domain from inclusive bounds.
    #[must_use]
    pub fn range(min: i32, max: i32) -> Self {
        Self::from_values(min..=max)
    }

    /// Returns contained values in ascending order.
    #[must_use]
    pub fn values(&self) -> Vec<i32> {
        self.values.iter().copied().collect()
    }

    /// Removes values not in `allowed`.
    #[must_use]
    pub fn retain(&self, allowed: &BTreeSet<i32>) -> Self {
        Self {
            values: self
                .values
                .iter()
                .copied()
                .filter(|value| allowed.contains(value))
                .collect(),
        }
    }
}

impl DomainView for SetDomain {
    type Value = i32;

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn is_fixed(&self) -> bool {
        self.values.len() == 1
    }

    fn size(&self) -> usize {
        self.values.len()
    }

    fn min(&self) -> Option<Self::Value> {
        self.values.iter().next().copied()
    }

    fn max(&self) -> Option<Self::Value> {
        self.values.iter().next_back().copied()
    }

    fn contains(&self, value: Self::Value) -> bool {
        self.values.contains(&value)
    }
}

impl Domain for SetDomain {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_contains_bounds() {
        let domain = SetDomain::range(2, 4);
        assert!(domain.contains(3));
        assert!(!domain.contains(5));
        assert_eq!(domain.size(), 3);
    }
}
