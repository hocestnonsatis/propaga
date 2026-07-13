/// Interval domain over floating-point values with inclusive bounds.
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

    /// Returns the interval product `self * other`.
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let corners = [
            self.min * other.min,
            self.min * other.max,
            self.max * other.min,
            self.max * other.max,
        ];
        let min = corners.iter().copied().fold(f64::INFINITY, f64::min);
        let max = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::new(min, max)
    }

    /// Returns a sound interval quotient `self / divisor`.
    ///
    /// When `divisor` contains zero the result is unbounded (no tightening).
    #[must_use]
    pub fn divide(self, divisor: Self) -> Self {
        if self.is_empty() || divisor.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if divisor.contains(0.0) {
            return Self::new(f64::NEG_INFINITY, f64::INFINITY);
        }
        let corners = [
            self.min / divisor.min,
            self.min / divisor.max,
            self.max / divisor.min,
            self.max / divisor.max,
        ];
        let min = corners.iter().copied().fold(f64::INFINITY, f64::min);
        let max = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::new(min, max)
    }

    /// Returns the interval sum `self + other`.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        Self::new(self.min + other.min, self.max + other.max)
    }

    /// Returns the interval negation `-self`.
    #[must_use]
    pub fn neg(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        Self::new(-self.max, -self.min)
    }

    /// Returns a sound absolute value interval.
    #[must_use]
    pub fn abs(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.min >= 0.0 {
            return self;
        }
        if self.max <= 0.0 {
            return Self::new(-self.max, -self.min);
        }
        Self::new(0.0, self.min.abs().max(self.max.abs()))
    }

    /// Returns a sound square root interval.
    #[must_use]
    pub fn sqrt(self) -> Self {
        if self.is_empty() || self.max < 0.0 {
            return Self::new(1.0, 0.0);
        }
        let min = self.min.max(0.0).sqrt();
        let max = self.max.max(0.0).sqrt();
        Self::new(min, max)
    }

    /// Returns a conservative sine interval.
    #[must_use]
    pub fn sin(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.max - self.min >= std::f64::consts::TAU {
            return Self::new(-1.0, 1.0);
        }
        let corners = [self.min.sin(), self.max.sin()];
        Self::new(
            corners.iter().copied().fold(f64::INFINITY, f64::min),
            corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Returns a conservative cosine interval.
    #[must_use]
    pub fn cos(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.max - self.min >= std::f64::consts::TAU {
            return Self::new(-1.0, 1.0);
        }
        let corners = [self.min.cos(), self.max.cos()];
        Self::new(
            corners.iter().copied().fold(f64::INFINITY, f64::min),
            corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Returns a conservative natural logarithm interval.
    #[must_use]
    pub fn ln(self) -> Self {
        if self.is_empty() || self.max <= 0.0 {
            return Self::new(1.0, 0.0);
        }
        Self::new(self.min.max(0.0).ln(), self.max.ln())
    }

    /// Returns a conservative exponential interval.
    #[must_use]
    pub fn exp(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        Self::new(self.min.exp(), self.max.exp())
    }

    /// Returns a conservative ceiling interval.
    #[must_use]
    pub fn ceil(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        Self::new(self.min.ceil(), self.max.ceil())
    }

    /// Returns a conservative floor interval.
    #[must_use]
    pub fn floor(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        Self::new(self.min.floor(), self.max.floor())
    }

    /// Returns a conservative round interval.
    #[must_use]
    pub fn round(self) -> Self {
        self.floor().plus(Self::new(0.0, 1.0))
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

    #[test]
    fn times_computes_interval_product() {
        let a = FloatDomain::new(2.0, 3.0);
        let b = FloatDomain::new(4.0, 5.0);
        let product = a.times(b);
        assert!((product.lower_bound() - 8.0).abs() < f64::EPSILON);
        assert!((product.upper_bound() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn divide_tightens_when_divisor_excludes_zero() {
        let c = FloatDomain::new(8.0, 15.0);
        let b = FloatDomain::new(4.0, 5.0);
        let a = c.divide(b);
        assert!((a.lower_bound() - 1.6).abs() < 1e-9);
        assert!((a.upper_bound() - 3.75).abs() < 1e-9);
    }

    #[test]
    fn divide_is_unbounded_when_divisor_contains_zero() {
        let c = FloatDomain::new(1.0, 2.0);
        let b = FloatDomain::new(-1.0, 1.0);
        let a = c.divide(b);
        assert!(a.lower_bound().is_infinite() && a.lower_bound() < 0.0);
        assert!(a.upper_bound().is_infinite() && a.upper_bound() > 0.0);
    }
}
