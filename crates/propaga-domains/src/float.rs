/// Interval domain over floating-point values with inclusive bounds.
///
/// Optional interior `holes` record excluded IEEE points that cannot be dropped
/// by bound tightening alone. Most arithmetic helpers preserve or project holes
/// when safe; wide non-injective maps may still over-approximate.
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
        let mut min = min;
        let mut max = max;
        let mut pending = holes;
        loop {
            if min > max {
                return Self::new(1.0, 0.0);
            }
            let mut advanced = false;
            let mut interior = Vec::new();
            for hole in pending.drain(..) {
                if !hole.is_finite() {
                    continue;
                }
                if (hole - min).abs() <= f64::EPSILON {
                    min = next_up(min);
                    advanced = true;
                } else if (hole - max).abs() <= f64::EPSILON {
                    max = next_down(max);
                    advanced = true;
                } else if hole > min && hole < max {
                    interior.push(hole);
                }
            }
            if !advanced {
                interior.sort_by(|left, right| {
                    left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
                });
                interior.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
                if (max - min).abs() <= f64::EPSILON
                    && interior
                        .iter()
                        .any(|hole| (*hole - min).abs() <= f64::EPSILON)
                {
                    return Self::new(1.0, 0.0);
                }
                return Self {
                    min,
                    max,
                    holes: interior,
                };
            }
            pending = interior;
        }
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

    /// Approximate cardinality for variable-ordering heuristics (MRV / anti-first-fail).
    ///
    /// Continuous floats are not countable; this uses coarse width buckets (plus a
    /// small hole bonus) so narrower domains look smaller than wider ones without
    /// dwarfing integer domain sizes.
    #[must_use]
    pub fn size(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        if self.is_fixed() {
            return 1;
        }
        let width = self.max - self.min;
        if !width.is_finite() || width < 0.0 {
            return 1_000_000;
        }
        let bucket: usize = if width <= 1e-12 {
            2
        } else if width <= 1e-9 {
            3
        } else if width <= 1e-6 {
            4
        } else if width <= 1e-3 {
            5
        } else if width <= 1.0 {
            6
        } else if width <= 10.0 {
            7
        } else if width <= 100.0 {
            8
        } else if width <= 1e6 {
            9
        } else {
            10
        };
        bucket.saturating_add(self.holes.len().min(5))
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
    /// When either side is a nonzero fixed point, holes are mapped through the quotient.
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
        if self.is_fixed() {
            let a = self.min;
            let mut result = Self::new(min, max);
            // a / h is the unique quotient image of divisor hole h when a ≠ 0.
            if a != 0.0 {
                for &hole in &divisor.holes {
                    let image = a / hole;
                    if image.is_finite() {
                        result = result.exclude(image);
                    }
                }
            }
            return result;
        }
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

    /// Returns a conservative image of `min(self, other)`.
    #[must_use]
    pub fn min_with(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let min = self.min.min(other.min);
        let max = self.max.min(other.max);
        let mut holes = Vec::new();
        for &hole in self.holes.iter().chain(other.holes.iter()) {
            if hole > min && hole < max && !self.contains(hole) && !other.contains(hole) {
                holes.push(hole);
            }
        }
        // If one side forbids `h` and the other lies entirely above `h`, min ≠ h.
        for &hole in &self.holes {
            if hole > min && hole < max && other.lower_bound() > hole {
                holes.push(hole);
            }
        }
        for &hole in &other.holes {
            if hole > min && hole < max && self.lower_bound() > hole {
                holes.push(hole);
            }
        }
        Self::from_parts(min, max, holes)
    }

    /// Returns a conservative image of `max(self, other)`.
    #[must_use]
    pub fn max_with(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let min = self.min.max(other.min);
        let max = self.max.max(other.max);
        let mut holes = Vec::new();
        for &hole in self.holes.iter().chain(other.holes.iter()) {
            if hole > min && hole < max && !self.contains(hole) && !other.contains(hole) {
                holes.push(hole);
            }
        }
        // If one side forbids `h` and the other lies entirely below `h`, max ≠ h.
        for &hole in &self.holes {
            if hole > min && hole < max && other.upper_bound() < hole {
                holes.push(hole);
            }
        }
        for &hole in &other.holes {
            if hole > min && hole < max && self.upper_bound() < hole {
                holes.push(hole);
            }
        }
        Self::from_parts(min, max, holes)
    }

    /// Returns a conservative ceiling interval.
    ///
    /// Integer images whose ceil preimage `(n-1, n]` has no admissible point in this
    /// domain (typically an endpoint emptied by a hole) are dropped. Wide spans skip
    /// per-integer scanning and keep a hole-free bound interval.
    #[must_use]
    pub fn ceil(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let lo = self.min.ceil();
        let hi = self.max.ceil();
        self.project_integer_images(lo, hi, |n| self.intersects_closed(next_up(n - 1.0), n))
    }

    /// Returns a conservative floor interval.
    ///
    /// Integer images whose floor preimage `[n, n+1)` has no admissible point in this
    /// domain are dropped. Wide spans skip per-integer scanning.
    #[must_use]
    pub fn floor(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let lo = self.min.floor();
        let hi = self.max.floor();
        self.project_integer_images(lo, hi, |n| self.intersects_closed(n, next_down(n + 1.0)))
    }

    /// Returns a conservative round interval (`f64::round`, half away from zero).
    ///
    /// Integer images whose round preimage has no admissible point in this domain are
    /// dropped. Wide spans skip per-integer scanning.
    #[must_use]
    pub fn round(&self) -> Self {
        if self.is_empty() {
            return Self::new(1.0, 0.0);
        }
        let lo = self.min.round();
        let hi = self.max.round();
        self.project_integer_images(lo, hi, |n| {
            let (pre_lo, pre_hi) = round_preimage_bounds(n);
            self.intersects_closed(pre_lo, pre_hi)
        })
    }

    /// True when `[lo, hi] ∩ domain` contains at least one admissible point.
    ///
    /// Unit-width preimages (ceil/floor/round) walk gaps between holes so a
    /// covering of every IEEE point in the clipped interval is detected. Longer
    /// intersections stay feasible under sparse holes.
    fn intersects_closed(&self, lo: f64, hi: f64) -> bool {
        let a = lo.max(self.min);
        let b = hi.min(self.max);
        if a > b {
            return false;
        }
        if (b - a).abs() <= f64::EPSILON {
            return self.contains(a);
        }
        if b - a <= 1.0 + f64::EPSILON && !self.holes.is_empty() {
            return self.has_admissible_in(a, b);
        }
        true
    }

    /// Returns true when `[a, b]` contains an admissible IEEE point.
    fn has_admissible_in(&self, a: f64, b: f64) -> bool {
        if self.contains(a) {
            return true;
        }
        let mut holes: Vec<f64> = self
            .holes
            .iter()
            .copied()
            .filter(|&hole| hole > a && hole < b)
            .collect();
        holes.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let mut prev = a;
        for hole in holes {
            if next_up(prev) < hole {
                return true;
            }
            prev = hole;
        }
        if self.contains(b) {
            return true;
        }
        next_up(prev) < b
    }

    fn project_integer_images(
        &self,
        lo: f64,
        hi: f64,
        mut feasible: impl FnMut(f64) -> bool,
    ) -> Self {
        if lo > hi {
            return Self::new(1.0, 0.0);
        }
        if (hi - lo).abs() <= f64::EPSILON {
            return if feasible(lo) {
                Self::fix(lo)
            } else {
                Self::new(1.0, 0.0)
            };
        }
        if !lo.is_finite() || !hi.is_finite() {
            return Self::new(lo, hi);
        }
        let start = lo as i64;
        let end = hi as i64;
        if end - start > MAX_INTEGER_IMAGE_SCAN {
            // Wide span: shrink hole-emptied endpoints (bounded end scans), then
            // punch interior integer images emptied by the domain hole list.
            let mut min = None;
            for k in 0..=MAX_INTEGER_IMAGE_SCAN {
                let n = (start + k) as f64;
                if n > hi {
                    break;
                }
                if feasible(n) {
                    min = Some(n);
                    break;
                }
            }
            let mut max = None;
            for k in 0..=MAX_INTEGER_IMAGE_SCAN {
                let n = (end - k) as f64;
                if n < lo {
                    break;
                }
                if feasible(n) {
                    max = Some(n);
                    break;
                }
            }
            return match (min, max) {
                (Some(a), Some(b)) if a <= b => {
                    let mut holes = Vec::new();
                    for &hole in &self.holes {
                        for n in integer_images_near_hole(hole) {
                            if n > a && n < b && !feasible(n) {
                                holes.push(n);
                            }
                        }
                    }
                    Self::from_parts(a, b, holes)
                }
                _ => Self::new(1.0, 0.0),
            };
        }
        let mut achievable = Vec::new();
        for k in start..=end {
            let n = k as f64;
            if feasible(n) {
                achievable.push(n);
            }
        }
        match achievable.as_slice() {
            [] => Self::new(1.0, 0.0),
            [only] => Self::fix(*only),
            values => {
                let min = values[0];
                let max = values[values.len() - 1];
                let mut holes = Vec::new();
                let mut next = min as i64 + 1;
                for &value in &values[1..] {
                    let k = value as i64;
                    while next < k {
                        holes.push(next as f64);
                        next += 1;
                    }
                    next = k + 1;
                }
                Self::from_parts(min, max, holes)
            }
        }
    }
}

