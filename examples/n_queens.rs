use propaga_domains::IntervalDomain;
use propaga_engine::Engine;
use propaga_propagators::{AllDifferentPropagator, NotEqualOffsetPropagator};
use propaga_search::{DepthFirstSearch, assignment_int, solution_int_values};

fn main() {
    let n: usize = 8;
    let mut engine = Engine::new();

    let queens: Vec<_> = (0..n)
        .map(|_| engine.new_variable(IntervalDomain::new(0, n as i32 - 1)))
        .collect();

    engine.add_propagator(Box::new(AllDifferentPropagator::new(queens.clone())));

    for i in 0..n {
        for j in (i + 1)..n {
            let offset = (j - i) as i32;
            engine.add_propagator(Box::new(NotEqualOffsetPropagator::new(
                queens[i], queens[j], offset,
            )));
            engine.add_propagator(Box::new(NotEqualOffsetPropagator::new(
                queens[i], queens[j], -offset,
            )));
        }
    }

    engine.propagate_all().expect("initial propagation");

    let mut search = DepthFirstSearch::new(queens.clone());
    let solution = search
        .solve(&mut engine)
        .expect("8-queens should have a solution");

    println!("{n}-Queens solution (row -> column):");
    for (row, var) in queens.iter().enumerate() {
        let column = assignment_int(&solution, *var).expect("assigned");
        println!("  row {row}: column {column}");
    }

    assert_solution_valid(n, &solution_int_values(&solution));
}

fn assert_solution_valid(n: usize, columns: &[i32]) {
    assert_eq!(columns.len(), n);

    for i in 0..columns.len() {
        for j in (i + 1)..columns.len() {
            assert_ne!(columns[i], columns[j], "columns must differ");
            let row_diff = (j as i32 - i as i32).abs();
            let col_diff = (columns[i] - columns[j]).abs();
            assert_ne!(row_diff, col_diff, "diagonals must not conflict");
        }
    }
}
