use propaga_core::VariableId;
use propaga_model::Model;

use crate::decompose::{bool_and, bool_clause, bool_or};

/// `count(xs, value, total)`: `total = |{ i | xs[i] = value }|`
pub fn count(model: &mut Model, xs: &[VariableId], value: VariableId, total: VariableId) {
    let reifs = membership_reifs(model, xs, value);
    post_sum_equals(model, &reifs, total);
}

/// `among(n, xs, values)`: exactly `n` variables in `xs` take a value from `values`.
pub fn among(model: &mut Model, n: VariableId, xs: &[VariableId], values: &[i32]) {
    let mut reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        let reif = model.int_var_aux(0, 1);
        in_set_reif(model, x, values, reif);
        reifs.push(reif);
    }
    post_sum_equals(model, &reifs, n);
}

/// `at_least(n, xs, value)`: at least `n` variables in `xs` equal `value`.
pub fn at_least(model: &mut Model, n: i32, xs: &[VariableId], value: i32) {
    let value_var = model.int_var_fixed(value);
    let mut reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        let reif = model.int_var_aux(0, 1);
        model.reified_equal(x, value_var, reif);
        reifs.push(reif);
    }
    let coeffs = vec![1; reifs.len()];
    model.scalar_ge(coeffs, reifs, n);
}

/// `at_most(n, xs, value)`: at most `n` variables in `xs` equal `value`.
pub fn at_most(model: &mut Model, n: i32, xs: &[VariableId], value: i32) {
    let value_var = model.int_var_fixed(value);
    let mut reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        let reif = model.int_var_aux(0, 1);
        model.reified_equal(x, value_var, reif);
        reifs.push(reif);
    }
    let coeffs = vec![1; reifs.len()];
    model.scalar_le(coeffs, reifs, n);
}

/// `distribute(card, value, base)`: `card[i]` counts occurrences of `value[i]` in `base`.
pub fn distribute(
    model: &mut Model,
    cards: &[VariableId],
    values: &[VariableId],
    base: &[VariableId],
) {
    for (&card, &value) in cards.iter().zip(values) {
        count(model, base, value, card);
    }
}

/// `lex_less(x, y)`: `x` is strictly lexicographically less than `y`.
pub fn lex_less(model: &mut Model, x: &[VariableId], y: &[VariableId]) {
    lex_compare(model, x, y, false);
}

/// `lex_lesseq(x, y)`: `x` is lexicographically less than or equal to `y`.
pub fn lex_lesseq(model: &mut Model, x: &[VariableId], y: &[VariableId]) {
    lex_compare(model, x, y, true);
}

/// `lex_greater(x, y)`: `x` is strictly lexicographically greater than `y`.
pub fn lex_greater(model: &mut Model, x: &[VariableId], y: &[VariableId]) {
    lex_less(model, y, x);
}

/// `lex_greatereq(x, y)`: `x` is lexicographically greater than or equal to `y`.
pub fn lex_greatereq(model: &mut Model, x: &[VariableId], y: &[VariableId]) {
    lex_lesseq(model, y, x);
}

/// `increasing(x)`: values in `x` are non-decreasing.
pub fn increasing(model: &mut Model, xs: &[VariableId]) {
    for pair in xs.windows(2) {
        model.less_equal(pair[0], pair[1]);
    }
}

/// `decreasing(x)`: values in `x` are non-increasing.
pub fn decreasing(model: &mut Model, xs: &[VariableId]) {
    for pair in xs.windows(2) {
        model.greater_equal(pair[0], pair[1]);
    }
}

/// `sort(x, y)`: `y` is the sorted multiset of `x`.
pub fn sort(model: &mut Model, x: &[VariableId], y: &[VariableId]) {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n == 0 {
        return;
    }

    let mut permutation = Vec::with_capacity(n);
    for _ in 0..n {
        permutation.push(model.int_var(0, (n - 1) as i32));
    }
    model.all_different(permutation.clone());
    for i in 0..n {
        model.element(permutation[i], y.to_vec(), x[i]);
    }
    increasing(model, y);
}

/// `nvalue(n, xs)`: `n` is the number of distinct values in `xs`.
pub fn nvalue(model: &mut Model, xs: &[VariableId], n: VariableId) {
    if xs.is_empty() {
        let zero = model.int_var_fixed(0);
        model.equal(n, zero);
        return;
    }

    let one = model.int_var_fixed(1);
    let mut is_first = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        let flag = model.int_var_aux(0, 1);
        if i == 0 {
            model.equal(flag, one);
        } else {
            let mut ne_prev = Vec::with_capacity(i);
            for j in 0..i {
                let ne = model.int_var_aux(0, 1);
                model.reified_not_equal(xs[i], xs[j], ne);
                ne_prev.push(ne);
            }
            bool_and_many(model, &ne_prev, flag);
        }
        is_first.push(flag);
    }
    post_sum_equals(model, &is_first, n);
}

fn lex_compare(model: &mut Model, x: &[VariableId], y: &[VariableId], allow_equal: bool) {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n == 0 {
        return;
    }

    let zero = model.int_var_fixed(0);
    let mut witnesses = Vec::with_capacity(n + usize::from(allow_equal));
    for i in 0..n {
        let prefix = model.int_var_aux(0, 1);
        prefix_all_equal(model, x, y, i, prefix);
        let less = model.int_var_aux(0, 1);
        model.reified_less_than(x[i], y[i], less);
        let witness = model.int_var_aux(0, 1);
        bool_and(model, prefix, less, witness);
        witnesses.push(witness);

        let greater = model.int_var_aux(0, 1);
        model.reified_less_than(y[i], x[i], greater);
        let forbidden = model.int_var_aux(0, 1);
        bool_and(model, prefix, greater, forbidden);
        model.equal(forbidden, zero);
    }

    if allow_equal {
        let all_equal = model.int_var_aux(0, 1);
        prefix_all_equal(model, x, y, n, all_equal);
        witnesses.push(all_equal);
    }

    bool_clause(model, &witnesses);
}

