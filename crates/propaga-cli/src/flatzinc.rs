use crate::output::{
    print_flatzinc_json, print_flatzinc_result, print_objective_plain, print_stats_plain,
};
use crate::puzzle_io::{GlobalOptions, OutputFormat};
use propaga_core::VariableId;
use propaga_flatzinc::{OutputDirective, compile, parse};
use propaga_search::{ObjectiveDirection, SearchStats, Solution};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Outcome of solving a single FlatZinc instance.
struct SolveOutcome {
    status: SolveStatus,
    stats: SearchStats,
    elapsed: Duration,
    solutions_found: u32,
    names: HashMap<VariableId, String>,
    solve_vars: Vec<VariableId>,
    outputs: Vec<OutputDirective>,
    solution: Option<Solution>,
    objective_value: Option<i32>,
    objective_direction: Option<ObjectiveDirection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SolveStatus {
    Sat,
    Unsat,
    Timeout,
}

impl SolveStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sat => "sat",
            Self::Unsat => "unsatisfiable",
            Self::Timeout => "timeout",
        }
    }

    fn is_success(self) -> bool {
        matches!(self, Self::Sat)
    }
}

/// Loads and solves a FlatZinc instance.
pub fn run(path: &Path, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let outcome = solve_source(&source, options)?;
    print_outcome(path, options, &outcome);
    outcome_to_result(outcome.status)
}

/// Solves every `.fzn` file in a directory.
pub fn run_dir(dir: &Path, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "fzn"))
        .collect();
    files.sort();

    if files.is_empty() {
        return Err(format!("no .fzn files found in `{}`", dir.display()).into());
    }

    let mut batch = Vec::with_capacity(files.len());
    for path in &files {
        let source = fs::read_to_string(path)?;
        let outcome = solve_source(&source, options)?;
        batch.push((path.clone(), outcome));
    }

    let passed = batch
        .iter()
        .filter(|(_, outcome)| outcome.status.is_success())
        .count();

    match options.format {
        OutputFormat::Plain => {
            if !options.quiet {
                for (path, outcome) in &batch {
                    println!(
                        "{}: {}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        outcome.status.as_str()
                    );
                }
                println!("{passed}/{} passed", batch.len());
            }
        }
        OutputFormat::Json => {
            print_batch_json(&batch, passed, batch.len());
        }
    }

    if passed == batch.len() {
        Ok(())
    } else {
        Err(format!("{passed}/{} benchmarks passed", batch.len()).into())
    }
}

fn solve_source(source: &str, options: GlobalOptions) -> Result<SolveOutcome, String> {
    let program = parse(source).map_err(|error| error.to_string())?;
    let mut instance = compile(program).map_err(|error| error.to_string())?;

    instance
        .model
        .set_search_config(options.merge_flatzinc_search_config(instance.annotation_search));

    let started = Instant::now();
    let (solution, stats, objective_value, solutions_found, objective_direction) =
        if let Some(objective) = instance.objective {
            let (solution, objective_value, stats, solutions_found) = instance.model.optimize(
                instance.solve_vars.clone(),
                objective.var,
                objective.direction,
            );
            (
                solution,
                stats,
                objective_value,
                solutions_found,
                Some(objective.direction),
            )
        } else if options.all {
            let (solutions, stats) = instance.model.solve_all_with_stats_limited(
                instance.solve_vars.clone(),
                options.effective_solutions_limit(),
            );
            let found = solutions.len() as u32;
            (solutions.into_iter().next(), stats, None, found, None)
        } else {
            let (solution, stats) = instance
                .model
                .solve_subset_with_stats(instance.solve_vars.clone());
            let found = u32::from(solution.is_some());
            (solution, stats, None, found, None)
        };
    let elapsed = started.elapsed();

    let status = if stats.timed_out {
        SolveStatus::Timeout
    } else if solution.is_some() || (options.all && solutions_found > 0) {
        SolveStatus::Sat
    } else {
        SolveStatus::Unsat
    };

    Ok(SolveOutcome {
        status,
        stats,
        elapsed,
        solutions_found,
        names: instance.names,
        solve_vars: instance.solve_vars,
        outputs: instance.outputs,
        solution,
        objective_value,
        objective_direction,
    })
}

fn print_outcome(path: &Path, options: GlobalOptions, outcome: &SolveOutcome) {
    match options.format {
        OutputFormat::Plain => {
            print_flatzinc_result(
                &outcome.names,
                &outcome.solve_vars,
                outcome.solution.as_ref(),
                &outcome.outputs,
                outcome.stats.timed_out,
                options.quiet,
            );
            if let Some(value) = outcome.objective_value {
                print_objective_plain(value, outcome.objective_direction, options.quiet);
            }
            if options.stats {
                print_stats_plain(outcome.stats, outcome.elapsed);
                if outcome.solutions_found > 0 && !options.quiet {
                    println!("solutions_found={}", outcome.solutions_found);
                }
            }
        }
        OutputFormat::Json => {
            print_flatzinc_json(
                &outcome.names,
                &outcome.solve_vars,
                outcome.solution.as_ref(),
                &outcome.outputs,
                outcome.objective_value,
                outcome.objective_direction,
                if options.stats {
                    Some((outcome.stats, outcome.elapsed, outcome.solutions_found))
                } else {
                    None
                },
            );
        }
    }

    let _ = path;
}

