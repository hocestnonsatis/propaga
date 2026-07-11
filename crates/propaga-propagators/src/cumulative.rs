use crate::scheduling::{
    MandatoryContribution, MandatoryInterval, TaskSpec, build_time_table, ect, est,
    find_excess_time, find_overload_time, lct, mandatory_interval, mandatory_literals_at_time,
};
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a cumulative scheduling constraint with overload checking and edge finding.
#[derive(Clone)]
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
            return PropagationStatus::Failure;
        }
        if changed {
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
            changed |= tighten_to_point(ctx, task.start, fixed_start);
            changed |= tighten_to_point(ctx, task.end, fixed_end);
        }
    }

    changed
}

fn tighten_to_point(ctx: &mut dyn PropagationContext, var: VariableId, value: i32) -> bool {
    let mut changed = false;
    changed |= ctx.remove_below(var, value);
    changed |= ctx.remove_above(var, value);
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
    use crate::test_support::{MutEngine, ReadOnlyEngine};
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
        assert!(engine.hybrid_domain(end).min().unwrap() >= 5);
    }

    #[test]
    fn fixed_start_fixes_end() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::fix(4));
        let end = engine.new_variable(IntervalDomain::new(4, 12));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(7));
    }

    #[test]
    fn fixed_end_fixes_start() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 10));
        let end = engine.new_variable(IntervalDomain::fix(9));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(6));
    }

    #[test]
    fn precedence_tightens_start_upper_bound() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 10));
        let end = engine.new_variable(IntervalDomain::new(2, 5));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(start).max().unwrap() <= 2);
    }

    #[test]
    fn variable_duration_and_demand_from_fixed_vars() {
        let mut engine = Engine::new();
        let duration_var = engine.new_variable(IntervalDomain::fix(4));
        let demand_var = engine.new_variable(IntervalDomain::fix(2));
        let start = engine.new_variable(IntervalDomain::new(2, 5));
        let end = engine.new_variable(IntervalDomain::new(0, 12));
        let tasks = vec![TaskSpec::with_variable_spec(
            start,
            end,
            2,
            Some(duration_var),
            1,
            Some(demand_var),
        )];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 2)));
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(end).min().unwrap() >= 6);
    }

    #[test]
    fn feasible_schedule_no_change() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(5, 10));
        let end = engine.new_variable(IntervalDomain::new(8, 13));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 2)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn time_table_prunes_overloaded_starts() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(2, 6));
        let start_b = engine.new_variable(IntervalDomain::fix(1));
        let end_b = engine.new_variable(IntervalDomain::new(3, 8));
        let tasks = vec![
            TaskSpec::new(start_a, 2, end_a),
            TaskSpec::new(start_b, 2, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn singleton_end_domain_collects_mandatory_contribution() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 8));
        let end = engine.new_variable(IntervalDomain::fix(6));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(4));
    }

    #[test]
    fn mandatory_interval_from_singleton_start_tightens_bounds() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::fix(2));
        let end = engine.new_variable(IntervalDomain::new(4, 10));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(2));
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(4));
    }

    #[test]
    fn fixed_end_contributes_mandatory_interval() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 6));
        let end_a = engine.new_variable(IntervalDomain::fix(8));
        let start_b = engine.new_variable(IntervalDomain::new(0, 6));
        let end_b = engine.new_variable(IntervalDomain::fix(8));
        let tasks = vec![
            TaskSpec::new(start_a, 2, end_a),
            TaskSpec::new(start_b, 2, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.trail_mark();
        engine.fix_variable(start_a, 6).unwrap();
        let _ = engine.fix_variable(start_b, 6);

        let conflict = engine.last_conflict().expect("conflict");
        let literals = conflict
            .explanation
            .propagator_conflict_literals()
            .expect("propagator conflict");
        assert_eq!(literals.len(), 2);
    }

    #[test]
    fn empty_domain_records_conflict_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::new(1, 3));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.trail_mark();
        let _ = engine.propagate_all();
        let conflict = engine.last_conflict().expect("conflict");
        assert!(
            conflict
                .explanation
                .propagator_conflict_literals()
                .is_some()
        );
    }

    #[test]
    fn singleton_start_without_fix_contributes_mandatory() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(2, 2));
        let end = engine.new_variable(IntervalDomain::new(4, 10));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(end).min(), Some(4));
    }

    #[test]
    fn time_table_excess_conflict_records_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(3, 6));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(3, 8));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 3, end_a, 2),
            TaskSpec::with_demand(start_b, 3, end_b, 2),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 2)));
        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        let _ = engine.fix_variable(start_b, 0);
        let conflict = engine.last_conflict().expect("conflict");
        assert!(
            conflict
                .explanation
                .propagator_conflict_literals()
                .is_some()
        );
    }

    #[test]
    fn fixed_start_and_end_tighten_both_bounds() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::fix(3));
        let end = engine.new_variable(IntervalDomain::new(3, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(5));

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 10));
        let end = engine.new_variable(IntervalDomain::fix(9));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(6));
    }

    #[test]
    fn propagate_time_table_directly_prunes_overlapping_start() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(2, 6));
        let start_b = engine.new_variable(IntervalDomain::fix(1));
        let end_b = engine.new_variable(IntervalDomain::new(3, 8));
        let start_c = engine.new_variable(IntervalDomain::new(0, 4));
        let end_c = engine.new_variable(IntervalDomain::new(2, 10));
        let tasks = vec![
            TaskSpec::new(start_a, 2, end_a),
            TaskSpec::new(start_b, 2, end_b),
            TaskSpec::new(start_c, 2, end_c),
        ];
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_time_table(&mut ctx, &tasks, 1));
        assert!(!engine.hybrid_domain(start_c).contains(0));
    }

    #[test]
    fn forbid_task_during_removes_overlapping_values() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 4).remove(2));
        let end = engine.new_variable(IntervalDomain::new(2, 10));
        let task = TaskSpec::new(start, 2, end);
        let mut ctx = MutEngine(&mut engine);
        assert!(forbid_task_during(&mut ctx, task, 1, 3));
        assert!(!engine.hybrid_domain(start).contains(0));
        assert!(!engine.hybrid_domain(start).contains(1));
        assert!(engine.hybrid_domain(start).contains(3));
    }

    #[test]
    fn fixed_start_tightens_end_from_both_sides() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::fix(3));
        let end = engine.new_variable(IntervalDomain::new(5, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(5));
    }

    #[test]
    fn fixed_end_tightens_start_from_both_sides() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 10));
        let end = engine.new_variable(IntervalDomain::fix(11));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(9));
    }

    #[test]
    fn singleton_end_without_fix_contributes_mandatory() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 8));
        let end = engine.new_variable(IntervalDomain::new(6, 6));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(4));
    }

    #[test]
    fn mandatory_interval_from_forced_singleton_start() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(2, 8));
        let start_b = engine.new_variable(IntervalDomain::new(0, 0));
        let end_b = engine.new_variable(IntervalDomain::new(2, 10));
        let tasks = vec![
            TaskSpec::new(start_a, 4, end_a),
            TaskSpec::new(start_b, 2, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(start_b).fixed_value(), Some(0));
    }

    #[test]
    fn collect_mandatory_contributions_from_singleton_domains() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(2, 2));
        let end = engine.new_variable(IntervalDomain::new(6, 6));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = ReadOnlyEngine(&engine);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].start_value, 2);
    }

    #[test]
    fn time_table_excess_literals_detects_weighted_overload() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(3, 6));
        let start_b = engine.new_variable(IntervalDomain::fix(1));
        let end_b = engine.new_variable(IntervalDomain::new(4, 8));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 3, end_a, 2),
            TaskSpec::with_demand(start_b, 3, end_b, 2),
        ];
        let ro = ReadOnlyEngine(&engine);
        assert!(time_table_excess_literals(&ro, &tasks, 2).is_some());
    }

    #[test]
    fn empty_domain_failure_after_precedence_pruning() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(5, 10));
        let end = engine.new_variable(IntervalDomain::new(0, 3));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(engine.hybrid_domain(end).is_empty());
    }

    #[test]
    fn empty_final_domain_records_conflict_literals() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(10, 12));
        let end = engine.new_variable(IntervalDomain::new(0, 5));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.trail_mark();
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(engine.last_conflict().is_some());
    }

    #[test]
    fn propagate_precedence_fixed_start_tightens_end() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::fix(3));
        let end = engine.new_variable(IntervalDomain::new(4, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(5));
    }

    #[test]
    fn propagate_precedence_fixed_end_tightens_start() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 10));
        let end = engine.new_variable(IntervalDomain::fix(9));
        let tasks = vec![TaskSpec::new(start, 3, end)];
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(engine.hybrid_domain(start).fixed_value(), Some(6));
    }

    #[test]
    fn collect_mandatory_from_singleton_end_only() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 8));
        let end = engine.new_variable(IntervalDomain::new(6, 6));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = ReadOnlyEngine(&engine);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 4);
    }

    #[test]
    fn collect_mandatory_from_singleton_start_only() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(3, 3));
        let end = engine.new_variable(IntervalDomain::new(5, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = ReadOnlyEngine(&engine);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 3);
    }

    #[test]
    fn domain_values_skips_empty_domain() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(1, 0));
        let ctx = ReadOnlyEngine(&engine);
        assert!(domain_values(&ctx, start).is_empty());
    }

    #[test]
    fn propagate_time_table_tightens_forced_singleton() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(2, 2));
        let end = engine.new_variable(IntervalDomain::new(4, 10));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_time_table(&mut ctx, &tasks, 1));
        assert_eq!(engine.hybrid_domain(end).fixed_value(), Some(4));
    }

    #[test]
    fn propagate_time_table_skips_task_without_bounds() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(1, 0));
        let end = engine.new_variable(IntervalDomain::new(2, 8));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MutEngine(&mut engine);
        assert!(!propagate_time_table(&mut ctx, &tasks, 1));
    }

    #[test]
    fn empty_domain_failure_records_literals_when_available() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::new(1, 0));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        engine.trail_mark();
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn collect_mandatory_singleton_start_branch() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(4, 4));
        let end = engine.new_variable(IntervalDomain::new(6, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = ReadOnlyEngine(&engine);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 4);
    }

    #[test]
    fn collect_mandatory_singleton_end_branch() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 8));
        let end = engine.new_variable(IntervalDomain::new(8, 8));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = ReadOnlyEngine(&engine);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 6);
    }

    #[test]
    fn propagate_time_table_skips_non_forced_tasks() {
        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 8));
        let end = engine.new_variable(IntervalDomain::new(2, 12));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MutEngine(&mut engine);
        assert!(!propagate_time_table(&mut ctx, &tasks, 1));
    }

    #[test]
    fn collect_mandatory_open_singleton_start_branch() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = MockIntCtx::new()
            .with_domain(start, vec![2])
            .with_domain(end, vec![4, 5, 6, 7]);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 2);
        assert_eq!(contributions[0].interval.end, 4);
    }

    #[test]
    fn collect_mandatory_open_singleton_end_branch() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3])
            .with_domain(end, vec![6]);
        let contributions = collect_mandatory_contributions(&ctx, &tasks);
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].interval.start, 4);
        assert_eq!(contributions[0].interval.end, 6);
    }

    #[test]
    fn propagate_precedence_fixed_start_tightens_both_end_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![3])
            .with_domain(end, vec![1, 2, 3, 4, 5, 6, 7, 8])
            .with_fixed(start, 3);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&end].values.borrow().as_slice(), &[5]);
    }

    #[test]
    fn propagate_precedence_fixed_end_tightens_both_start_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3, 4, 5, 6, 7, 8])
            .with_domain(end, vec![9])
            .with_fixed(end, 9);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&start].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn propagate_time_table_tightens_open_singleton_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![2])
            .with_domain(end, vec![2, 3, 4, 5, 6, 7, 8]);
        assert!(propagate_time_table(&mut ctx, &tasks, 1));
        assert_eq!(ctx.domains[&start].values.borrow().as_slice(), &[2]);
        assert_eq!(ctx.domains[&end].values.borrow().as_slice(), &[4]);
    }

    #[test]
    fn propagate_time_table_skips_task_with_empty_end_max() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![2])
            .with_domain(end, vec![]);
        assert!(!propagate_time_table(&mut ctx, &tasks, 1));
    }

    #[test]
    fn empty_domain_after_loop_records_conflict_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::new(1, 3));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_precedence(
            &mut ctx,
            &[
                TaskSpec::new(start_a, 1, end_a),
                TaskSpec::new(start_b, 1, end_b),
            ],
        ));
        engine.set_domain(
            end_b,
            propaga_domains::AnyDomain::Int(propaga_domains::HybridDomain::Interval(
                IntervalDomain::new(1, 0),
            )),
        );
        engine.trail_mark();
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        assert!(engine.last_conflict().is_some());
    }

    #[test]
    fn propagate_precedence_fixed_bounds_remove_both_sides() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![3])
            .with_domain(end, vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
            .with_fixed(start, 3);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&end].values.borrow().as_slice(), &[5]);

        let start2 = engine.new_variable(IntervalDomain::new(0, 0));
        let end2 = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks2 = vec![TaskSpec::new(start2, 2, end2)];
        let mut ctx2 = MockIntCtx::new()
            .with_domain(start2, vec![0, 1, 2, 3, 4, 5, 6, 7, 8])
            .with_domain(end2, vec![9])
            .with_fixed(end2, 9);
        assert!(propagate_precedence(&mut ctx2, &tasks2));
        assert_eq!(ctx2.domains[&start2].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn mock_propagate_precedence_fixed_start_tightens_end_both_ways() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![5])
            .with_domain(end, vec![5, 6, 7, 8])
            .with_fixed(start, 5);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&end].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn mock_propagate_precedence_fixed_end_tightens_start_both_ways() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3, 4, 5])
            .with_domain(end, vec![7])
            .with_fixed(end, 7);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&start].values.borrow().as_slice(), &[5]);
    }

    #[test]
    fn mock_propagate_time_table_tightens_open_singleton_mandatory_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let start_b = engine.new_variable(IntervalDomain::new(0, 0));
        let end_b = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![
            TaskSpec::new(start, 2, end),
            TaskSpec::new(start_b, 1, end_b),
        ];
        let mut ctx = MockIntCtx::new()
            .with_open_singleton(start, 2)
            .with_domain(end, vec![2, 3, 4, 5, 6, 7, 8])
            .with_open_singleton(start_b, 0)
            .with_domain(end_b, vec![1]);
        assert!(propagate_time_table(&mut ctx, &tasks, 1));
        assert_eq!(ctx.domain_values(start), vec![2]);
        assert_eq!(ctx.domain_values(end), vec![4]);
    }

    #[test]
    fn empty_domain_after_propagation_records_conflict_literals() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0])
            .with_domain(end, vec![]);
        let mut prop = CumulativePropagator::new(vec![TaskSpec::new(start, 1, end)], 1);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn cumulative_failure_records_literals_on_empty_domain() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(1, 3));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::new(1, 0));
        engine.trail_mark();
        engine.add_propagator(Box::new(CumulativePropagator::new(
            vec![
                TaskSpec::new(start_a, 1, end_a),
                TaskSpec::new(start_b, 1, end_b),
            ],
            1,
        )));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn mock_cumulative_records_conflict_after_loop_empties_domain() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 0));
        let end_a = engine.new_variable(IntervalDomain::new(0, 0));
        let start_b = engine.new_variable(IntervalDomain::new(0, 0));
        let end_b = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(start_a, vec![0])
            .with_domain(end_a, vec![0, 1])
            .with_domain(start_b, vec![10, 11, 12])
            .with_domain(end_b, vec![13, 14, 15])
            .with_fixed(start_a, 0);
        let mut prop = CumulativePropagator::new(
            vec![
                TaskSpec::new(start_a, 3, end_a),
                TaskSpec::new(start_b, 3, end_b),
            ],
            1,
        );
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
        assert!(ctx.domains[&end_a].values.borrow().is_empty());
    }

    #[test]
    fn mock_propagate_precedence_fixed_start_only_block_tightens_end() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3, 4, 5])
            .with_domain(end, vec![6, 7, 8])
            .with_fixed(start, 5);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&end].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn mock_propagate_precedence_fixed_end_only_block_tightens_start() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![TaskSpec::new(start, 2, end)];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])
            .with_domain(end, vec![7, 8, 9, 10])
            .with_fixed(end, 9);
        assert!(propagate_precedence(&mut ctx, &tasks));
        assert_eq!(ctx.domains[&start].values.borrow().as_slice(), &[7]);
    }

    #[test]
    fn mock_propagate_time_table_tightens_all_mandatory_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let start = engine.new_variable(IntervalDomain::new(0, 0));
        let end = engine.new_variable(IntervalDomain::new(0, 0));
        let start_b = engine.new_variable(IntervalDomain::new(0, 0));
        let end_b = engine.new_variable(IntervalDomain::new(0, 0));
        let tasks = vec![
            TaskSpec::new(start, 2, end),
            TaskSpec::new(start_b, 1, end_b),
        ];
        let mut ctx = MockIntCtx::new()
            .with_domain(start, vec![0, 1, 2, 3])
            .with_domain(end, vec![2, 3, 4, 5, 6, 7, 8])
            .with_fixed(start, 2)
            .with_open_singleton(start_b, 0)
            .with_domain(end_b, vec![1]);
        assert!(propagate_time_table(&mut ctx, &tasks, 1));
    }
}