fn prefix_all_equal(
    model: &mut Model,
    x: &[VariableId],
    y: &[VariableId],
    upto: usize,
    reif: VariableId,
) {
    if upto == 0 {
        let one = model.int_var_fixed(1);
        model.equal(reif, one);
        return;
    }
    let mut eqs = Vec::with_capacity(upto);
    for i in 0..upto {
        let eq = model.int_var_aux(0, 1);
        model.reified_equal(x[i], y[i], eq);
        eqs.push(eq);
    }
    bool_and_many(model, &eqs, reif);
}

fn bool_and_many(model: &mut Model, inputs: &[VariableId], output: VariableId) {
    match inputs.len() {
        0 => {
            let one = model.int_var_fixed(1);
            model.equal(output, one);
        }
        1 => {
            model.equal(inputs[0], output);
        }
        _ => {
            let mut acc = inputs[0];
            for &next in &inputs[1..inputs.len() - 1] {
                let aux = model.int_var_aux(0, 1);
                bool_and(model, acc, next, aux);
                acc = aux;
            }
            bool_and(model, acc, inputs[inputs.len() - 1], output);
        }
    }
}

fn membership_reifs(model: &mut Model, xs: &[VariableId], value: VariableId) -> Vec<VariableId> {
    let mut reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        let reif = model.int_var_aux(0, 1);
        model.reified_equal(x, value, reif);
        reifs.push(reif);
    }
    reifs
}

fn post_sum_equals(model: &mut Model, reifs: &[VariableId], total: VariableId) {
    if reifs.is_empty() {
        let zero = model.int_var_fixed(0);
        model.equal(total, zero);
        return;
    }
    let max_sum = i32::try_from(reifs.len()).unwrap_or(i32::MAX);
    let mut acc = reifs[0];
    for &reif in &reifs[1..] {
        let next = model.int_var_aux(0, max_sum);
        model.linear_eq(acc, reif, next);
        acc = next;
    }
    model.equal(acc, total);
}

fn in_set_reif(model: &mut Model, x: VariableId, values: &[i32], reif: VariableId) {
    if values.is_empty() {
        let zero = model.int_var_fixed(0);
        model.equal(reif, zero);
        return;
    }
    let mut eq_reifs = Vec::with_capacity(values.len());
    for &value in values {
        let eq = model.int_var_aux(0, 1);
        let fixed = model.int_var_fixed(value);
        model.reified_equal(x, fixed, eq);
        eq_reifs.push(eq);
    }
    bool_or_many(model, &eq_reifs, reif);
}

fn bool_or_many(model: &mut Model, inputs: &[VariableId], output: VariableId) {
    match inputs.len() {
        0 => {
            let zero = model.int_var_fixed(0);
            model.equal(output, zero);
        }
        1 => {
            model.equal(inputs[0], output);
        }
        _ => {
            let mut acc = inputs[0];
            for &next in &inputs[1..inputs.len() - 1] {
                let aux = model.int_var_aux(0, 1);
                bool_or(model, acc, next, aux);
                acc = aux;
            }
            bool_or(model, acc, inputs[inputs.len() - 1], output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_decomposition_fixes_total() {
        let mut model = Model::new();
        let xs: Vec<_> = (0..3).map(|_| model.int_var(1, 3)).collect();
        let c = model.int_var(0, 3);
        let value = model.int_var_fixed(2);
        count(&mut model, &xs, value, c);
        let _ = model.propagate();
    }

    #[test]
    fn among_decomposition_posts() {
        let mut model = Model::new();
        let xs: Vec<_> = (0..3).map(|_| model.int_var(1, 3)).collect();
        let n = model.int_var(0, 3);
        among(&mut model, n, &xs, &[1, 3]);
        let _ = model.propagate();
    }

    #[test]
    fn distribute_decomposition_posts() {
        let mut model = Model::new();
        let base: Vec<_> = (0..4).map(|_| model.int_var(1, 3)).collect();
        let values = vec![model.int_var_fixed(1), model.int_var_fixed(2)];
        let cards = vec![model.int_var(0, 4), model.int_var(0, 4)];
        distribute(&mut model, &cards, &values, &base);
        let _ = model.propagate();
    }

    #[test]
    fn lex_less_chain_enforced() {
        use propaga_search::assignment_int;

        let mut model = Model::new();
        let a = model.int_var(1, 3);
        let b = model.int_var(1, 3);
        let c = model.int_var(1, 3);
        let d = model.int_var(1, 3);
        lex_less(&mut model, &[a, b], &[c, d]);
        let (solution, _) = model.solve_subset_with_stats(vec![a, b, c, d]);
        let solution = solution.expect("solution");
        let av = assignment_int(&solution, a).expect("a");
        let bv = assignment_int(&solution, b).expect("b");
        let cv = assignment_int(&solution, c).expect("c");
        let dv = assignment_int(&solution, d).expect("d");
        assert!(av < cv || (av == cv && bv < dv));
    }

    #[test]
    fn sort_posts_permutation_and_increasing() {
        let mut model = Model::new();
        let x = vec![
            model.int_var_fixed(3),
            model.int_var_fixed(1),
            model.int_var_fixed(2),
        ];
        let y = vec![
            model.int_var(1, 3),
            model.int_var(1, 3),
            model.int_var(1, 3),
        ];
        sort(&mut model, &x, &y);
        model.propagate().unwrap();
    }
}
