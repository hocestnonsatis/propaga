use crate::puzzle_io::OutputFormat;
use propaga_model::Model;
use propaga_search::SearchStats;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct StatsJson {
    nodes: u64,
    backtracks: u64,
    conflicts: u64,
    nogoods_learned: u64,
    restarts: u64,
    elapsed_ms: u128,
}

#[derive(Serialize)]
pub(crate) struct SudokuResultJson {
    puzzle_index: usize,
    solution: Vec<Vec<i32>>,
    stats: StatsJson,
}

#[derive(Serialize)]
struct SudokuBatchJson {
    solutions: Vec<SudokuResultJson>,
}

#[derive(Serialize)]
struct NQueensResultJson {
    size: usize,
    queens: Vec<i32>,
    stats: StatsJson,
}

#[derive(Serialize)]
struct NQueensBatchJson {
    solutions: Vec<NQueensResultJson>,
}

/// Prints Sudoku solutions according to `options`.
pub fn print_sudoku_results(
    options: crate::puzzle_io::GlobalOptions,
    puzzle_index: usize,
    values: &[i32],
    stats: SearchStats,
    elapsed: Duration,
) {
    match options.format {
        OutputFormat::Plain => {
            if !options.quiet {
                if puzzle_index > 0 {
                    println!("--- puzzle {} ---", puzzle_index + 1);
                }
                print_sudoku_grid_plain(values);
            }
            if options.stats {
                print_stats_plain(stats, elapsed);
            }
        }
        OutputFormat::Json => {
            let payload = SudokuResultJson {
                puzzle_index,
                solution: to_grid(values),
                stats: stats_json(stats, elapsed),
            };
            println!("{}", serde_json::to_string(&payload).expect("json"));
        }
    }
}

/// Prints a batch of Sudoku results as JSON.
pub fn print_sudoku_batch_json(results: Vec<SudokuResultJson>) {
    let payload = SudokuBatchJson { solutions: results };
    println!("{}", serde_json::to_string(&payload).expect("json"));
}

/// Builds a JSON Sudoku result entry.
#[must_use]
pub fn sudoku_result_json(
    puzzle_index: usize,
    values: &[i32],
    stats: SearchStats,
    elapsed: Duration,
) -> SudokuResultJson {
    SudokuResultJson {
        puzzle_index,
        solution: to_grid(values),
        stats: stats_json(stats, elapsed),
    }
}

/// Prints N-Queens solutions according to `options`.
pub fn print_n_queens_results(
    options: crate::puzzle_io::GlobalOptions,
    size: usize,
    solutions: &[(Vec<(propaga_core::VariableId, i32)>, SearchStats, Duration)],
) {
    match options.format {
        OutputFormat::Plain => {
            for (index, (solution, stats, elapsed)) in solutions.iter().enumerate() {
                if options.all && !options.quiet {
                    println!("--- solution {} ---", index + 1);
                }
                if options.visual {
                    print_n_queens_board(size, solution);
                } else if !options.quiet {
                    print_n_queens_plain(size, solution);
                }
                if options.stats {
                    print_stats_plain(*stats, *elapsed);
                }
            }
        }
        OutputFormat::Json => {
            if options.all {
                let payload = NQueensBatchJson {
                    solutions: solutions
                        .iter()
                        .map(|(solution, stats, elapsed)| NQueensResultJson {
                            size,
                            queens: extract_columns(solution),
                            stats: stats_json(*stats, *elapsed),
                        })
                        .collect(),
                };
                println!("{}", serde_json::to_string(&payload).expect("json"));
            } else if let Some((solution, stats, elapsed)) = solutions.first() {
                let payload = NQueensResultJson {
                    size,
                    queens: extract_columns(solution),
                    stats: stats_json(*stats, *elapsed),
                };
                println!("{}", serde_json::to_string(&payload).expect("json"));
            }
        }
    }
}

fn print_sudoku_grid_plain(values: &[i32]) {
    println!("Sudoku solution:");
    println!("+-------+-------+-------+");
    for row in 0..9 {
        print!("| ");
        for col in 0..9 {
            print!("{} ", values[row * 9 + col]);
            if col == 2 || col == 5 {
                print!("| ");
            }
        }
        println!("|");
        if row == 2 || row == 5 {
            println!("+-------+-------+-------+");
        }
    }
    println!("+-------+-------+-------+");
}

fn print_n_queens_plain(size: usize, solution: &[(propaga_core::VariableId, i32)]) {
    println!("{size}-Queens solution (row -> column):");
    for (row, column) in extract_columns(solution).into_iter().enumerate() {
        println!("  row {row}: column {column}");
    }
}

fn print_n_queens_board(size: usize, solution: &[(propaga_core::VariableId, i32)]) {
    let columns = extract_columns(solution);
    println!("{size}-Queens board:");
    for row in 0..size {
        for col in 0..size {
            if columns.get(row).copied() == Some(col as i32) {
                print!("Q ");
            } else {
                print!(". ");
            }
        }
        println!();
    }
}

/// Prints a FlatZinc solution as name=value lines or formatted output directives.
pub(crate) fn print_flatzinc_result(
    names: &std::collections::HashMap<propaga_core::VariableId, String>,
    order: &[propaga_core::VariableId],
    solution: Option<&propaga_search::Solution>,
    outputs: &[propaga_flatzinc::OutputDirective],
    quiet: bool,
) {
    if quiet {
        return;
    }
    let Some(solution) = solution else {
        println!("UNSATISFIABLE");
        return;
    };

    if !outputs.is_empty() {
        for directive in outputs {
            print!("{}", format_output_directive(directive, names, solution));
        }
        println!();
        return;
    }

    let values: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    for var in order {
        let name = names.get(var).map(String::as_str).unwrap_or("var");
        if let Some(value) = values.get(var) {
            println!("{name} = {value}");
        }
    }
}

