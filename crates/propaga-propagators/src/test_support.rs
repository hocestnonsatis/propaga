//! Shared test doubles for propagator unit tests.
#![cfg(test)]

use propaga_core::{DomainView, PropagationContext, VariableId};
use propaga_domains::{AnyDomain, IntervalDomain};
use propaga_engine::Engine;

/// Engine-backed context without extended set/float support.
pub(crate) struct NoExtendedCtx<'a> {
    engine: &'a mut Engine,
}

impl<'a> NoExtendedCtx<'a> {
    pub(crate) fn new(engine: &'a mut Engine) -> Self {
        Self { engine }
    }

    pub(crate) fn exercise_mutators(&mut self, var: VariableId) {
        let _ = self.domain(var);
        let _ = self.fixed_value(var);
        let _ = self.remove_below(var, 0);
        let _ = self.remove_above(var, 0);
        let _ = self.remove_value(var, 0);
    }
}

impl PropagationContext for NoExtendedCtx<'_> {
    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
        self.engine.hybrid_domain(var)
    }

    fn remove_below(&mut self, var: VariableId, bound: i32) -> bool {
        let current = self.engine.hybrid_domain(var).clone();
        let next = current.remove_below(bound);
        if next == current {
            return false;
        }
        self.engine.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn remove_above(&mut self, var: VariableId, bound: i32) -> bool {
        let current = self.engine.hybrid_domain(var).clone();
        let next = current.remove_above(bound);
        if next == current {
            return false;
        }
        self.engine.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn remove_value(&mut self, var: VariableId, value: i32) -> bool {
        let current = self.engine.hybrid_domain(var).clone();
        let next = current.remove(value);
        if next == current {
            return false;
        }
        self.engine.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn fixed_value(&self, var: VariableId) -> Option<i32> {
        self.engine.hybrid_domain(var).fixed_value()
    }
}

/// Mutable engine wrapper for direct helper tests.
pub(crate) struct MutEngine<'a>(pub &'a mut Engine);

impl PropagationContext for MutEngine<'_> {
    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
        self.0.hybrid_domain(var)
    }

    fn remove_below(&mut self, var: VariableId, bound: i32) -> bool {
        let current = self.0.hybrid_domain(var).clone();
        let next = current.remove_below(bound);
        if next == current {
            return false;
        }
        self.0.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn remove_above(&mut self, var: VariableId, bound: i32) -> bool {
        let current = self.0.hybrid_domain(var).clone();
        let next = current.remove_above(bound);
        if next == current {
            return false;
        }
        self.0.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn remove_value(&mut self, var: VariableId, value: i32) -> bool {
        let current = self.0.hybrid_domain(var).clone();
        let next = current.remove(value);
        if next == current {
            return false;
        }
        self.0.set_domain(var, AnyDomain::Int(next));
        true
    }

    fn fixed_value(&self, var: VariableId) -> Option<i32> {
        self.0.hybrid_domain(var).fixed_value()
    }
}

/// Read-only engine wrapper for literal collection tests.
pub(crate) struct ReadOnlyEngine<'a>(pub &'a Engine);

