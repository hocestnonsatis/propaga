use propaga_flatzinc::{compile, parse};
use propaga_search::ObjectiveValue;

const MAGIC_SQUARE: &str = include_str!("../../../benchmarks/magic_square.fzn");
const MAXIMIZE_X: &str = include_str!("../../../benchmarks/maximize_x.fzn");
const BOOL_REIFY: &str = include_str!("../../../benchmarks/bool_reify.fzn");

#[test]
fn float_lin_le_model_api_is_satisfiable() {
    use propaga_model::Model;
    let mut model = Model::new();
    let x = model.float_var(0.0, 1.0);
    let y = model.float_var(0.0, 1.0);
    model.float_scalar_le(vec![1.0, 1.0], vec![x, y], 1.5);
    model.propagate().unwrap();
    let (solution, stats) = model.solve_subset_with_stats([x, y]);
    assert!(
        solution.is_some(),
        "float_lin_le should be SAT: timed_out={}",
        stats.timed_out
    );
}

#[test]
fn float_lin_le_benchmark_is_satisfiable() {
    let source = include_str!("../../../benchmarks/float_lin_le.fzn");
    let mut instance = compile(parse(source).expect("parse")).expect("compile");
    let prop = instance.model.propagate();
    assert!(prop.is_ok(), "propagate failed: {prop:?}");
    let (solution, stats) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(
        solution.is_some(),
        "expected SAT, got UNSAT (timed_out={})",
        stats.timed_out
    );
}

#[test]
fn set_param_benchmark_is_satisfiable() {
    let source = include_str!("../../../benchmarks/set_param.fzn");
    let mut instance = compile(parse(source).expect("parse")).expect("compile");
    let prop = instance.model.propagate();
    assert!(prop.is_ok(), "propagate failed: {prop:?}");
    let (solution, stats) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(
        solution.is_some(),
        "expected SAT, got UNSAT (timed_out={})",
        stats.timed_out
    );
}

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
    let (solution, best, _stats, _solutions) = instance.model.optimize_objective(
        instance.solve_vars.clone(),
        objective.optimization_target(),
        objective.direction(),
    );
    assert!(solution.is_some());
    assert_eq!(best, Some(ObjectiveValue::Int(10)));
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

