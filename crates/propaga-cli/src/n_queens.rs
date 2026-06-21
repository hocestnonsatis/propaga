use crate::output::print_n_queens_results;
use crate::puzzle_io::GlobalOptions;
use propaga_core::VariableId;
use propaga_model::Model;
use std::time::Instant;

/// Solves the N-Queens problem from CLI input.
pub fn run(size: usize, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut model = Model::new();
    model.set_search_config(options.search_config());
    let n = size as i32;
    let queens: Vec<_> = (0..size).map(|_| model.int_var(0, n - 1)).collect();

    model.all_different(queens.clone());
    for i in 0..size {
        for j in (i + 1)..size {
            let offset = (j - i) as i32;
            model.not_equal_offset(queens[i], queens[j], offset);
            model.not_equal_offset(queens[i], queens[j], -offset);
        }
    }

    model.propagate()?;

    let elapsed_base = started.elapsed();
    let outputs = if options.all {
        let (solutions, stats) =
            model.solve_all_with_stats_limited(queens.clone(), options.effective_solutions_limit());
        if solutions.is_empty() {
            if stats.timed_out {
                return Err("timeout".into());
            }
            return Err(format!("{size}-queens has no solution").into());
        }
        solutions
            .into_iter()
            .map(|solution| {
                (
                    order_solution(&queens, &solution)
                        .into_iter()
                        .enumerate()
                        .map(|(row, column)| (queens[row], column))
                        .collect(),
                    stats,
                    elapsed_base,
                )
            })
            .collect()
    } else {
        let solve_started = Instant::now();
        let (solution, stats) = model.solve_subset_with_stats(queens.clone());
        let Some(solution) = solution else {
            if stats.timed_out {
                return Err("timeout".into());
            }
            return Err(format!("{size}-queens has no solution").into());
        };
        let ordered: Vec<(VariableId, i32)> = queens
            .iter()
            .enumerate()
            .map(|(row, var)| (*var, order_solution(&queens, &solution)[row]))
            .collect();
        vec![(ordered, stats, solve_started.elapsed())]
    };

    print_n_queens_results(options, size, &outputs);
    Ok(())
}

fn order_solution(queens: &[VariableId], solution: &[(VariableId, i32)]) -> Vec<i32> {
    queens
        .iter()
        .map(|var| {
            solution
                .iter()
                .find(|(candidate, _)| candidate == var)
                .map(|(_, value)| *value)
                .expect("missing queen assignment")
        })
        .collect()
}
