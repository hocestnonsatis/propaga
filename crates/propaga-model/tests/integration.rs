use propaga_model::Model;
use propaga_search::{ObjectiveDirection, SearchConfig};

#[test]
fn all_different_with_equality_is_unsat() {
    let mut model = Model::new();
    let x = model.int_var(1, 5);
    let y = model.int_var(1, 5);
    model.all_different([x, y]);
    model.equal(x, y);
    assert!(model.solve_subset(vec![x, y]).is_none());
}

#[test]
fn distinct_values_are_sat() {
    let mut model = Model::new();
    let x = model.int_var(1, 5);
    let y = model.int_var(1, 5);
    model.all_different([x, y]);
    model.less_than(x, y);
    assert!(model.solve_subset(vec![x, y]).is_some());
}

#[test]
fn pareto_minimize_two_objectives() {
    let mut model = Model::new();
    model.set_search_config(SearchConfig::without_learning());
    let x = model.int_var(1, 3);
    let y = model.int_var(1, 3);
    let sum = model.int_var_fixed(4);
    model.linear_eq(x, y, sum);
    let result = model.pareto_optimize(
        vec![x, y],
        vec![
            (
                propaga_search::OptimizationTarget::Int(x),
                ObjectiveDirection::Minimize,
            ),
            (
                propaga_search::OptimizationTarget::Int(y),
                ObjectiveDirection::Minimize,
            ),
        ],
    );
    assert!(result.front.len() >= 2);
}

#[test]
fn set_cardinality_satisfiable() {
    let mut model = Model::new();
    let s = model.set_var(1, 3, 2, 2);
    model.set_card(s);
    let result = model.solve_subset(vec![s]);
    assert!(result.is_some());
}

#[test]
fn float_le_satisfiable() {
    let mut model = Model::new();
    let x = model.float_var(0.0, 5.0);
    let y = model.float_var(3.0, 10.0);
    model.float_le(x, y);
    let result = model.solve_subset(vec![x, y]);
    assert!(result.is_some());
}

#[test]
fn set_union_satisfiable() {
    let mut model = Model::new();
    let a = model.set_var(1, 3, 1, 2);
    let b = model.set_var(1, 3, 1, 2);
    let u = model.set_var(1, 3, 2, 3);
    model.set_union(a, b, u);
    model.constrain_set_cardinality(u, 3, 3);
    let result = model.solve_subset(vec![a, b, u]);
    assert!(result.is_some());
}

#[test]
fn set_intersect_satisfiable() {
    let mut model = Model::new();
    let a = model.set_var(1, 4, 2, 3);
    let b = model.set_var(2, 5, 2, 3);
    let i = model.set_var(1, 5, 1, 2);
    model.set_intersect(a, b, i);
    model.constrain_set_cardinality(i, 2, 2);
    let result = model.solve_subset(vec![a, b, i]);
    assert!(result.is_some());
}

#[test]
fn float_times_satisfiable() {
    let mut model = Model::new();
    let a = model.float_var(2.0, 3.0);
    let b = model.float_var(4.0, 5.0);
    let c = model.float_var(0.0, 100.0);
    model.float_times(a, b, c);
    let result = model.solve_subset(vec![a, b, c]);
    assert!(result.is_some());
}
