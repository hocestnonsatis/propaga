use criterion::{Criterion, black_box, criterion_group, criterion_main};
use propaga_domains::IntervalDomain;
use propaga_engine::Engine;
use propaga_propagators::{AllDifferentPropagator, CumulativePropagator, TaskSpec};

fn bench_all_different_gac(c: &mut Criterion) {
    c.bench_function("all_different_gac_30", |b| {
        b.iter(|| {
            let mut engine = Engine::new();
            let vars: Vec<_> = (0..30)
                .map(|index| engine.new_variable(IntervalDomain::new(index + 1, index + 5)))
                .collect();
            engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));
            for (index, var) in vars.iter().enumerate().take(8) {
                let _ = engine.fix_variable(*var, (index + 1) as i32);
            }
            black_box(engine.propagate_all().unwrap());
        });
    });
}

fn bench_cumulative_propagation(c: &mut Criterion) {
    c.bench_function("cumulative_20_tasks", |b| {
        b.iter(|| {
            let mut engine = Engine::new();
            let mut tasks = Vec::with_capacity(20);
            for _ in 0..20 {
                let start = engine.new_variable(IntervalDomain::new(0, 20));
                let end = engine.new_variable(IntervalDomain::new(2, 25));
                tasks.push(TaskSpec::new(start, 2, end));
            }
            engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 3)));
            black_box(engine.propagate_all().unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_all_different_gac,
    bench_cumulative_propagation
);
criterion_main!(benches);
