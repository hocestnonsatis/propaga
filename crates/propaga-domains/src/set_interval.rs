use std::collections::BTreeSet;

/// Interval set domain: GLB ⊆ S ⊆ LUB with cardinality bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetIntervalDomain {
    glb: BTreeSet<i32>,
    lub: BTreeSet<i32>,
    card_min: usize,
    card_max: usize,
}

impl SetIntervalDomain {
    #[must_use]
    pub fn universe(universe: impl IntoIterator<Item = i32>) -> Self {
        let lub: BTreeSet<_> = universe.into_iter().collect();
        let card_max = lub.len();
        Self {
            glb: BTreeSet::new(),
            lub,
            card_min: 0,
            card_max,
        }
    }

    #[must_use]
    pub fn with_cardinality(mut self, min: usize, max: usize) -> Self {
        self.card_min = min;
        self.card_max = max;
        self
    }

    #[must_use]
    pub fn glb(&self) -> &BTreeSet<i32> {
        &self.glb
    }

    #[must_use]
    pub fn lub(&self) -> &BTreeSet<i32> {
        &self.lub
    }

    #[must_use]
    pub fn card_min(&self) -> usize {
        self.card_min
    }

    #[must_use]
    pub fn card_max(&self) -> usize {
        self.card_max
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.card_min > self.card_max
            || self.glb.len() > self.card_max
            || self.lub.len() < self.card_min
            || !self.glb.is_subset(&self.lub)
    }

    #[must_use]
    pub fn is_fixed(&self) -> bool {
        !self.is_empty() && self.glb.len() == self.lub.len()
    }

    #[must_use]
    pub fn size(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        let undecided = self.lub.len().saturating_sub(self.glb.len());
        let min_extra = self.card_min.saturating_sub(self.glb.len());
        let max_extra = self.card_max.saturating_sub(self.glb.len());
        (min_extra..=max_extra.min(undecided))
            .map(|k| binomial(undecided, k))
            .sum()
    }

    /// Forces `value` into the set (GLB).
    #[must_use]
    pub fn force_in(&self, value: i32) -> Option<Self> {
        if !self.lub.contains(&value) {
            return None;
        }
        let mut next = self.clone();
        next.glb.insert(value);
        if next.glb.len() == next.card_max {
            next.lub = next.glb.clone();
        }
        if next.glb.len() > next.card_max || next.is_empty() {
            None
        } else {
            Some(next)
        }
    }

    /// Forces `value` out of the set (remove from LUB).
    #[must_use]
    pub fn force_out(&self, value: i32) -> Option<Self> {
        if self.glb.contains(&value) {
            return None;
        }
        let mut next = self.clone();
        next.lub.remove(&value);
        if next.lub.len() == next.card_min {
            next.glb = next.lub.clone();
        }
        if next.lub.len() < next.card_min || next.is_empty() {
            None
        } else {
            Some(next)
        }
    }

    /// Returns fixed set values when domain is singleton.
    #[must_use]
    pub fn fixed_values(&self) -> Option<Vec<i32>> {
        self.is_fixed().then(|| self.glb.iter().copied().collect())
    }

    /// Returns undecided elements still in LUB \ GLB.
    #[must_use]
    pub fn undecided(&self) -> Vec<i32> {
        self.lub
            .iter()
            .copied()
            .filter(|value| !self.glb.contains(value))
            .collect()
    }
}

fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    (1..=k).fold(1usize, |acc, i| acc * (n - k + i) / i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_in_respects_cardinality() {
        let domain = SetIntervalDomain::universe(1..=3).with_cardinality(2, 2);
        let next = domain.force_in(1).unwrap().force_in(2).unwrap();
        assert!(next.is_fixed());
        assert_eq!(next.fixed_values(), Some(vec![1, 2]));
        assert!(
            domain
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap()
                .force_in(3)
                .is_none()
        );
    }

    #[test]
    fn force_out_prunes_universe() {
        let domain = SetIntervalDomain::universe(1..=3).with_cardinality(1, 2);
        let next = domain.force_out(3).unwrap();
        assert!(!next.lub().contains(&3));
    }
}