#[test]
fn int_abs_instance_yields_two() {
    use propaga_search::assignment_int;

    let source = include_str!("../../../benchmarks/int_abs.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    let y = instance
        .names
        .iter()
        .find(|(_, name)| *name == "y")
        .map(|(var, _)| *var)
        .expect("y");
    assert_eq!(solution.and_then(|s| assignment_int(&s, y)), Some(2));
}

#[test]
fn bool_logic_not_and() {
    let source = include_str!("../../../benchmarks/bool_logic.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn int_times_instance() {
    let source = include_str!("../../../benchmarks/int_times.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn nested_predicate_expands() {
    let source = include_str!("../../../benchmarks/nested_predicate.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}

#[test]
fn automaton_chain_compiles() {
    let source = include_str!("../../../benchmarks/automaton_chain.fzn");
    let program = parse(source).expect("parse");
    compile(program).expect("compile automaton");
}

#[test]
fn compiles_bool_parameter() {
    let source = r#"
        bool: flag = true;
        var 0..1: x;
        constraint int_eq(x, flag);
        solve satisfy;
    "#;
    let program = parse(source).expect("parse bool param");
    compile(program).expect("compile bool param");
}

#[test]
fn compiles_float_minimize_instance() {
    let source = r#"
        var 0.0..10.0: x;
        solve minimize x;
    "#;
    let program = parse(source).expect("parse");
    let instance = compile(program).expect("compile float objective");
    assert_eq!(instance.objectives.len(), 1);
    assert!(matches!(
        instance.objectives[0],
        propaga_flatzinc::ObjectiveSpec::Float { .. }
    ));
}

#[test]
fn minimizes_float_objective() {
    let source = r#"
        var 0.0..10.0: x;
        solve minimize x;
    "#;
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile float objective");
    let objective = instance.objectives[0];
    let (solution, best, _stats, _solutions) = instance.model.optimize_objective(
        instance.solve_vars.clone(),
        objective.optimization_target(),
        objective.direction(),
    );
    assert!(solution.is_some());
    assert_eq!(best, Some(ObjectiveValue::Float(0.0)));
}

#[test]
fn lexicographic_float_then_int_objectives() {
    let source = r#"
        var 1.0..3.0: x;
        var 1..3: y;
        solve minimize x, y;
    "#;
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile lex float/int");
    assert_eq!(instance.objectives.len(), 2);
    assert!(matches!(
        instance.objectives[0],
        propaga_flatzinc::ObjectiveSpec::Float { .. }
    ));
    assert!(matches!(
        instance.objectives[1],
        propaga_flatzinc::ObjectiveSpec::Int { .. }
    ));
    let objectives = instance
        .objectives
        .iter()
        .map(|objective| propaga_search::Objective {
            target: objective.optimization_target(),
            direction: objective.direction(),
        })
        .collect();
    let result = instance
        .model
        .optimize_lexicographic(instance.solve_vars.clone(), objectives);
    assert_eq!(
        result.objective_values,
        vec![ObjectiveValue::Float(1.0), ObjectiveValue::Int(1)]
    );
}

#[test]
fn compiles_set_minimize_instance() {
    let source = include_str!("../../../benchmarks/set_optimize.fzn");
    let program = parse(source).expect("parse");
    let instance = compile(program).expect("compile set objective");
    assert_eq!(instance.objectives.len(), 1);
    assert!(matches!(
        instance.objectives[0],
        propaga_flatzinc::ObjectiveSpec::SetCardinality { .. }
    ));
}

#[test]
fn compiles_float_parameter() {
    let source = r#"
        float: pi = 3.14;
        var 3.0..4.0: x;
        solve satisfy;
    "#;
    let program = parse(source).expect("parse float param");
    compile(program).expect("compile float param");
}

#[test]
fn generic_min_instance_solves() {
    use propaga_search::assignment_int;

    let source = include_str!("../../../benchmarks/generic_min.fzn");
    let program = parse(source).expect("parse generic_min");
    let mut instance = compile(program).expect("compile generic_min");
    let c = instance
        .names
        .iter()
        .find(|(_, name)| *name == "c")
        .map(|(var, _)| *var)
        .expect("c");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(4));
}

#[test]
fn set_diff_auxiliaries_are_not_decision_variables() {
    let source = include_str!("../../../benchmarks/set_diff.fzn");
    let program = parse(source).expect("parse set_diff");
    let mut instance = compile(program).expect("compile set_diff");
    assert_eq!(
        instance.model.decision_variables().len(),
        3,
        "only a, b, d should be decision vars"
    );
    assert!(instance.model.solve().is_some());
}

#[test]
fn float_log2_auxiliaries_are_not_decision_variables() {
    let source = include_str!("../../../benchmarks/float_log2.fzn");
    let program = parse(source).expect("parse float_log2");
    let mut instance = compile(program).expect("compile float_log2");
    assert_eq!(
        instance.model.decision_variables().len(),
        2,
        "only a, b should be decision vars"
    );
    assert!(instance.model.solve().is_some());
}

#[test]
fn float_max_auxiliaries_are_not_decision_variables() {
    let source = r#"
var 0.0..5.0: a;
var 0.0..5.0: b;
var 0.0..5.0: c;
constraint float_max(a, b, c);
solve satisfy;
"#;
    let program = parse(source).expect("parse float_max");
    let mut instance = compile(program).expect("compile float_max");
    assert_eq!(
        instance.model.decision_variables().len(),
        3,
        "only a, b, c should be decision vars"
    );
    assert!(instance.model.solve().is_some());
}
