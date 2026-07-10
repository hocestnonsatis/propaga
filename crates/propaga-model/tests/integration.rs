use propaga_model::Model;
use propaga_search::{ObjectiveDirection, SearchConfig};

#[test]
fn all_different_with_equality_is_unsat() {
    let mut model = Model::new();
    let x = model.int_var(1, 5);
    let y = model.int_var(1, 5);
    model.all_different(&[x, y]);
    model.equal(x, y);
    assert!(model.solve_subset(vec![x, y]).is_none());
}

#[test]
fn distinct_values_are_sat() {
    let mut model = Model::new();
    let x = model.int_var(1, 5);
    let y = model.int_var(1, 5);
    model.all_different(&[x, y]);
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
            (x, ObjectiveDirection::Minimize),
            (y, ObjectiveDirection::Minimize),
        ],
    );
    assert!(result.front.len() >= 2);
}