fn outcome_to_result(status: SolveStatus) -> Result<(), Box<dyn std::error::Error>> {
    match status {
        SolveStatus::Sat => Ok(()),
        SolveStatus::Timeout => Err("timeout".into()),
        SolveStatus::Unsat => Err("unsatisfiable".into()),
    }
}

fn print_batch_json(batch: &[(PathBuf, SolveOutcome)], passed: usize, total: usize) {
    use serde_json::json;

    let results: Vec<_> = batch
        .iter()
        .map(|(path, outcome)| {
            json!({
                "file": path.file_name().unwrap_or_default().to_string_lossy(),
                "status": outcome.status.as_str(),
                "elapsed_ms": outcome.elapsed.as_millis(),
                "solutions_found": outcome.solutions_found,
                "timed_out": outcome.stats.timed_out,
            })
        })
        .collect();

    println!(
        "{}",
        json!({
            "passed": passed,
            "total": total,
            "results": results,
        })
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    macro_rules! flatzinc_test {
        ($name:ident, $file:expr) => {
            #[test]
            fn $name() {
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join($file);
                run(
                    &path,
                    GlobalOptions {
                        quiet: true,
                        ..GlobalOptions::default()
                    },
                )
                .unwrap();
            }
        };
    }

    flatzinc_test!(
        solves_magic_square_flatzinc,
        "../../benchmarks/magic_square.fzn"
    );
    flatzinc_test!(
        solves_weighted_sum_flatzinc,
        "../../benchmarks/weighted_sum.fzn"
    );
    flatzinc_test!(
        solves_bounded_sum_flatzinc,
        "../../benchmarks/bounded_sum.fzn"
    );
    flatzinc_test!(
        solves_weighted_sum_ge_flatzinc,
        "../../benchmarks/weighted_sum_ge.fzn"
    );
    flatzinc_test!(
        solves_reified_lt_flatzinc,
        "../../benchmarks/reified_lt.fzn"
    );
    flatzinc_test!(
        solves_reified_eq_flatzinc,
        "../../benchmarks/reified_eq.fzn"
    );
    flatzinc_test!(
        solves_reified_ne_flatzinc,
        "../../benchmarks/reified_ne.fzn"
    );
    flatzinc_test!(
        solves_disjunctive_edge_flatzinc,
        "../../benchmarks/disjunctive_edge.fzn"
    );
    flatzinc_test!(
        solves_cumulative_demand_flatzinc,
        "../../benchmarks/cumulative_demand.fzn"
    );
    flatzinc_test!(
        solves_disjunctive_two_flatzinc,
        "../../benchmarks/disjunctive_two.fzn"
    );
    flatzinc_test!(
        solves_ordered_chain_flatzinc,
        "../../benchmarks/ordered_chain.fzn"
    );
    flatzinc_test!(solves_gcc_exact_flatzinc, "../../benchmarks/gcc_exact.fzn");
    flatzinc_test!(
        solves_table_puzzle_flatzinc,
        "../../benchmarks/table_puzzle.fzn"
    );
    flatzinc_test!(
        solves_maximize_x_flatzinc,
        "../../benchmarks/maximize_x.fzn"
    );
    flatzinc_test!(
        solves_bool_reify_flatzinc,
        "../../benchmarks/bool_reify.fzn"
    );
    flatzinc_test!(
        solves_minimize_cost_flatzinc,
        "../../benchmarks/minimize_cost.fzn"
    );
    flatzinc_test!(
        solves_int_search_order_flatzinc,
        "../../benchmarks/int_search_order.fzn"
    );
    flatzinc_test!(
        solves_int_search_restart_flatzinc,
        "../../benchmarks/int_search_restart.fzn"
    );

    #[test]
    fn flatzinc_annotation_overrides_default_search() {
        let source = r#"
            var 1..3: x;
            solve :: restart_none :: int_search([x], input_order, indomain_max, complete) satisfy;
        "#;
        let program = propaga_flatzinc::parse(source).unwrap();
        let instance = propaga_flatzinc::compile(program).unwrap();
        let config =
            GlobalOptions::default().merge_flatzinc_search_config(instance.annotation_search);
        assert_eq!(
            config.variable_ordering,
            propaga_search::VariableOrdering::InputOrder
        );
        assert_eq!(
            config.value_ordering,
            propaga_search::ValueOrdering::Descending
        );
        assert_eq!(config.restart_policy, propaga_search::RestartPolicy::None);
    }

    #[test]
    fn flatzinc_cli_flag_overrides_annotation() {
        let source = r#"
            var 1..3: x;
            solve :: restart_none :: int_search([x], input_order, indomain_max, complete) satisfy;
        "#;
        let program = propaga_flatzinc::parse(source).unwrap();
        let instance = propaga_flatzinc::compile(program).unwrap();
        let config = GlobalOptions {
            variable_ordering: propaga_search::VariableOrdering::Mrv,
            variable_ordering_explicit: true,
            ..GlobalOptions::default()
        }
        .merge_flatzinc_search_config(instance.annotation_search);
        assert_eq!(
            config.variable_ordering,
            propaga_search::VariableOrdering::Mrv
        );
        assert_eq!(
            config.value_ordering,
            propaga_search::ValueOrdering::Descending
        );
        assert_eq!(config.restart_policy, propaga_search::RestartPolicy::None);
    }

    #[test]
    fn batch_dir_solves_benchmarks() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks");
        run_dir(
            &dir,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }
}
