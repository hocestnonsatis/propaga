use propaga_core::{DomainView, VariableId};
use propaga_model::Model;

const MAX_TABLE_TUPLES: usize = 10_000;

fn domain_range(model: &Model, var: VariableId) -> (i32, i32) {
    let domain = model.engine().hybrid_domain(var);
    (domain.min().unwrap(), domain.max().unwrap())
}

fn table_too_large(count: usize) -> bool {
    count > MAX_TABLE_TUPLES
}

/// Returns whether any of the variables has a float domain.
pub fn uses_float_domain(model: &Model, vars: &[VariableId]) -> bool {
    vars.iter()
        .any(|var| model.engine().domain(*var).as_float().is_some())
}

/// Posts `c = min(a, b)` for integer or float variables.
pub fn generic_min(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    if uses_float_domain(model, &[a, b, c]) {
        crate::decompose_float::float_min(model, a, b, c);
    } else {
        int_min(model, a, b, c);
    }
}

/// Posts `c = max(a, b)` for integer or float variables.
pub fn generic_max(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    if uses_float_domain(model, &[a, b, c]) {
        crate::decompose_float::float_max(model, a, b, c);
    } else {
        int_max(model, a, b, c);
    }
}

/// Posts `c = min(a, b)` with bound-consistent propagation.
pub fn int_min(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.int_min(a, b, c);
}

/// Posts `c = max(a, b)` with bound-consistent propagation.
pub fn int_max(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.int_max(a, b, c);
}

/// Posts `result = base ** exp` using a domain table.
pub fn int_pow(
    model: &mut Model,
    base: VariableId,
    exp: VariableId,
    result: VariableId,
) -> Result<(), String> {
    let (bmin, bmax) = domain_range(model, base);
    let (emin, emax) = domain_range(model, exp);
    let mut tuples = Vec::new();
    for b in bmin..=bmax {
        for e in emin..=emax {
            let value = b.pow(e.max(0) as u32);
            tuples.push(vec![b, e, value]);
        }
    }
    if table_too_large(tuples.len()) {
        return Err("int_pow domain too large".to_string());
    }
    model.table(vec![base, exp, result], tuples);
    Ok(())
}

/// Posts `result = base ** exp_const` via a multiply chain (no domain table).
pub fn int_pow_fixed(
    model: &mut Model,
    base: VariableId,
    exp: i32,
    result: VariableId,
) -> Result<(), String> {
    if exp < 0 {
        return Err("int_pow_fixed does not support negative exponents".to_string());
    }
    if exp == 0 {
        let one = model.int_var_fixed(1);
        model.equal(result, one);
        return Ok(());
    }
    if exp == 1 {
        model.equal(base, result);
        return Ok(());
    }

    let mut acc = base;
    for step in 1..exp {
        let next = if step + 1 == exp {
            result
        } else {
            model.int_var_aux(i32::MIN / 4, i32::MAX / 4)
        };
        model.int_times(acc, base, next);
        acc = next;
    }
    Ok(())
}

/// Posts `m = max(xs)`.
pub fn array_int_maximum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    match xs {
        [] => {}
        [only] => model.equal(*only, m),
        [first, rest @ ..] => {
            let mut running = *first;
            for (i, &x) in rest.iter().enumerate() {
                let next = if i + 1 == rest.len() {
                    m
                } else {
                    model.int_var_aux(i32::MIN / 4, i32::MAX / 4)
                };
                model.int_max(running, x, next);
                running = next;
            }
        }
    }
}

/// Posts `m = min(xs)`.
pub fn array_int_minimum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    match xs {
        [] => {}
        [only] => model.equal(*only, m),
        [first, rest @ ..] => {
            let mut running = *first;
            for (i, &x) in rest.iter().enumerate() {
                let next = if i + 1 == rest.len() {
                    m
                } else {
                    model.int_var_aux(i32::MIN / 4, i32::MAX / 4)
                };
                model.int_min(running, x, next);
                running = next;
            }
        }
    }
}

/// Posts `b = |a|` with bound-consistent propagation.
pub fn int_abs(model: &mut Model, a: VariableId, b: VariableId) {
    model.int_abs(a, b);
}

/// Posts `c = a * b` with bound-consistent propagation.
pub fn int_times(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    model.int_times(a, b, c);
    Ok(())
}

/// Posts `c = a / b` (trunc toward zero) with bound-consistent propagation.
pub fn int_div(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    model.int_div(a, b, c);
    Ok(())
}

/// Posts `c = a mod b` with bound-consistent propagation.
pub fn int_mod(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    model.int_mod(a, b, c);
    Ok(())
}

/// Posts `c = a + b` via linear equality.
pub fn int_plus(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.linear_eq(a, b, c);
}

