//! Search configuration and restart policies.

use std::time::Duration;

/// Restart strategy for the search loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never restart.
    None,
    /// Luby restarts with the given base node limit multiplier.
    Luby { base: u64 },
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
            "luby" => Some(Self::Luby { base: 512 }),
            _ if text.starts_with("luby:") => text
                .strip_prefix("luby:")
                .and_then(|base| base.parse().ok())
                .map(|base| Self::Luby { base }),
            _ => None,
        }
    }

    /// Returns the node limit before the next restart.
    #[must_use]
    pub fn node_limit(&self, restart_index: u32) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Luby { base } => Some(base.saturating_mul(luby_sequence(restart_index))),
        }
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
    /// Least constraining value: prefer values that appear in fewer other domains.
    Lcv,
}

impl ValueOrdering {
    /// Parses a value ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "asc" | "ascending" | "min" => Some(Self::Ascending),
            "lcv" => Some(Self::Lcv),
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
}

impl VariableOrdering {
    /// Parses a variable ordering from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mrv" | "size" => Some(Self::Mrv),
            "dom" => Some(Self::Dom),
            "dom-wdeg" | "wdeg" | "domwdeg" => Some(Self::DomWdeg),
            _ => None,
        }
    }
}

/// Configuration for depth-first search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
