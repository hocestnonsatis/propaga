/// Interval domain over floating-point values with inclusive bounds.
///
/// Not yet integrated into the propagation engine; provides interval arithmetic helpers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatDomain {
    min: f64,
    max: f64,
}

impl FloatDomain {
    /// Creates an inclusive float interval.
    #[must_use]
    pub fn new(min: f64, max: f64) -> Self {
        if min <= max {
            Self { min, max }
        } else {
            Self { min: 1.0, max: 0.0 }
        }
    }

    /// Creates a fixed float domain.
    #[must_use]
    pub fn fix(value: f64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    /// Returns `true` when the domain is empty.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.min > self.max
    }

    /// Returns `true` when the domain is a single point.
    #[must_use]
    pub fn is_fixed(self) -> bool {
        !self.is_empty() && (self.max - self.min).abs() < f64::EPSILON
    }

    /// Returns the lower bound.
    #[must_use]
    pub const fn lower_bound(self) -> f64 {
        self.min
    }

    /// Returns the upper bound.
    #[must_use]
    pub const fn upper_bound(self) -> f64 {
        self.max
    }

    /// Returns `true` when `value` is inside the interval.
    #[must_use]
    pub fn contains(self, value: f64) -> bool {
        !self.is_empty() && value >= self.min && value <= self.max
    }

    /// Tightens the lower bound.
    #[must_use]
    pub fn remove_below(self, bound: f64) -> Self {
        Self::new(self.min.max(bound), self.max)
    }

    /// Tightens the upper bound.
    #[must_use]
    pub fn remove_above(self, bound: f64) -> Self {
        Self::new(self.min, self.max.min(bound))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tightens_bounds() {
        let domain = FloatDomain::new(0.0, 10.0);
        let narrowed = domain.remove_below(2.5).remove_above(7.5);
        assert!(narrowed.contains(5.0));
        assert!(!narrowed.contains(1.0));
    }
}
