use propaga_model::Model;

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
