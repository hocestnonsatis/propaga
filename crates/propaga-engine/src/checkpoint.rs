use crate::Engine;
use crate::event_queue::EventQueue;
use crate::trail::Trail;
use dyn_clone::clone_box;
use propaga_core::Explanation;
use propaga_core::id::{PropagatorKey, VariableKey};
use propaga_domains::AnyDomain;
use slotmap::SlotMap;
use std::collections::{HashMap, HashSet};

/// Snapshot of engine variable domains and base propagators after root propagation.
#[derive(Clone, Debug)]
pub struct EngineCheckpoint {
    pub(crate) domains: SlotMap<VariableKey, AnyDomain>,
    propagator_keys: Vec<PropagatorKey>,
}

impl EngineCheckpoint {
    /// Returns variable handles captured in this checkpoint.
    #[must_use]
    pub fn variable_ids(&self) -> Vec<propaga_core::VariableId> {
        self.domains
            .keys()
            .map(propaga_core::VariableId::from_key)
            .collect()
    }

    /// Returns the number of propagators captured at checkpoint time.
    #[must_use]
    pub fn propagator_count(&self) -> usize {
        self.propagator_keys.len()
    }
}

impl Engine {
    /// Captures variable domains and propagator set after root propagation.
    #[must_use]
    pub fn checkpoint(&self) -> EngineCheckpoint {
        EngineCheckpoint {
            domains: self.variables.clone(),
            propagator_keys: self.propagators.keys().collect(),
        }
    }

    /// Restores domains and removes propagators added after the checkpoint.
    pub fn restore_checkpoint(&mut self, checkpoint: &EngineCheckpoint) {
        if self.trail_depth() > 0 {
            self.trail_backtrack(0);
        }

        for (key, domain) in checkpoint.domains.iter() {
            if self.variables.contains_key(key) {
                self.variables[key] = domain.clone();
            }
        }

        let keep: HashSet<_> = checkpoint.propagator_keys.iter().copied().collect();
        let remove: Vec<_> = self
            .propagators
            .keys()
            .filter(|key| !keep.contains(key))
            .collect();
        for key in remove {
            self.remove_propagator(key);
        }

        self.queue.clear();
        self.explanation.reset();
        self.last_conflict = None;
    }

    /// Creates an independent engine copy at the checkpoint state for parallel search.
    #[must_use]
    pub fn fork_at_checkpoint(&self, checkpoint: &EngineCheckpoint) -> Engine {
        let mut forked = Engine {
            variables: checkpoint.domains.clone(),
            propagators: SlotMap::with_key(),
            subscriptions: HashMap::new(),
            priorities: HashMap::new(),
            queue: EventQueue::new(),
            trail: Trail::new(),
            explanation: Explanation::new(),
            last_conflict: None,
        };

        for key in &checkpoint.propagator_keys {
            if let Some(propagator) = self.propagators.get(*key) {
                forked.add_propagator(clone_box(propagator.as_ref()));
            }
        }

        forked
    }

    pub(crate) fn remove_propagator(&mut self, key: PropagatorKey) {
        if let Some(propagator) = self.propagators.remove(key) {
            self.priorities.remove(&key);
            for var in propagator.watched_variables() {
                if let Some(subscribers) = self.subscriptions.get_mut(&var.key()) {
                    subscribers.retain(|subscriber| subscriber != &key);
                    if subscribers.is_empty() {
                        self.subscriptions.remove(&var.key());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Engine;
    use propaga_core::{DomainView, PropagationContext, PropagationStatus, Propagator};
    use propaga_domains::IntervalDomain;

    #[derive(Clone)]
    struct LowerBoundPropagator {
        var: propaga_core::VariableId,
        bound: i32,
    }

    impl Propagator for LowerBoundPropagator {
        fn watched_variables(&self) -> &[propaga_core::VariableId] {
            std::slice::from_ref(&self.var)
        }

        fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
            if ctx.remove_below(self.var, self.bound) {
                PropagationStatus::OkChanged
            } else {
                PropagationStatus::OkNoChange
            }
        }
    }

    #[derive(Clone)]
    struct NoOpPropagator {
        var: propaga_core::VariableId,
    }

    impl Propagator for NoOpPropagator {
        fn watched_variables(&self) -> &[propaga_core::VariableId] {
            std::slice::from_ref(&self.var)
        }

        fn propagate(&mut self, _ctx: &mut dyn PropagationContext) -> PropagationStatus {
            PropagationStatus::OkNoChange
        }
    }

    #[test]
    fn checkpoint_round_trip_restores_domains() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(LowerBoundPropagator { var, bound: 4 }));
        engine.commit_initial_propagation().unwrap();
        let checkpoint = engine.checkpoint();

        engine.fix_variable(var, 7).unwrap();
        assert_eq!(engine.int_domain(var).unwrap().fixed_value(), Some(7));

        engine.restore_checkpoint(&checkpoint);
        assert_eq!(engine.int_domain(var).unwrap().min(), Some(4));
        assert_eq!(engine.trail_depth(), 0);
    }

    #[test]
    fn restore_removes_propagators_added_after_checkpoint() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(LowerBoundPropagator { var, bound: 1 }));
        engine.commit_initial_propagation().unwrap();
        let checkpoint = engine.checkpoint();

        engine.add_propagator(Box::new(NoOpPropagator { var }));
        assert_eq!(engine.checkpoint().propagator_count(), 2);

        engine.restore_checkpoint(&checkpoint);
        assert_eq!(engine.checkpoint().propagator_count(), 1);
    }

    #[test]
    fn fork_preserves_variable_keys() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(LowerBoundPropagator { var, bound: 1 }));
        engine.commit_initial_propagation().unwrap();
        let checkpoint = engine.checkpoint();

        let forked = engine.fork_at_checkpoint(&checkpoint);
        assert_eq!(
            engine.int_domain(var).unwrap().min(),
            forked.int_domain(var).unwrap().min()
        );
    }

    #[test]
    fn checkpoint_preserves_set_domain() {
        use propaga_domains::{AnyDomain, DomainKind, SetIntervalDomain};

        let mut engine = Engine::new();
        let set_domain = SetIntervalDomain::universe(1..=3).with_cardinality(1, 2);
        let var = engine.new_variable(AnyDomain::Set(set_domain));
        engine.commit_initial_propagation().unwrap();
        let checkpoint = engine.checkpoint();

        engine.restore_checkpoint(&checkpoint);
        assert!(engine.domain(var).as_set().is_some());
        assert_eq!(engine.domain(var).kind(), DomainKind::Set);
    }
}
