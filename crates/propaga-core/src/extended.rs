use crate::VariableId;

/// Snapshot of a set variable domain for propagation reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetDomainSnapshot {
    /// Greatest lower bound elements.
    pub glb: Vec<i32>,
    /// Least upper bound elements.
    pub lub: Vec<i32>,
    /// Minimum cardinality.
    pub card_min: usize,
    /// Maximum cardinality.
    pub card_max: usize,
}

impl SetDomainSnapshot {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.card_min > self.card_max
            || self.glb.len() > self.card_max
            || self.lub.len() < self.card_min
            || !self.glb.iter().all(|value| self.lub.contains(value))
    }

    #[must_use]
    pub fn undecided(&self) -> Vec<i32> {
        self.lub
            .iter()
            .copied()
            .filter(|value| !self.glb.contains(value))
            .collect()
    }
}

/// Snapshot of a float variable domain for propagation reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatDomainSnapshot {
    /// Lower bound.
    pub min: f64,
    /// Upper bound.
    pub max: f64,
}

impl FloatDomainSnapshot {
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.min > self.max
    }

    #[must_use]
    pub fn contains(self, value: f64) -> bool {
        !self.is_empty() && value >= self.min && value <= self.max
    }
}

/// Extended propagation operations for set and float variables.
pub trait ExtendedPropagationContext {
    fn set_domain(&self, var: VariableId) -> Option<SetDomainSnapshot>;
    fn float_domain(&self, var: VariableId) -> Option<FloatDomainSnapshot>;
    fn force_set_in(&mut self, var: VariableId, value: i32) -> bool;
    fn force_set_out(&mut self, var: VariableId, value: i32) -> bool;
    fn tighten_float_below(&mut self, var: VariableId, bound: f64) -> bool;
    fn tighten_float_above(&mut self, var: VariableId, bound: f64) -> bool;
}
