use crate::event_queue::EventQueue;
use crate::trail::Trail;
use propaga_core::id::{PropagatorKey, VariableKey};
use propaga_core::{
    ChangeReason, DomainView, Explanation, ExtendedPropagationContext, FloatDomainSnapshot,
    PropagationContext, PropagatorId, SetDomainSnapshot, VariableId,
};
use propaga_domains::{AnyDomain, HybridDomain, SetIntervalDomain};
use slotmap::SlotMap;
use std::collections::HashMap;

/// Borrowed engine state used while running a propagator.
pub(crate) struct EnginePropagationParts<'a> {
    pub variables: &'a mut SlotMap<VariableKey, AnyDomain>,
    pub subscriptions: &'a HashMap<VariableKey, Vec<PropagatorKey>>,
    pub priorities: &'a HashMap<PropagatorKey, u32>,
    pub queue: &'a mut EventQueue,
    pub trail: &'a mut Trail,
    pub explanation: &'a mut Explanation,
}

/// Mutable propagation view over engine state.
pub struct EnginePropagationContext<'a> {
    variables: &'a mut SlotMap<VariableKey, AnyDomain>,
    subscriptions: &'a HashMap<VariableKey, Vec<PropagatorKey>>,
    priorities: &'a HashMap<PropagatorKey, u32>,
    queue: &'a mut EventQueue,
    trail: &'a mut Trail,
    explanation: &'a mut Explanation,
    changed: bool,
    record_trail: bool,
    current_propagator: Option<PropagatorId>,
}

impl<'a> EnginePropagationContext<'a> {
    pub(crate) fn new(
        parts: EnginePropagationParts<'a>,
        record_trail: bool,
        current_propagator: Option<PropagatorId>,
    ) -> Self {
        Self {
            variables: parts.variables,
            subscriptions: parts.subscriptions,
            priorities: parts.priorities,
            queue: parts.queue,
            trail: parts.trail,
            explanation: parts.explanation,
            changed: false,
            record_trail,
            current_propagator,
        }
    }

    /// Returns whether any domain was modified through this context.
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.changed
    }

    fn domain_for(&self, var: VariableId) -> &HybridDomain {
        self.variables[var.key()]
            .as_int()
            .expect("int propagator accessed non-int variable")
    }

    fn schedule_propagators_for(&mut self, var: VariableId) {
        if let Some(subscribers) = self.subscriptions.get(&var.key()) {
            for propagator_key in subscribers {
                let priority = self.priorities.get(propagator_key).copied().unwrap_or(0);
                self.queue
                    .enqueue(PropagatorId::from_key(*propagator_key), priority);
            }
        }
    }

    fn mutate<F>(&mut self, var: VariableId, reason: Option<ChangeReason>, mutate: F) -> bool
    where
        F: FnOnce(&HybridDomain) -> HybridDomain,
    {
        let current = self.domain_for(var).clone();
        let updated = mutate(&current);
        if updated == current {
            return false;
        }

        if self.record_trail {
            self.trail
                .push(var, AnyDomain::Int(current), reason, self.explanation);
        }

        self.variables[var.key()] = AnyDomain::Int(updated);
        self.schedule_propagators_for(var);
        self.changed = true;
        true
    }

    fn mutate_any<F>(&mut self, var: VariableId, reason: Option<ChangeReason>, mutate: F) -> bool
    where
        F: FnOnce(AnyDomain) -> AnyDomain,
    {
        let current = self.variables[var.key()].clone();
        let updated = mutate(current.clone());
        if updated == current {
            return false;
        }

        if self.record_trail {
            self.trail.push(var, current, reason, self.explanation);
        }

        self.variables[var.key()] = updated;
        self.schedule_propagators_for(var);
        self.changed = true;
        true
    }

    fn propagator_reason(
        &self,
        variable: VariableId,
        removed_value: Option<i32>,
        bound: Option<(propaga_core::BoundKind, i32)>,
    ) -> Option<ChangeReason> {
        self.current_propagator
            .map(|propagator| ChangeReason::Propagator {
                propagator,
                variable,
                removed_value,
                bound,
            })
    }
}

