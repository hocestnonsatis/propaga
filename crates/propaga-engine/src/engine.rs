use crate::context::{EnginePropagationContext, EnginePropagationParts};
use crate::event_queue::EventQueue;
use crate::trail::Trail;
use propaga_core::id::{PropagatorKey, VariableKey};
use propaga_core::{
    ChangeReason, DomainView, Explanation, PropagaError, PropagationStatus, Propagator,
    PropagatorId, VariableId,
};
use propaga_domains::{AnyDomain, HybridDomain};
use propaga_domains::{FloatDomain, SetIntervalDomain};
use slotmap::SlotMap;
use std::collections::HashMap;

/// Summary of the most recent propagation conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictInfo {
    /// Variable whose domain became empty.
    pub variable: VariableId,
    /// Explanation collected for the failing branch.
    pub explanation: Explanation,
}

/// Constraint propagation engine with trail-based backtracking.
pub struct Engine {
    pub(crate) variables: SlotMap<VariableKey, AnyDomain>,
    pub(crate) propagators: SlotMap<PropagatorKey, Box<dyn Propagator>>,
    pub(crate) subscriptions: HashMap<VariableKey, Vec<PropagatorKey>>,
    pub(crate) priorities: HashMap<PropagatorKey, u32>,
    pub(crate) queue: EventQueue,
    pub(crate) trail: Trail,
    pub(crate) explanation: Explanation,
    pub(crate) last_conflict: Option<ConflictInfo>,
}

