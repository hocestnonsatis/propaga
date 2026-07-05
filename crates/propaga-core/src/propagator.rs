use crate::{PropagationContext, PropagationStatus, VariableId};
use dyn_clone::DynClone;

dyn_clone::clone_trait_object!(Propagator);

/// A constraint propagator that reacts to domain changes on watched variables.
pub trait Propagator: DynClone + Send {
    /// Variables whose domains this propagator depends on.
    fn watched_variables(&self) -> &[VariableId];

    /// Tightens watched domains according to the constraint.
    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus;

    /// Scheduling priority. Lower values run first.
    fn priority(&self) -> u32 {
        0
    }
}
