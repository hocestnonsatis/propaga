//! Scheduling helpers for cumulative propagation.

/// Task specification for cumulative constraints.
#[derive(Clone, Copy, Debug)]
pub struct TaskSpec {
    /// Start time variable.
    pub start: propaga_core::VariableId,
    /// Fixed task duration.
    pub duration: i32,
    /// End time variable.
    pub end: propaga_core::VariableId,
    /// Resource demand while the task runs.
    pub demand: i32,
}

impl TaskSpec {
    /// Creates a task with unit resource demand.
    #[must_use]
    pub fn new(
        start: propaga_core::VariableId,
        duration: i32,
        end: propaga_core::VariableId,
    ) -> Self {
        Self {
            start,
            duration,
            end,
            demand: 1,
        }
    }

    /// Creates a task with explicit resource demand.
    #[must_use]
    pub fn with_demand(
        start: propaga_core::VariableId,
        duration: i32,
        end: propaga_core::VariableId,
        demand: i32,
    ) -> Self {
        Self {
            start,
            duration,
            end,
            demand,
        }
    }
}

/// Returns earliest start time from domain minimum.
#[must_use]
pub fn est(min_start: i32) -> i32 {
    min_start
}

/// Returns latest start time: latest completion minus duration.
#[must_use]
#[allow(dead_code)]
pub fn lst(max_end: i32, duration: i32) -> i32 {
    max_end - duration
}

/// Returns earliest completion time.
#[must_use]
pub fn ect(min_start: i32, duration: i32) -> i32 {
    min_start + duration
}

/// Returns latest completion time from domain maximum.
#[must_use]
pub fn lct(max_end: i32) -> i32 {
    max_end
}

/// Mandatory interval `[start, end)` for energetic reasoning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MandatoryInterval {
    /// Inclusive start.
    pub start: i32,
    /// Exclusive end.
    pub end: i32,
}

/// Computes mandatory part `[est, ect)` intersected with `[est, lct)`.
#[must_use]
pub fn mandatory_interval(est: i32, ect: i32, lct: i32) -> Option<MandatoryInterval> {
    let start = est;
    let end = ect.min(lct);
    if start < end {
        Some(MandatoryInterval { start, end })
    } else {
        None
    }
}

/// Returns `true` when mandatory demand exceeds capacity in `[start, end)`.
#[must_use]
#[allow(dead_code)]
pub fn detect_overload(
    intervals: &[(MandatoryInterval, i32)],
    capacity: i32,
    horizon_start: i32,
    horizon_end: i32,
) -> bool {
    find_overload_time(intervals, capacity, horizon_start, horizon_end).is_some()
}

/// Returns the earliest time where mandatory demand exceeds `capacity`.
#[must_use]
pub fn find_overload_time(
    intervals: &[(MandatoryInterval, i32)],
    capacity: i32,
    horizon_start: i32,
    horizon_end: i32,
) -> Option<i32> {
    if capacity <= 0 {
        return (!intervals.is_empty()).then_some(horizon_start);
    }

    let mut events: Vec<(i32, i32)> = Vec::new();
    for (interval, demand) in intervals {
        let start = interval.start.max(horizon_start);
        let end = interval.end.min(horizon_end);
        if start < end {
            events.push((start, *demand));
            events.push((end, -*demand));
        }
    }

    events.sort_unstable();
    let mut usage = 0;
    for (time, delta) in events {
        usage += delta;
        if usage > capacity {
            return Some(time);
        }
    }
    None
}

/// Mandatory interval with the start assignment that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MandatoryContribution {
    /// Mandatory interval.
    pub interval: MandatoryInterval,
    /// Resource demand during the interval.
    pub demand: i32,
    /// Start decision variable.
    pub start_var: propaga_core::VariableId,
    /// Start value that makes the interval mandatory.
    pub start_value: i32,
}

