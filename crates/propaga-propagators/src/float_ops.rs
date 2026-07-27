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
            let left_fixed = (left.min - left.max).abs() <= f64::EPSILON;
            let right_fixed = (right.min - right.max).abs() <= f64::EPSILON;
            if left_fixed && right_fixed {
                if (left.min - right.min).abs() <= f64::EPSILON {
                    changed |= tighten_reif(ctx, reif_id, 1);
                } else {
                    changed |= tighten_reif(ctx, reif_id, 0);
                }
            } else if left.max < right.min || right.max < left.min {
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
                if left.max <= right.min {
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
            if left.max <= right.min {
                changed |= tighten_reif(ctx, reif_id, 1);
            } else if left.min > right.max {
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

fn fixed_integer_image(snap: &propaga_core::FloatDomainSnapshot) -> Option<f64> {
    if !snap.is_fixed() {
        return None;
    }
    near_integer(snap.min)
}

fn near_integer(value: f64) -> Option<f64> {
    let n = value.round();
    ((value - n).abs() <= 1e-9).then_some(n)
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
            let c_interval =
                FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &c_snap.holes);
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
            // c = a / b  ⇒  a = c * b when b is fixed and nonzero
            if b_snap.is_fixed() && b_snap.min != 0.0 {
                let c_dom =
                    FloatDomain::from_bounds_with_holes(c_snap.min, c_snap.max, &c_snap.holes);
                let a_from_c = c_dom.times(&FloatDomain::fix(b_snap.min));
                changed |= ext.tighten_float_below(self.watched[0], a_from_c.lower_bound());
                changed |= ext.tighten_float_above(self.watched[0], a_from_c.upper_bound());
                for hole in a_from_c.holes() {
                    changed |= ext.exclude_float_point(self.watched[0], *hole);
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
                        // sign when the input cannot reach it.
                        changed |= ext.tighten_float_below(self.watched[0], -y);
                        changed |= ext.tighten_float_above(self.watched[0], y);
                        if input_dom.lower_bound() >= 0.0 {
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
            }
            FloatUnaryOp::Cos => {
                for hole in &reverse_holes {
                    if let Some(preimage) =
                        unique_cos_preimage(*hole, input_dom.lower_bound(), input_dom.upper_bound())
                    {
                        changed |= ext.exclude_float_point(self.watched[0], preimage);
                    }
                }
            }
            FloatUnaryOp::Floor => {
                if let Some(n) = fixed_integer_image(&output_snap) {
                    // floor⁻¹(n) = [n, n+1)
                    changed |= ext.tighten_float_below(self.watched[0], n);
                    changed |= ext.tighten_float_above(self.watched[0], next_down(n + 1.0));
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
                if let Some(n) = fixed_integer_image(&output_snap) {
                    // ceil⁻¹(n) = (n-1, n]
                    changed |= ext.tighten_float_below(self.watched[0], next_up(n - 1.0));
                    changed |= ext.tighten_float_above(self.watched[0], n);
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
                if let Some(n) = fixed_integer_image(&output_snap) {
                    // Rust round is half-away-from-zero.
                    let (lo, hi) = round_preimage_bounds(n);
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
        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::{AnyDomain, HybridDomain};
    use propaga_engine::Engine;

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
    fn float_sin_projects_holes_on_monotonic_domain() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 1.0).exclude(0.5)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(-1.0, 1.0)));
        engine.add_propagator(Box::new(FloatUnaryPropagator::new(x, y, FloatUnaryOp::Sin)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(!engine.domain(y).as_float().unwrap().contains(0.5_f64.sin()));
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
}
