//! Search configuration and restart policies.

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
    /// Try the median domain value first, then ascending.
    Median,
}

impl ValueOrdering {
    /// Parses a value ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "asc" | "ascending" | "min" => Some(Self::Ascending),
            "desc" | "descending" | "max" => Some(Self::Descending),
            "lcv" => Some(Self::Lcv),
            "split" | "indomain_split" => Some(Self::Split),
            "median" | "indomain_median" => Some(Self::Median),
            _ => None,
        }
    }
}

/// Variable ordering strategy during branch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VariableOrdering {
    /// Minimum remaining values (default).
    #[default]
    Mrv,
    /// Domain size over minimum, tie-break by index.
    Dom,
    /// Domain size divided by conflict weight (W-DEG style).
    DomWdeg,
    /// First unfixed variable in the configured search order.
    InputOrder,
    /// Activity-based ordering (VSIDS-style): prefer variables involved in recent conflicts.
    Activity,
}

impl VariableOrdering {
    /// Parses a variable ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mrv" | "size" => Some(Self::Mrv),
            "dom" => Some(Self::Dom),
            "dom-wdeg" | "wdeg" | "domwdeg" => Some(Self::DomWdeg),
            "input" | "input-order" | "input_order" => Some(Self::InputOrder),
            "activity" | "vsids" => Some(Self::Activity),
            _ => None,
        }
    }
}

/// Configuration for depth-first search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchConfig {
    /// Enables nogood learning and backjumping.
    pub learning: bool,
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
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            learning: true,
            restart_policy: RestartPolicy::default(),
            value_ordering: ValueOrdering::default(),
            variable_ordering: VariableOrdering::default(),
            phase_saving: true,
            time_limit: None,
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
    }
}
