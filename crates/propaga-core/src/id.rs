use slotmap::new_key_type;
use std::fmt;

new_key_type! {
    /// Opaque key for a decision variable stored in the engine arena.
    pub struct VariableKey;
    /// Opaque key for a propagator stored in the engine arena.
    pub struct PropagatorKey;
}

/// Stable handle to a decision variable.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId(pub(crate) VariableKey);

impl VariableId {
    /// Returns the underlying arena key.
    #[must_use]
    pub const fn key(self) -> VariableKey {
        self.0
    }

    /// Creates an id from an arena key.
    #[must_use]
    pub const fn from_key(key: VariableKey) -> Self {
        Self(key)
    }
}

impl fmt::Debug for VariableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VariableId({:?})", self.0)
    }
}

/// Stable handle to a propagator.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropagatorId(pub(crate) PropagatorKey);

impl PropagatorId {
    /// Returns the underlying arena key.
    #[must_use]
    pub const fn key(self) -> PropagatorKey {
        self.0
    }

    /// Creates an id from an arena key.
    #[must_use]
    pub const fn from_key(key: PropagatorKey) -> Self {
        Self(key)
    }
}

impl fmt::Debug for PropagatorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PropagatorId({:?})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn variable_id_roundtrips_key() {
        let mut sm: SlotMap<VariableKey, ()> = SlotMap::with_key();
        let key = sm.insert(());
        let id = VariableId::from_key(key);
        assert_eq!(id.key(), key);
    }

    #[test]
    fn propagator_id_roundtrips_key() {
        let mut sm: SlotMap<PropagatorKey, ()> = SlotMap::with_key();
        let key = sm.insert(());
        let id = PropagatorId::from_key(key);
        assert_eq!(id.key(), key);
    }
}
