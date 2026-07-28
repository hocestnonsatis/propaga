//! Search configuration and restart policies.

use propaga_core::VariableId;
use std::time::Duration;

/// Restart strategy for the search loop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RestartPolicy {
    /// Never restart.
    None,
    /// Restart after a fixed number of nodes.
    Constant { scale: u64 },
    /// Geometric restarts: scale * base^k.
    Geometric { base: f64, scale: u64 },
    /// Luby restarts with the given base node limit multiplier.
    Luby { base: u64 },
    /// Linear restarts: scale * (restart_index + 1).
    Linear { scale: u64 },
    /// Restart after each solution is found.
    OnSolution,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::Luby { base: 512 }
    }
}

impl RestartPolicy {
    /// Parses a restart policy from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let text = value.to_ascii_lowercase();
        match text.as_str() {
            "none" | "off" => Some(Self::None),
            _ if text.starts_with("constant:") => text
                .strip_prefix("constant:")
                .and_then(|scale| scale.parse().ok())
                .map(|scale| Self::Constant { scale }),
            _ if text.starts_with("geometric:") => {
                let params = text.strip_prefix("geometric:")?;
                let (base, scale) = params.split_once(':')?;
                let base = base.parse().ok()?;
                let scale = scale.parse().ok()?;
                if base <= 0.0 {
                    return None;
                }
                Some(Self::Geometric { base, scale })
            }
            "luby" => Some(Self::Luby { base: 512 }),
            _ if text.starts_with("luby:") => text
                .strip_prefix("luby:")
                .and_then(|base| base.parse().ok())
                .map(|base| Self::Luby { base }),
            _ if text.starts_with("linear:") => text
                .strip_prefix("linear:")
                .and_then(|scale| scale.parse().ok())
                .map(|scale| Self::Linear { scale }),
            "on-solution" | "on_solution" => Some(Self::OnSolution),
            _ => None,
        }
    }

    /// Returns the node limit before the next restart.
    #[must_use]
    pub fn node_limit(&self, restart_index: u32) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Constant { scale } => Some(*scale),
            Self::Geometric { base, scale } => {
                let limit = (*scale as f64) * base.powi(restart_index as i32);
                Some(float_node_limit(limit))
            }
            Self::Luby { base } => Some(base.saturating_mul(luby_sequence(restart_index))),
            Self::Linear { scale } => Some(scale.saturating_mul(u64::from(restart_index + 1))),
            Self::OnSolution => Some(0),
        }
    }
}

fn float_node_limit(limit: f64) -> u64 {
    if !limit.is_finite() || limit >= u64::MAX as f64 {
        u64::MAX
    } else if limit <= 0.0 {
        0
    } else {
        limit.floor() as u64
    }
}

/// Returns the Luby sequence value for `index`.
#[must_use]
pub fn luby_sequence(index: u32) -> u64 {
    let mut n = u64::from(index + 1);
    let mut size = 1u64;
    while n > size {
        n -= size;
        size *= 2;
    }
    if n > size / 2 { 2 * size - n } else { n }
}

/// Value ordering strategy during branch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ValueOrdering {
    /// Try values from smallest to largest.
    #[default]
    Ascending,
    /// Try values from largest to smallest.
    Descending,
    /// Least constraining value: prefer values that appear in fewer other domains.
    Lcv,
    /// Try values near the domain midpoint first (binary split style).
    Split,
    /// Like [`Split`], but prefer the upper half of the domain first.
    ReverseSplit,
    /// Try the median domain value first, then ascending.
    Median,
    /// Prefer the domain value closest to the mean of the current bounds.
    Middle,
    /// Deterministic pseudo-random order (stable for a given variable/domain).
    Random,
    /// Prefer values in the first contiguous domain interval; otherwise behave like [`Split`].
    ///
    /// For floats, splits at the leftmost interior hole when present.
    Interval,
}

impl ValueOrdering {
    /// Parses a value ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "asc" | "ascending" | "min" | "indomain_min" => Some(Self::Ascending),
            "desc" | "descending" | "max" | "indomain_max" => Some(Self::Descending),
            "lcv" => Some(Self::Lcv),
            "split" | "indomain_split" => Some(Self::Split),
            "reverse-split" | "reverse_split" | "indomain_reverse_split" => {
                Some(Self::ReverseSplit)
            }
            "median" | "indomain_median" => Some(Self::Median),
            "middle" | "indomain_middle" => Some(Self::Middle),
            "random" | "indomain_random" => Some(Self::Random),
            "interval" | "indomain_interval" => Some(Self::Interval),
            _ => None,
        }
    }
}

/// Variable ordering strategy during branch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VariableOrdering {
    /// Minimum remaining values / first-fail (default).
    #[default]
    Mrv,
    /// Prefer variables with the largest domain (anti-first-fail).
    Dom,
    /// Domain size divided by conflict weight (W-DEG style).
    DomWdeg,
    /// First unfixed variable in the configured search order.
    InputOrder,
    /// Activity-based ordering (VSIDS-style): prefer variables involved in recent conflicts.
    Activity,
    /// Prefer the variable whose current domain minimum is smallest.
    SmallestMin,
    /// Prefer the variable whose current domain maximum is largest.
    LargestMax,
    /// Prefer the variable with the largest gap between its two smallest domain values.
    MaxRegret,
}

impl VariableOrdering {
    /// Parses a variable ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mrv" | "size" | "first_fail" | "most_constrained" => Some(Self::Mrv),
            "dom" | "anti_first_fail" | "least_constrained" => Some(Self::Dom),
            "dom-wdeg" | "wdeg" | "domwdeg" | "dom_w_deg" | "occurrence" | "degree" => {
                Some(Self::DomWdeg)
            }
            "input" | "input-order" | "input_order" => Some(Self::InputOrder),
            "activity" | "vsids" => Some(Self::Activity),
            "smallest" => Some(Self::SmallestMin),
            "largest" => Some(Self::LargestMax),
            "max_regret" | "max-regret" => Some(Self::MaxRegret),
            _ => None,
        }
    }
}