/// Posts `b = not a` for 0/1 variables (`a + b = 1`).
pub fn bool_not(model: &mut Model, a: VariableId, b: VariableId) {
    model.scalar_eq(vec![1, 1], vec![a, b], 1);
}

/// Posts `c = a xor b` for 0/1 variables (`c <=> a != b`).
pub fn bool_xor(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let eq = model.int_var_aux(0, 1);
    model.reified_equal(a, b, eq);
    model.scalar_eq(vec![1, 1], vec![eq, c], 1);
}

/// Posts at-least-one clause over 0/1 literals.
pub fn bool_clause(model: &mut Model, literals: &[VariableId]) {
    if literals.is_empty() {
        return;
    }
    model.scalar_ge(vec![1; literals.len()], literals.to_vec(), 1);
}

/// Posts `reif <=> bool_clause(literals)`.
pub fn bool_clause_reif(model: &mut Model, literals: &[VariableId], reif: VariableId) {
    if literals.is_empty() {
        let one = model.int_var_fixed(1);
        model.equal(reif, one);
        return;
    }
    model.reified_scalar_ge(vec![1; literals.len()], literals.to_vec(), 1, reif);
}

/// Posts `c = /\ xs` for 0/1 variables.
pub fn array_bool_and(model: &mut Model, xs: &[VariableId], c: VariableId) {
    if xs.is_empty() {
        let one = model.int_var_fixed(1);
        model.equal(c, one);
        return;
    }
    for &x in xs {
        model.less_equal(c, x);
    }
    let n = xs.len() as i32;
    model.reified_scalar_ge(vec![1; xs.len()], xs.to_vec(), n, c);
}

/// Posts `c = xor(xs)` for 0/1 variables.
pub fn array_bool_xor(model: &mut Model, xs: &[VariableId], c: VariableId) {
    if xs.is_empty() {
        let zero = model.int_var_fixed(0);
        model.equal(c, zero);
        return;
    }
    let mut acc = xs[0];
    for &x in xs.iter().skip(1) {
        let next = model.int_var_aux(0, 1);
        bool_xor(model, acc, x, next);
        acc = next;
    }
    model.equal(acc, c);
}

/// Posts `c = a /\ b` for 0/1 variables.
pub fn bool_and(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.less_equal(c, a);
    model.less_equal(c, b);
    // c >= a + b - 1  ⇔  a + b - c <= 1
    model.scalar_le(vec![1, 1, -1], vec![a, b, c], 1);
}

/// Posts `c = a \/ b` for 0/1 variables.
pub fn bool_or(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.less_equal(a, c);
    model.less_equal(b, c);
    // c <= a + b  ⇔  a + b - c >= 0
    model.scalar_ge(vec![1, 1, -1], vec![a, b, c], 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_search::{ObjectiveDirection, assignment_int};

    #[test]
    fn bool_xor_matches_parity() {
        let mut model = Model::new();
        let a = model.int_var_fixed(1);
        let b = model.int_var_fixed(0);
        let c = model.int_var(0, 1);
        bool_xor(&mut model, a, b, c);
        let (solution, _) = model.solve_subset_with_stats(vec![c]);
        assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(1));
    }

    #[test]
    fn generic_min_dispatches_to_int_min() {
        let mut model = Model::new();
        let a = model.int_var_fixed(4);
        let b = model.int_var_fixed(9);
        let c = model.int_var(0, 10);
        generic_min(&mut model, a, b, c);
        let (solution, _) = model.solve_subset_with_stats(vec![c]);
        assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(4));
    }

    #[test]
    fn int_min_selects_smaller() {
        let mut model = Model::new();
        let a = model.int_var_fixed(3);
        let b = model.int_var_fixed(7);
        let c = model.int_var(0, 10);
        int_min(&mut model, a, b, c);
        let (solution, _) = model.solve_subset_with_stats(vec![c]);
        assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(3));
    }

    #[test]
    fn int_plus_table_posts_sum() {
        let mut model = Model::new();
        let a = model.int_var(1, 3);
        let b = model.int_var(1, 3);
        let c = model.int_var(2, 6);
        int_plus(&mut model, a, b, c);
        let (solution, _, _, _) = model.optimize(vec![c], c, ObjectiveDirection::Maximize);
        assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(6));
    }

    #[test]
    fn int_abs_fixes_magnitude() {
        let mut model = Model::new();
        let a = model.int_var_fixed(-3);
        let b = model.int_var(0, 5);
        int_abs(&mut model, a, b);
        let (solution, _, _, _) = model.optimize(vec![b], b, ObjectiveDirection::Maximize);
        assert_eq!(solution.and_then(|s| assignment_int(&s, b)), Some(3));
    }
}
