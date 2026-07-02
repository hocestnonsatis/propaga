use crate::matching::{has_perfect_matching, remove_unsupported_values};
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a Hamiltonian circuit over successor variables.
pub struct CircuitPropagator {
    successors: Vec<VariableId>,
    /// Index base for node numbering (0 for 0-based, 1 for 1-based FlatZinc).
    index_base: i32,
}

impl CircuitPropagator {
    /// Creates a circuit propagator over `successors[i] = j` edges.
    #[must_use]
    pub fn new(successors: Vec<VariableId>) -> Self {
        Self::with_index_base(successors, 0)
    }

    /// Creates a circuit propagator with explicit index base.
    #[must_use]
    pub fn with_index_base(successors: Vec<VariableId>, index_base: i32) -> Self {
        Self {
            successors,
            index_base,
        }
    }
}

impl Propagator for CircuitPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.successors
    }

    fn priority(&self) -> u32 {
        22
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;

        for (index, &successor) in self.successors.iter().enumerate() {
            let node = i32::try_from(index).expect("circuit index fits in i32") + self.index_base;
            if ctx.domain(successor).contains(node) && ctx.remove_value(successor, node) {
                changed = true;
            }
        }

        match remove_unsupported_values(ctx, &self.successors) {
            Ok(matching_changed) => changed |= matching_changed,
            Err(()) => return PropagationStatus::Failure,
        }

        if !has_perfect_matching(ctx, &self.successors) {
            return PropagationStatus::Failure;
        }

        if self
            .successors
            .iter()
            .any(|&var| ctx.domain(var).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn removes_self_loops() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 2));
        let x1 = engine.new_variable(IntervalDomain::new(0, 2));
        let x2 = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1, x2])));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(x0).contains(0));
        assert!(!engine.domain(x1).contains(1));
        assert!(!engine.domain(x2).contains(2));
    }
}
