use crate::TablePropagator;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// DFA-based regular constraint compiled to a table propagator.
#[derive(Clone)]
pub struct RegularPropagator {
    inner: TablePropagator,
}

impl RegularPropagator {
    /// Creates a regular propagator from a transition matrix.
    pub fn new(
        variables: Vec<VariableId>,
        num_states: usize,
        transitions: Vec<Vec<i32>>,
        start_state: i32,
        accepting: &[i32],
    ) -> Self {
        let tuples = enumerate_accepting_tuples(
            variables.len(),
            num_states,
            &transitions,
            start_state,
            accepting,
        );
        Self {
            inner: TablePropagator::new(variables, tuples),
        }
    }
}

impl Propagator for RegularPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        self.inner.watched_variables()
    }

    fn priority(&self) -> u32 {
        self.inner.priority()
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        self.inner.propagate(ctx)
    }
}

fn enumerate_accepting_tuples(
    length: usize,
    num_states: usize,
    transitions: &[Vec<i32>],
    start: i32,
    accepting: &[i32],
) -> Vec<Vec<i32>> {
    let mut tuples = Vec::new();
    if length == 0 {
        if accepting.contains(&start) {
            tuples.push(vec![]);
        }
        return tuples;
    }
    let max_symbol = transitions.first().map(|row| row.len()).unwrap_or(0);
    for symbol in 1..=max_symbol {
        dfs_tuple(
            &mut tuples,
            transitions,
            num_states,
            start,
            accepting,
            length.saturating_sub(1),
            vec![symbol as i32],
        );
    }
    tuples
}

fn dfs_tuple(
    out: &mut Vec<Vec<i32>>,
    transitions: &[Vec<i32>],
    num_states: usize,
    state: i32,
    accepting: &[i32],
    remaining: usize,
    prefix: Vec<i32>,
) {
    if remaining == 0 {
        if accepting.contains(&state) {
            out.push(prefix);
        }
        return;
    }
    let Some(row) = transitions.get(state as usize - 1) else {
        return;
    };
    for (col, &next) in row.iter().enumerate() {
        if next <= 0 || next as usize > num_states {
            continue;
        }
        let mut next_prefix = prefix.clone();
        next_prefix.push((col + 1) as i32);
        dfs_tuple(
            out,
            transitions,
            num_states,
            next,
            accepting,
            remaining - 1,
            next_prefix,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn prunes_invalid_sequence() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let transitions = vec![vec![2, 0], vec![0, 2]];
        engine.add_propagator(Box::new(RegularPropagator::new(
            vec![a, b],
            2,
            transitions,
            1,
            &[2],
        )));
        let status = engine.propagate_all().unwrap();
        assert_ne!(status, propaga_core::PropagationStatus::Failure);
    }
}
