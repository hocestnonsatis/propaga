/// Interval domain over floating-point values with inclusive bounds.
///
/// Optional interior `holes` record excluded IEEE points that cannot be dropped
/// by bound tightening alone. Arithmetic helpers return hole-free intervals
/// (sound over-approximation).
#[derive(Clone, Debug, PartialEq)]
pub struct FloatDomain {
    min: f64,
    max: f64,
    holes: Vec<f64>,
}

impl FloatDomain {
    /// Creates an inclusive float interval.
    #[must_use]
    pub fn new(min: f64, max: f64) -> Self {
        if min <= max {
            Self {
                min,
                max,
                holes: Vec::new(),
            }
        } else {
            Self {
                min: 1.0,
                max: 0.0,
                holes: Vec::new(),
            }
        }
    }

    /// Creates a fixed float domain.
    #[must_use]
    pub fn fix(value: f64) -> Self {
        Self {
            min: value,
            max: value,
            holes: Vec::new(),
        }
    }

    fn from_parts(min: f64, max: f64, holes: Vec<f64>) -> Self {
        if min > max {
            return Self::new(1.0, 0.0);
        }
        let mut holes: Vec<f64> = holes
            .into_iter()
            .filter(|hole| *hole > min && *hole < max)
            .collect();
        holes.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        holes.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
        if (max - min).abs() <= f64::EPSILON
            && holes.iter().any(|hole| (*hole - min).abs() <= f64::EPSILON)
        {
            return Self::new(1.0, 0.0);
        }
        Self { min, max, holes }
    }