impl PropagationContext for EnginePropagationContext<'_> {
    fn as_extended(&mut self) -> Option<&mut dyn ExtendedPropagationContext> {
        Some(self)
    }

    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
        self.domain_for(var)
    }

    fn current_propagator(&self) -> Option<PropagatorId> {
        self.current_propagator
    }

    fn remove_below(&mut self, var: VariableId, bound: i32) -> bool {
        let reason =
            self.propagator_reason(var, None, Some((propaga_core::BoundKind::Below, bound)));
        self.mutate(var, reason, |domain| domain.remove_below(bound))
    }

    fn remove_above(&mut self, var: VariableId, bound: i32) -> bool {
        let reason =
            self.propagator_reason(var, None, Some((propaga_core::BoundKind::Above, bound)));
        self.mutate(var, reason, |domain| domain.remove_above(bound))
    }

    fn remove_value(&mut self, var: VariableId, value: i32) -> bool {
        let reason = self.propagator_reason(var, Some(value), None);
        self.mutate(var, reason, |domain| domain.remove(value))
    }

    fn fixed_value(&self, var: VariableId) -> Option<i32> {
        self.domain_for(var).fixed_value()
    }

    fn record_propagator_conflict(&mut self, literals: &[(VariableId, i32)]) {
        if literals.is_empty() {
            return;
        }
        self.explanation.record(ChangeReason::PropagatorConflict {
            literals: literals.to_vec(),
        });
    }
}

impl ExtendedPropagationContext for EnginePropagationContext<'_> {
    fn set_domain(&self, var: VariableId) -> Option<SetDomainSnapshot> {
        let set = self.variables[var.key()].as_set()?;
        Some(SetDomainSnapshot {
            glb: set.glb().iter().copied().collect(),
            lub: set.lub().iter().copied().collect(),
            card_min: set.card_min(),
            card_max: set.card_max(),
        })
    }

    fn float_domain(&self, var: VariableId) -> Option<FloatDomainSnapshot> {
        let float = self.variables[var.key()].as_float()?;
        Some(FloatDomainSnapshot {
            min: float.lower_bound(),
            max: float.upper_bound(),
        })
    }

    fn force_set_in(&mut self, var: VariableId, value: i32) -> bool {
        let reason = self.propagator_reason(var, None, None);
        self.mutate_any(var, reason, |domain| {
            let AnyDomain::Set(set) = domain else {
                return domain;
            };
            match set.force_in(value) {
                Some(next) => AnyDomain::Set(next),
                None => AnyDomain::Set(SetIntervalDomain::universe([])),
            }
        })
    }

    fn force_set_out(&mut self, var: VariableId, value: i32) -> bool {
        let reason = self.propagator_reason(var, None, None);
        self.mutate_any(var, reason, |domain| {
            let AnyDomain::Set(set) = domain else {
                return domain;
            };
            match set.force_out(value) {
                Some(next) => AnyDomain::Set(next),
                None => AnyDomain::Set(SetIntervalDomain::universe([])),
            }
        })
    }

    fn tighten_float_below(&mut self, var: VariableId, bound: f64) -> bool {
        let reason = self.propagator_reason(var, None, None);
        self.mutate_any(var, reason, |domain| {
            let AnyDomain::Float(float) = domain else {
                return domain;
            };
            AnyDomain::Float(float.remove_below(bound))
        })
    }

    fn tighten_float_above(&mut self, var: VariableId, bound: f64) -> bool {
        let reason = self.propagator_reason(var, None, None);
        self.mutate_any(var, reason, |domain| {
            let AnyDomain::Float(float) = domain else {
                return domain;
            };
            AnyDomain::Float(float.remove_above(bound))
        })
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;
    use crate::Engine;
    use propaga_core::{PropagationStatus, Propagator};
    use propaga_domains::{AnyDomain, FloatDomain};

    #[derive(Clone)]
    struct FloatLowerBound {
        var: VariableId,
        bound: f64,
    }

    impl Propagator for FloatLowerBound {
        fn watched_variables(&self) -> &[VariableId] {
            std::slice::from_ref(&self.var)
        }

        fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            if ext.tighten_float_below(self.var, self.bound) {
                PropagationStatus::OkChanged
            } else {
                PropagationStatus::OkNoChange
            }
        }
    }

    #[test]
    fn extended_context_tightens_float_domain() {
        let mut engine = Engine::new();
        let var = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 10.0)));
        engine.add_propagator(Box::new(FloatLowerBound { var, bound: 3.0 }));
        engine.propagate_all().unwrap();
        let domain = engine.domain(var).as_float().unwrap();
        assert!(domain.contains(5.0));
        assert!(!domain.contains(1.0));
    }
}
