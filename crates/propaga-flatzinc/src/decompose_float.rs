use propaga_core::VariableId;
use propaga_model::Model;

pub fn float_plus(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.float_plus(a, b, c);
}

pub fn float_div(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.float_div(a, b, c);
}

pub fn float_abs(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Abs);
}

pub fn float_sqrt(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Sqrt);
}

pub fn float_sin(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Sin);
}

pub fn float_cos(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Cos);
}

pub fn float_ln(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Ln);
}

pub fn float_log2(model: &mut Model, a: VariableId, b: VariableId) {
    let ln_a = model.float_var_aux(f64::NEG_INFINITY, f64::INFINITY);
    float_ln(model, a, ln_a);
    let ln2 = model.float_var_aux(std::f64::consts::LN_2, std::f64::consts::LN_2);
    model.float_div(ln_a, ln2, b);
}

pub fn float_exp(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Exp);
}

pub fn float_ceil(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Ceil);
}

pub fn float_floor(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Floor);
}

pub fn float_round(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_unary(a, b, propaga_propagators::FloatUnaryOp::Round);
}

pub fn float_lt(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_lt(a, b);
}

pub fn float_ne(model: &mut Model, a: VariableId, b: VariableId) {
    model.float_ne(a, b);
}

pub fn float_max(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.float_max(a, b, c);
}

pub fn float_min(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.float_min(a, b, c);
}

pub fn int2float(model: &mut Model, int_var: VariableId, float_var: VariableId) {
    model.int2float(int_var, float_var);
}

pub fn float_lin_le(model: &mut Model, coeffs: &[f64], vars: &[VariableId], rhs: f64) {
    model.float_scalar_le(coeffs.to_vec(), vars.to_vec(), rhs);
}

pub fn float_lin_ge(model: &mut Model, coeffs: &[f64], vars: &[VariableId], rhs: f64) {
    model.float_scalar_ge(coeffs.to_vec(), vars.to_vec(), rhs);
}

pub fn float_lin_eq(model: &mut Model, coeffs: &[f64], vars: &[VariableId], rhs: f64) {
    model.float_scalar_eq(coeffs.to_vec(), vars.to_vec(), rhs);
}

pub fn float_lin_ne(model: &mut Model, coeffs: &[f64], vars: &[VariableId], rhs: f64) {
    model.float_scalar_ne(coeffs.to_vec(), vars.to_vec(), rhs);
}

pub fn float_lin_le_reif(
    model: &mut Model,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
    reif: VariableId,
) {
    model.reified_float_scalar_le(coeffs.to_vec(), vars.to_vec(), rhs, reif);
}

pub fn float_lin_ge_reif(
    model: &mut Model,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
    reif: VariableId,
) {
    model.reified_float_scalar_ge(coeffs.to_vec(), vars.to_vec(), rhs, reif);
}

pub fn float_lin_eq_reif(
    model: &mut Model,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
    reif: VariableId,
) {
    model.reified_float_scalar_eq(coeffs.to_vec(), vars.to_vec(), rhs, reif);
}

pub fn float_eq_reif(model: &mut Model, a: VariableId, b: VariableId, reif: VariableId) {
    model.float_eq_reif(a, b, reif);
}

pub fn float_le_reif(model: &mut Model, a: VariableId, b: VariableId, reif: VariableId) {
    model.float_le_reif(a, b, reif);
}

pub fn float_lt_reif(model: &mut Model, a: VariableId, b: VariableId, reif: VariableId) {
    let le_reif = model.int_var_aux(0, 1);
    model.float_le_reif(a, b, le_reif);
    let eq_reif = model.int_var_aux(0, 1);
    model.float_eq_reif(a, b, eq_reif);
    let not_eq = model.int_var_aux(0, 1);
    crate::decompose::bool_not(model, eq_reif, not_eq);
    crate::decompose::bool_and(model, le_reif, not_eq, reif);
}

pub fn float_ne_reif(model: &mut Model, a: VariableId, b: VariableId, reif: VariableId) {
    let eq_reif = model.int_var_aux(0, 1);
    model.float_eq_reif(a, b, eq_reif);
    crate::decompose::bool_not(model, eq_reif, reif);
}