/// Returns start literals for mandatory tasks active at `time`.
#[must_use]
pub fn mandatory_literals_at_time(
    contributions: &[MandatoryContribution],
    time: i32,
) -> Vec<(propaga_core::VariableId, i32)> {
    let mut literals = Vec::new();
    for contribution in contributions {
        if contribution.interval.start <= time && time < contribution.interval.end {
            literals.push((contribution.start_var, contribution.start_value));
        }
    }
    literals.sort_unstable_by_key(|(var, _)| var.key());
    literals.dedup_by_key(|(var, _)| *var);
    literals
}

/// Time point for edge finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimePoint {
    /// Time coordinate.
    pub time: i32,
    /// Resource usage at this point after mandatory propagation.
    pub usage: i32,
}

/// Builds a time table of mandatory usage at integer time points.
#[must_use]
pub fn build_time_table(
    intervals: &[(MandatoryInterval, i32)],
    horizon_start: i32,
    horizon_end: i32,
) -> Vec<TimePoint> {
    let mut usage = vec![0; (horizon_end - horizon_start).max(0) as usize];
    for (interval, demand) in intervals {
        for t in interval.start.max(horizon_start)..interval.end.min(horizon_end) {
            let index = (t - horizon_start) as usize;
            if index < usage.len() {
                usage[index] += demand;
            }
        }
    }

    usage
        .into_iter()
        .enumerate()
        .map(|(index, load)| TimePoint {
            time: horizon_start + index as i32,
            usage: load,
        })
        .collect()
}

/// Returns the earliest time where mandatory usage in `table` exceeds `capacity`.
#[must_use]
pub fn find_excess_time(table: &[TimePoint], capacity: i32) -> Option<i32> {
    table
        .iter()
        .find(|point| point.usage > capacity)
        .map(|point| point.time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_demand_detects_overload() {
        let intervals = vec![
            (
                MandatoryInterval { start: 0, end: 2 },
                2,
            ),
            (
                MandatoryInterval { start: 0, end: 2 },
                2,
            ),
        ];
        assert_eq!(find_overload_time(&intervals, 2, 0, 3), Some(0));
    }

    #[test]
    fn find_excess_time_from_table() {
        let table = vec![
            TimePoint { time: 0, usage: 2 },
            TimePoint { time: 1, usage: 3 },
        ];
        assert_eq!(find_excess_time(&table, 2), Some(1));
    }

    #[test]
    fn detects_mandatory_overload() {
        let intervals = vec![
            (
                MandatoryInterval {
                    start: 0,
                    end: 3,
                },
                1,
            ),
            (
                MandatoryInterval {
                    start: 1,
                    end: 4,
                },
                1,
            ),
        ];
        assert!(detect_overload(&intervals, 1, 0, 5));
        assert_eq!(find_overload_time(&intervals, 1, 0, 5), Some(1));
    }

    #[test]
    fn mandatory_literals_include_only_overlapping_tasks() {
        use propaga_domains::IntervalDomain;
        use propaga_engine::Engine;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(0));
        let b = engine.new_variable(IntervalDomain::fix(4));
        let c = engine.new_variable(IntervalDomain::fix(0));
        let contributions = vec![
            MandatoryContribution {
                interval: MandatoryInterval { start: 0, end: 4 },
                demand: 1,
                start_var: a,
                start_value: 0,
            },
            MandatoryContribution {
                interval: MandatoryInterval { start: 4, end: 7 },
                demand: 1,
                start_var: b,
                start_value: 4,
            },
            MandatoryContribution {
                interval: MandatoryInterval { start: 0, end: 2 },
                demand: 1,
                start_var: c,
                start_value: 0,
            },
        ];
        let literals = mandatory_literals_at_time(&contributions, 0);
        assert_eq!(literals.len(), 2);
        assert!(literals.iter().any(|(var, _)| *var == a));
        assert!(literals.iter().any(|(var, _)| *var == c));
        assert!(!literals.iter().any(|(var, _)| *var == b));
    }
}
