use crate::scheduling::{
    EdgeFindingTask, MandatoryContribution, MandatoryInterval, TaskSpec, build_time_table, ect,
    edge_finding_new_est, edge_finding_new_lct, est, find_excess_time, find_overload_time, lct,
    mandatory_interval, mandatory_literals_at_time, residual_energy,
};
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a cumulative scheduling constraint with overload checking and edge finding.
#[derive(Clone)]
pub struct CumulativePropagator {
    watched: Vec<VariableId>,
    tasks: Vec<TaskSpec>,
    /// Fixed capacity when [`Self::capacity_var`] is `None`.
    capacity: i32,
    /// Optional variable capacity (domain bounds used as cap min/max).
    capacity_var: Option<VariableId>,
}

impl CumulativePropagator {
    /// Creates a cumulative propagator over `tasks` with fixed resource `capacity`.
    #[must_use]
    pub fn new(tasks: impl Into<Vec<TaskSpec>>, capacity: i32) -> Self {
        let tasks = tasks.into();
        let watched = watch_list(&tasks, None);
        Self {
            watched,
            tasks,
            capacity,
            capacity_var: None,
        }
    }

    /// Creates a cumulative propagator with a variable resource capacity.
    #[must_use]
    pub fn with_capacity_var(tasks: impl Into<Vec<TaskSpec>>, capacity: VariableId) -> Self {
        let tasks = tasks.into();
        let watched = watch_list(&tasks, Some(capacity));
        Self {
            watched,
            tasks,
            capacity: 0,
            capacity_var: Some(capacity),
        }
    }
}

fn watch_list(tasks: &[TaskSpec], capacity_var: Option<VariableId>) -> Vec<VariableId> {
    let mut watched = Vec::with_capacity(tasks.len() * 4 + usize::from(capacity_var.is_some()));
    for task in tasks {
        watched.push(task.start);
        watched.push(task.end);
        if let Some(duration) = task.duration_var {
            watched.push(duration);
        }
        if let Some(demand) = task.demand_var {
            watched.push(demand);
        }
    }
    if let Some(capacity) = capacity_var {
        watched.push(capacity);
    }
    watched
}

