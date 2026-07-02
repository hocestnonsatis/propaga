use crate::output::{print_schedule_result, print_schedule_result_json, print_stats_plain};
use crate::puzzle_io::{GlobalOptions, OutputFormat};
use propaga_model::Model;
use propaga_propagators::{DisjunctiveTask, TaskSpec};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScheduleMode {
    Cumulative,
    Sequential,
    Disjunctive,
}

#[derive(Debug, Deserialize, Serialize)]
struct ScheduleSpec {
    capacity: i32,
    horizon: i32,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    sequential: bool,
    #[serde(default)]
    disjunctive: bool,
    tasks: Vec<ScheduleTaskSpec>,
}

fn resolve_schedule_mode(spec: &ScheduleSpec) -> Result<ScheduleMode, String> {
    if let Some(mode) = &spec.mode {
        return match mode.to_ascii_lowercase().as_str() {
            "cumulative" => Ok(ScheduleMode::Cumulative),
            "sequential" => Ok(ScheduleMode::Sequential),
            "disjunctive" => Ok(ScheduleMode::Disjunctive),
            other => Err(format!("unknown schedule mode `{other}`")),
        };
    }

    if spec.sequential && spec.disjunctive {
        return Err("schedule cannot set both sequential and disjunctive".into());
    }
    if spec.sequential {
        Ok(ScheduleMode::Sequential)
    } else if spec.disjunctive {
        Ok(ScheduleMode::Disjunctive)
    } else {
        Ok(ScheduleMode::Cumulative)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ScheduleTaskSpec {
    pub(crate) duration: i32,
    #[serde(default = "default_task_demand")]
    pub(crate) demand: i32,
}

fn default_task_demand() -> i32 {
    1
}

/// Solves a cumulative scheduling instance from JSON.
pub fn run(path: &Path, options: GlobalOptions) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let spec: ScheduleSpec = serde_json::from_str(&source)?;
    if spec.tasks.is_empty() {
        return Err("schedule must contain at least one task".into());
    }
    if spec.capacity <= 0 {
        return Err("capacity must be positive".into());
    }

    let mode = resolve_schedule_mode(&spec)?;

    let mut model = Model::new();
    let search_config = {
        let mut config = options.search_config();
        if mode != ScheduleMode::Sequential {
            config.value_ordering = propaga_search::ValueOrdering::Lcv;
        }
        config
    };
    model.set_search_config(search_config);

    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut tasks = Vec::new();

    for task in &spec.tasks {
        if task.duration <= 0 {
            return Err("task duration must be positive".into());
        }
        if task.demand <= 0 {
            return Err("task demand must be positive".into());
        }
        let start = model.int_var(0, spec.horizon);
        let duration = model.int_var_fixed(task.duration);
        let end = model.int_var(task.duration, spec.horizon + task.duration);
        model.linear_eq(start, duration, end);
        tasks.push(TaskSpec::with_demand(
            start,
            task.duration,
            end,
            task.demand,
        ));
        starts.push(start);
        ends.push(end);
    }

    match mode {
        ScheduleMode::Sequential => {
            for index in 0..tasks.len().saturating_sub(1) {
                model.equal(ends[index], starts[index + 1]);
            }
        }
        ScheduleMode::Disjunctive => {
            if spec.capacity != 1 {
                return Err("disjunctive schedule requires capacity 1".into());
            }
            model.disjunctive(
                tasks
                    .iter()
                    .map(|task| DisjunctiveTask {
                        start: task.start,
                        duration: task.duration,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        ScheduleMode::Cumulative => {
            model.cumulative(tasks, spec.capacity);
        }
    }

    let started = Instant::now();
    let (solution, stats) = model.solve_subset_with_stats(starts.clone());
    let elapsed = started.elapsed();

    match options.format {
        OutputFormat::Plain => {
            print_schedule_result(
                &model,
                &starts,
                &ends,
                &spec.tasks,
                solution.as_ref(),
                options.quiet,
            );
            if options.stats {
                print_stats_plain(stats, elapsed);
            }
        }
        OutputFormat::Json => {
            print_schedule_result_json(
                &model,
                &starts,
                &ends,
                &spec.tasks,
                solution.as_ref(),
                if options.stats {
                    Some((stats, elapsed))
                } else {
                    None
                },
            );
        }
    }

    if solution.is_none() {
        if stats.timed_out {
            return Err("timeout".into());
        }
        return Err("unsatisfiable schedule".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn single_task_start_zero_is_consistent() {
        let mut model = Model::new();
        let start = model.int_var(0, 20);
        let duration = model.int_var_fixed(4);
        let end = model.int_var(4, 24);
        model.linear_eq(start, duration, end);
        model.cumulative(vec![TaskSpec::new(start, 4, end)], 1);
        let status = model.engine_mut().fix_variable(start, 0).unwrap();
        assert_ne!(status, propaga_core::PropagationStatus::Failure);
        assert_eq!(model.engine().domain(end).fixed_value(), Some(4));
    }

    #[test]
    fn model_two_tasks_fixed_starts_become_solved() {
        let mut model = Model::new();
        let start0 = model.int_var(0, 5);
        let end0 = model.int_var(2, 8);
        let dur0 = model.int_var_fixed(2);
        model.linear_eq(start0, dur0, end0);
        let start1 = model.int_var(0, 5);
        let end1 = model.int_var(3, 8);
        let dur1 = model.int_var_fixed(3);
        model.linear_eq(start1, dur1, end1);
        model.cumulative(
            vec![
                TaskSpec::new(start0, 2, end0),
                TaskSpec::new(start1, 3, end1),
            ],
            1,
        );
        model.engine_mut().fix_variable(start0, 0).unwrap();
        model.engine_mut().fix_variable(start1, 2).unwrap();
        model.propagate().unwrap();
        assert!(model.engine().is_solved());
    }

    #[test]
    fn overlapping_second_fix_fails_fast() {
        let mut model = Model::new();
        let start0 = model.int_var(0, 5);
        let end0 = model.int_var(2, 8);
        let dur0 = model.int_var_fixed(2);
        model.linear_eq(start0, dur0, end0);
        let start1 = model.int_var(0, 5);
        let end1 = model.int_var(3, 8);
        let dur1 = model.int_var_fixed(3);
        model.linear_eq(start1, dur1, end1);
        model.cumulative(
            vec![
                TaskSpec::new(start0, 2, end0),
                TaskSpec::new(start1, 3, end1),
            ],
            1,
        );
        let engine = model.engine_mut();
        let level = engine.trail_mark();
        engine.fix_variable(start0, 0).unwrap();
        let status = engine.fix_variable(start1, 0).unwrap();
        assert_eq!(status, propaga_core::PropagationStatus::Failure);
        engine.trail_backtrack(level);
    }

    #[test]
    fn cumulative_two_tasks_solve_via_search() {
        let mut model = Model::new();
        model.set_search_config(propaga_search::SearchConfig {
            learning: false,
            restart_policy: propaga_search::RestartPolicy::None,
            value_ordering: propaga_search::ValueOrdering::Lcv,
            ..Default::default()
        });
        let start0 = model.int_var(0, 5);
        let end0 = model.int_var(2, 8);
        let dur0 = model.int_var_fixed(2);
        model.linear_eq(start0, dur0, end0);
        let start1 = model.int_var(0, 5);
        let end1 = model.int_var(3, 8);
        let dur1 = model.int_var_fixed(3);
        model.linear_eq(start1, dur1, end1);
        model.cumulative(
            vec![
                TaskSpec::new(start0, 2, end0),
                TaskSpec::new(start1, 3, end1),
            ],
            1,
        );
        let (solution, stats) = model.solve_subset_with_stats(vec![start0, start1]);
        assert!(
            stats.nodes < 500,
            "search did not terminate quickly (nodes={})",
            stats.nodes
        );
        assert!(solution.is_some(), "nodes={}", stats.nodes);
    }

    #[test]
    fn cumulative_solution_fixes_end_times() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_cumulative.json");
        let source = fs::read_to_string(path).unwrap();
        let spec: ScheduleSpec = serde_json::from_str(&source).unwrap();
        let mut model = Model::new();
        model.set_search_config(propaga_search::SearchConfig {
            learning: false,
            restart_policy: propaga_search::RestartPolicy::None,
            value_ordering: propaga_search::ValueOrdering::Lcv,
            ..Default::default()
        });
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut tasks = Vec::new();
        for task in &spec.tasks {
            let start = model.int_var(0, spec.horizon);
            let duration = model.int_var_fixed(task.duration);
            let end = model.int_var(task.duration, spec.horizon + task.duration);
            model.linear_eq(start, duration, end);
            tasks.push(TaskSpec::with_demand(
                start,
                task.duration,
                end,
                task.demand,
            ));
            starts.push(start);
            ends.push(end);
        }
        model.cumulative(tasks, spec.capacity);
        let (solution, _) = model.solve_subset_with_stats(starts.clone());
        assert!(solution.is_some());
        for (index, end) in ends.iter().enumerate() {
            let expected = starts[index];
            let start_time = model.engine().domain(expected).fixed_value().unwrap();
            assert_eq!(
                model.engine().domain(*end).fixed_value(),
                Some(start_time + spec.tasks[index].duration)
            );
        }
    }

    #[test]
    fn cumulative_demand_schedule_with_learning() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_cumulative_demand.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                learning: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn cumulative_schedule_with_learning_enabled() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_cumulative.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                learning: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn rejects_sequential_and_disjunctive_together() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_disjunctive.json");
        let mut spec: ScheduleSpec =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        spec.sequential = true;
        let temp = std::env::temp_dir().join("propaga_schedule_conflict_test.json");
        fs::write(&temp, serde_json::to_string(&spec).unwrap()).unwrap();
        assert!(
            run(
                &temp,
                GlobalOptions {
                    quiet: true,
                    ..GlobalOptions::default()
                },
            )
            .is_err()
        );
        let _ = fs::remove_file(temp);
    }

    #[test]
    fn disjunctive_schedule_with_learning_enabled() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_disjunctive.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                learning: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn cumulative_capacity_two_schedule_with_learning() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_cumulative_cap2.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                learning: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_disjunctive_schedule_json() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_disjunctive.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_three_task_schedule() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_three_tasks.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn solves_cumulative_schedule_without_sequential_mode() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchmarks/schedule_cumulative.json");
        run(
            &path,
            GlobalOptions {
                quiet: true,
                ..GlobalOptions::default()
            },
        )
        .unwrap();
    }
}
