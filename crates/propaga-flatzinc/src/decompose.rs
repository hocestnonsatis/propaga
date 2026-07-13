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

fn build_binary_op_tuples(
    model: &Model,
    a: VariableId,
    b: VariableId,
    op: impl Fn(i32, i32) -> i32,
) -> Vec<Vec<i32>> {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let mut tuples = Vec::new();
    for av in amin..=amax {
        for bv in bmin..=bmax {
            tuples.push(vec![av, bv, op(av, bv)]);
        }
    }
    tuples
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

/// Posts `c = min(a, b)` using a domain table.
pub fn int_min(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        build_binary_op_tuples(model, a, b, |x, y| x.min(y)),
    );
}

/// Posts `c = max(a, b)` using a domain table.
pub fn int_max(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        build_binary_op_tuples(model, a, b, |x, y| x.max(y)),
    );
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

/// Posts `result = base ** exp_const`.
pub fn int_pow_fixed(
    model: &mut Model,
    base: VariableId,
    exp: i32,
    result: VariableId,
) -> Result<(), String> {
    let (bmin, bmax) = domain_range(model, base);
    let mut tuples = Vec::new();
    for b in bmin..=bmax {
        tuples.push(vec![b, b.pow(exp.max(0) as u32)]);
    }
    model.table(vec![base, result], tuples);
    Ok(())
}

/// Posts `m = max(xs)`.
pub fn array_int_maximum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    let mut eq_reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        model.less_equal(x, m);
        let reif = model.int_var(0, 1);
        model.reified_equal(x, m, reif);
        eq_reifs.push(reif);
    }
    if !eq_reifs.is_empty() {
        model.scalar_ge(vec![1; eq_reifs.len()], eq_reifs, 1);
    }
}

/// Posts `m = min(xs)`.
pub fn array_int_minimum(model: &mut Model, xs: &[VariableId], m: VariableId) {
    let mut eq_reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        model.less_equal(m, x);
        let reif = model.int_var(0, 1);
        model.reified_equal(x, m, reif);
        eq_reifs.push(reif);
    }
    if !eq_reifs.is_empty() {
        model.scalar_ge(vec![1; eq_reifs.len()], eq_reifs, 1);
    }
}

/// Posts `b = |a|` using a domain table.
pub fn int_abs(model: &mut Model, a: VariableId, b: VariableId) {
    let (min, max) = domain_range(model, a);
    let mut tuples = Vec::new();
    for value in min..=max {
        tuples.push(vec![value, value.abs()]);
    }
    model.table(vec![a, b], tuples);
}

/// Posts `c = a * b` using a domain table.
pub fn int_times(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let a_len = (amax - amin + 1) as usize;
    let b_len = (bmax - bmin + 1) as usize;
    if table_too_large(a_len.saturating_mul(b_len)) {
        return Err("int_times domain too large".to_string());
    }
    let mut tuples = Vec::with_capacity(a_len * b_len);
    for av in amin..=amax {
        for bv in bmin..=bmax {
            tuples.push(vec![av, bv, av * bv]);
        }
    }
    model.table(vec![a, b, c], tuples);
    Ok(())
}

/// Posts `c = a / b` (integer division) using a domain table.
pub fn int_div(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let a_len = (amax - amin + 1) as usize;
    let b_len = (bmax - bmin + 1) as usize;
    if table_too_large(a_len.saturating_mul(b_len)) {
        return Err("int_div domain too large".to_string());
    }
    let mut tuples = Vec::new();
    for av in amin..=amax {
        for bv in bmin..=bmax {
            if bv != 0 {
                tuples.push(vec![av, bv, av / bv]);
            }
        }
    }
    if tuples.is_empty() {
        return Err("int_div has no valid divisor values".to_string());
    }
    model.table(vec![a, b, c], tuples);
    Ok(())
}

/// Posts `c = a mod b` using a domain table.
pub fn int_mod(
    model: &mut Model,
    a: VariableId,
    b: VariableId,
    c: VariableId,
) -> Result<(), String> {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let a_len = (amax - amin + 1) as usize;
    let b_len = (bmax - bmin + 1) as usize;
    if table_too_large(a_len.saturating_mul(b_len)) {
        return Err("int_mod domain too large".to_string());
    }
    let mut tuples = Vec::new();
    for av in amin..=amax {
        for bv in bmin..=bmax {
            if bv != 0 {
                tuples.push(vec![av, bv, av % bv]);
            }
        }
    }
    if tuples.is_empty() {
        return Err("int_mod has no valid divisor values".to_string());
    }
    model.table(vec![a, b, c], tuples);
    Ok(())
}

/// Posts `c = a + b` using a domain table.
pub fn int_plus(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let a_len = (amax - amin + 1) as usize;
    let b_len = (bmax - bmin + 1) as usize;
    if table_too_large(a_len.saturating_mul(b_len)) {
        let sum = model.int_var(amin.saturating_add(bmin), amax.saturating_add(bmax));
        model.linear_eq(a, b, sum);
        model.equal(sum, c);
        return;
    }
    let mut tuples = Vec::with_capacity(a_len * b_len);
    for av in amin..=amax {
        for bv in bmin..=bmax {
            tuples.push(vec![av, bv, av.saturating_add(bv)]);
        }
    }
    model.table(vec![a, b, c], tuples);
}

/// Posts `b = not a` for 0/1 variables.
pub fn bool_not(model: &mut Model, a: VariableId, b: VariableId) {
    model.table(vec![a, b], vec![vec![0, 1], vec![1, 0]]);
}

/// Posts `c = a xor b` for 0/1 variables.
pub fn bool_xor(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        vec![vec![0, 0, 0], vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 0]],
    );
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
        let next = model.int_var(0, 1);
        bool_xor(model, acc, x, next);
        acc = next;
    }
    model.equal(acc, c);
}

/// Posts `c = a /\ b` for 0/1 variables.
pub fn bool_and(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        vec![vec![0, 0, 0], vec![0, 1, 0], vec![1, 0, 0], vec![1, 1, 1]],
    );
}

/// Posts `c = a \/ b` for 0/1 variables.
pub fn bool_or(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        vec![vec![0, 0, 0], vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 1]],
    );
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