impl Engine {
    /// Creates an empty engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: SlotMap::with_key(),
            propagators: SlotMap::with_key(),
            subscriptions: HashMap::new(),
            priorities: HashMap::new(),
            queue: EventQueue::new(),
            trail: Trail::new(),
            explanation: Explanation::new(),
            last_conflict: None,
        }
    }

    /// Registers a new decision variable and returns its handle.
    pub fn new_variable(&mut self, domain: impl Into<AnyDomain>) -> VariableId {
        VariableId::from_key(self.variables.insert(domain.into()))
    }

    /// Registers a propagator and subscribes it to its watched variables.
    pub fn add_propagator(&mut self, propagator: Box<dyn Propagator>) -> PropagatorId {
        let watched = propagator.watched_variables().to_vec();
        let priority = propagator.priority();
        let id = PropagatorId::from_key(self.propagators.insert(propagator));
        self.priorities.insert(id.key(), priority);

        for var in watched {
            self.subscriptions
                .entry(var.key())
                .or_default()
                .push(id.key());
        }

        id
    }

    /// Returns a read-only view of `var`'s domain.
    pub fn domain(&self, var: VariableId) -> &AnyDomain {
        &self.variables[var.key()]
    }

    /// Returns the integer domain of `var` when it is an int variable.
    #[must_use]
    pub fn int_domain(&self, var: VariableId) -> Option<&HybridDomain> {
        self.domain(var).as_int()
    }

    /// Returns the integer domain of `var`, panicking when it is not int.
    #[must_use]
    pub fn hybrid_domain(&self, var: VariableId) -> &HybridDomain {
        self.int_domain(var).expect("expected int variable domain")
    }

    /// Returns the explanation log for the latest propagation conflict, if any.
    #[must_use]
    pub fn last_conflict(&self) -> Option<&ConflictInfo> {
        self.last_conflict.as_ref()
    }

    /// Returns the live explanation log accumulated during the current branch.
    #[must_use]
    pub fn explanation(&self) -> &Explanation {
        &self.explanation
    }

    /// Returns `true` when every variable is assigned.
    #[must_use]
    pub fn is_solved(&self) -> bool {
        self.variables.values().all(AnyDomain::is_fixed)
    }

    /// Creates a backtrack choice point and returns its level index.
    pub fn trail_mark(&mut self) -> usize {
        self.trail.mark(self.explanation.entries().len())
    }

    /// Restores domains to the state at `level` and clears pending events.
    pub fn trail_backtrack(&mut self, level: usize) {
        if self.trail.decision_levels() == 0 {
            return;
        }
        let (entries, explanation_len) = self.trail.backtrack(level);
        for (variable, old_domain) in entries.into_iter().rev() {
            self.restore_domain(variable, old_domain);
        }
        self.explanation.truncate(explanation_len);
        self.queue.clear();
    }

    /// Returns the current number of search decision levels.
    #[must_use]
    pub fn trail_depth(&self) -> usize {
        self.trail.decision_levels()
    }

    /// Assigns `value` to `var`, schedules affected propagators, and propagates.
    pub fn fix_variable(
        &mut self,
        var: VariableId,
        value: i32,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .int_domain(var)
            .cloned()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "int".to_string(),
            })?;

        if !current.contains(value) {
            if self.trail.has_choice_point() {
                self.explanation.record(ChangeReason::Branch {
                    variable: var,
                    value,
                });
            }
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        }

        if current.is_fixed() && current.min() == Some(value) {
            return self.propagate();
        }

        self.trail.push(
            var,
            self.domain(var).clone(),
            Some(ChangeReason::Branch {
                variable: var,
                value,
            }),
            &mut self.explanation,
        );
        self.set_domain(var, AnyDomain::Int(HybridDomain::fix(value)));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Assigns a float `value` to `var`, schedules affected propagators, and propagates.
    pub fn fix_float(
        &mut self,
        var: VariableId,
        value: f64,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .domain(var)
            .as_float()
            .copied()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "float".to_string(),
            })?;

        if !current.contains(value) {
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        }

        if current.is_fixed() && (current.lower_bound() - value).abs() < f64::EPSILON {
            return self.propagate();
        }

        self.trail.push(
            var,
            self.domain(var).clone(),
            Some(ChangeReason::Branch {
                variable: var,
                value: 0,
            }),
            &mut self.explanation,
        );
        self.set_domain(var, AnyDomain::Float(FloatDomain::fix(value)));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Forces `value` into a set variable.
    pub fn force_set_in(
        &mut self,
        var: VariableId,
        value: i32,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .domain(var)
            .as_set()
            .cloned()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "set".to_string(),
            })?;

        let Some(next) = current.force_in(value) else {
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        };

        if current.is_fixed() {
            return self.propagate();
        }

        self.trail.push(
            var,
            self.domain(var).clone(),
            Some(ChangeReason::Branch {
                variable: var,
                value,
            }),
            &mut self.explanation,
        );
        self.set_domain(var, AnyDomain::Set(next));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Forces `value` out of a set variable.
    pub fn force_set_out(
        &mut self,
        var: VariableId,
        value: i32,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .domain(var)
            .as_set()
            .cloned()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "set".to_string(),
            })?;

        let Some(next) = current.force_out(value) else {
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        };

        self.trail.push(
            var,
            self.domain(var).clone(),
            Some(ChangeReason::Branch {
                variable: var,
                value,
            }),
            &mut self.explanation,
        );
        self.set_domain(var, AnyDomain::Set(next));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Tightens a float variable's upper bound.
    pub fn tighten_float_above(
        &mut self,
        var: VariableId,
        bound: f64,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .domain(var)
            .as_float()
            .copied()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "float".to_string(),
            })?;
        let next = current.remove_above(bound);
        if next.is_empty() {
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        }
        if next == current {
            return self.propagate();
        }
        self.trail
            .push(var, self.domain(var).clone(), None, &mut self.explanation);
        self.set_domain(var, AnyDomain::Float(next));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Tightens a float variable's lower bound.
    pub fn tighten_float_below(
        &mut self,
        var: VariableId,
        bound: f64,
    ) -> Result<PropagationStatus, PropagaError> {
        let current = self
            .domain(var)
            .as_float()
            .copied()
            .ok_or(PropagaError::TypeMismatch {
                variable: var,
                expected: "float".to_string(),
            })?;
        let next = current.remove_below(bound);
        if next.is_empty() {
            self.record_conflict(var);
            return Ok(PropagationStatus::Failure);
        }
        if next == current {
            return self.propagate();
        }
        self.trail
            .push(var, self.domain(var).clone(), None, &mut self.explanation);
        self.set_domain(var, AnyDomain::Float(next));
        self.schedule_propagators_for(var);
        self.propagate()
    }

    /// Enqueues every propagator and runs propagation to fixpoint.
    pub fn propagate_all(&mut self) -> Result<PropagationStatus, PropagaError> {
        self.enqueue_all_propagators();
        let status = self.propagate()?;
        self.trail.clear(&mut self.explanation);
        Ok(status)
    }

    /// Propagates at search root, committing domain changes while keeping explanations.
    pub fn commit_initial_propagation(&mut self) -> Result<PropagationStatus, PropagaError> {
        let level = self.trail_mark();
        self.enqueue_all_propagators();
        let status = self.propagate()?;
        if status.is_failure() {
            self.trail_backtrack(level);
            return Ok(status);
        }
        self.trail.commit_base_level();
        Ok(status)
    }

    /// Runs scheduled propagators until the queue is empty.
    pub fn propagate(&mut self) -> Result<PropagationStatus, PropagaError> {
        let mut overall = PropagationStatus::OkNoChange;
        self.last_conflict = None;

        while let Some(propagator_id) = self.queue.pop() {
            let status = self.run_propagator(propagator_id)?;
            overall = overall.merge(status);
            if status.is_failure() {
                return Ok(PropagationStatus::Failure);
            }
        }

        Ok(overall)
    }

    pub(crate) fn set_domain(&mut self, var: VariableId, domain: AnyDomain) {
        self.variables[var.key()] = domain;
    }

    pub(crate) fn restore_domain(&mut self, var: VariableId, domain: AnyDomain) {
        self.variables[var.key()] = domain;
    }

    pub(crate) fn schedule_propagators_for(&mut self, var: VariableId) {
        if let Some(subscribers) = self.subscriptions.get(&var.key()) {
            for propagator_key in subscribers {
                let priority = self.priorities.get(propagator_key).copied().unwrap_or(0);
                self.queue
                    .enqueue(PropagatorId::from_key(*propagator_key), priority);
            }
        }
    }

    fn enqueue_all_propagators(&mut self) {
        let ids: Vec<_> = self.propagators.keys().collect();
        for key in ids {
            let priority = self.priorities.get(&key).copied().unwrap_or(0);
            self.queue.enqueue(PropagatorId::from_key(key), priority);
        }
    }

    fn record_conflict(&mut self, var: VariableId) {
        self.last_conflict = Some(ConflictInfo {
            variable: var,
            explanation: self.explanation.clone(),
        });
    }

    fn run_propagator(
        &mut self,
        propagator_id: PropagatorId,
    ) -> Result<PropagationStatus, PropagaError> {
        let watched = {
            let propagator = self
                .propagators
                .get(propagator_id.key())
                .ok_or(PropagaError::UnknownPropagator)?;
            propagator.watched_variables().to_vec()
        };

        for var in &watched {
            if self.domain(*var).is_empty() {
                self.record_conflict(*var);
                return Ok(PropagationStatus::Failure);
            }
        }

        let record_trail = self.trail.has_choice_point();
        let mut ctx = EnginePropagationContext::new(
            EnginePropagationParts {
                variables: &mut self.variables,
                subscriptions: &self.subscriptions,
                priorities: &self.priorities,
                queue: &mut self.queue,
                trail: &mut self.trail,
                explanation: &mut self.explanation,
            },
            record_trail,
            Some(propagator_id),
        );
        let status = self.propagators[propagator_id.key()].propagate(&mut ctx);
        let changed = ctx.changed();

        if status.is_failure() {
            let conflict_var = watched
                .iter()
                .find(|var| self.domain(**var).is_empty())
                .or_else(|| watched.first());
            if let Some(&var) = conflict_var {
                self.record_conflict(var);
            }
            return Ok(PropagationStatus::Failure);
        }

        for var in &watched {
            if self.domain(*var).is_empty() {
                self.record_conflict(*var);
                return Ok(PropagationStatus::Failure);
            }
        }

        if changed {
            Ok(PropagationStatus::OkChanged)
        } else {
            Ok(status)
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::PropagationContext;
    use propaga_domains::{AnyDomain, HybridDomain, IntervalDomain};

    #[derive(Clone)]
    struct LowerBoundPropagator {
        var: VariableId,
        bound: i32,
    }

    impl Propagator for LowerBoundPropagator {
        fn watched_variables(&self) -> &[VariableId] {
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
    struct EqualizingPropagator {
        watched: [VariableId; 2],
    }

    impl Propagator for EqualizingPropagator {
        fn watched_variables(&self) -> &[VariableId] {
            &self.watched
        }

        fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
            let [left, right] = self.watched;
            let mut changed = false;

            if let Some(value) = ctx.fixed_value(left) {
                if ctx.remove_below(right, value) {
                    changed = true;
                }
                if ctx.remove_above(right, value) {
                    changed = true;
                }
            }

            if let Some(value) = ctx.fixed_value(right) {
                if ctx.remove_below(left, value) {
                    changed = true;
                }
                if ctx.remove_above(left, value) {
                    changed = true;
                }
            }

            let left_min = ctx.domain(left).min();
            let left_max = ctx.domain(left).max();
            let right_min = ctx.domain(right).min();
            let right_max = ctx.domain(right).max();

            if let (Some(min), Some(max)) = (left_min, left_max) {
                if ctx.remove_below(right, min) {
                    changed = true;
                }
                if ctx.remove_above(right, max) {
                    changed = true;
                }
            }

            if let (Some(min), Some(max)) = (right_min, right_max) {
                if ctx.remove_below(left, min) {
                    changed = true;
                }
                if ctx.remove_above(left, max) {
                    changed = true;
                }
            }

            if ctx.domain(left).is_empty() || ctx.domain(right).is_empty() {
                PropagationStatus::Failure
            } else if changed {
                PropagationStatus::OkChanged
            } else {
                PropagationStatus::OkNoChange
            }
        }
    }

    #[test]
    fn stores_and_returns_domains() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 5));
        assert_eq!(engine.domain(var), &AnyDomain::Int(HybridDomain::new(1, 5)));
    }

    #[test]
    fn mock_propagator_reaches_fixpoint() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(LowerBoundPropagator { var, bound: 4 }));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkChanged);
        assert_eq!(engine.int_domain(var).unwrap().min(), Some(4));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkNoChange);
    }

    #[test]
    fn scheduled_propagation_runs_subscribed_propagators() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        engine.add_propagator(Box::new(EqualizingPropagator {
            watched: [left, right],
        }));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::OkChanged);
        assert_eq!(engine.int_domain(right).unwrap().fixed_value(), Some(3));
    }

    #[test]
    fn trail_restores_domains() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 10));
        let level = engine.trail_mark();

        engine.fix_variable(var, 5).unwrap();
        assert_eq!(engine.int_domain(var).unwrap().fixed_value(), Some(5));

        engine.trail_backtrack(level);
        assert_eq!(
            engine.domain(var),
            &AnyDomain::Int(HybridDomain::new(1, 10))
        );
    }

    #[test]
    fn records_explanation_on_branch() {
        let mut engine = Engine::new();
        let var = engine.new_variable(HybridDomain::new(1, 3));
        let level = engine.trail_mark();
        engine.fix_variable(var, 2).unwrap();
        assert!(engine.explanation().entries().iter().any(|entry| {
            matches!(entry, ChangeReason::Branch { variable, value: 2 } if *variable == var)
        }));
        engine.trail_backtrack(level);
        assert!(engine.explanation().entries().is_empty());
    }
}
