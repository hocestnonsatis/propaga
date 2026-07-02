use crate::output::{print_sudoku_batch_json, print_sudoku_results, sudoku_result_json};
use crate::puzzle_io::{GlobalOptions, load_sudoku_file, parse_sudoku_input};
use propaga_core::VariableId;
use propaga_domains::IntervalDomain;
use propaga_model::Model;
use std::path::Path;
use std::time::Instant;

const DEFAULT_SUDOKU: &str = "\
534678912\
126349578\
789125346\
213400000\
000000000\
000000000\
000000000\
000000000\
000000000";

/// Solves Sudoku puzzles from CLI input.
pub fn run(
    puzzle: Option<String>,
    file: Option<&Path>,
    options: GlobalOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let puzzles = if let Some(path) = file {
        load_sudoku_file(path)?
    } else if let Some(puzzle) = puzzle {
        vec![parse_sudoku_input(&puzzle)?]
    } else {
        vec![parse_sudoku_input(DEFAULT_SUDOKU)?]
    };

    let mut json_results = Vec::new();

    for (index, digits) in puzzles.iter().enumerate() {
        let started = Instant::now();
        let (values, stats) = solve_one(digits, options)?;
        let elapsed = started.elapsed();

        if options.format == crate::puzzle_io::OutputFormat::Json && puzzles.len() > 1 {
            json_results.push(sudoku_result_json(index, &values, stats, elapsed));
        } else {
            print_sudoku_results(options, index, &values, stats, elapsed);
        }
    }

    if options.format == crate::puzzle_io::OutputFormat::Json && puzzles.len() > 1 {
        print_sudoku_batch_json(json_results);
    }

    Ok(())
}

fn solve_one(
    digits: &[i32],
    options: GlobalOptions,
) -> Result<(Vec<i32>, propaga_search::SearchStats), Box<dyn std::error::Error>> {
    let mut model = Model::new();
    model.set_search_config(options.search_config());
    let mut cells = Vec::with_capacity(81);
    for &digit in digits {
        let var = if digit == 0 {
            model.int_var(1, 9)
        } else {
            model.int_var_domain(IntervalDomain::fix(digit))
        };
        cells.push(var);
    }

    post_sudoku_constraints(&mut model, &cells);
    model.propagate()?;

    if options.all {
        let (solutions, stats) =
            model.solve_all_with_stats_limited(cells.clone(), options.effective_solutions_limit());
        let Some(first) = solutions.first() else {
            if stats.timed_out {
                return Err("timeout".into());
            }
            return Err("sudoku puzzle is unsatisfiable".into());
        };
        Ok((solution_to_grid(first, &cells), stats))
    } else {
        let (solution, stats) = if options.workers > 1 {
            model.solve_portfolio(
                cells.clone(),
                propaga_search::PortfolioConfig {
                    workers: options.workers,
                    deterministic: options.deterministic,
                },
            )
        } else {
            model.solve_subset_with_stats(cells.clone())
        };
        let Some(solution) = solution else {
            if stats.timed_out {
                return Err("timeout".into());
            }
            return Err("sudoku puzzle is unsatisfiable".into());
        };
        Ok((solution_to_grid(&solution, &cells), stats))
    }
}

fn post_sudoku_constraints(model: &mut Model, cells: &[VariableId]) {
    for row in 0..9 {
        let vars: Vec<_> = (0..9).map(|col| cells[row * 9 + col]).collect();
        model.all_different(vars);
    }
    for col in 0..9 {
        let vars: Vec<_> = (0..9).map(|row| cells[row * 9 + col]).collect();
        model.all_different(vars);
    }
    for box_row in 0..3 {
        for box_col in 0..3 {
            let mut vars = Vec::with_capacity(9);
            for row in 0..3 {
                for col in 0..3 {
                    vars.push(cells[(box_row * 3 + row) * 9 + (box_col * 3 + col)]);
                }
            }
            model.all_different(vars);
        }
    }
}

fn solution_to_grid(solution: &[(VariableId, i32)], cells: &[VariableId]) -> Vec<i32> {
    let mut values = vec![0; 81];
    for (var, value) in solution {
        if let Some(index) = cells.iter().position(|cell| cell == var) {
            values[index] = *value;
        }
    }
    values
}
