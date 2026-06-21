use propaga_flatzinc::{compile, parse};

const MAGIC_SQUARE: &str = include_str!("../../../benchmarks/magic_square.fzn");
const MAXIMIZE_X: &str = include_str!("../../../benchmarks/maximize_x.fzn");
const BOOL_REIFY: &str = include_str!("../../../benchmarks/bool_reify.fzn");

#[test]
fn magic_square_is_satisfiable() {
    let program = parse(MAGIC_SQUARE).expect("parse magic square");
    let mut instance = compile(program).expect("compile magic square");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn maximize_x_finds_optimum() {
    let program = parse(MAXIMIZE_X).expect("parse maximize_x");
    let mut instance = compile(program).expect("compile maximize_x");
    let objective = instance.objective.expect("objective");
    let (solution, best, _stats, _solutions) =
        instance
            .model
            .optimize(instance.solve_vars, objective.var, objective.direction);
    assert!(solution.is_some());
    assert_eq!(best, Some(10));
}

#[test]
fn bool_reify_is_satisfiable() {
    let program = parse(BOOL_REIFY).expect("parse bool_reify");
    let mut instance = compile(program).expect("compile bool_reify");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn rejects_predicate_declaration() {
    let source = r#"
        predicate p(var int: x) = int_eq(x, 1);
        var 1..3: y;
        solve satisfy;
    "#;
    let err = parse(source).unwrap_err();
    assert!(err.to_string().contains("predicate"));
}
