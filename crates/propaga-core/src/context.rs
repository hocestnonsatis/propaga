use crate::{DomainView, PropagatorId, VariableId};

/// Mutable propagation view used by propagators to read and tighten domains.
pub trait PropagationContext {
    /// Returns a read-only view of `var`'s domain.
    fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32>;

    /// Returns the propagator currently executing, if any.
    fn current_propagator(&self) -> Option<PropagatorId> {
        None
    }

    /// Removes values strictly below `bound`. Returns `true` if the domain changed.
    fn remove_below(&mut self, var: VariableId, bound: i32) -> bool;

    /// Removes values strictly above `bound`. Returns `true` if the domain changed.
    fn remove_above(&mut self, var: VariableId, bound: i32) -> bool;

    /// Removes `value` from the domain. Returns `true` if the domain changed.
    fn remove_value(&mut self, var: VariableId, value: i32) -> bool;

    /// Returns the fixed value of `var`, if it is assigned.
    fn fixed_value(&self, var: VariableId) -> Option<i32>;

    /// Records branch literals explaining a propagator-detected conflict.
    fn record_propagator_conflict(&mut self, _literals: &[(VariableId, i32)]) {}
}
