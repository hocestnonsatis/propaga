use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Single task in a disjunctive (single-machine) constraint.
#[derive(Clone, Copy, Debug)]
pub struct DisjunctiveTask {
    /// Task start time variable.
    pub start: VariableId,
    /// Fixed task duration.
    pub duration: i32,
}

/// Propagates pairwise disjunctive scheduling: tasks do not overlap on one machine.
pub struct DisjunctivePropagator {
    watched: Vec<VariableId>,
    tasks: Vec<DisjunctiveTask>,
}

impl DisjunctivePropagator {
    /// Creates a disjunctive propagator over at least two tasks.
    #[must_use]
    pub fn new(tasks: impl Into<Vec<DisjunctiveTask>>) -> Self {
        let tasks = tasks.into();
        let mut watched = Vec::with_capacity(tasks.len());
        for task in &tasks {
            watched.push(task.start);
        }
        Self { watched, tasks }
    }
}

impl Propagator for DisjunctivePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        24
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        if self.tasks.len() < 2 {
            return PropagationStatus::OkNoChange;
        }

        for left in 0..self.tasks.len() {
            for right in left + 1..self.tasks.len() {
                if fixed_tasks_overlap(ctx, self.tasks[left], self.tasks[right]) {
                    if let Some(literals) =
                        overlap_conflict_literals(ctx, self.tasks[left], self.tasks[right])
                    {
                        ctx.record_propagator_conflict(&literals);
                    }
                    return PropagationStatus::Failure;
                }
            }
        }

        if self.tasks.len() > 2 && disjunctive_energy_overload(ctx, &self.tasks) {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        loop {
            let mut round_changed = false;
            round_changed |= propagate_edge_finding(ctx, &self.tasks);
            for left in 0..self.tasks.len() {
                for right in left + 1..self.tasks.len() {
                    if fixed_tasks_overlap(ctx, self.tasks[left], self.tasks[right]) {
                        if let Some(literals) =
                            overlap_conflict_literals(ctx, self.tasks[left], self.tasks[right])
                        {
                            ctx.record_propagator_conflict(&literals);
                        }
                        return PropagationStatus::Failure;
                    }
                    round_changed |= propagate_pair(ctx, self.tasks[left], self.tasks[right]);
                    round_changed |= forbid_overlap_with_known_start(
                        ctx,
                        self.tasks[left],
                        self.tasks[right],
                    );
                    round_changed |= forbid_overlap_with_known_start(
                        ctx,
                        self.tasks[right],
                        self.tasks[left],
                    );
                }
            }
            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if self
            .tasks
            .iter()
            .any(|task| ctx.domain(task.start).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn fixed_tasks_overlap(
    ctx: &dyn PropagationContext,
    left: DisjunctiveTask,
    right: DisjunctiveTask,
) -> bool {
    let (Some(left_start), Some(right_start)) =
        (ctx.fixed_value(left.start), ctx.fixed_value(right.start))
    else {
        return false;
    };
    let left_end = left_start.saturating_add(left.duration);
    let right_end = right_start.saturating_add(right.duration);
    left_end > right_start && right_end > left_start
}

fn overlap_conflict_literals(
    ctx: &dyn PropagationContext,
    left: DisjunctiveTask,
    right: DisjunctiveTask,
) -> Option<Vec<(VariableId, i32)>> {
    let (left_start, right_start) = (
        ctx.fixed_value(left.start)?,
        ctx.fixed_value(right.start)?,
    );
    Some(vec![
        (left.start, left_start),
        (right.start, right_start),
    ])
}

fn propagate_pair(
    ctx: &mut dyn PropagationContext,
    left: DisjunctiveTask,
    right: DisjunctiveTask,
) -> bool {
    let mut changed = false;
    changed |= enforce_before(ctx, left, right);
    changed |= enforce_before(ctx, right, left);
    changed
}

fn enforce_before(
    ctx: &mut dyn PropagationContext,
    before: DisjunctiveTask,
    after: DisjunctiveTask,
) -> bool {
    let Some(before_min_end) = ctx
        .domain(before.start)
        .min()
        .map(|start| start.saturating_add(before.duration))
    else {
        return false;
    };
    let Some(after_max_start) = ctx.domain(after.start).max() else {
        return false;
    };

    if before_min_end > after_max_start {
        return force_before(ctx, after, before);
    }

    false
}

fn force_before(
    ctx: &mut dyn PropagationContext,
    before: DisjunctiveTask,
    after: DisjunctiveTask,
) -> bool {
    let mut changed = false;

    if let (Some(before_min_start), Some(after_max_start)) = (
        ctx.domain(before.start).min(),
        ctx.domain(after.start).max(),
    ) {
        let min_after_start = before_min_start.saturating_add(before.duration);
        if ctx.remove_below(after.start, min_after_start) {
            changed = true;
        }
        let max_before_start = after_max_start.saturating_sub(before.duration);
        if ctx.remove_above(before.start, max_before_start) {
            changed = true;
        }
    }

    changed
}

fn known_start(ctx: &dyn PropagationContext, task: DisjunctiveTask) -> Option<i32> {
    if let Some(start) = ctx.fixed_value(task.start) {
        return Some(start);
    }
    if ctx.domain(task.start).size() == 1 {
        return ctx.domain(task.start).min();
    }
    None
}

fn forbid_overlap_with_known_start(
    ctx: &mut dyn PropagationContext,
    fixed: DisjunctiveTask,
    other: DisjunctiveTask,
) -> bool {
    let Some(fixed_start) = known_start(ctx, fixed) else {
        return false;
    };
    let fixed_end = fixed_start.saturating_add(fixed.duration);
    let mut changed = false;

    for value in start_values(ctx, other.start) {
        let other_end = value.saturating_add(other.duration);
        if value < fixed_end && other_end > fixed_start && ctx.remove_value(other.start, value) {
            changed = true;
        }
    }

    changed
}

fn start_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
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

#[derive(Clone, Copy, Debug)]
struct TaskBounds {
    est: i32,
    lst: i32,
    ect: i32,
}

fn task_bounds(ctx: &dyn PropagationContext, task: DisjunctiveTask) -> Option<TaskBounds> {
    let est = ctx.domain(task.start).min()?;
    let lst = ctx.domain(task.start).max()?;
    Some(TaskBounds {
        est,
        lst,
        ect: est.saturating_add(task.duration),
    })
}

fn disjunctive_energy_overload(ctx: &dyn PropagationContext, tasks: &[DisjunctiveTask]) -> bool {
    let Some(min_est) = tasks
        .iter()
        .filter_map(|task| ctx.domain(task.start).min())
        .min()
    else {
        return false;
    };
    let Some(max_lct) = tasks
        .iter()
        .filter_map(|task| {
            ctx.domain(task.start)
                .max()
                .map(|start| start.saturating_add(task.duration))
        })
        .max()
    else {
        return false;
    };
    let total_duration: i32 = tasks.iter().map(|task| task.duration).sum();
    total_duration > max_lct.saturating_sub(min_est)
}

fn theta_tree_ect(entries: &[(i32, i32)]) -> i32 {
    let mut sorted = entries.to_vec();
    sorted.sort_unstable_by_key(|(est, _)| *est);
    let mut completion = 0;
    for (est, duration) in sorted {
        completion = completion.max(est).saturating_add(duration);
    }
    completion
}

fn propagate_edge_finding(ctx: &mut dyn PropagationContext, tasks: &[DisjunctiveTask]) -> bool {
    let bounds: Vec<Option<TaskBounds>> = tasks
        .iter()
        .map(|&task| task_bounds(ctx, task))
        .collect();
    let mut changed = false;

    for (origin, origin_bounds) in bounds.iter().enumerate() {
        let Some(TaskBounds { lst, .. }) = origin_bounds else {
            continue;
        };

        let mut others: Vec<usize> = (0..tasks.len()).filter(|&index| index != origin).collect();
        others.sort_by_key(|&index| {
            bounds[index]
                .as_ref()
                .map(|bound| bound.ect)
                .unwrap_or(i32::MAX)
        });

        let mut tree_entries = Vec::new();
        for index in others {
            let Some(TaskBounds { est, ect, .. }) = bounds[index] else {
                continue;
            };
            if ect > *lst {
                continue;
            }
            tree_entries.push((est, tasks[index].duration));
            if theta_tree_ect(&tree_entries) > *lst {
                changed |= force_before(ctx, tasks[index], tasks[origin]);
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_start_forbids_overlapping_values() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 4,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 2,
            },
        ])));
        engine.fix_variable(start_a, 0).unwrap();
        engine.propagate_all().unwrap();
        assert!(!engine.domain(start_b).contains(0));
        assert!(!engine.domain(start_b).contains(1));
        assert!(!engine.domain(start_b).contains(2));
        assert!(!engine.domain(start_b).contains(3));
        assert!(engine.domain(start_b).contains(4));
    }

    #[test]
    fn overlapping_fixed_starts_record_conflict_literals() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        engine.fix_variable(start_b, 0).unwrap();
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 1,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 1,
            },
        ])));
        let _ = engine.propagate_all();

        let conflict = engine.last_conflict().expect("conflict");
        let literals = conflict
            .explanation
            .propagator_conflict_literals()
            .expect("propagator conflict");
        assert_eq!(literals.len(), 2);
        assert!(literals
            .iter()
            .any(|literal| literal.variable == start_a && literal.value == 0));
        assert!(literals
            .iter()
            .any(|literal| literal.variable == start_b && literal.value == 0));
    }

    #[test]
    fn overlapping_fixed_starts_fail() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 3,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 3,
            },
        ])));
        engine.fix_variable(start_a, 0).unwrap();
        let status = engine.fix_variable(start_b, 0).unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }

    #[test]
    fn sequential_starts_are_consistent() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 3,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 2,
            },
        ])));
        engine.fix_variable(start_a, 0).unwrap();
        let status = engine.fix_variable(start_b, 3).unwrap();
        assert_ne!(status, PropagationStatus::Failure);
    }

    #[test]
    fn energy_overload_detects_three_task_infeasibility() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 2));
        let start_b = engine.new_variable(IntervalDomain::new(0, 2));
        let start_c = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 2,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 2,
            },
            DisjunctiveTask {
                start: start_c,
                duration: 2,
            },
        ])));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::Failure
        );
    }
}