/// Formats a single output directive using the solution values.
#[must_use]
pub(crate) fn format_output_directive(
    directive: &propaga_flatzinc::OutputDirective,
    names: &std::collections::HashMap<propaga_core::VariableId, String>,
    solution: &propaga_search::Solution,
) -> String {
    use propaga_flatzinc::OutputSegment;

    let values: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    let name_to_var: std::collections::HashMap<_, _> =
        names.iter().map(|(var, name)| (name.as_str(), *var)).collect();

    let mut rendered = String::new();
    for segment in &directive.segments {
        match segment {
            OutputSegment::Text(text) => rendered.push_str(text),
            OutputSegment::Variable(name) => {
                if let Some(&var) = name_to_var.get(name.as_str()) {
                    if let Some(value) = values.get(&var) {
                        rendered.push_str(&value.to_string());
                    }
                }
            }
        }
    }
    rendered
}

/// Prints FlatZinc JSON including optional formatted outputs.
pub(crate) fn print_flatzinc_json(
    names: &std::collections::HashMap<propaga_core::VariableId, String>,
    order: &[propaga_core::VariableId],
    solution: Option<&propaga_search::Solution>,
    outputs: &[propaga_flatzinc::OutputDirective],
    objective_value: Option<i32>,
    direction: Option<propaga_search::ObjectiveDirection>,
    stats: Option<(SearchStats, Duration, u32)>,
) {
    use serde_json::json;

    let Some(solution) = solution else {
        println!("{}", json!({ "status": "unsatisfiable" }));
        return;
    };

    let values: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    let variables: std::collections::HashMap<String, i32> = order
        .iter()
        .filter_map(|var| {
            let name = names.get(var)?.clone();
            let value = *values.get(var)?;
            Some((name, value))
        })
        .collect();

    let formatted: Vec<String> = outputs
        .iter()
        .map(|directive| format_output_directive(directive, names, solution))
        .collect();

    let mut payload = json!({
        "status": "sat",
        "variables": variables,
    });
    if !formatted.is_empty() {
        payload["outputs"] = json!(formatted);
    }
    if let (Some(value), Some(direction)) = (objective_value, direction) {
        payload["objective"] = json!({
            "value": value,
            "direction": match direction {
                propaga_search::ObjectiveDirection::Minimize => "minimize",
                propaga_search::ObjectiveDirection::Maximize => "maximize",
            },
        });
    }
    if let Some((stats, elapsed, solutions_found)) = stats {
        payload["stats"] = json!({
            "nodes": stats.nodes,
            "backtracks": stats.backtracks,
            "conflicts": stats.conflicts,
            "nogoods_learned": stats.nogoods_learned,
            "restarts": stats.restarts,
            "elapsed_ms": elapsed.as_millis(),
            "solutions_found": solutions_found,
        });
    }
    println!("{}", payload);
}

/// Prints the optimized objective value in plain text.
pub(crate) fn print_objective_plain(
    value: i32,
    direction: Option<propaga_search::ObjectiveDirection>,
    quiet: bool,
) {
    if quiet {
        return;
    }
    match direction {
        Some(propaga_search::ObjectiveDirection::Minimize) => {
            println!("objective (minimize) = {value}");
        }
        Some(propaga_search::ObjectiveDirection::Maximize) => {
            println!("objective (maximize) = {value}");
        }
        None => println!("objective = {value}"),
    }
}

/// Prints a schedule solution with start/end times per task.
pub(crate) fn print_schedule_result(
    model: &Model,
    starts: &[propaga_core::VariableId],
    ends: &[propaga_core::VariableId],
    tasks: &[crate::schedule::ScheduleTaskSpec],
    solution: Option<&propaga_search::Solution>,
    quiet: bool,
) {
    if quiet {
        return;
    }
    let Some(solution) = solution else {
        println!("UNSATISFIABLE");
        return;
    };
    let values: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    let engine = model.engine();
    for (index, ((start, end), task)) in starts
        .iter()
        .zip(ends.iter())
        .zip(tasks.iter())
        .enumerate()
    {
        let start_time = values
            .get(start)
            .copied()
            .or_else(|| engine.domain(*start).fixed_value())
            .unwrap_or(-1);
        let end_time = engine
            .domain(*end)
            .fixed_value()
            .or_else(|| values.get(end).copied())
            .unwrap_or_else(|| {
                if start_time >= 0 {
                    start_time + task.duration
                } else {
                    -1
                }
            });
        println!(
            "task {}: start={start_time} duration={} end={end_time}",
            index + 1,
            task.duration
        );
    }
}

pub(crate) fn print_stats_plain(stats: SearchStats, elapsed: Duration) {
    println!(
        "stats: nodes={} backtracks={} conflicts={} nogoods={} restarts={} time={}ms",
        stats.nodes,
        stats.backtracks,
        stats.conflicts,
        stats.nogoods_learned,
        stats.restarts,
        elapsed.as_millis()
    );
}

fn stats_json(stats: SearchStats, elapsed: Duration) -> StatsJson {
    StatsJson {
        nodes: stats.nodes,
        backtracks: stats.backtracks,
        conflicts: stats.conflicts,
        nogoods_learned: stats.nogoods_learned,
        restarts: stats.restarts,
        elapsed_ms: elapsed.as_millis(),
    }
}

fn to_grid(values: &[i32]) -> Vec<Vec<i32>> {
    values
        .chunks(9)
        .map(<[i32]>::to_vec)
        .collect()
}

fn extract_columns(solution: &[(propaga_core::VariableId, i32)]) -> Vec<i32> {
    solution.iter().map(|(_, column)| *column).collect()
}
