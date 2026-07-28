use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator, VariableId,
};
use propaga_domains::{FloatDomain, unique_cos_preimage, unique_sin_preimage};

use super::float_eq::FloatEqPropagator;
use super::float_le::FloatLePropagator;
use super::float_ne::FloatNePropagator;
use crate::reified::reif_literal;

#[derive(Clone, Debug)]
pub struct FloatEqReifPropagator {
    watched: [VariableId; 3],
}

impl FloatEqReifPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for FloatEqReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let mut changed = false;

        match reif_literal(ctx, reif_id) {
            Some(1) => {
                let mut eq = FloatEqPropagator::new(left_id, right_id);
                return eq.propagate(ctx);
            }
            Some(0) => {
                let mut ne = FloatNePropagator::new(left_id, right_id);
                return ne.propagate(ctx);
            }
            _ => {}
        }

        if let Some(ext) = ctx.as_extended()
            && let (Some(left), Some(right)) =
                (ext.float_domain(left_id), ext.float_domain(right_id))
        {
            let left_fixed = (left.min - left.max).abs() <= f64::EPSILON && left.contains(left.min);
            let right_fixed =
                (right.min - right.max).abs() <= f64::EPSILON && right.contains(right.min);
            if left_fixed && right_fixed {
                if (left.min - right.min).abs() <= f64::EPSILON {
                    changed |= tighten_reif(ctx, reif_id, 1);
                } else {
                    changed |= tighten_reif(ctx, reif_id, 0);
                }
            } else if left.max < right.min || right.max < left.min {
                changed |= tighten_reif(ctx, reif_id, 0);
            } else {
                // Only overlapping IEEE point is excluded on either side ⇒ equality impossible.
                let overlap_lo = left.min.max(right.min);
                let overlap_hi = left.max.min(right.max);
                if (overlap_hi - overlap_lo).abs() <= f64::EPSILON
                    && (!left.contains(overlap_lo) || !right.contains(overlap_lo))
                {
                    changed |= tighten_reif(ctx, reif_id, 0);
                } else if let (Some(l_lo), Some(l_hi), Some(r_lo), Some(r_hi)) = (
                    min_admissible(&left),
                    max_admissible(&left),
                    min_admissible(&right),
                    max_admissible(&right),
                ) {
                    if (l_hi - l_lo).abs() <= f64::EPSILON
                        && (r_hi - r_lo).abs() <= f64::EPSILON
                        && (l_lo - r_lo).abs() <= f64::EPSILON
                    {
                        changed |= tighten_reif(ctx, reif_id, 1);
                    }
                }
            }
        }

        if ctx.domain(reif_id).is_empty()
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(left_id))
                .is_some_and(|d| d.is_empty())
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(right_id))
                .is_some_and(|d| d.is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[derive(Clone, Debug)]
pub struct FloatLeReifPropagator {
    watched: [VariableId; 3],
}

impl FloatLeReifPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for FloatLeReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let mut changed = false;

        match reif_literal(ctx, reif_id) {
            Some(1) => {
                let mut le = FloatLePropagator::new(left_id, right_id);
                return le.propagate(ctx);
            }
            Some(0) => {
                let Some(ext) = ctx.as_extended() else {
                    return PropagationStatus::OkNoChange;
                };
                let (Some(left), Some(right)) =
                    (ext.float_domain(left_id), ext.float_domain(right_id))
                else {
                    return PropagationStatus::Failure;
                };
                if le_inevitable(&left, &right) {
                    return PropagationStatus::Failure;
                }
                // ¬(left ≤ right) ⇒ left > right.max and right < left.min
                changed |= ext.tighten_float_below(left_id, next_up(right.max));
                let left_min = ext
                    .float_domain(left_id)
                    .map(|domain| domain.min)
                    .unwrap_or(left.min);
                changed |= ext.tighten_float_above(right_id, next_down(left_min));
            }
            _ => {}
        }

        if let Some(ext) = ctx.as_extended()
            && let (Some(left), Some(right)) =
                (ext.float_domain(left_id), ext.float_domain(right_id))
        {
            if le_inevitable(&left, &right) {
                changed |= tighten_reif(ctx, reif_id, 1);
            } else if le_impossible(&left, &right) {
                changed |= tighten_reif(ctx, reif_id, 0);
            }
        }

        if ctx.domain(reif_id).is_empty()
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(left_id))
                .is_some_and(|d| d.is_empty())
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(right_id))
                .is_some_and(|d| d.is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[derive(Clone, Debug)]
pub struct FloatLtReifPropagator {
    watched: [VariableId; 3],
}

impl FloatLtReifPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for FloatLtReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        let mut changed = false;

        match reif_literal(ctx, reif_id) {
            Some(1) => {
                let Some(ext) = ctx.as_extended() else {
                    return PropagationStatus::OkNoChange;
                };
                let (Some(left), Some(right)) =
                    (ext.float_domain(left_id), ext.float_domain(right_id))
                else {
                    return PropagationStatus::Failure;
                };
                if lt_impossible(&left, &right) {
                    return PropagationStatus::Failure;
                }
                changed |= ext.tighten_float_above(left_id, next_down(right.max));
                let left_max = ext
                    .float_domain(left_id)
                    .map(|domain| domain.max)
                    .unwrap_or(left.max);
                changed |= ext.tighten_float_below(right_id, next_up(left_max));
            }
            Some(0) => {
                let mut ge = FloatLePropagator::new(right_id, left_id);
                return ge.propagate(ctx);
            }
            _ => {}
        }

        if let Some(ext) = ctx.as_extended()
            && let (Some(left), Some(right)) =
                (ext.float_domain(left_id), ext.float_domain(right_id))
        {
            if lt_inevitable(&left, &right) {
                changed |= tighten_reif(ctx, reif_id, 1);
            } else if lt_impossible(&left, &right) {
                changed |= tighten_reif(ctx, reif_id, 0);
            }
        }

        if ctx.domain(reif_id).is_empty()
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(left_id))
                .is_some_and(|d| d.is_empty())
            || ctx
                .as_extended()
                .and_then(|ext| ext.float_domain(right_id))
                .is_some_and(|d| d.is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
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

fn min_admissible(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut v = snap.min;
    loop {
        if snap.contains(v) {
            return Some(v);
        }
        if v >= snap.max {
            break;
        }
        let next = next_up(v);
        if next <= v || next > snap.max {
            break;
        }
        v = next;
    }
    None
}

fn max_admissible(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut v = snap.max;
    loop {
        if snap.contains(v) {
            return Some(v);
        }
        if v <= snap.min {
            break;
        }
        let prev = next_down(v);
        if prev >= v || prev < snap.min {
            break;
        }
        v = prev;
    }
    None
}

fn le_inevitable(
    left: &propaga_core::FloatDomainSnapshot,
    right: &propaga_core::FloatDomainSnapshot,
) -> bool {
    matches!(
        (max_admissible(left), min_admissible(right)),
        (Some(l_max), Some(r_min)) if l_max <= r_min
    )
}

fn le_impossible(
    left: &propaga_core::FloatDomainSnapshot,
    right: &propaga_core::FloatDomainSnapshot,
) -> bool {
    matches!(
        (min_admissible(left), max_admissible(right)),
        (Some(l_min), Some(r_max)) if l_min > r_max
    )
}

fn lt_inevitable(
    left: &propaga_core::FloatDomainSnapshot,
    right: &propaga_core::FloatDomainSnapshot,
) -> bool {
    matches!(
        (max_admissible(left), min_admissible(right)),
        (Some(l_max), Some(r_min)) if l_max < r_min
    )
}

fn lt_impossible(
    left: &propaga_core::FloatDomainSnapshot,
    right: &propaga_core::FloatDomainSnapshot,
) -> bool {
    matches!(
        (min_admissible(left), max_admissible(right)),
        (Some(l_min), Some(r_max)) if l_min >= r_max
    )
}

fn near_integer(value: f64) -> Option<f64> {
    let n = value.round();
    ((value - n).abs() <= 1e-9).then_some(n)
}

/// Scan limit when walking integer images of ceil/floor/round outputs.
const MAX_INTEGER_IMAGE_SCAN: i64 = 10_000;

/// Least integer still admissible in `snap` (bounded end scan).
fn least_admissible_integer(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut k = snap.min.ceil();
    let end = snap.max.floor();
    let mut scanned = 0_i64;
    while k <= end + 1e-9 && scanned <= MAX_INTEGER_IMAGE_SCAN {
        if snap.contains(k) {
            return Some(k);
        }
        k += 1.0;
        scanned += 1;
    }
    None
}

/// Greatest integer still admissible in `snap` (bounded end scan).
fn greatest_admissible_integer(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    if snap.is_empty() {
        return None;
    }
    let mut m = snap.max.floor();
    let start = snap.min.ceil();
    let mut scanned = 0_i64;
    while m >= start - 1e-9 && scanned <= MAX_INTEGER_IMAGE_SCAN {
        if snap.contains(m) {
            return Some(m);
        }
        m -= 1.0;
        scanned += 1;
    }
    None
}

fn reverse_integer_image_domain(
    output_snap: &propaga_core::FloatDomainSnapshot,
    reverse_holes: &[f64],
) -> propaga_core::FloatDomainSnapshot {
    propaga_core::FloatDomainSnapshot {
        min: output_snap.min,
        max: output_snap.max,
        holes: reverse_holes.to_vec(),
    }
}

fn exclude_singleton_preimage(
    ext: &mut dyn ExtendedPropagationContext,
    var: VariableId,
    input: &FloatDomain,
    pre_lo: f64,
    pre_hi: f64,
) -> bool {
    let a = pre_lo.max(input.lower_bound());
    let b = pre_hi.min(input.upper_bound());
    if a > b {
        return false;
    }
    if (b - a).abs() <= f64::EPSILON && input.contains(a) {
        return ext.exclude_float_point(var, a);
    }
    false
}

fn fixed_float_image(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    snap.is_fixed().then_some(snap.min)
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

fn tighten_reif(ctx: &mut dyn PropagationContext, reif: VariableId, value: i32) -> bool {
    let mut changed = false;
    if ctx.remove_below(reif, value) {
        changed = true;
    }
    if ctx.remove_above(reif, value) {
        changed = true;
    }
    changed
}

#[derive(Clone, Copy, Debug)]
pub enum FloatBinaryOp {
    Plus,
    Div,
}

#[derive(Clone, Debug)]
pub struct FloatBinaryPropagator {
    watched: [VariableId; 3],
    op: FloatBinaryOp,
}

impl FloatBinaryPropagator {
    #[must_use]
    pub fn new(a: VariableId, b: VariableId, c: VariableId, op: FloatBinaryOp) -> Self {
        Self {
            watched: [a, b, c],
            op,
        }
    }
}

impl Propagator for FloatBinaryPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(a), Some(b), Some(c)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
            ext.float_domain(self.watched[2]),
        ) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        let a_dom = FloatDomain::from_bounds_with_holes(a.min, a.max, &a.holes);
        let b_dom = FloatDomain::from_bounds_with_holes(b.min, b.max, &b.holes);

        let result = match self.op {
            FloatBinaryOp::Plus => a_dom.plus(&b_dom),
            FloatBinaryOp::Div => a_dom.divide(&b_dom),
        };
        changed |= ext.tighten_float_below(self.watched[2], result.lower_bound());
        changed |= ext.tighten_float_above(self.watched[2], result.upper_bound());
        for hole in result.holes() {
            changed |= ext.exclude_float_point(self.watched[2], *hole);
        }

        if matches!(self.op, FloatBinaryOp::Plus) {
            let c_snap = ext
                .float_domain(self.watched[2])
                .unwrap_or_else(|| c.clone());
            // Prefer holes recorded before bound sync: tightening can move a hole to an
            // endpoint and drop it from the snapshot while it remains semantically forbidden.
            let mut reverse_holes = c.holes.clone();
            for hole in &c_snap.holes {
                if !reverse_holes
                    .iter()
                    .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
                {
                    reverse_holes.push(*hole);
                }
            }
            let c_interval =
                FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &reverse_holes);
            let a_from_c = c_interval.plus(&(-b_dom.clone()));
            changed |= ext.tighten_float_below(self.watched[0], a_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[0], a_from_c.upper_bound());
            for hole in a_from_c.holes() {
                changed |= ext.exclude_float_point(self.watched[0], *hole);
            }
            let b_from_c = c_interval.plus(&(-a_dom.clone()));
            changed |= ext.tighten_float_below(self.watched[1], b_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[1], b_from_c.upper_bound());
            for hole in b_from_c.holes() {
                changed |= ext.exclude_float_point(self.watched[1], *hole);
            }
        }

        if matches!(self.op, FloatBinaryOp::Div) {
            let c_snap = ext
                .float_domain(self.watched[2])
                .unwrap_or_else(|| c.clone());
            let b_snap = ext
                .float_domain(self.watched[1])
                .unwrap_or_else(|| b.clone());
            let a_snap = ext
                .float_domain(self.watched[0])
                .unwrap_or_else(|| a.clone());
            let mut reverse_holes = c.holes.clone();
            for hole in &c_snap.holes {
                if !reverse_holes
                    .iter()
                    .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
                {
                    reverse_holes.push(*hole);
                }
            }
            // c = a / b  ⇒  a = c * b and b = a / c (when 0 ∉ Dom(c))
            let a_from_cb =
                FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &reverse_holes).times(
                    &FloatDomain::from_bounds_with_holes(b_snap.min, b_snap.max, &b_snap.holes),
                );
            if a_from_cb.lower_bound().is_finite() {
                changed |= ext.tighten_float_below(self.watched[0], a_from_cb.lower_bound());
                changed |= ext.tighten_float_above(self.watched[0], a_from_cb.upper_bound());
                for hole in a_from_cb.holes() {
                    changed |= ext.exclude_float_point(self.watched[0], *hole);
                }
            }
            let b_from_ac =
                FloatDomain::from_bounds_with_holes(a_snap.min, a_snap.max, &a_snap.holes).divide(
                    &FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &reverse_holes),
                );
            if b_from_ac.lower_bound().is_finite() {
                changed |= ext.tighten_float_below(self.watched[1], b_from_ac.lower_bound());
                changed |= ext.tighten_float_above(self.watched[1], b_from_ac.upper_bound());
                for hole in b_from_ac.holes() {
                    changed |= ext.exclude_float_point(self.watched[1], *hole);
                }
            }
        }

        if ext
            .float_domain(self.watched[0])
            .is_some_and(|d| d.is_empty())
            || ext
                .float_domain(self.watched[1])
                .is_some_and(|d| d.is_empty())
            || ext
                .float_domain(self.watched[2])
                .is_some_and(|d| d.is_empty())
        {
            return PropagationStatus::Failure;
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FloatUnaryOp {
    Abs,
    Sqrt,
    Sin,
    Cos,
    Ceil,
    Floor,
    Round,
    Ln,
    Exp,
}

#[derive(Clone, Debug)]
pub struct FloatUnaryPropagator {
    watched: [VariableId; 2],
    op: FloatUnaryOp,
}

impl FloatUnaryPropagator {
    #[must_use]
    pub fn new(input: VariableId, output: VariableId, op: FloatUnaryOp) -> Self {
        Self {
            watched: [input, output],
            op,
        }
    }
}

impl Propagator for FloatUnaryPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (Some(input), Some(output)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let input_dom = FloatDomain::from_bounds_with_holes(input.min, input.max, &input.holes);
        let mapped = match self.op {
            FloatUnaryOp::Abs => input_dom.abs(),
            FloatUnaryOp::Sqrt => input_dom.sqrt(),
            FloatUnaryOp::Sin => input_dom.sin(),
            FloatUnaryOp::Cos => input_dom.cos(),
            FloatUnaryOp::Ceil => input_dom.ceil(),
            FloatUnaryOp::Floor => input_dom.floor(),
            FloatUnaryOp::Round => input_dom.round(),
            FloatUnaryOp::Ln => input_dom.ln(),
            FloatUnaryOp::Exp => input_dom.exp(),
        };
        let mut changed = false;
        changed |= ext.tighten_float_below(self.watched[1], mapped.lower_bound());
        changed |= ext.tighten_float_above(self.watched[1], mapped.upper_bound());
        for hole in mapped.holes() {
            changed |= ext.exclude_float_point(self.watched[1], *hole);
        }

        // Reverse-project output holes through locally invertible maps.
        // Prefer holes recorded before bound sync: tightening can move a hole to an
        // endpoint and drop it from the snapshot while it remains semantically forbidden.
        let output_snap = ext
            .float_domain(self.watched[1])
            .unwrap_or_else(|| output.clone());
        let mut reverse_holes = output.holes.clone();
        for hole in &output_snap.holes {
            if !reverse_holes
                .iter()
                .any(|existing| (*existing - hole).abs() <= f64::EPSILON)
            {
                reverse_holes.push(*hole);
            }
        }
        match self.op {
            FloatUnaryOp::Abs => {
                for hole in &reverse_holes {
                    if *hole > 0.0 {
                        changed |= ext.exclude_float_point(self.watched[0], *hole);
                        changed |= ext.exclude_float_point(self.watched[0], -hole);
                    }
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    if y >= 0.0 {
                        // abs⁻¹(y) = {-y, y}; tighten to the hull, then drop the opposite
                        // sign when the input cannot realize that preimage.
                        changed |= ext.tighten_float_below(self.watched[0], -y);
                        changed |= ext.tighten_float_above(self.watched[0], y);
                        let input_after = ext
                            .float_domain(self.watched[0])
                            .map(|snap| {
                                FloatDomain::from_bounds_with_holes(snap.min, snap.max, &snap.holes)
                            })
                            .unwrap_or_else(|| input_dom.clone());
                        let pos_ok = y == 0.0 || input_after.contains(y);
                        let neg_ok = y == 0.0 || input_after.contains(-y);
                        if pos_ok && !neg_ok {
                            changed |= ext.tighten_float_below(self.watched[0], y);
                        } else if neg_ok && !pos_ok {
                            changed |= ext.tighten_float_above(self.watched[0], -y);
                        } else if input_dom.lower_bound() >= 0.0 {
                            changed |= ext.tighten_float_below(self.watched[0], y);
                        } else if input_dom.upper_bound() <= 0.0 {
                            changed |= ext.tighten_float_above(self.watched[0], -y);
                        }
                    }
                }
            }
            FloatUnaryOp::Sqrt => {
                for hole in &reverse_holes {
                    if *hole >= 0.0 {
                        changed |= ext.exclude_float_point(self.watched[0], hole * hole);
                    }
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    if y >= 0.0 {
                        let x = y * y;
                        changed |= ext.tighten_float_below(self.watched[0], x);
                        changed |= ext.tighten_float_above(self.watched[0], x);
                    }
                }
            }
            FloatUnaryOp::Exp => {
                for hole in &reverse_holes {
                    if *hole > 0.0 {
                        changed |= ext.exclude_float_point(self.watched[0], hole.ln());
                    }
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    if y > 0.0 {
                        let x = y.ln();
                        changed |= ext.tighten_float_below(self.watched[0], x);
                        changed |= ext.tighten_float_above(self.watched[0], x);
                    }
                }
            }
            FloatUnaryOp::Ln => {
                for hole in &reverse_holes {
                    changed |= ext.exclude_float_point(self.watched[0], hole.exp());
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    let x = y.exp();
                    changed |= ext.tighten_float_below(self.watched[0], x);
                    changed |= ext.tighten_float_above(self.watched[0], x);
                }
            }
            FloatUnaryOp::Sin => {
                for hole in &reverse_holes {
                    if let Some(preimage) =
                        unique_sin_preimage(*hole, input_dom.lower_bound(), input_dom.upper_bound())
                    {
                        changed |= ext.exclude_float_point(self.watched[0], preimage);
                    }
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    if let Some(x) =
                        unique_sin_preimage(y, input_dom.lower_bound(), input_dom.upper_bound())
                    {
                        changed |= ext.tighten_float_below(self.watched[0], x);
                        changed |= ext.tighten_float_above(self.watched[0], x);
                    }
                }
            }
            FloatUnaryOp::Cos => {
                for hole in &reverse_holes {
                    if let Some(preimage) =
                        unique_cos_preimage(*hole, input_dom.lower_bound(), input_dom.upper_bound())
                    {
                        changed |= ext.exclude_float_point(self.watched[0], preimage);
                    }
                }
                if let Some(y) = fixed_float_image(&output_snap) {
                    if let Some(x) =
                        unique_cos_preimage(y, input_dom.lower_bound(), input_dom.upper_bound())
                    {
                        changed |= ext.tighten_float_below(self.watched[0], x);
                        changed |= ext.tighten_float_above(self.watched[0], x);
                    }
                }
            }
            FloatUnaryOp::Floor => {
                // floor(x) ∈ Dom(y) ⇒ x ∈ [least, greatest+1); also covers fixed images.
                let image = reverse_integer_image_domain(&output_snap, &reverse_holes);
                if let (Some(k), Some(m)) = (
                    least_admissible_integer(&image),
                    greatest_admissible_integer(&image),
                ) {
                    changed |= ext.tighten_float_below(self.watched[0], k);
                    changed |= ext.tighten_float_above(self.watched[0], next_down(m + 1.0));
                }
                for hole in &reverse_holes {
                    if let Some(n) = near_integer(*hole) {
                        changed |= exclude_singleton_preimage(
                            ext,
                            self.watched[0],
                            &input_dom,
                            n,
                            next_down(n + 1.0),
                        );
                    }
                }
            }
            FloatUnaryOp::Ceil => {
                // ceil(x) ∈ Dom(y) ⇒ x ∈ (least-1, greatest]; also covers fixed images.
                let image = reverse_integer_image_domain(&output_snap, &reverse_holes);
                if let (Some(k), Some(m)) = (
                    least_admissible_integer(&image),
                    greatest_admissible_integer(&image),
                ) {
                    changed |= ext.tighten_float_below(self.watched[0], next_up(k - 1.0));
                    changed |= ext.tighten_float_above(self.watched[0], m);
                }
                for hole in &reverse_holes {
                    if let Some(n) = near_integer(*hole) {
                        changed |= exclude_singleton_preimage(
                            ext,
                            self.watched[0],
                            &input_dom,
                            next_up(n - 1.0),
                            n,
                        );
                    }
                }
            }
            FloatUnaryOp::Round => {
                // round(x) ∈ Dom(y) ⇒ x between extreme half-away-from-zero preimages.
                let image = reverse_integer_image_domain(&output_snap, &reverse_holes);
                if let (Some(k), Some(m)) = (
                    least_admissible_integer(&image),
                    greatest_admissible_integer(&image),
                ) {
                    let (lo, _) = round_preimage_bounds(k);
                    let (_, hi) = round_preimage_bounds(m);
                    changed |= ext.tighten_float_below(self.watched[0], lo);
                    changed |= ext.tighten_float_above(self.watched[0], hi);
                }
                for hole in &reverse_holes {
                    if let Some(n) = near_integer(*hole) {
                        let (lo, hi) = round_preimage_bounds(n);
                        changed |=
                            exclude_singleton_preimage(ext, self.watched[0], &input_dom, lo, hi);
                    }
                }
            }
        }

        if ext
            .float_domain(self.watched[0])
            .is_some_and(|d| d.is_empty())
            || ext
                .float_domain(self.watched[1])
                .is_some_and(|d| d.is_empty())
        {
            return PropagationStatus::Failure;
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[derive(Clone, Debug)]
pub struct Int2FloatPropagator {
    watched: [VariableId; 2],
}

impl Int2FloatPropagator {
    #[must_use]
    pub fn new(int_var: VariableId, float_var: VariableId) -> Self {
        Self {
            watched: [int_var, float_var],
        }
    }
}

impl Propagator for Int2FloatPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let int_domain = ctx.domain(self.watched[0]);
        let (Some(min), Some(max)) = (int_domain.min(), int_domain.max()) else {
            return PropagationStatus::Failure;
        };
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let Some(float) = ext.float_domain(self.watched[1]) else {
            return PropagationStatus::Failure;
        };
        let mut changed = false;
        changed |= ext.tighten_float_below(self.watched[1], f64::from(min));
        changed |= ext.tighten_float_above(self.watched[1], f64::from(max));
        if let Some(float_after) = ext.float_domain(self.watched[1]) {
            if float_after.is_empty() {
                return PropagationStatus::Failure;
            }
            let fmin = float_after.min.ceil() as i32;
            let fmax = float_after.max.floor() as i32;
            changed |= ctx.remove_below(self.watched[0], fmin);
            changed |= ctx.remove_above(self.watched[0], fmax);
        } else if float.is_empty() {
            return PropagationStatus::Failure;
        }

        // Keep discrete holes aligned: int values without a float image, and float
        // integer points without a matching int value (bounded scan for large spans).
        changed |= sync_int2float_holes(ctx, self.watched[0], self.watched[1]);

        if ctx.domain(self.watched[0]).is_empty() {
            return PropagationStatus::Failure;
        }
        if ctx
            .as_extended()
            .and_then(|ext| ext.float_domain(self.watched[1]))
            .is_none_or(|d| d.is_empty())
        {
            return PropagationStatus::Failure;
        }
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

const MAX_INT2FLOAT_HOLE_SCAN: i64 = 10_000;
/// End-window size when the float span is too wide for a full int↔float hole scan.
const MAX_INT2FLOAT_END_SCAN: i64 = 1_000;

fn sync_int2float_holes(
    ctx: &mut dyn PropagationContext,
    int_var: VariableId,
    float_var: VariableId,
) -> bool {
    let Some(float) = ctx
        .as_extended()
        .and_then(|ext| ext.float_domain(float_var))
    else {
        return false;
    };
    let int_domain = ctx.domain(int_var);
    let (Some(imin), Some(imax)) = (int_domain.min(), int_domain.max()) else {
        return false;
    };

    let mut changed = false;

    // Explicit float holes are sparse: always map near-integer ones onto the int
    // domain, even when the int span is too wide for a full scan.
    let mut remove_from_int: Vec<i32> = Vec::new();
    for &hole in &float.holes {
        if let Some(n) = near_integer(hole)
            && (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&n)
        {
            let value = n as i32;
            if value >= imin && value <= imax && int_domain.contains(value) {
                remove_from_int.push(value);
            }
        }
    }
    // Cheap endpoint check: bound ints with no float image.
    for endpoint in [imin, imax] {
        if int_domain.contains(endpoint) && !float.contains(f64::from(endpoint)) {
            remove_from_int.push(endpoint);
        }
    }
    remove_from_int.sort_unstable();
    remove_from_int.dedup();
    for value in remove_from_int {
        changed |= ctx.remove_value(int_var, value);
    }

    let int_domain = ctx.domain(int_var);
    let (Some(imin), Some(imax)) = (int_domain.min(), int_domain.max()) else {
        return changed;
    };
    if i64::from(imax) - i64::from(imin) <= MAX_INT2FLOAT_HOLE_SCAN {
        let forbidden: Vec<i32> = (imin..=imax)
            .filter(|&value| int_domain.contains(value) && !float.contains(f64::from(value)))
            .collect();
        for value in forbidden {
            changed |= ctx.remove_value(int_var, value);
        }
    }

    let Some(float_after) = ctx
        .as_extended()
        .and_then(|ext| ext.float_domain(float_var))
    else {
        return changed;
    };
    let flo = float_after.min.ceil() as i32;
    let fhi = float_after.max.floor() as i32;
    if flo > fhi {
        return changed;
    }
    let span = i64::from(fhi) - i64::from(flo);
    let missing: Vec<f64> = {
        let int_after = ctx.domain(int_var);
        if span <= MAX_INT2FLOAT_HOLE_SCAN {
            (flo..=fhi)
                .filter(|&value| !int_after.contains(value))
                .map(f64::from)
                .collect()
        } else {
            // Wide float span: still punch holes for missing ints near each endpoint.
            let lo_hi = flo.saturating_add(MAX_INT2FLOAT_END_SCAN as i32);
            let hi_lo = fhi.saturating_sub(MAX_INT2FLOAT_END_SCAN as i32);
            let mut missing = Vec::new();
            for value in flo..=lo_hi.min(fhi) {
                if !int_after.contains(value) {
                    missing.push(f64::from(value));
                }
            }
            let start = hi_lo.max(flo).max(lo_hi.saturating_add(1));
            for value in start..=fhi {
                if !int_after.contains(value) {
                    missing.push(f64::from(value));
                }
            }
            missing
        }
    };
    let Some(ext) = ctx.as_extended() else {
        return changed;
    };
    for hole in missing {
        changed |= ext.exclude_float_point(float_var, hole);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::{AnyDomain, HybridDomain, IntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn int2float_punches_float_holes_for_missing_integers() {
        let mut engine = Engine::new();
        let i = engine.new_variable(IntervalDomain::new(1, 3).remove(2));
        let f = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        engine.add_propagator(Box::new(Int2FloatPropagator::new(i, f)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(f).as_float().unwrap();
        assert!(!domain.contains(2.0));
        assert!((domain.lower_bound() - 1.0).abs() < 1e-9);
        assert!((domain.upper_bound() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn int2float_removes_int_values_forbidden_by_float_holes() {
        let mut engine = Engine::new();
        let i = engine.new_variable(IntervalDomain::new(1, 3));
        let f = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0).exclude(2.0)));
        engine.add_propagator(Box::new(Int2FloatPropagator::new(i, f)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.hybrid_domain(i).contains(2));
        assert!(engine.hybrid_domain(i).contains(1));
        assert!(engine.hybrid_domain(i).contains(3));
    }

    #[test]
    fn int2float_maps_float_holes_even_on_wide_int_span() {
        let mut engine = Engine::new();
        // Span exceeds MAX_INT2FLOAT_HOLE_SCAN; explicit hole must still remove 5.
        let i = engine.new_variable(IntervalDomain::new(0, 20_000));
        let f = engine.new_variable(AnyDomain::Float(
            FloatDomain::new(0.0, 20_000.0).exclude(5.0),
        ));
        engine.add_propagator(Box::new(Int2FloatPropagator::new(i, f)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.hybrid_domain(i).contains(5));
        assert!(engine.hybrid_domain(i).contains(0));
        assert!(engine.hybrid_domain(i).contains(20_000));
    }

    #[test]
    fn int2float_punches_endpoint_float_holes_on_wide_span() {
        let mut engine = Engine::new();
        // Wide float span: missing int at the upper endpoint still punches a float hole.
        let i = engine.new_variable(IntervalDomain::new(0, 19_999));
        let f = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 20_000.0)));
        engine.add_propagator(Box::new(Int2FloatPropagator::new(i, f)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(f).as_float().unwrap().contains(20_000.0));
    }

    #[test]
    fn float_floor_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Floor,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 2.0).abs() < 1e-9);
        assert!(domain.upper_bound() < 3.0);
    }

    #[test]
    fn float_ceil_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Ceil,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
        assert!((domain.upper_bound() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn float_ceil_reverse_projects_unfixed_output_bounds() {
        let mut engine = Engine::new();
        // ceil(x) ∈ [3, 4] ⇒ x > 2 and x ≤ 4.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.5, 3.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(3.0, 4.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Ceil,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.lower_bound() > 2.0);
        assert!((domain.upper_bound() - 3.5).abs() < 1e-9);
    }

    #[test]
    fn float_floor_reverse_projects_unfixed_output_bounds() {
        let mut engine = Engine::new();
        // floor(x) ∈ [1, 2] ⇒ x ≥ 1 and x < 3.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.5, 3.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Floor,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 1.5).abs() < 1e-9);
        assert!(domain.upper_bound() < 3.0);
    }

    #[test]
    fn float_ceil_reverse_projects_bounds_after_image_hole() {
        let mut engine = Engine::new();
        // y ∈ [2, 4] \ {2} forces least image 3 ⇒ x > 2.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.5, 3.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 4.0).exclude(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Ceil,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.lower_bound() > 2.0);
    }

    #[test]
    fn float_round_reverse_projects_unfixed_output_bounds() {
        let mut engine = Engine::new();
        // round(x) ∈ [1, 2] ⇒ x ≥ 0.5 and x < 2.5.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 3.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Round,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 0.5).abs() < 1e-9);
        assert!(domain.upper_bound() < 2.5);
    }

    #[test]
    fn float_ceil_reverse_projects_singleton_preimage_hole() {
        let mut engine = Engine::new();
        // Interior hole at 2 on a wide output; ceil⁻¹(2) ∩ [2, 2.5] = {2}.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 2.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 4.0).exclude(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Ceil,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(
            !domain.contains(2.0),
            "singleton ceil preimage of forbidden image 2 should be excluded"
        );
        assert!(domain.lower_bound() > 2.0);
    }

    #[test]
    fn float_round_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Round,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 0.5).abs() < 1e-9);
        assert!(domain.upper_bound() < 1.5);
    }

    #[test]
    fn float_exp_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(-2.0, 2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0_f64.exp())));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Exp)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 1.0).abs() < 1e-9);
        assert!((domain.upper_bound() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn float_ln_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.1, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(0.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Ln)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 1.0).abs() < 1e-9);
        assert!((domain.upper_bound() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn float_sqrt_reverse_projects_fixed_image() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 20.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(3.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Sqrt,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 9.0).abs() < 1e-9);
        assert!((domain.upper_bound() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn float_plus_projects_holes_when_addend_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0).exclude(1.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::fix(3.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        engine.add_propagator(Box::new(FloatBinaryPropagator::new(
            a,
            b,
            c,
            FloatBinaryOp::Plus,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(c).as_float().unwrap().contains(4.0));
    }

    #[test]
    fn float_plus_reverse_projects_result_hole_when_addend_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0).exclude(3.0)));
        engine.add_propagator(Box::new(FloatBinaryPropagator::new(
            a,
            b,
            c,
            FloatBinaryOp::Plus,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(a).as_float().unwrap().contains(2.0));
    }

    #[test]
    fn float_div_reverse_projects_holes_when_dividend_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::fix(6.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.5, 10.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 10.0).exclude(2.0)));
        engine.add_propagator(Box::new(FloatBinaryPropagator::new(
            a,
            b,
            c,
            FloatBinaryOp::Div,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(b).as_float().unwrap().contains(3.0));
    }

    #[test]
    fn float_div_reverse_projects_holes_when_quotient_is_fixed() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0).exclude(4.0)));
        let b = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.5, 10.0)));
        let c = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatBinaryPropagator::new(
            a,
            b,
            c,
            FloatBinaryOp::Div,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(b).as_float().unwrap().contains(2.0));
        assert!(!engine.domain(a).as_float().unwrap().contains(4.0));
    }

    #[test]
    fn float_sqrt_projects_holes_forward_and_back() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 9.0).exclude(4.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Sqrt,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(y).as_float().unwrap().contains(2.0));

        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 9.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0).exclude(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(
            x,
            y,
            FloatUnaryOp::Sqrt,
        )));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(x).as_float().unwrap().contains(4.0));
    }

    #[test]
    fn float_abs_reverse_projects_holes_across_zero() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(-3.0, 3.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 3.0).exclude(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Abs)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(!domain.contains(2.0));
        assert!(!domain.contains(-2.0));
    }

    #[test]
    fn float_abs_reverse_projects_fixed_image_on_nonnegative_input() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 5.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Abs)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - 2.0).abs() < 1e-9);
        assert!((domain.upper_bound() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn float_abs_fixed_image_forces_unique_preimage_when_other_side_is_hole() {
        let mut engine = Engine::new();
        // y = 2 and x cannot be +2 ⇒ x must be -2.
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(-3.0, 3.0).exclude(2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(2.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Abs)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!(domain.is_fixed());
        assert!((domain.lower_bound() + 2.0).abs() < 1e-9);
    }

    #[test]
    fn float_sin_projects_holes_on_monotonic_domain() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(0.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(-1.0, 1.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Sin)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(y).as_float().unwrap().contains(0.5_f64.sin()));
    }

    #[test]
    fn float_sin_reverse_projects_fixed_image_on_monotonic_domain() {
        let mut engine = Engine::new();
        let target = std::f64::consts::FRAC_PI_4;
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(
            0.0,
            std::f64::consts::FRAC_PI_2,
        )));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(target.sin())));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Sin)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - target).abs() < 1e-9);
        assert!((domain.upper_bound() - target).abs() < 1e-9);
    }

    #[test]
    fn float_cos_reverse_projects_fixed_image_on_monotonic_domain() {
        let mut engine = Engine::new();
        let target = std::f64::consts::FRAC_PI_4;
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(
            0.0,
            std::f64::consts::FRAC_PI_2,
        )));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::fix(target.cos())));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Cos)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(x).as_float().unwrap();
        assert!((domain.lower_bound() - target).abs() < 1e-9);
        assert!((domain.upper_bound() - target).abs() < 1e-9);
    }

    #[test]
    fn float_le_reif_false_forces_strict_greater() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatLeReifPropagator::new(left, right, reif)));
        let status = engine.fix_variable(reif, 0).unwrap();
        assert!(!status.is_failure());
        let domain = engine.domain(left).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn float_le_reif_infers_false_when_left_fixed_above_holed_right_max() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(1.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatLeReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn float_le_reif_infers_true_when_left_cannot_exceed_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatLeReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn float_le_reif_false_fails_when_le_inevitable() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(0.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0)));
        let reif = engine.new_variable(HybridDomain::fix(0));
        engine.add_propagator(Box::new(FloatLeReifPropagator::new(left, right, reif)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn float_lt_reif_false_forces_geq() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let reif = engine.new_variable(HybridDomain::fix(0));
        engine.add_propagator(Box::new(FloatLtReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(left).as_float().unwrap();
        assert!(domain.lower_bound() >= 1.0);
    }

    #[test]
    fn float_lt_reif_infers_true_when_right_holed_above_left() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0).exclude(1.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatLtReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn float_eq_reif_false_prunes_endpoint() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::fix(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 3.0)));
        let reif = engine.new_variable(HybridDomain::fix(0));
        engine.add_propagator(Box::new(FloatEqReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let domain = engine.domain(right).as_float().unwrap();
        assert!(domain.lower_bound() > 1.0);
    }

    #[test]
    fn float_eq_reif_infers_false_when_disjoint() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(2.0, 3.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatEqReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn float_eq_reif_infers_false_when_singleton_overlap_is_a_hole() {
        let mut engine = Engine::new();
        // Overlap is only {1.0}, excluded on left ⇒ equality impossible.
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(1.0)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(1.0, 2.0)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatEqReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn float_eq_reif_infers_true_when_both_sides_have_same_single_admissible_point() {
        let mut engine = Engine::new();
        // Use a large magnitude so adjacent IEEE points are farther than f64::EPSILON
        // (FloatDomainSnapshot::contains uses an EPSILON-tolerance hole check).
        let x = 1e20_f64;
        let y = next_up(x);
        // Both domains look like a wide interval [x, y], but y is a hole boundary.
        // So the only admissible point is x on both sides.
        let left = engine.new_variable(AnyDomain::Float(FloatDomain::new(x, y).exclude(y)));
        let right = engine.new_variable(AnyDomain::Float(FloatDomain::new(x, y).exclude(y)));
        let reif = engine.new_variable(HybridDomain::new(0, 1));
        engine.add_propagator(Box::new(FloatEqReifPropagator::new(left, right, reif)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }
}
