use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};
use propaga_domains::FloatDomain;

use super::float_eq::FloatEqPropagator;
use super::float_le::FloatLePropagator;
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

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id, reif_id) = (self.watched[0], self.watched[1], self.watched[2]);
        if ctx.fixed_value(reif_id) == Some(1) {
            let mut eq = FloatEqPropagator::new(left_id, right_id);
            return eq.propagate(ctx);
        }
        if ctx.fixed_value(reif_id) == Some(0) {
            let left = ctx.as_extended().and_then(|ext| ext.float_domain(left_id));
            let right = ctx.as_extended().and_then(|ext| ext.float_domain(right_id));
            if let (Some(left), Some(right)) = (left, right) {
                if left.min == left.max
                    && right.min == right.max
                    && (left.min - right.min).abs() < f64::EPSILON
                {
                    return PropagationStatus::Failure;
                }
            }
        }
        PropagationStatus::OkNoChange
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
                changed |= ext.tighten_float_above(left_id, next_up(right.max));
                changed |= ext.tighten_float_below(right_id, next_down(left.min));
            }
            _ => {}
        }

        if let Some(ext) = ctx.as_extended() {
            if let (Some(left), Some(right)) =
                (ext.float_domain(left_id), ext.float_domain(right_id))
            {
                if left.max <= right.min {
                    changed |= tighten_reif(ctx, reif_id, 1);
                } else if left.min > right.max {
                    changed |= tighten_reif(ctx, reif_id, 0);
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
        let a_dom = FloatDomain::new(a.min, a.max);
        let b_dom = FloatDomain::new(b.min, b.max);

        let result = match self.op {
            FloatBinaryOp::Plus => a_dom.plus(b_dom),
            FloatBinaryOp::Div => a_dom.divide(b_dom),
        };
        changed |= ext.tighten_float_below(self.watched[2], result.lower_bound());
        changed |= ext.tighten_float_above(self.watched[2], result.upper_bound());

        if matches!(self.op, FloatBinaryOp::Plus) {
            let c_snap = ext.float_domain(self.watched[2]).unwrap_or(c);
            let c_interval = FloatDomain::new(c_snap.min, c_snap.max);
            let a_from_c = c_interval.plus(b_dom.neg());
            changed |= ext.tighten_float_below(self.watched[0], a_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[0], a_from_c.upper_bound());
            let b_from_c = c_interval.plus(a_dom.neg());
            changed |= ext.tighten_float_below(self.watched[1], b_from_c.lower_bound());
            changed |= ext.tighten_float_above(self.watched[1], b_from_c.upper_bound());
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
        let (Some(input), Some(_output)) = (
            ext.float_domain(self.watched[0]),
            ext.float_domain(self.watched[1]),
        ) else {
            return PropagationStatus::Failure;
        };
        let input_dom = FloatDomain::new(input.min, input.max);
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
        if ext
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
