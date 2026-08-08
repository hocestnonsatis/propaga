use crate::output::{
    print_flatzinc_json, print_flatzinc_result, print_objective_plain, print_stats_plain,
};
use crate::puzzle_io::{GlobalOptions, OutputFormat};
use propaga_core::VariableId;
use propaga_flatzinc::{OutputDirective, compile, parse};
use propaga_search::{
    AssignmentValue, LnsConfig, Objective, ObjectiveDirection, ObjectiveValue, OptimizationTarget,
    ParetoSolution, PortfolioConfig, SearchStats, Solution,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Optional warm-start / LNS controls for FlatZinc optimize solves.
#[derive(Clone, Debug)]
pub struct SolveExtras {
    /// Path to a JSON hint file (`{ "x": 1 }` or `{ "variables": { "x": 1 } }`).
    pub hint_path: Option<PathBuf>,
    /// When `Some(n)` with `n > 0`, use LNS with `n` repair iterations.
    pub lns_iterations: Option<u32>,
    /// Fraction of decision vars freed each LNS iteration.
    pub lns_destroy_fraction: f64,
    /// Deterministic LNS destroy seed.
    pub lns_seed: u64,
}

impl Default for SolveExtras {
    fn default() -> Self {
        Self {
            hint_path: None,
            lns_iterations: None,
            lns_destroy_fraction: 0.3,
            lns_seed: 1,
        }
    }
}

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
    objective_value: Option<ObjectiveValue>,
    objective_values: Vec<ObjectiveValue>,
    objective_direction: Option<ObjectiveDirection>,
    pareto_solutions: Vec<ParetoSolution>,
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
    run_ex(path, options, SolveExtras::default())
}

/// Loads and solves a FlatZinc instance with optional warm-start / LNS.
pub fn run_ex(
    path: &Path,
    options: GlobalOptions,
    extras: SolveExtras,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let outcome = solve_source(&source, options, &extras)?;
    print_outcome(path, options, &outcome);
    outcome_to_result(outcome.status)
}

/// Solves every `.fzn` file in a directory.
pub fn run_dir(dir: &Path, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    run_dir_ex(dir, options, SolveExtras::default())
}

/// Directory batch solve with optional warm-start / LNS.
pub fn run_dir_ex(
    dir: &Path,
    options: GlobalOptions,
    extras: SolveExtras,
) -> Result<(), Box<dyn std::error::Error>> {
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
        let outcome = solve_source(&source, options, &extras)?;
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

fn solve_source(
    source: &str,
    options: GlobalOptions,
    extras: &SolveExtras,
) -> Result<SolveOutcome, String> {
    let program = parse(source).map_err(|error| error.to_string())?;
    let mut instance = compile(program).map_err(|error| error.to_string())?;

    instance
        .model
        .set_search_config(options.merge_flatzinc_search_config(instance.annotation_search));
    instance
        .model
        .set_search_phases(instance.search_phases.clone());

    let hint = match &extras.hint_path {
        Some(path) => {
            let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
            Some(parse_int_hint(&text, &instance.names)?)
        }
        None => None,
    };
    let lns = extras
        .lns_iterations
        .filter(|n| *n > 0)
        .map(|iterations| LnsConfig {
            iterations,
            destroy_fraction: extras.lns_destroy_fraction,
            seed: extras.lns_seed,
        });

    let started = Instant::now();
    let (
        solution,
        stats,
        objective_value,
        objective_values,
        solutions_found,
        objective_direction,
        pareto_solutions,
    ) = if instance.pareto {
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
        let result = if options.workers > 1 {
            instance.model.pareto_optimize_portfolio(
                instance.solve_vars.clone(),
                objectives,
                PortfolioConfig {
                    workers: options.workers,
                    deterministic: options.deterministic,
                },
            )
        } else {
            instance
                .model
                .pareto_optimize(instance.solve_vars.clone(), objectives)
        };
        let found = result.front.len() as u32;
        let first = result.front.first().cloned();
        (
            first.as_ref().map(|entry| entry.assignment.clone()),
            result.stats,
            first
                .as_ref()
                .and_then(|entry| entry.objective_values.first().cloned()),
            first
                .map(|entry| entry.objective_values)
                .unwrap_or_default(),
            found,
            Some(ObjectiveDirection::Minimize),
            result.front,
        )
    } else if !instance.objectives.is_empty() {
        if instance.objectives.len() > 1 {
            let objectives: Vec<Objective> = instance
                .objectives
                .iter()
                .map(|objective| Objective {
                    target: objective.optimization_target(),
                    direction: objective.direction(),
                })
                .collect();
            let result = if options.workers > 1 {
                instance.model.optimize_lexicographic_portfolio(
                    instance.solve_vars.clone(),
                    objectives,
                    PortfolioConfig {
                        workers: options.workers,
                        deterministic: options.deterministic,
                    },
                )
            } else {
                instance
                    .model
                    .optimize_lexicographic(instance.solve_vars.clone(), objectives)
            };
            let direction = instance
                .objectives
                .first()
                .map(|objective| objective.direction());
            let found = u32::from(result.solution.is_some());
            (
                result.solution,
                result.stats,
                result.objective_values.first().cloned(),
                result.objective_values,
                found,
                direction,
                Vec::new(),
            )
        } else {
            let objective = instance.objectives[0];
            let (solution, value, stats, solutions_found) = if let Some(lns) = lns {
                if options.workers > 1 {
                    eprintln!(
                        "warning: --lns-iterations uses single-threaded LNS and ignores --workers"
                    );
                }
                instance.model.optimize_objective_lns(
                    instance.solve_vars.clone(),
                    objective.optimization_target(),
                    objective.direction(),
                    lns,
                    hint,
                )
            } else if options.workers > 1 {
                instance.model.optimize_objective_portfolio(
                    instance.solve_vars.clone(),
                    objective.optimization_target(),
                    objective.direction(),
                    PortfolioConfig {
                        workers: options.workers,
                        deterministic: options.deterministic,
                    },
                    hint,
                )
            } else if let Some(hint) = hint {
                instance.model.optimize_objective_with_hint(
                    instance.solve_vars.clone(),
                    objective.optimization_target(),
                    objective.direction(),
                    hint,
                )
            } else {
                instance.model.optimize_objective(
                    instance.solve_vars.clone(),
                    objective.optimization_target(),
                    objective.direction(),
                )
            };
            let objective_values = value.clone().into_iter().collect();
            (
                solution,
                stats,
                value,
                objective_values,
                solutions_found,
                Some(objective.direction()),
                Vec::new(),
            )
        }
    } else if options.all {
        let (solutions, stats) = instance.model.solve_all_with_stats_limited(
            instance.solve_vars.clone(),
            options.effective_solutions_limit(),
        );
        let found = solutions.len() as u32;
        (
            solutions.into_iter().next(),
            stats,
            None,
            Vec::new(),
            found,
            None,
            Vec::new(),
        )
    } else if options.workers > 1 {
        let (solution, stats) = instance.model.solve_portfolio(
            instance.solve_vars.clone(),
            PortfolioConfig {
                workers: options.workers,
                deterministic: options.deterministic,
            },
        );
        let found = u32::from(solution.is_some());
        (solution, stats, None, Vec::new(), found, None, Vec::new())
    } else {
        let (solution, stats) = instance
            .model
            .solve_subset_with_stats(instance.solve_vars.clone());
        let found = u32::from(solution.is_some());
        (solution, stats, None, Vec::new(), found, None, Vec::new())
    };
    let elapsed = started.elapsed();

    let status = if stats.timed_out {
        SolveStatus::Timeout
    } else if solution.is_some()
        || !pareto_solutions.is_empty()
        || (options.all && solutions_found > 0)
    {
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
        objective_values,
        objective_direction,
        pareto_solutions,
    })
}

