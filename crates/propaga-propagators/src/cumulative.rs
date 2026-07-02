use crate::scheduling::{
    MandatoryContribution, MandatoryInterval, TaskSpec, build_time_table, ect, est,
    find_excess_time, find_overload_time, lct, mandatory_interval, mandatory_literals_at_time,
};
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a cumulative scheduling constraint with overload checking and edge finding.
pub struct CumulativePropagator {
    watched: Vec<VariableId>,
    tasks: Vec<TaskSpec>,
    capacity: i32,
}

impl CumulativePropagator {
    /// Creates a cumulative propagator over `tasks` with resource `capacity`.
    #[must_use]
    pub fn new(tasks: impl Into<Vec<TaskSpec>>, capacity: i32) -> Self {
        let tasks = tasks.into();
        let mut watched = Vec::with_capacity(tasks.len() * 2);
        for task in &tasks {
            watched.push(task.start);
            watched.push(task.end);
        }
        Self {
            watched,
            tasks,
            capacity,
        }
    }
}

impl Propagator for CumulativePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        25
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;
        loop {
            if let Some(literals) = cumulative_conflict_literals(ctx, &self.tasks, self.capacity) {
                ctx.record_propagator_conflict(&literals);
                return PropagationStatus::Failure;
            }

            let mut round_changed = false;
            round_changed |= propagate_precedence(ctx, &self.tasks);
            round_changed |= propagate_time_table(ctx, &self.tasks, self.capacity);
            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if self
            .tasks
            .iter()
            .any(|task| ctx.domain(task.start).is_empty() || ctx.domain(task.end).is_empty())
        {
            if let Some(literals) = cumulative_conflict_literals(ctx, &self.tasks, self.capacity) {
                ctx.record_propagator_conflict(&literals);
            }
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn propagate_precedence(ctx: &mut dyn PropagationContext, tasks: &[TaskSpec]) -> bool {
    let mut changed = false;
    for task in tasks {
        let duration = effective_duration(ctx, task);
        if let (Some(start_min), Some(end_max)) =
            (ctx.domain(task.start).min(), ctx.domain(task.end).max())
        {
            let min_end = start_min + duration;
            if ctx.remove_below(task.end, min_end) {
                changed = true;
            }
            let max_start = end_max - duration;
            if ctx.remove_above(task.start, max_start) {
                changed = true;
            }
        }
        if let Some(start) = ctx.fixed_value(task.start) {
            let end = start + duration;
            if ctx.remove_below(task.end, end) {
                changed = true;
            }
            if ctx.remove_above(task.end, end) {
                changed = true;
            }
        }
        if let Some(end) = ctx.fixed_value(task.end) {
            let start = end - duration;
            if ctx.remove_below(task.start, start) {
                changed = true;
            }
            if ctx.remove_above(task.start, start) {
                changed = true;
            }
        }
    }
    changed
}

fn effective_duration(ctx: &dyn PropagationContext, task: &TaskSpec) -> i32 {
    task.duration_var
        .and_then(|var| ctx.fixed_value(var))
        .unwrap_or(task.duration)
}

fn effective_demand(ctx: &dyn PropagationContext, task: &TaskSpec) -> i32 {
    task.demand_var
        .and_then(|var| ctx.fixed_value(var))
        .unwrap_or(task.demand)
}

fn cumulative_conflict_literals(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> Option<Vec<(VariableId, i32)>> {
    mandatory_overload_literals(ctx, tasks, capacity)
        .or_else(|| time_table_excess_literals(ctx, tasks, capacity))
}

fn mandatory_overload_literals(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> Option<Vec<(VariableId, i32)>> {
    let contributions = collect_mandatory_contributions(ctx, tasks);
    if contributions.is_empty() {
        return None;
    }

    let intervals = mandatory_intervals(&contributions);
    let (horizon_start, horizon_end) = interval_horizon(&intervals);

    let overload_time = find_overload_time(&intervals, capacity, horizon_start, horizon_end)?;
    Some(mandatory_literals_at_time(&contributions, overload_time))
}

fn time_table_excess_literals(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> Option<Vec<(VariableId, i32)>> {
    let contributions = collect_mandatory_contributions(ctx, tasks);
    if contributions.is_empty() {
        return None;
    }

    let intervals = mandatory_intervals(&contributions);
    let (horizon_start, horizon_end) = interval_horizon(&intervals);
    let table = build_time_table(&intervals, horizon_start, horizon_end);
    let excess_time = find_excess_time(&table, capacity)?;
    Some(mandatory_literals_at_time(&contributions, excess_time))
}

fn mandatory_intervals(contributions: &[MandatoryContribution]) -> Vec<(MandatoryInterval, i32)> {
    contributions
        .iter()
        .map(|contribution| (contribution.interval, contribution.demand))
        .collect()
}

fn interval_horizon(intervals: &[(MandatoryInterval, i32)]) -> (i32, i32) {
    let horizon_start = intervals
        .iter()
        .map(|(interval, _)| interval.start)
        .min()
        .unwrap_or(0);
    let horizon_end = intervals
        .iter()
        .map(|(interval, _)| interval.end)
        .max()
        .unwrap_or(0);
    (horizon_start, horizon_end)
}

fn collect_mandatory_contributions(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
) -> Vec<MandatoryContribution> {
    let mut contributions = Vec::new();
    for task in tasks {
        if let Some(start) = ctx.fixed_value(task.start) {
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval {
                    start,
                    end: start + task.duration,
                },
                demand: effective_demand(ctx, task),
                start_var: task.start,
                start_value: start,
            });
            continue;
        }

        if let Some(end) = ctx.fixed_value(task.end) {
            let start = end - task.duration;
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval { start, end },
                demand: effective_demand(ctx, task),
                start_var: task.start,
                start_value: start,
            });
            continue;
        }

        if ctx.domain(task.start).size() == 1 {
            let start = ctx.domain(task.start).min().expect("singleton");
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval {
                    start,
                    end: start + task.duration,
                },
                demand: effective_demand(ctx, task),
                start_var: task.start,
                start_value: start,
            });
            continue;
        }

        if ctx.domain(task.end).size() == 1 {
            let end = ctx.domain(task.end).max().expect("singleton");
            let start = end - task.duration;
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval { start, end },
                demand: effective_demand(ctx, task),
                start_var: task.start,
                start_value: start,
            });
        }
    }
    contributions
}

