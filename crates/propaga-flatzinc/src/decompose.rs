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

/// Posts `b = not a` for 0/1 variables.
pub fn bool_not(model: &mut Model, a: VariableId, b: VariableId) {
    model.table(vec![a, b], vec![vec![0, 1], vec![1, 0]]);
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
    fn int_abs_fixes_magnitude() {
        let mut model = Model::new();
        let a = model.int_var_fixed(-3);
        let b = model.int_var(0, 5);
        int_abs(&mut model, a, b);
        let (solution, _, _, _) = model.optimize(vec![b], b, ObjectiveDirection::Maximize);
        assert_eq!(solution.and_then(|s| assignment_int(&s, b)), Some(3));
    }
}