impl Propagator for CumulativePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        25
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some((cap_min, cap_max)) = capacity_bounds(ctx, self.capacity, self.capacity_var)
        else {
            return PropagationStatus::Failure;
        };

        let mut changed = false;
        loop {
            if let Some(literals) = cumulative_conflict_literals(ctx, &self.tasks, cap_max) {
                ctx.record_propagator_conflict(&literals);
                return PropagationStatus::Failure;
            }

            let mut round_changed = false;
            round_changed |= propagate_precedence(ctx, &self.tasks);
            round_changed |= propagate_time_table(ctx, &self.tasks, cap_max);
            round_changed |= propagate_edge_finding(ctx, &self.tasks, cap_max);
            round_changed |=
                tighten_capacity_lower_bound(ctx, &self.tasks, self.capacity_var, cap_min);
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
        if let Some(cap) = self.capacity_var
            && ctx.domain(cap).is_empty()
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

fn capacity_bounds(
    ctx: &dyn PropagationContext,
    fixed: i32,
    capacity_var: Option<VariableId>,
) -> Option<(i32, i32)> {
    match capacity_var {
        Some(var) => Some((ctx.domain(var).min()?, ctx.domain(var).max()?)),
        None => Some((fixed, fixed)),
    }
}

/// Raises variable capacity so it can cover the peak mandatory usage.
fn tighten_capacity_lower_bound(
    ctx: &mut dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity_var: Option<VariableId>,
    current_min: i32,
) -> bool {
    let Some(cap) = capacity_var else {
        return false;
    };
    let contributions = collect_mandatory_contributions(ctx, tasks);
    if contributions.is_empty() {
        return false;
    }
    let intervals = mandatory_intervals(&contributions);
    let (horizon_start, horizon_end) = interval_horizon(&intervals);
    let table = build_time_table(&intervals, horizon_start, horizon_end);
    let peak = table.iter().map(|point| point.usage).max().unwrap_or(0);
    if peak > current_min {
        ctx.remove_below(cap, peak)
    } else {
        false
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
    duration_min(ctx, task)
}

fn duration_min(ctx: &dyn PropagationContext, task: &TaskSpec) -> i32 {
    match task.duration_var {
        Some(var) => ctx.domain(var).min().unwrap_or(task.duration),
        None => task.duration,
    }
}

fn demand_min(ctx: &dyn PropagationContext, task: &TaskSpec) -> i32 {
    match task.demand_var {
        Some(var) => ctx.domain(var).min().unwrap_or(task.demand),
        None => task.demand,
    }
}

fn cumulative_conflict_literals(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> Option<Vec<(VariableId, i32)>> {
    mandatory_overload_literals(ctx, tasks, capacity)
        .or_else(|| time_table_excess_literals(ctx, tasks, capacity))
        .or_else(|| energy_overload_literals(ctx, tasks, capacity))
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

fn collect_edge_finding_tasks(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
) -> Vec<EdgeFindingTask> {
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        let Some(start_min) = ctx.domain(task.start).min() else {
            continue;
        };
        let Some(end_max) = ctx.domain(task.end).max() else {
            continue;
        };
        let duration = duration_min(ctx, task);
        let demand = demand_min(ctx, task);
        if duration <= 0 || demand <= 0 {
            continue;
        }
        let task_est = est(start_min);
        let task_lct = lct(end_max);
        // Leave horizon-infeasible tasks to precedence / domain wipeout.
        if task_est.saturating_add(duration) > task_lct {
            continue;
        }
        let energy = i64::from(duration) * i64::from(demand);
        out.push(EdgeFindingTask {
            start: task.start,
            end: task.end,
            est: task_est,
            lct: task_lct,
            duration,
            demand,
            energy,
        });
    }
    out
}

/// Energetic overload of an LCT-sorted prefix (classical cumulative EF).
fn energy_overload_literals(
    ctx: &dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> Option<Vec<(VariableId, i32)>> {
    let mut by_lct = collect_edge_finding_tasks(ctx, tasks);
    if by_lct.is_empty() {
        return None;
    }
    by_lct.sort_by_key(|task| (task.lct, task.est, task.start.key()));

    let mut energy: i64 = 0;
    let mut est_theta = i32::MAX;
    for (index, task) in by_lct.iter().enumerate() {
        energy += task.energy;
        est_theta = est_theta.min(task.est);
        let lct_theta = task.lct;
        if residual_energy(capacity, est_theta, lct_theta, energy) < 0 {
            return Some(
                by_lct[..=index]
                    .iter()
                    .map(|member| (member.start, member.est))
                    .collect(),
            );
        }
    }
    None
}

/// Classical edge-finding: energetic overload pruning of EST / LCT bounds.
fn propagate_edge_finding(
    ctx: &mut dyn PropagationContext,
    tasks: &[TaskSpec],
    capacity: i32,
) -> bool {
    let snapshot = collect_edge_finding_tasks(ctx, tasks);
    if snapshot.len() < 2 {
        return false;
    }

    let mut changed = false;
    changed |= edge_finding_update_est(ctx, &snapshot, capacity);
    changed |= edge_finding_update_lct(ctx, &snapshot, capacity);
    changed
}

fn edge_finding_update_est(
    ctx: &mut dyn PropagationContext,
    tasks: &[EdgeFindingTask],
    capacity: i32,
) -> bool {
    let mut by_lct = tasks.to_vec();
    by_lct.sort_by_key(|task| (task.lct, task.est, task.start.key()));

    let mut changed = false;
    let mut energy: i64 = 0;
    let mut est_theta = i32::MAX;
    for theta_end in 0..by_lct.len() {
        let theta_task = by_lct[theta_end];
        energy += theta_task.energy;
        est_theta = est_theta.min(theta_task.est);
        let lct_theta = theta_task.lct;
        let available = residual_energy(capacity, est_theta, lct_theta, energy);
        if available < 0 {
            continue;
        }

        for outer in &by_lct[theta_end + 1..] {
            let est_union = est_theta.min(outer.est);
            if energy + outer.energy
                <= i64::from(capacity) * (i64::from(lct_theta) - i64::from(est_union))
            {
                continue;
            }
            let diff = outer.energy - available;
            if diff <= 0 {
                continue;
            }
            let new_est = edge_finding_new_est(lct_theta, outer.duration, outer.demand, diff);
            if new_est > outer.est && ctx.remove_below(outer.start, new_est) {
                changed = true;
            }
        }
    }
    changed
}

fn edge_finding_update_lct(
    ctx: &mut dyn PropagationContext,
    tasks: &[EdgeFindingTask],
    capacity: i32,
) -> bool {
    let mut by_est = tasks.to_vec();
    by_est.sort_by_key(|task| {
        (
            std::cmp::Reverse(task.est),
            std::cmp::Reverse(task.lct),
            task.start.key(),
        )
    });

    let mut changed = false;
    let mut energy: i64 = 0;
    let mut lct_theta = i32::MIN;
    for theta_end in 0..by_est.len() {
        let theta_task = by_est[theta_end];
        energy += theta_task.energy;
        lct_theta = lct_theta.max(theta_task.lct);
        let est_theta = theta_task.est;
        let available = residual_energy(capacity, est_theta, lct_theta, energy);
        if available < 0 {
            continue;
        }

        for outer in &by_est[theta_end + 1..] {
            let lct_union = lct_theta.max(outer.lct);
            if energy + outer.energy
                <= i64::from(capacity) * (i64::from(lct_union) - i64::from(est_theta))
            {
                continue;
            }
            let diff = outer.energy - available;
            if diff <= 0 {
                continue;
            }
            let new_lct = edge_finding_new_lct(est_theta, outer.duration, outer.demand, diff);
            if new_lct < outer.lct && ctx.remove_above(outer.end, new_lct) {
                changed = true;
            }
            let new_lst = new_lct - outer.duration;
            if new_lst < outer.lct - outer.duration && ctx.remove_above(outer.start, new_lst) {
                changed = true;
            }
        }
    }
    changed
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
        let duration = duration_min(ctx, task);
        let demand = demand_min(ctx, task);
        if let Some(start) = ctx.fixed_value(task.start) {
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval {
                    start,
                    end: start + duration,
                },
                demand,
                start_var: task.start,
                start_value: start,
            });
            continue;
        }

        if let Some(end) = ctx.fixed_value(task.end) {
            let start = end - duration;
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval { start, end },
                demand,
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
                    end: start + duration,
                },
                demand,
                start_var: task.start,
                start_value: start,
            });
            continue;
        }

        if ctx.domain(task.end).size() == 1 {
            let end = ctx.domain(task.end).max().expect("singleton");
            let start = end - duration;
            contributions.push(MandatoryContribution {
                interval: MandatoryInterval { start, end },
                demand,
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
        let duration = duration_min(ctx, task);
        for point in &table {
            if point.usage > capacity
                && forbid_task_during(ctx, *task, duration, point.time, point.time + 1)
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
            mandatory_interval(est(start_min), ect(start_min, duration), lct(end_max))
            && mandatory.end - mandatory.start >= duration
        {
            let fixed_start = mandatory.start;
            let fixed_end = mandatory.start + duration;
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
    duration: i32,
    start: i32,
    end: i32,
) -> bool {
    let mut changed = false;
    for value in domain_values(ctx, task.start) {
        let task_end = value + duration;
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
    fn variable_capacity_raises_lower_bound_from_mandatory_usage() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::fix(2));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::fix(2));
        let capacity = engine.new_variable(IntervalDomain::new(0, 5));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 2, end_a, 2),
            TaskSpec::with_demand(start_b, 2, end_b, 1),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::with_capacity_var(
            tasks, capacity,
        )));
        assert!(!engine.commit_initial_propagation().unwrap().is_failure());
        assert_eq!(engine.hybrid_domain(capacity).min(), Some(3));
    }

    #[test]
    fn variable_capacity_conflicts_when_max_too_small() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::fix(1));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::fix(1));
        let capacity = engine.new_variable(IntervalDomain::new(0, 1));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::with_capacity_var(
            tasks, capacity,
        )));
        assert!(engine.commit_initial_propagation().unwrap().is_failure());
    }

    #[test]
    fn variable_duration_min_counts_in_mandatory_overload() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::new(2, 5));
        let dur_a = engine.new_variable(IntervalDomain::new(2, 4));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::new(2, 5));
        let dur_b = engine.new_variable(IntervalDomain::new(2, 4));
        let tasks = vec![
            TaskSpec::with_variable_spec(start_a, end_a, 2, Some(dur_a), 1, None),
            TaskSpec::with_variable_spec(start_b, end_b, 2, Some(dur_b), 1, None),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::new(tasks, 1)));
        assert!(engine.commit_initial_propagation().unwrap().is_failure());
    }

    #[test]
    fn weighted_demand_overload_records_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::fix(2));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::fix(2));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 2, end_a, 2),
            TaskSpec::with_demand(start_b, 2, end_b, 2),
        ];
        let ro = ReadOnlyEngine(&engine);
        let literals = mandatory_overload_literals(&ro, &tasks, 2).expect("overload");
        assert_eq!(literals.len(), 2);
    }

    #[test]
    fn overload_records_mandatory_start_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::fix(1));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::fix(1));
        let tasks = vec![
            TaskSpec::new(start_a, 1, end_a),
            TaskSpec::new(start_b, 1, end_b),
        ];
        let ro = ReadOnlyEngine(&engine);
        let literals = mandatory_overload_literals(&ro, &tasks, 1).expect("overload");
        assert_eq!(literals.len(), 2);
        assert!(
            literals
                .iter()
                .any(|(var, value)| *var == start_a && *value == 0)
        );
        assert!(
            literals
                .iter()
                .any(|(var, value)| *var == start_b && *value == 0)
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
        assert_ne!(
            engine.fix_variable(start_a, 0).unwrap(),
            PropagationStatus::Failure
        );
        // Edge-finding / capacity-1 reasoning removes the overlapping start.
        assert!(!engine.hybrid_domain(start_b).contains(0));
        assert_eq!(
            engine.fix_variable(start_b, 0).unwrap(),
            PropagationStatus::Failure
        );
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
        let start_a = engine.new_variable(IntervalDomain::fix(0));
        let end_a = engine.new_variable(IntervalDomain::fix(3));
        let start_b = engine.new_variable(IntervalDomain::fix(0));
        let end_b = engine.new_variable(IntervalDomain::fix(3));
        let tasks = vec![
            TaskSpec::with_demand(start_a, 3, end_a, 2),
            TaskSpec::with_demand(start_b, 3, end_b, 2),
        ];
        let ro = ReadOnlyEngine(&engine);
        assert!(
            time_table_excess_literals(&ro, &tasks, 2).is_some()
                || mandatory_overload_literals(&ro, &tasks, 2).is_some()
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
        assert!(forbid_task_during(&mut ctx, task, 2, 1, 3));
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

    #[test]
    fn edge_finding_prunes_start_beyond_time_table() {
        // Open domains: no mandatory contributions, so time-table is idle.
        // A,B pack [0,4) at capacity 2 with energy 6; C (demand 2, dur 2) is pushed.
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(3, 4));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(3, 4));
        let start_c = engine.new_variable(IntervalDomain::new(0, 4));
        let end_c = engine.new_variable(IntervalDomain::new(2, 6));
        let tasks = vec![
            TaskSpec::new(start_a, 3, end_a),
            TaskSpec::new(start_b, 3, end_b),
            TaskSpec::with_demand(start_c, 2, end_c, 2),
        ];
        let snapshot = {
            let ro = ReadOnlyEngine(&engine);
            collect_edge_finding_tasks(&ro, &tasks)
        };
        assert_eq!(snapshot.len(), 3);
        assert!(!propagate_time_table(
            &mut MutEngine(&mut engine),
            &tasks,
            2
        ));
        assert!(propagate_edge_finding(
            &mut MutEngine(&mut engine),
            &tasks,
            2
        ));
        assert!(engine.hybrid_domain(start_c).min().unwrap() >= 3);
    }

    #[test]
    fn edge_finding_energy_overload_records_est_literals() {
        let mut engine = Engine::new();
        // Two unit tasks of length 2 in a window of length 2 at capacity 1 → energy 4 > 2.
        let start_a = engine.new_variable(IntervalDomain::new(0, 0));
        let end_a = engine.new_variable(IntervalDomain::new(2, 2));
        let start_b = engine.new_variable(IntervalDomain::new(0, 0));
        let end_b = engine.new_variable(IntervalDomain::new(2, 2));
        let tasks = vec![
            TaskSpec::new(start_a, 2, end_a),
            TaskSpec::new(start_b, 2, end_b),
        ];
        let ro = ReadOnlyEngine(&engine);
        let literals = energy_overload_literals(&ro, &tasks, 1).expect("energy overload");
        assert_eq!(literals.len(), 2);
    }

    #[test]
    fn edge_finding_with_variable_duration_demand_and_capacity() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let end_a = engine.new_variable(IntervalDomain::new(3, 4));
        let dur_a = engine.new_variable(IntervalDomain::new(3, 5));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let end_b = engine.new_variable(IntervalDomain::new(3, 4));
        let dur_b = engine.new_variable(IntervalDomain::new(3, 5));
        let start_c = engine.new_variable(IntervalDomain::new(0, 4));
        let end_c = engine.new_variable(IntervalDomain::new(2, 6));
        let dem_c = engine.new_variable(IntervalDomain::new(2, 3));
        let capacity = engine.new_variable(IntervalDomain::fix(2));
        let tasks = vec![
            TaskSpec::with_variable_spec(start_a, end_a, 3, Some(dur_a), 1, None),
            TaskSpec::with_variable_spec(start_b, end_b, 3, Some(dur_b), 1, None),
            TaskSpec::with_variable_spec(start_c, end_c, 2, None, 2, Some(dem_c)),
        ];
        engine.add_propagator(Box::new(CumulativePropagator::with_capacity_var(
            tasks.clone(),
            capacity,
        )));
        assert!(!engine.commit_initial_propagation().unwrap().is_failure());
        assert!(engine.hybrid_domain(start_c).min().unwrap() >= 3);
    }

    #[test]
    fn edge_finding_symmetric_lct_prune() {
        let mut engine = Engine::new();
        // Same packing as EST prune, mirrored: late window for A,B pushes C's LCT down.
        let start_a = engine.new_variable(IntervalDomain::new(0, 1));
        let end_a = engine.new_variable(IntervalDomain::new(3, 4));
        let start_b = engine.new_variable(IntervalDomain::new(0, 1));
        let end_b = engine.new_variable(IntervalDomain::new(3, 4));
        let start_c = engine.new_variable(IntervalDomain::new(0, 4));
        let end_c = engine.new_variable(IntervalDomain::new(2, 6));
        let tasks = vec![
            TaskSpec::new(start_a, 3, end_a),
            TaskSpec::new(start_b, 3, end_b),
            TaskSpec::with_demand(start_c, 2, end_c, 2),
        ];
        let before_max = engine.hybrid_domain(end_c).max().unwrap();
        let changed = propagate_edge_finding(&mut MutEngine(&mut engine), &tasks, 2);
        assert!(changed || engine.hybrid_domain(start_c).min().unwrap() >= 3);
        if changed && engine.hybrid_domain(start_c).min().unwrap() < 3 {
            assert!(engine.hybrid_domain(end_c).max().unwrap() < before_max);
        }
    }
}