fn propagate_time_table(
    ctx: &mut dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> bool {
    let contributions = collect_mandatory_contributions(ctx, tasks);
    if contributions.is_empty() {
        return false;
    }

    let intervals = mandatory_intervals(&contributions);
    let (horizon_start, horizon_end) = interval_horizon(&intervals);
    let table = build_time_table(&intervals, horizon_start, horizon_end);
    let mut changed = false;

    for task in tasks {
        for point in &table {
            if point.usage > capacity && forbid_task_during(ctx, *task, point.time, point.time + 1)
            {
                changed = true;
            }
        }

        let Some(start_min) = ctx.domain(task.start).min() else {
            continue;
        };
        let Some(end_max) = ctx.domain(task.end).max() else {
            continue;
        };

        let forced = ctx.fixed_value(task.start).is_some()
            || ctx.fixed_value(task.end).is_some()
            || ctx.domain(task.start).size() == 1
            || ctx.domain(task.end).size() == 1;

        if !forced {
            continue;
        }

        if let Some(mandatory) =
            mandatory_interval(est(start_min), ect(start_min, task.duration), lct(end_max))
            && mandatory.end - mandatory.start >= task.duration
        {
            let fixed_start = mandatory.start;
            let fixed_end = mandatory.start + task.duration;
            if ctx.remove_below(task.start, fixed_start) {
                changed = true;
            }
            if ctx.remove_above(task.start, fixed_start) {
                changed = true;
            }
            if ctx.remove_below(task.end, fixed_end) {
                changed = true;
            }
            if ctx.remove_above(task.end, fixed_end) {
                changed = true;
            }
        }
    }

    changed
}

fn forbid_task_during(
    ctx: &mut dyn PropagationContext,
    task: TaskSpec,
    start: i32,
    end: i32,
) -> bool {
    let mut changed = false;
    for value in domain_values(ctx, task.start) {
        let task_end = value + task.duration;
        if value < end && task_end > start && ctx.remove_value(task.start, value) {
            changed = true;
        }
    }
    changed
}

fn domain_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
    let domain = ctx.domain(var);
    let mut values = Vec::new();
    if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
        for value in min..=max {
            if domain.contains(value) {
                values.push(value);
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn weighted_demand_overload_records_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(2, 4));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(2, 4));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 2, end_a, 2),
            TaskSpec::with_demand(start_b, 2, end_b, 2),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 2)));
        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        let _ = engine.fix_variable(start_b, 0);

        let conflict = engine.last_conflict().expect("conflict");
        let literals = conflict
            .explanation
            .propagator_conflict_literals()
            .expect("propagator conflict");
        assert_eq!(literals.len(), 2);
    }

    #[test]
    fn overload_records_mandatory_start_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(1, 3));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        let _ = engine.fix_variable(start_b, 0);

        let conflict = engine.last_conflict().expect("conflict");
        let literals = conflict
            .explanation
            .propagator_conflict_literals()
            .expect("propagator conflict");
        assert_eq!(literals.len(), 2);
        assert!(
            literals
                .iter()
                .any(|literal| literal.variable == start_a && literal.value == 0)
        );
        assert!(
            literals
                .iter()
                .any(|literal| literal.variable == start_b && literal.value == 0)
        );
    }

    #[test]
    fn two_unit_tasks_with_capacity_one_conflict() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(1, 3));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.fix_variable(start_a, 0).unwrap();
        engine.fix_variable(start_b, 0).unwrap();
        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn three_tasks_allow_sequential_starts() {
        let mut engine = Engine::new();
        let mut tasks = Vec::new();
        for duration in [4, 3, 2] {
            let start = engine.new_variable(IntervalDomain::new(0, 20));
            let end = engine.new_variable(IntervalDomain::new(duration, 24));
            tasks.push(TaskSpec::new(start, duration, end));
        }
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks.clone(), 1)));
        let status = engine.fix_variable(tasks[0].start, 0).unwrap();
        assert_ne!(status, PropagationStatus::Failure);
    }

    #[test]
    fn fixing_non_overlapping_starts_is_solved() {
        let mut engine = Engine::new();
        let start0 = engine.new_variable(IntervalDomain::new(0, 5));
        let end0 = engine.new_variable(IntervalDomain::new(2, 8));
        let start1 = engine.new_variable(IntervalDomain::new(0, 5));
        let end1 = engine.new_variable(IntervalDomain::new(3, 8));
        let tasks = vec![
            TaskSpec::new(start0, 2, end0),
            TaskSpec::new(start1, 3, end1),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.fix_variable(start0, 0).unwrap();
        engine.fix_variable(start1, 2).unwrap();
        let status = engine.propagate_all().unwrap();
        assert_ne!(status, PropagationStatus::Failure);
        assert!(engine.is_solved());
    }

    #[test]
    fn precedence_tightens_end_bounds() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(2, 5));
        let end = engine.new_variable(IntervalDomain::new(0, 10));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 2)));
        engine.propagate_all().unwrap();
        assert!(engine.domain(end).min().unwrap() >= 5);
    }
}
