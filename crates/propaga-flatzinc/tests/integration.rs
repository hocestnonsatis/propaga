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
    let objective = instance.objectives.first().expect("objective");
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
fn compiles_inline_predicate() {
    let source = r#"
        predicate p(var int: a, var int: b) = int_eq(a, b);
        var 1..3: x;
        var 1..3: y;
        constraint p(x, y);
        solve satisfy;
    "#;
    let program = parse(source).expect("parse predicate program");
    let mut instance = compile(program).expect("compile predicate program");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn compiles_pareto_solve_directive() {
    let source = r#"
var 1..2: x;
var 1..2: y;
constraint int_ne(x, y);
solve :: pareto([x, y]) satisfy;
"#;
    let program = parse(source).expect("parse");
    let compiled = compile(program).expect("compile");
    assert!(compiled.pareto);
    assert_eq!(compiled.pareto_objectives.len(), 2);
}

#[test]
fn compiles_regular_constraint() {
    let source = include_str!("../../../benchmarks/regular_chain.fzn");
    let program = parse(source).expect("parse");
    compile(program).expect("compile");
}

#[test]
fn compiles_set_cardinality_instance() {
    let source = include_str!("../../../benchmarks/set_cardinality.fzn");
    let program = parse(source).expect("parse set instance");
    let instance = compile(program).expect("compile set instance");
    assert!(!instance.model.decision_variables().is_empty());
}

#[test]
fn compiles_float_bounds_instance() {
    let source = include_str!("../../../benchmarks/float_bounds.fzn");
    let program = parse(source).expect("parse float instance");
    let instance = compile(program).expect("compile float instance");
    assert!(instance.model.decision_variables().len() >= 2);
}

#[test]
fn compiles_set_union_instance() {
    let source = include_str!("../../../benchmarks/set_union.fzn");
    let program = parse(source).expect("parse set_union");
    compile(program).expect("compile set_union");
}

#[test]
fn compiles_set_intersect_instance() {
    let source = include_str!("../../../benchmarks/set_intersect.fzn");
    let program = parse(source).expect("parse set_intersect");
    compile(program).expect("compile set_intersect");
}

#[test]
fn compiles_float_times_instance() {
    let source = include_str!("../../../benchmarks/float_times.fzn");
    let program = parse(source).expect("parse float_times");
    compile(program).expect("compile float_times");
}

#[test]
fn solves_set_union_benchmark() {
    let source = include_str!("../../../benchmarks/set_union.fzn");
    let program = parse(source).expect("parse");
    let instance = compile(program).expect("compile");
    let mut model = instance.model;
    let vars = model.decision_variables().to_vec();
    let solution = model.solve_subset(vars);
    assert!(solution.is_some());
}

#[test]
fn solves_float_times_benchmark() {
    let source = include_str!("../../../benchmarks/float_times.fzn");
    let program = parse(source).expect("parse");
    let instance = compile(program).expect("compile");
    let mut model = instance.model;
    let vars = model.decision_variables().to_vec();
    let solution = model.solve_subset(vars);
    assert!(solution.is_some());
}