/// One phase of a sequenced search (`seq_search` group).
#[derive(Clone, Debug, PartialEq)]
pub struct SearchPhase {
    /// Decision variables belonging to this phase.
    pub variables: Vec<VariableId>,
    /// Variable ordering used while this phase still has unfixed variables.
    pub variable_ordering: VariableOrdering,
    /// Value ordering used while this phase is active.
    pub value_ordering: ValueOrdering,
    /// Optional float precision from a nested `float_search` in this phase.
    ///
    /// When set, DFS uses it while this phase is active instead of
    /// [`SearchConfig::float_precision`].
    pub float_precision: Option<f64>,
}

impl SearchPhase {
    /// Creates a search phase.
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        variable_ordering: VariableOrdering,
        value_ordering: ValueOrdering,
    ) -> Self {
        Self {
            variables: variables.into(),
            variable_ordering,
            value_ordering,
            float_precision: None,
        }
    }

    /// Sets the float domain width treated as fixed for this phase.
    #[must_use]
    pub fn with_float_precision(mut self, precision: f64) -> Self {
        self.float_precision = Some(precision);
        self
    }
}

/// Configuration for depth-first search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchConfig {
    /// Enables nogood learning and backjumping.
    pub learning: bool,
    /// Enables lazy clause pruning during search.
    pub clause_learning: bool,
    /// Restart policy applied during search.
    pub restart_policy: RestartPolicy,
    /// Branch value ordering strategy.
    pub value_ordering: ValueOrdering,
    /// Branch variable ordering strategy.
    pub variable_ordering: VariableOrdering,
    /// Reuses the last assigned value as the first branch candidate after backtrack/restart.
    pub phase_saving: bool,
    /// Wall-clock time limit for search; `None` means no limit.
    pub time_limit: Option<Duration>,
    /// Float domain width at which a variable is treated as fixed (FlatZinc `float_search` precision).
    pub float_precision: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            learning: true,
            clause_learning: false,
            restart_policy: RestartPolicy::default(),
            value_ordering: ValueOrdering::default(),
            variable_ordering: VariableOrdering::default(),
            phase_saving: true,
            time_limit: None,
            float_precision: f64::EPSILON,
        }
    }
}

impl SearchConfig {
    /// Creates a config with learning disabled.
    #[must_use]
    pub fn without_learning() -> Self {
        Self {
            learning: false,
            restart_policy: RestartPolicy::None,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luby_sequence_starts_with_one() {
        assert_eq!(luby_sequence(0), 1);
        assert_eq!(luby_sequence(1), 1);
        assert_eq!(luby_sequence(2), 2);
        assert_eq!(luby_sequence(6), 4);
    }

    #[test]
    fn parses_restart_policy() {
        assert_eq!(RestartPolicy::parse("none"), Some(RestartPolicy::None));
        assert_eq!(
            RestartPolicy::parse("luby:256"),
            Some(RestartPolicy::Luby { base: 256 })
        );
        assert_eq!(
            RestartPolicy::parse("constant:100"),
            Some(RestartPolicy::Constant { scale: 100 })
        );
        assert_eq!(
            RestartPolicy::parse("geometric:1.5:100"),
            Some(RestartPolicy::Geometric {
                base: 1.5,
                scale: 100
            })
        );
        assert_eq!(
            RestartPolicy::Geometric {
                base: 2.0,
                scale: 10
            }
            .node_limit(3),
            Some(80)
        );
    }

    #[test]
    fn parses_variable_ordering() {
        assert_eq!(
            VariableOrdering::parse("dom-wdeg"),
            Some(VariableOrdering::DomWdeg)
        );
        assert_eq!(VariableOrdering::parse("mrv"), Some(VariableOrdering::Mrv));
        assert_eq!(
            VariableOrdering::parse("activity"),
            Some(VariableOrdering::Activity)
        );
        assert_eq!(
            VariableOrdering::parse("most_constrained"),
            Some(VariableOrdering::Mrv)
        );
        assert_eq!(
            VariableOrdering::parse("smallest"),
            Some(VariableOrdering::SmallestMin)
        );
        assert_eq!(
            VariableOrdering::parse("largest"),
            Some(VariableOrdering::LargestMax)
        );
        assert_eq!(
            VariableOrdering::parse("occurrence"),
            Some(VariableOrdering::DomWdeg)
        );
        assert_eq!(
            VariableOrdering::parse("dom_w_deg"),
            Some(VariableOrdering::DomWdeg)
        );
        assert_eq!(
            VariableOrdering::parse("max_regret"),
            Some(VariableOrdering::MaxRegret)
        );
        assert_eq!(
            VariableOrdering::parse("anti_first_fail"),
            Some(VariableOrdering::Dom)
        );
    }

    #[test]
    fn parses_value_ordering() {
        assert_eq!(
            ValueOrdering::parse("indomain_min"),
            Some(ValueOrdering::Ascending)
        );
        assert_eq!(
            ValueOrdering::parse("reverse-split"),
            Some(ValueOrdering::ReverseSplit)
        );
        assert_eq!(
            ValueOrdering::parse("indomain_interval"),
            Some(ValueOrdering::Interval)
        );
        assert_eq!(
            ValueOrdering::parse("indomain_random"),
            Some(ValueOrdering::Random)
        );
        assert_eq!(
            ValueOrdering::parse("indomain_middle"),
            Some(ValueOrdering::Middle)
        );
        assert_eq!(ValueOrdering::parse("nope"), None);
    }
}