impl PropagationContext for ReadOnlyEngine<'_> {
    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
        self.0.hybrid_domain(var)
    }

    fn remove_below(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn remove_above(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn remove_value(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn fixed_value(&self, var: VariableId) -> Option<i32> {
        self.0.hybrid_domain(var).fixed_value()
    }
}

/// Domain with `size() == 1` but `is_fixed() == false`.
#[derive(Clone)]
pub(crate) struct SingletonOpenDomain {
    pub value: RefCell<i32>,
    pub removed: RefCell<bool>,
}

impl SingletonOpenDomain {
    pub(crate) fn new(value: i32) -> Self {
        Self {
            value: RefCell::new(value),
            removed: RefCell::new(false),
        }
    }
}

impl DomainView for SingletonOpenDomain {
    type Value = i32;

    fn contains(&self, value: i32) -> bool {
        !*self.removed.borrow() && *self.value.borrow() == value
    }

    fn size(&self) -> usize {
        usize::from(!*self.removed.borrow())
    }

    fn min(&self) -> Option<i32> {
        (!*self.removed.borrow()).then_some(*self.value.borrow())
    }

    fn max(&self) -> Option<i32> {
        self.min()
    }

    fn is_empty(&self) -> bool {
        *self.removed.borrow()
    }

    fn is_fixed(&self) -> bool {
        false
    }
}

use propaga_core::{ExtendedPropagationContext, FloatDomainSnapshot, SetDomainSnapshot};
use std::cell::RefCell;
use std::collections::HashMap;

/// Configurable integer domain for mock propagation contexts.
#[derive(Clone, Default)]
pub(crate) struct MockIntDomain {
    pub values: RefCell<Vec<i32>>,
}

impl DomainView for MockIntDomain {
    type Value = i32;

    fn contains(&self, value: i32) -> bool {
        self.values.borrow().contains(&value)
    }

    fn size(&self) -> usize {
        self.values.borrow().len()
    }

    fn min(&self) -> Option<i32> {
        self.values.borrow().iter().copied().min()
    }

    fn max(&self) -> Option<i32> {
        self.values.borrow().iter().copied().max()
    }

    fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }

    fn is_fixed(&self) -> bool {
        self.size() == 1
    }
}

/// Mock context backed by per-variable integer domains.
pub(crate) struct MockIntCtx {
    pub domains: HashMap<VariableId, MockIntDomain>,
    open_singletons: HashMap<VariableId, SingletonOpenDomain>,
    pub fixed: HashMap<VariableId, i32>,
    pub conflicts: RefCell<Vec<Vec<(VariableId, i32)>>>,
}

impl MockIntCtx {
    pub(crate) fn new() -> Self {
        Self {
            domains: HashMap::new(),
            open_singletons: HashMap::new(),
            fixed: HashMap::new(),
            conflicts: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn domain_values(&self, var: VariableId) -> Vec<i32> {
        if let Some(domain) = self.open_singletons.get(&var) {
            return domain.min().into_iter().collect();
        }
        self.domains[&var].values.borrow().clone()
    }

    pub(crate) fn insert_domain(&mut self, var: VariableId, values: Vec<i32>) {
        self.open_singletons.remove(&var);
        self.domains.insert(
            var,
            MockIntDomain {
                values: RefCell::new(values),
            },
        );
    }

    pub(crate) fn with_domain(mut self, var: VariableId, values: Vec<i32>) -> Self {
        self.insert_domain(var, values);
        self
    }

    pub(crate) fn with_open_singleton(mut self, var: VariableId, value: i32) -> Self {
        self.domains.remove(&var);
        self.open_singletons
            .insert(var, SingletonOpenDomain::new(value));
        self
    }

    pub(crate) fn with_fixed(mut self, var: VariableId, value: i32) -> Self {
        self.fixed.insert(var, value);
        self
    }

    fn stored_domain_mut(&mut self, var: VariableId) -> DomainMut<'_> {
        if self.open_singletons.contains_key(&var) {
            DomainMut::OpenSingleton(self.open_singletons.get_mut(&var).expect("open singleton"))
        } else {
            DomainMut::Values(self.domains.get_mut(&var).expect("domain"))
        }
    }
}

enum DomainMut<'a> {
    Values(&'a mut MockIntDomain),
    OpenSingleton(&'a mut SingletonOpenDomain),
}

impl DomainMut<'_> {
    fn remove_below(&mut self, bound: i32) -> bool {
        match self {
            Self::Values(domain) => {
                let mut values = domain.values.borrow_mut();
                let before = values.len();
                values.retain(|value| *value >= bound);
                values.len() != before
            }
            Self::OpenSingleton(domain) => {
                if *domain.removed.borrow() {
                    return false;
                }
                if *domain.value.borrow() < bound {
                    *domain.removed.borrow_mut() = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn remove_above(&mut self, bound: i32) -> bool {
        match self {
            Self::Values(domain) => {
                let mut values = domain.values.borrow_mut();
                let before = values.len();
                values.retain(|value| *value <= bound);
                values.len() != before
            }
            Self::OpenSingleton(domain) => {
                if *domain.removed.borrow() {
                    return false;
                }
                if *domain.value.borrow() > bound {
                    *domain.removed.borrow_mut() = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn remove_value(&mut self, value: i32) -> bool {
        match self {
            Self::Values(domain) => {
                let mut values = domain.values.borrow_mut();
                let before = values.len();
                values.retain(|candidate| *candidate != value);
                values.len() != before
            }
            Self::OpenSingleton(domain) => {
                if *domain.removed.borrow() {
                    return false;
                }
                if *domain.value.borrow() == value {
                    *domain.removed.borrow_mut() = true;
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl PropagationContext for MockIntCtx {
    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
        if let Some(domain) = self.open_singletons.get(&var) {
            return domain;
        }
        &self.domains[&var]
    }

    fn fixed_value(&self, var: VariableId) -> Option<i32> {
        self.fixed.get(&var).copied()
    }

    fn remove_below(&mut self, var: VariableId, bound: i32) -> bool {
        self.stored_domain_mut(var).remove_below(bound)
    }

    fn remove_above(&mut self, var: VariableId, bound: i32) -> bool {
        self.stored_domain_mut(var).remove_above(bound)
    }

    fn remove_value(&mut self, var: VariableId, value: i32) -> bool {
        self.stored_domain_mut(var).remove_value(value)
    }

    fn record_propagator_conflict(&mut self, literals: &[(VariableId, i32)]) {
        self.conflicts.borrow_mut().push(literals.to_vec());
    }
}

/// Mock context with configurable set snapshots for extended propagators.
pub(crate) struct MockSetCtx {
    dummy: IntervalDomain,
    pub sets: HashMap<VariableId, SetDomainSnapshot>,
}

impl MockSetCtx {
    pub(crate) fn new() -> Self {
        Self {
            dummy: IntervalDomain::new(0, 0),
            sets: HashMap::new(),
        }
    }

    pub(crate) fn with_set(mut self, var: VariableId, snapshot: SetDomainSnapshot) -> Self {
        self.sets.insert(var, snapshot);
        self
    }
}

impl PropagationContext for MockSetCtx {
    fn domain(&self, _: VariableId) -> &dyn DomainView<Value = i32> {
        &self.dummy
    }

    fn as_extended(&mut self) -> Option<&mut dyn ExtendedPropagationContext> {
        Some(self)
    }

    fn remove_below(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn remove_above(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn remove_value(&mut self, _: VariableId, _: i32) -> bool {
        false
    }

    fn fixed_value(&self, _: VariableId) -> Option<i32> {
        None
    }
}

impl ExtendedPropagationContext for MockSetCtx {
    fn set_domain(&self, var: VariableId) -> Option<SetDomainSnapshot> {
        self.sets.get(&var).cloned()
    }

    fn float_domain(&self, _: VariableId) -> Option<FloatDomainSnapshot> {
        None
    }

    fn force_set_in(&mut self, var: VariableId, value: i32) -> bool {
        let Some(snap) = self.sets.get_mut(&var) else {
            return false;
        };
        if !snap.lub.contains(&value) {
            return false;
        }
        if !snap.glb.contains(&value) {
            snap.glb.push(value);
            snap.glb.sort_unstable();
        }
        true
    }

    fn force_set_out(&mut self, var: VariableId, value: i32) -> bool {
        let Some(snap) = self.sets.get_mut(&var) else {
            return false;
        };
        if snap.glb.contains(&value) {
            return false;
        }
        let before = snap.lub.len();
        snap.lub.retain(|candidate| *candidate != value);
        snap.lub.len() != before
    }

    fn tighten_float_below(&mut self, _: VariableId, _: f64) -> bool {
        false
    }

    fn tighten_float_above(&mut self, _: VariableId, _: f64) -> bool {
        false
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    use propaga_core::DomainView;

    #[test]
    fn wrappers_exercise_all_trait_methods() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 5));
        let fresh = engine.new_variable(IntervalDomain::new(3, 5));

        let mut mut_engine = MutEngine(&mut engine);
        let _ = mut_engine.domain(var);
        let _ = mut_engine.fixed_value(var);
        assert!(mut_engine.remove_below(var, 2));
        assert!(!mut_engine.remove_below(var, 2));
        assert!(mut_engine.remove_above(var, 4));
        assert!(!mut_engine.remove_above(var, 4));
        assert!(mut_engine.remove_value(var, 3));
        assert!(!mut_engine.remove_value(var, 3));

        let mutable = engine.new_variable(IntervalDomain::new(1, 5));
        let tight = engine.new_variable(IntervalDomain::new(4, 4));
        {
            let mut no_ext = NoExtendedCtx::new(&mut engine);
            assert!(no_ext.remove_below(mutable, 2));
            assert!(no_ext.remove_above(mutable, 4));
            assert!(no_ext.remove_value(mutable, 3));
            assert!(!no_ext.remove_below(tight, 4));
            assert!(!no_ext.remove_above(tight, 4));
            assert!(!no_ext.remove_value(tight, 99));
            assert!(!no_ext.remove_below(fresh, 3));
            assert!(!no_ext.remove_above(fresh, 5));
            assert!(!no_ext.remove_value(fresh, 99));
            no_ext.exercise_mutators(fresh);
        }

        let read_only = ReadOnlyEngine(&engine);
        let _ = read_only.domain(var);
        let _ = read_only.fixed_value(var);
        let mut read_only_mut = ReadOnlyEngine(&engine);
        let _ = read_only_mut.remove_below(var, 0);
        let _ = read_only_mut.remove_above(var, 0);
        let _ = read_only_mut.remove_value(var, 0);
    }

    #[test]
    fn singleton_open_domain_trait_methods() {
        let domain = SingletonOpenDomain::new(7);
        assert!(domain.contains(7));
        assert!(!domain.contains(6));
        assert_eq!(domain.size(), 1);
        assert_eq!(domain.min(), Some(7));
        assert_eq!(domain.max(), Some(7));
        assert!(!domain.is_empty());
        assert!(!domain.is_fixed());
    }

    #[test]
    fn mock_open_singleton_domain_mutators() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new().with_open_singleton(var, 5);
        assert!(ctx.remove_below(var, 6));
        assert!(ctx.domain(var).is_empty());
    }

    #[test]
    fn mock_int_ctx_exercises_all_paths() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![4, 5])
            .with_fixed(left, 2);

        let domain = ctx.domain(left);
        assert!(domain.contains(2));
        assert_eq!(domain.size(), 3);
        assert_eq!(domain.min(), Some(1));
        assert_eq!(domain.max(), Some(3));
        assert!(!domain.is_empty());
        assert!(!domain.is_fixed());
        assert_eq!(ctx.fixed_value(left), Some(2));
        assert_eq!(ctx.fixed_value(right), None);

        assert!(ctx.remove_below(left, 2));
        assert!(!ctx.remove_below(left, 2));
        assert!(ctx.remove_above(right, 4));
        assert!(!ctx.remove_above(right, 4));
        assert!(ctx.remove_value(left, 3));
        assert!(!ctx.remove_value(left, 3));
        assert_eq!(ctx.domain_values(left), vec![2]);
        assert_eq!(ctx.domain_values(right), vec![4]);

        let open = engine.new_variable(IntervalDomain::new(0, 0));
        let mut open_ctx = MockIntCtx::new().with_open_singleton(open, 9);
        assert_eq!(open_ctx.domain_values(open), vec![9]);
        assert!(!open_ctx.remove_below(open, 0));
        assert!(!open_ctx.remove_value(open, 8));
        assert!(open_ctx.remove_value(open, 9));
        assert!(open_ctx.domain(open).is_empty());
        assert!(!open_ctx.remove_below(open, 0));
        assert!(!open_ctx.remove_above(open, 10));
        assert!(!open_ctx.remove_value(open, 9));

        let open2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut open_ctx2 = MockIntCtx::new().with_open_singleton(open2, 9);
        assert!(open_ctx2.remove_above(open2, 5));
        assert!(!open_ctx2.remove_above(open2, 20));
    }

    #[test]
    fn mock_set_ctx_exercises_extended_trait() {
        use propaga_core::SetDomainSnapshot;

        let mut engine = Engine::new();
        let set_var = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockSetCtx::new().with_set(
            set_var,
            SetDomainSnapshot {
                glb: vec![1],
                lub: vec![1, 2, 3],
                card_min: 1,
                card_max: 3,
            },
        );
        let _ = ctx.as_extended();
        let _ = ctx.set_domain(set_var);
        assert!(ctx.set_domain(set_var).is_some());
        assert!(ctx.float_domain(set_var).is_none());
        assert!(ctx.force_set_out(set_var, 3));
        assert!(!ctx.force_set_out(set_var, 1));
        assert!(ctx.force_set_in(set_var, 2));
        assert!(!ctx.force_set_in(set_var, 9));
        assert!(!ctx.tighten_float_below(set_var, 0.0));
        assert!(!ctx.tighten_float_above(set_var, 1.0));

        let missing = engine.new_variable(IntervalDomain::new(0, 0));
        assert!(!ctx.force_set_in(missing, 1));
        assert!(!ctx.force_set_out(missing, 1));
        assert!(!ctx.force_set_out(set_var, 1));
        let _ = ctx.domain(set_var);
        let _ = ctx.fixed_value(set_var);
        let _ = ctx.remove_below(set_var, 0);
        let _ = ctx.remove_above(set_var, 0);
        let _ = ctx.remove_value(set_var, 0);
    }
}
