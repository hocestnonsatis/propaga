use crate::matching::remove_unsupported_values;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a Hamiltonian circuit over successor variables.
#[derive(Clone)]
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

        if changed {
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
    fn empty_successor_domain_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(1, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 2));
        let x2 = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1, x2])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn empty_successor_domain_after_matching_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 2));
        let x1 = engine.new_variable(IntervalDomain::new(1, 0));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn removes_self_loops() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 2));
        let x1 = engine.new_variable(IntervalDomain::new(0, 2));
        let x2 = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1, x2])));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(x0).contains(0));
        assert!(!engine.hybrid_domain(x1).contains(1));
        assert!(!engine.hybrid_domain(x2).contains(2));
    }

    #[test]
    fn impossible_circuit_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(1));
        let x1 = engine.new_variable(IntervalDomain::fix(1));
        let x2 = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1, x2])));

        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn no_perfect_matching_after_pruning_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(1, 1));
        let x1 = engine.new_variable(IntervalDomain::new(1, 1));
        let x2 = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1, x2])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn empty_successor_after_self_loop_removal_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(0));
        let x1 = engine.new_variable(IntervalDomain::new(1, 0));
        engine.add_propagator(Box::new(CircuitPropagator::new(vec![x0, x1])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn matching_exists_but_successor_domain_empty_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x0, vec![1])
            .with_domain(x1, vec![]);
        let mut prop = CircuitPropagator::new(vec![x0, x1]);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn matching_succeeds_but_no_hamiltonian_circuit() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 0));
        let x2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x0, vec![1])
            .with_domain(x1, vec![1])
            .with_domain(x2, vec![2]);
        let mut prop = CircuitPropagator::new(vec![x0, x1, x2]);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn no_perfect_matching_after_regin_pruning_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 0));
        let x2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x0, vec![1])
            .with_domain(x1, vec![1])
            .with_domain(x2, vec![2]);
        let mut prop = CircuitPropagator::new(vec![x0, x1, x2]);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_empty_successor_after_matching_check_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x0, vec![1])
            .with_domain(x1, vec![]);
        let mut prop = CircuitPropagator::new(vec![x0, x1]);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn regin_pruning_breaks_perfect_matching_after_ok_prune() {
        use crate::matching::remove_unsupported_values;
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 0));
        let x2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(x0, vec![0, 1])
            .with_domain(x1, vec![0, 1])
            .with_domain(x2, vec![0, 1, 2]);
        assert!(remove_unsupported_values(&mut ctx, &[x0, x1, x2]).is_ok());
        let mut prop = CircuitPropagator::new(vec![x0, x1, x2]);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }
}
