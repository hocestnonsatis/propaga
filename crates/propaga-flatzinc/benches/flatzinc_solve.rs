use criterion::{Criterion, black_box, criterion_group, criterion_main};
use propaga_flatzinc::{compile, parse};
use propaga_search::{Objective, ObjectiveDirection, OptimizationTarget};
use std::path::PathBuf;
use std::time::Duration;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks")
        .join(name)
}

fn solve_source(source: &str) {
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    if instance.pareto {
        let objectives: Vec<(OptimizationTarget, ObjectiveDirection)> = instance
            .pareto_objectives
            .iter()
            .map(|objective| {
                (
                    objective.optimization_target(),
                    ObjectiveDirection::Minimize,
                )
            })
            .collect();
        black_box(
            instance
                .model
                .pareto_optimize(instance.solve_vars, objectives),
        );
    } else if instance.objectives.len() > 1 {
        let objectives: Vec<Objective> = instance
            .objectives
            .iter()
            .map(|objective| Objective {
                target: objective.optimization_target(),
                direction: objective.direction(),
            })
            .collect();
        black_box(
            instance
                .model
                .optimize_lexicographic(instance.solve_vars, objectives),
        );
    } else if let Some(objective) = instance.objectives.first().copied() {
        black_box(instance.model.optimize_objective(
            instance.solve_vars,
            objective.optimization_target(),
            objective.direction(),
        ));
    } else {
        black_box(instance.model.solve_subset_with_stats(instance.solve_vars));
    }
}

fn bench_named(c: &mut Criterion, name: &str, file: &str) {
    let source = std::fs::read_to_string(fixture(file)).expect(file);
    c.bench_function(name, |b| {
        b.iter(|| solve_source(black_box(&source)));
    });
}

fn bench_flatzinc_corpus(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatzinc_corpus");
    group.warm_up_time(Duration::from_millis(200));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(20);

    for (name, file) in [
        ("magic_square", "magic_square.fzn"),
        ("cumulative", "cumulative.fzn"),
        ("diffn_fixed", "diffn_fixed.fzn"),
        ("maximize_x", "maximize_x.fzn"),
        ("gcc_exact", "gcc_exact.fzn"),
    ] {
        let source = std::fs::read_to_string(fixture(file)).expect(file);
        group.bench_function(name, |b| {
            b.iter(|| solve_source(black_box(&source)));
        });
    }
    group.finish();
}

fn bench_smoke(c: &mut Criterion) {
    bench_named(c, "flatzinc_all_different_only", "all_different_only.fzn");
}

criterion_group!(benches, bench_flatzinc_corpus, bench_smoke);
criterion_main!(benches);