/// Parses a JSON integer hint map against FlatZinc variable names.
fn parse_int_hint(text: &str, names: &HashMap<VariableId, String>) -> Result<Solution, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("hint JSON: {error}"))?;
    let map = if let Some(vars) = value.get("variables").and_then(|v| v.as_object()) {
        vars
    } else if let Some(obj) = value.as_object() {
        obj
    } else {
        return Err("hint JSON must be an object of name → int".into());
    };

    let name_to_id: HashMap<&str, VariableId> = names
        .iter()
        .map(|(id, name)| (name.as_str(), *id))
        .collect();

    let mut solution = Solution::new();
    for (name, raw) in map {
        if name == "type" || name == "status" || name == "output" || name == "sections" {
            continue;
        }
        let Some(id) = name_to_id.get(name.as_str()).copied() else {
            return Err(format!("hint references unknown variable `{name}`"));
        };
        let int_value = raw
            .as_i64()
            .ok_or_else(|| format!("hint value for `{name}` must be an integer"))?;
        let int_value = i32::try_from(int_value)
            .map_err(|_| format!("hint value for `{name}` is out of i32 range"))?;
        solution.push((id, AssignmentValue::Int(int_value)));
    }
    if solution.is_empty() {
        return Err("hint JSON contained no integer assignments".into());
    }
    Ok(solution)
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
            if let Some(value) = &outcome.objective_value {
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
                outcome.objective_values.as_slice(),
                outcome.objective_direction,
                &outcome.pareto_solutions,
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

    #[test]
    fn warm_start_hint_reaches_optimum() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/maximize_x.fzn");
        let hint =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/maximize_x_hint.json");
        run_ex(
            &path,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
            SolveExtras {
                hint_path: Some(hint),
                ..SolveExtras::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn lns_from_hint_reaches_optimum() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/maximize_x.fzn");
        let hint =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/maximize_x_hint.json");
        run_ex(
            &path,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
            SolveExtras {
                hint_path: Some(hint),
                lns_iterations: Some(8),
                lns_destroy_fraction: 0.5,
                lns_seed: 7,
            },
        )
        .unwrap();
    }

    #[test]
    fn parse_int_hint_accepts_variables_wrapper() {
        let program = parse("var 0..10: x;\nsolve satisfy;\n").unwrap();
        let instance = compile(program).unwrap();
        let solution = parse_int_hint(r#"{"variables":{"x":4}}"#, &instance.names).unwrap();
        assert_eq!(solution.len(), 1);
        assert_eq!(solution[0].1, AssignmentValue::Int(4));
    }

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

    flatzinc_test!(
        solves_predicate_multi_flatzinc,
        "../../benchmarks/predicate_multi.fzn"
    );
    flatzinc_test!(
        solves_lexicographic_multi_flatzinc,
        "../../benchmarks/lexicographic_multi.fzn"
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