#[allow(dead_code)]
pub fn float_lin_ne_reif(
    model: &mut Model,
    coeffs: &[f64],
    vars: &[VariableId],
    rhs: f64,
    reif: VariableId,
) {
    let eq_reif = model.int_var_aux(0, 1);
    model.reified_float_scalar_eq(coeffs.to_vec(), vars.to_vec(), rhs, eq_reif);
    crate::decompose::bool_not(model, eq_reif, reif);
}

/// Posts `lo <= x <= hi`.
pub fn float_in(model: &mut Model, x: VariableId, lo: f64, hi: f64) {
    let lo_var = model.float_var_aux(lo, lo);
    let hi_var = model.float_var_aux(hi, hi);
    model.float_le(lo_var, x);
    model.float_le(x, hi_var);
}

/// Posts `reif <=> lo <= x <= hi`.
pub fn float_in_reif(model: &mut Model, x: VariableId, lo: f64, hi: f64, reif: VariableId) {
    let lo_var = model.float_var_aux(lo, lo);
    let hi_var = model.float_var_aux(hi, hi);
    let lower = model.int_var_aux(0, 1);
    let upper = model.int_var_aux(0, 1);
    model.float_le_reif(lo_var, x, lower);
    model.float_le_reif(x, hi_var, upper);
    crate::decompose::bool_and(model, lower, upper, reif);
}

/// Posts `x` belonging to interval ranges or discrete values in `as`.
pub fn float_dom(model: &mut Model, x: VariableId, values: &[f64]) {
    if values.is_empty() {
        let empty = model.float_var_aux(1.0, 0.0);
        model.float_eq(x, empty);
        return;
    }

    if values.len() >= 2 && values.len().is_multiple_of(2) {
        let mut reifs = Vec::with_capacity(values.len() / 2);
        for chunk in values.chunks(2) {
            let reif = model.int_var_aux(0, 1);
            float_in_reif(model, x, chunk[0], chunk[1], reif);
            reifs.push(reif);
        }
        model.scalar_ge(vec![1; reifs.len()], reifs, 1);
        return;
    }

    let mut reifs = Vec::with_capacity(values.len());
    for &value in values {
        let fixed = model.float_var_aux(value, value);
        let reif = model.int_var_aux(0, 1);
        model.float_eq_reif(x, fixed, reif);
        reifs.push(reif);
    }
    model.scalar_ge(vec![1; reifs.len()], reifs, 1);
}

/// Posts `value = array[index]` for a float array (0-based index).
pub fn array_var_float_element(
    model: &mut Model,
    array: &[VariableId],
    index: VariableId,
    value: VariableId,
) {
    model.float_element(index, array.to_vec(), value);
}

/// Posts `m = max(xs)` for float variables.
pub fn array_float_maximum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    match xs {
        [] => {}
        [only] => model.float_eq(*only, m),
        [first, rest @ ..] => {
            let mut running = *first;
            for (i, &x) in rest.iter().enumerate() {
                let next = if i + 1 == rest.len() {
                    m
                } else {
                    model.float_var_aux(f64::NEG_INFINITY, f64::INFINITY)
                };
                model.float_max(running, x, next);
                running = next;
            }
        }
    }
}

/// Posts `m = min(xs)` for float variables.
pub fn array_float_minimum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    match xs {
        [] => {}
        [only] => model.float_eq(*only, m),
        [first, rest @ ..] => {
            let mut running = *first;
            for (i, &x) in rest.iter().enumerate() {
                let next = if i + 1 == rest.len() {
                    m
                } else {
                    model.float_var_aux(f64::NEG_INFINITY, f64::INFINITY)
                };
                model.float_min(running, x, next);
                running = next;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_lin_le_posts_constraint() {
        let mut model = Model::new();
        let x = model.float_var(0.0, 1.0);
        let y = model.float_var(0.0, 1.0);
        float_lin_le(&mut model, &[1.0, 1.0], &[x, y], 1.5);
        model.propagate().unwrap();
        let (solution, _) = model.solve_subset_with_stats([x, y]);
        assert!(solution.is_some());
    }
}