    /// Returns `true` when the domain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.min > self.max
            || ((self.max - self.min).abs() <= f64::EPSILON && self.is_hole(self.min))
    }

    /// Returns `true` when the domain is a single admissible point.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        !self.is_empty() && (self.max - self.min).abs() < f64::EPSILON && !self.is_hole(self.min)
    }

    /// Returns the lower bound.
    #[must_use]
    pub const fn lower_bound(&self) -> f64 {
        self.min
    }

    /// Returns the upper bound.
    #[must_use]
    pub const fn upper_bound(&self) -> f64 {
        self.max
    }

    /// Returns excluded interior IEEE points.
    #[must_use]
    pub fn holes(&self) -> &[f64] {
        &self.holes
    }

    fn is_hole(&self, value: f64) -> bool {
        self.holes
            .iter()
            .any(|hole| (*hole - value).abs() <= f64::EPSILON)
    }

    /// Returns `true` when `value` is inside the interval and not excluded.
    #[must_use]
    pub fn contains(&self, value: f64) -> bool {
        !self.is_empty() && value >= self.min && value <= self.max && !self.is_hole(value)
    }

    /// Excludes a single IEEE point: bound-tighten at endpoints, else record a hole.
    #[must_use]
    pub fn exclude(&self, value: f64) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        if (self.min - value).abs() <= f64::EPSILON {
            return self.remove_below(next_up(value));
        }
        if (self.max - value).abs() <= f64::EPSILON {
            return self.remove_above(next_down(value));
        }
        if value > self.min && value < self.max {
            let mut holes = self.holes.clone();
            if !holes
                .iter()
                .any(|hole| (*hole - value).abs() <= f64::EPSILON)
            {
                holes.push(value);
            }
            return Self::from_parts(self.min, self.max, holes);
        }
        self.clone()
    }

    /// Tightens the lower bound.
    #[must_use]
    pub fn remove_below(&self, bound: f64) -> Self {
        Self::from_parts(self.min.max(bound), self.max, self.holes.clone())
    }

    /// Tightens the upper bound.
    #[must_use]
    pub fn remove_above(&self, bound: f64) -> Self {
        Self::from_parts(self.min, self.max.min(bound), self.holes.clone())
    }

    /// Creates a domain from bounds and excluded interior points.
    #[must_use]
    pub fn from_bounds_with_holes(min: f64, max: f64, holes: &[f64]) -> Self {
        Self::from_parts(min, max, holes.to_vec())
    }

    /// Maps this domain through `scale * x + shift`, preserving holes.
    #[must_use]
    pub fn affine(&self, scale: f64, shift: f64) -> Self {
        if self.is_empty() || !scale.is_finite() || !shift.is_finite() {
            return Self::new(1.0, 0.0);
        }
        if scale == 0.0 {
            return Self::fix(shift);
        }
        let (min, max) = if scale > 0.0 {
            (scale * self.min + shift, scale * self.max + shift)
        } else {
            (scale * self.max + shift, scale * self.min + shift)
        };
        let holes = self
            .holes
            .iter()
            .map(|hole| scale * hole + shift)
            .filter(|hole| hole.is_finite())
            .collect();
        Self::from_parts(min, max, holes)
    }

    /// Returns the interval product `self * other`.
    ///
    /// When either operand is fixed, holes are mapped through the affine product.
    #[must_use]
    pub fn times(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if other.is_fixed() {
            return self.affine(other.min, 0.0);
        }
        if self.is_fixed() {
            return other.affine(self.min, 0.0);
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
    /// When `divisor` is a nonzero fixed point, holes are mapped through the quotient.
    #[must_use]
    pub fn divide(&self, divisor: &Self) -> Self {
        if self.is_empty() || divisor.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if divisor.contains(0.0) {
            return Self::new(f64::NEG_INFINITY, f64::INFINITY);
        }
        if divisor.is_fixed() {
            return self.affine(1.0 / divisor.min, 0.0);
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
    ///
    /// When either operand is fixed, holes are mapped through the translation.
    #[must_use]
    pub fn plus(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if other.is_fixed() {
            return self.affine(1.0, other.min);
        }
        if self.is_fixed() {
            return other.affine(1.0, self.min);
        }
        Self::new(self.min + other.min, self.max + other.max)
    }

    /// Returns a sound absolute value interval, preserving holes when safe.
    #[must_use]
    pub fn abs(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.min >= 0.0 {
            return self.clone();
        }
        if self.max <= 0.0 {
            return -self.clone();
        }
        let max_abs = self.min.abs().max(self.max.abs());
        let mut result = Self::new(0.0, max_abs);
        for &hole in &self.holes {
            let image = hole.abs();
            // 0 remains a limit point of abs even if 0 itself is excluded.
            if image == 0.0 {
                continue;
            }
            if !self.contains(image) && !self.contains(-image) {
                result = result.exclude(image);
            }
        }
        result
    }

    /// Returns a sound square root interval, mapping nonnegative holes.
    #[must_use]
    pub fn sqrt(&self) -> Self {
        if self.is_empty() || self.max < 0.0 {
            return Self::new(1.0, 0.0);
        }
        let min = self.min.max(0.0).sqrt();
        let max = self.max.max(0.0).sqrt();
        let holes = self
            .holes
            .iter()
            .copied()
            .filter(|hole| *hole >= 0.0)
            .map(f64::sqrt)
            .collect::<Vec<_>>();
        Self::from_parts(min, max, holes)
    }

    /// Returns a conservative sine interval, mapping holes when locally monotonic.
    #[must_use]
    pub fn sin(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.max - self.min >= std::f64::consts::TAU {
            return Self::new(-1.0, 1.0);
        }
        if sin_monotonic_on(self.min, self.max) {
            let lo = self.min.sin();
            let hi = self.max.sin();
            let holes = self.holes.iter().copied().map(f64::sin).collect::<Vec<_>>();
            return Self::from_parts(lo.min(hi), lo.max(hi), holes);
        }
        let corners = [self.min.sin(), self.max.sin()];
        Self::new(
            corners.iter().copied().fold(f64::INFINITY, f64::min),
            corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Returns a conservative cosine interval, mapping holes when locally monotonic.
    #[must_use]
    pub fn cos(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        if self.max - self.min >= std::f64::consts::TAU {
            return Self::new(-1.0, 1.0);
        }
        if cos_monotonic_on(self.min, self.max) {
            let lo = self.min.cos();
            let hi = self.max.cos();
            let holes = self.holes.iter().copied().map(f64::cos).collect::<Vec<_>>();
            return Self::from_parts(lo.min(hi), lo.max(hi), holes);
        }
        let corners = [self.min.cos(), self.max.cos()];
        Self::new(
            corners.iter().copied().fold(f64::INFINITY, f64::min),
            corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Returns a conservative natural logarithm interval, mapping positive holes.
    #[must_use]
    pub fn ln(&self) -> Self {
        if self.is_empty() || self.max <= 0.0 {
            return Self::new(1.0, 0.0);
        }
        let min = self.min.max(0.0).ln();
        let max = self.max.ln();
        let holes = self
            .holes
            .iter()
            .copied()
            .filter(|hole| *hole > 0.0)
            .map(f64::ln)
            .collect::<Vec<_>>();
        Self::from_parts(min, max, holes)
    }

    /// Returns a conservative exponential interval, mapping holes.
    #[must_use]
    pub fn exp(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let holes = self.holes.iter().copied().map(f64::exp).collect::<Vec<_>>();
        Self::from_parts(self.min.exp(), self.max.exp(), holes)
    }

    /// Returns a conservative ceiling interval.
    ///
    /// Sparse interior holes rarely empty a ceil preimage `(n-1, n]`, so holes are
    /// dropped unless the map is constant on the domain.
    #[must_use]
    pub fn ceil(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let lo = self.min.ceil();
        let hi = self.max.ceil();
        if (lo - hi).abs() <= f64::EPSILON {
            return Self::fix(lo);
        }
        Self::new(lo, hi)
    }

    /// Returns a conservative floor interval.
    ///
    /// Sparse interior holes rarely empty a floor preimage `[n, n+1)`, so holes are
    /// dropped unless the map is constant on the domain.
    #[must_use]
    pub fn floor(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let lo = self.min.floor();
        let hi = self.max.floor();
        if (lo - hi).abs() <= f64::EPSILON {
            return Self::fix(lo);
        }
        Self::new(lo, hi)
    }

    /// Returns a conservative round interval.
    #[must_use]
    pub fn round(&self) -> Self {
        self.floor().plus(&Self::new(0.0, 1.0))
    }
}

/// `sin` is monotonic on `[min, max]` when no odd multiple of `π/2` lies inside.
#[must_use]
pub fn sin_monotonic_on(min: f64, max: f64) -> bool {
    !contains_odd_half_pi(min, max)
}

/// `cos` is monotonic on `[min, max]` when no integer multiple of `π` lies inside.
#[must_use]
pub fn cos_monotonic_on(min: f64, max: f64) -> bool {
    !contains_integer_pi(min, max)
}

fn contains_odd_half_pi(min: f64, max: f64) -> bool {
    if max <= min {
        return false;
    }
    let start = ((min / std::f64::consts::FRAC_PI_2).floor() as i64) - 1;
    let end = ((max / std::f64::consts::FRAC_PI_2).ceil() as i64) + 1;
    for k in start..=end {
        if k.rem_euclid(2) == 1 {
            let point = k as f64 * std::f64::consts::FRAC_PI_2;
            if point > min && point < max {
                return true;
            }
        }
    }
    false
}

fn contains_integer_pi(min: f64, max: f64) -> bool {
    if max <= min {
        return false;
    }
    let start = ((min / std::f64::consts::PI).floor() as i64) - 1;
    let end = ((max / std::f64::consts::PI).ceil() as i64) + 1;
    for k in start..=end {
        let point = k as f64 * std::f64::consts::PI;
        if point > min && point < max {
            return true;
        }
    }
    false
}

/// Unique `x ∈ [min, max]` with `sin(x) = y`, if any.
#[must_use]
pub fn unique_sin_preimage(y: f64, min: f64, max: f64) -> Option<f64> {
    if !(-1.0..=1.0).contains(&y) || !sin_monotonic_on(min, max) {
        return None;
    }
    let base = y.asin();
    let mut found = None;
    for k in -3..=3 {
        let shift = 2.0 * std::f64::consts::PI * f64::from(k);
        for candidate in [base + shift, std::f64::consts::PI - base + shift] {
            if candidate >= min && candidate <= max {
                if found.is_some_and(|existing: f64| (existing - candidate).abs() > f64::EPSILON) {
                    return None;
                }
                found = Some(candidate);
            }
        }
    }
    found
}

/// Unique `x ∈ [min, max]` with `cos(x) = y`, if any.
#[must_use]
pub fn unique_cos_preimage(y: f64, min: f64, max: f64) -> Option<f64> {
    if !(-1.0..=1.0).contains(&y) || !cos_monotonic_on(min, max) {
        return None;
    }
    let base = y.acos();
    let mut found = None;
    for k in -3..=3 {
        let shift = 2.0 * std::f64::consts::PI * f64::from(k);
        for candidate in [base + shift, -base + shift] {
            if candidate >= min && candidate <= max {
                if found.is_some_and(|existing: f64| (existing - candidate).abs() > f64::EPSILON) {
                    return None;
                }
                found = Some(candidate);
            }
        }
    }
    found
}

fn next_up(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_positive() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_add(1))
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_negative() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_sub(1))
    }
}

impl std::ops::Neg for FloatDomain {
    type Output = Self;

    fn neg(self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let holes = self.holes.into_iter().map(|hole| -hole).collect::<Vec<_>>();
        Self::from_parts(-self.max, -self.min, holes)
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
    fn excludes_interior_point_as_hole() {
        let domain = FloatDomain::new(0.0, 2.0).exclude(1.0);
        assert!(domain.contains(0.5));
        assert!(!domain.contains(1.0));
        assert_eq!(domain.holes(), &[1.0]);
    }

    #[test]
    fn excludes_endpoint_by_bound_tightening() {
        let domain = FloatDomain::new(1.0, 2.0).exclude(1.0);
        assert!(domain.lower_bound() > 1.0);
        assert!(domain.holes().is_empty());
    }

    #[test]
    fn plus_maps_holes_when_other_operand_is_fixed() {
        let left = FloatDomain::new(0.0, 2.0).exclude(1.0);
        let right = FloatDomain::fix(3.0);
        let sum = left.plus(&right);
        assert!(!sum.contains(4.0));
        assert_eq!(sum.holes(), &[4.0]);
    }

    #[test]
    fn times_maps_holes_when_factor_is_fixed() {
        let left = FloatDomain::new(0.0, 2.0).exclude(1.0);
        let right = FloatDomain::fix(2.0);
        let product = left.times(&right);
        assert!(!product.contains(2.0));
        assert_eq!(product.holes(), &[2.0]);
    }

    #[test]
    fn abs_preserves_holes_on_nonnegative_domain() {
        let domain = FloatDomain::new(0.0, 3.0).exclude(1.0);
        assert_eq!(domain.abs().holes(), &[1.0]);
    }

    #[test]
    fn abs_maps_holes_on_nonpositive_domain() {
        let domain = FloatDomain::new(-3.0, 0.0).exclude(-2.0);
        let abs = domain.abs();
        assert!(!abs.contains(2.0));
        assert_eq!(abs.holes(), &[2.0]);
    }

    #[test]
    fn abs_excludes_image_only_when_both_preimages_blocked() {
        let one_side = FloatDomain::new(-3.0, 3.0).exclude(2.0);
        assert!(one_side.abs().contains(2.0));
        let both = FloatDomain::new(-3.0, 3.0).exclude(2.0).exclude(-2.0);
        assert!(!both.abs().contains(2.0));
    }

    #[test]
    fn sqrt_and_exp_map_holes() {
        let sqrt = FloatDomain::new(0.0, 9.0).exclude(4.0).sqrt();
        assert!(!sqrt.contains(2.0));
        let exp = FloatDomain::new(0.0, 2.0).exclude(1.0).exp();
        assert!(!exp.contains(1.0_f64.exp()));
    }

    #[test]
    fn sin_maps_holes_on_monotonic_interval() {
        let domain = FloatDomain::new(0.0, 1.0).exclude(0.5);
        let image = domain.sin();
        assert!(!image.contains(0.5_f64.sin()));
        assert!(sin_monotonic_on(0.0, 1.0));
        assert!(!sin_monotonic_on(0.0, 2.0));
    }

    #[test]
    fn cos_maps_holes_on_monotonic_interval() {
        let domain = FloatDomain::new(0.1, 1.0).exclude(0.5);
        let image = domain.cos();
        assert!(!image.contains(0.5_f64.cos()));
        assert!(cos_monotonic_on(0.1, 1.0));
        assert!(!cos_monotonic_on(-0.1, 0.1));
    }

    #[test]
    fn ceil_collapses_to_fixed_when_constant() {
        let domain = FloatDomain::new(1.1, 1.9).exclude(1.5);
        let image = domain.ceil();
        assert!(image.is_fixed());
        assert!((image.lower_bound() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn times_computes_interval_product() {
        let a = FloatDomain::new(2.0, 3.0);
        let b = FloatDomain::new(4.0, 5.0);
        let product = a.times(&b);
        assert!((product.lower_bound() - 8.0).abs() < f64::EPSILON);
        assert!((product.upper_bound() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn divide_tightens_when_divisor_excludes_zero() {
        let c = FloatDomain::new(8.0, 15.0);
        let b = FloatDomain::new(4.0, 5.0);
        let a = c.divide(&b);
        assert!((a.lower_bound() - 1.6).abs() < 1e-9);
        assert!((a.upper_bound() - 3.75).abs() < 1e-9);
    }

    #[test]
    fn divide_is_unbounded_when_divisor_contains_zero() {
        let c = FloatDomain::new(1.0, 2.0);
        let b = FloatDomain::new(-1.0, 1.0);
        let a = c.divide(&b);
        assert!(a.lower_bound().is_infinite() && a.lower_bound() < 0.0);
        assert!(a.upper_bound().is_infinite() && a.upper_bound() > 0.0);
    }
}
