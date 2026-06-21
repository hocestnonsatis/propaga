use crate::output::{print_flatzinc_json, print_flatzinc_result, print_objective_plain, print_stats_plain};
use crate::puzzle_io::{GlobalOptions, OutputFormat};
use propaga_flatzinc::{compile, parse};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Loads and solves a FlatZinc instance.
pub fn run(path: &Path, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    compile_and_solve(&source, options)
}

fn compile_and_solve(source: &str, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let program = parse(source).map_err(|error| error.to_string())?;
    let mut instance = compile(program).map_err(|error| error.to_string())?;

    instance.model.set_search_config(options.search_config());

    let started = Instant::now();
    let (solution, stats, objective_value, solutions_found) =
        if let Some(objective) = instance.objective {
            let (solution, objective_value, stats, solutions_found) = instance.model.optimize(
                instance.solve_vars.clone(),
                objective.var,
                objective.direction,
            );
            (solution, stats, objective_value, solutions_found)
        } else if options.all {
            let (solutions, stats) = instance.model.solve_all_with_stats_limited(
                instance.solve_vars.clone(),
                options.effective_solutions_limit(),
            );
            (
                solutions.into_iter().next(),
                stats,
                None,
                solutions.len() as u32,
            )
        } else {
            let (solution, stats) = instance
                .model
                .solve_subset_with_stats(instance.solve_vars.clone());
            (solution, stats, None, if solution.is_some() { 1 } else { 0 })
        };
    let elapsed = started.elapsed();

    match options.format {
        OutputFormat::Plain => {
            print_flatzinc_result(
                &instance.names,
                &instance.solve_vars,
                solution.as_ref(),
                &instance.outputs,
                options.quiet,
            );
            if let Some(value) = objective_value {
                print_objective_plain(value, instance.objective.map(|obj| obj.direction), options.quiet);
            }
            if options.stats {
                print_stats_plain(stats, elapsed);
                if solutions_found > 0 && !options.quiet {
                    println!("solutions_found={solutions_found}");
                }
            }
        }
        OutputFormat::Json => {
            print_flatzinc_json(
                &instance.names,
                &instance.solve_vars,
                solution.as_ref(),
                &instance.outputs,
                objective_value,
                instance.objective.map(|obj| obj.direction),
                if options.stats {
                    Some((stats, elapsed, solutions_found))
                } else {
                    None
                },
            );
        }
    }

    if solution.is_none() && !options.all {
        return Err("unsatisfiable".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn solves_magic_square_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/magic_square.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_weighted_sum_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/weighted_sum.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_bounded_sum_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/bounded_sum.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_weighted_sum_ge_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/weighted_sum_ge.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_reified_lt_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/reified_lt.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_reified_eq_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/reified_eq.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_reified_ne_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/reified_ne.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_disjunctive_edge_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/disjunctive_edge.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_cumulative_demand_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/cumulative_demand.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_disjunctive_two_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/disjunctive_two.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_ordered_chain_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/ordered_chain.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_gcc_exact_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/gcc_exact.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_table_puzzle_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/table_puzzle.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_maximize_x_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/maximize_x.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_minimize_cost_flatzinc() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/minimize_cost.fzn");
        let source = std::fs::read_to_string(path).unwrap();
        compile_and_solve(
            &source,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }
}