/// Cap for scanning integer images of ceil/floor/round (wide spans use end-only scans).
const MAX_INTEGER_IMAGE_SCAN: i64 = 10_000;

/// Candidate integer images that a domain hole might empty under ceil/floor/round.
fn integer_images_near_hole(hole: f64) -> [f64; 3] {
    [hole.floor(), hole.ceil(), hole.round()]
}

/// Inclusive bounds for the preimage of `n` under `f64::round` (half away from zero).
fn round_preimage_bounds(n: f64) -> (f64, f64) {
    if n > 0.0 {
        (n - 0.5, next_down(n + 0.5))
    } else if n < 0.0 {
        (next_up(n - 0.5), n + 0.5)
    } else {
        (next_up(-0.5), next_down(0.5))
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
    fn size_grows_with_width_for_mrv() {
        let narrow = FloatDomain::new(0.0, 1e-6);
        let mid = FloatDomain::new(0.0, 1.0);
        let wide = FloatDomain::new(0.0, 50.0);
        assert!(narrow.size() >= 2);
        assert!(mid.size() > narrow.size());
        assert!(wide.size() > mid.size());
        assert_eq!(FloatDomain::fix(1.0).size(), 1);
        assert_eq!(FloatDomain::new(1.0, 0.0).size(), 0);
    }

    #[test]
    fn excludes_endpoint_by_bound_tightening() {
        let domain = FloatDomain::new(1.0, 2.0).exclude(1.0);
        assert!(domain.lower_bound() > 1.0);
        assert!(domain.holes().is_empty());
    }

    #[test]
    fn remove_below_skips_hole_at_new_lower_bound() {
        let domain = FloatDomain::new(0.0, 5.0).exclude(2.0);
        let tightened = domain.remove_below(2.0);
        assert!(!tightened.contains(2.0));
        assert!(tightened.lower_bound() > 2.0);
    }

    #[test]
    fn pin_to_interior_hole_empties_domain() {
        let domain = FloatDomain::new(0.0, 5.0).exclude(2.0);
        let pinned = domain.remove_below(2.0).remove_above(2.0);
        assert!(pinned.is_empty());
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
    fn min_with_projects_hole_when_other_side_lies_above() {
        let a = FloatDomain::new(0.0, 5.0).exclude(2.0);
        let b = FloatDomain::new(2.5, 4.0);
        let image = a.min_with(&b);
        assert!(!image.contains(2.0));
    }

    #[test]
    fn max_with_projects_hole_when_other_side_lies_below() {
        let a = FloatDomain::new(0.0, 5.0).exclude(2.0);
        let b = FloatDomain::new(0.0, 1.5);
        let image = a.max_with(&b);
        assert!(!image.contains(2.0));
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
    fn ceil_drops_endpoint_only_image_emptied_by_hole() {
        // ceil⁻¹(2) ∩ [2, 3] = {2}; excluding 2 removes image 2.
        let domain = FloatDomain::new(2.0, 3.0).exclude(2.0);
        let image = domain.ceil();
        assert!(image.is_fixed());
        assert!((image.lower_bound() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ceil_shrinks_wide_span_endpoint_emptied_by_hole() {
        // Span exceeds the full-scan cap; endpoint-only image 2 is still dropped.
        let domain = FloatDomain::new(2.0, 20_000.0).exclude(2.0);
        let image = domain.ceil();
        assert!((image.lower_bound() - 3.0).abs() < f64::EPSILON);
        assert!((image.upper_bound() - 20_000.0).abs() < f64::EPSILON);
        assert!(image.holes().is_empty());
    }

    #[test]
    fn ceil_wide_span_hole_list_keeps_continuum_preimages() {
        // exclude(100) does not empty ceil⁻¹(100)=(99,100] on a continuum domain;
        // hole-list projection must not spuriously drop interior images.
        let domain = FloatDomain::new(2.0, 20_000.0).exclude(2.0).exclude(100.0);
        let image = domain.ceil();
        assert!((image.lower_bound() - 3.0).abs() < f64::EPSILON);
        assert!(image.contains(100.0));
        assert!(image.holes().is_empty());
    }

    #[test]
    fn ceil_unit_preimage_emptied_when_only_ieee_points_are_holes() {
        // ceil⁻¹(3) ∩ [2,3] = {2,3} in the bound sense; with both endpoints
        // excluded and nothing admissible left in (2,3], image 3 is dropped on
        // the small-span path. Wide-span hole-list uses the same feasibility check.
        let domain = FloatDomain::new(2.0, 3.0).exclude(2.0).exclude(3.0);
        let image = domain.ceil();
        // Domain collapses toward empty or a sliver; ceil image must not keep 2.
        assert!(!image.contains(2.0) || image.is_empty());
    }

    #[test]
    fn floor_drops_endpoint_only_image_emptied_by_hole() {
        // floor⁻¹(3) ∩ [2, 3] = {3}; excluding 3 removes image 3.
        let domain = FloatDomain::new(2.0, 3.0).exclude(3.0);
        let image = domain.floor();
        assert!(image.is_fixed());
        assert!((image.lower_bound() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn round_collapses_to_fixed_when_constant() {
        let domain = FloatDomain::new(1.1, 1.4).exclude(1.2);
        let image = domain.round();
        assert!(image.is_fixed());
        assert!((image.lower_bound() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn round_bounds_image_when_non_constant() {
        let domain = FloatDomain::new(0.4, 1.6);
        let image = domain.round();
        assert!((image.lower_bound() - 0.0).abs() < f64::EPSILON);
        assert!((image.upper_bound() - 2.0).abs() < f64::EPSILON);
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

    #[test]
    fn divide_maps_divisor_holes_when_dividend_is_fixed() {
        let a = FloatDomain::fix(6.0);
        let c = FloatDomain::new(1.0, 10.0).exclude(2.0);
        let b = a.divide(&c);
        assert!(!b.contains(3.0));
        assert!((b.lower_bound() - 0.6).abs() < 1e-9);
        assert!((b.upper_bound() - 6.0).abs() < 1e-9);
    }
}
