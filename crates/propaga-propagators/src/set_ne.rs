use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator,
    SetDomainSnapshot, VariableId,
};

/// Propagates `left != right` for set variables.
#[derive(Clone, Debug)]
pub struct SetNePropagator {
    watched: [VariableId; 2],
}

impl SetNePropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId) -> Self {
        Self {
            watched: [left, right],
        }
    }
}

fn is_fixed(set: &SetDomainSnapshot) -> bool {
    set.glb.len() == set.lub.len()
}

fn definitely_equal(left: &SetDomainSnapshot, right: &SetDomainSnapshot) -> bool {
    is_fixed(left) && is_fixed(right) && left.glb == right.glb
}

fn definitely_ne(left: &SetDomainSnapshot, right: &SetDomainSnapshot) -> bool {
    left.glb.iter().any(|v| !right.lub.contains(v))
        || right.glb.iter().any(|v| !left.lub.contains(v))
        || left.card_max < right.card_min
        || right.card_max < left.card_min
}

/// When `fixed` is a fixed set `S` and `other` can only equal `S` by taking one
/// remaining undecided member, force that member out so inequality holds.
fn break_last_equalizer(
    ext: &mut dyn ExtendedPropagationContext,
    fixed_id: VariableId,
    other_id: VariableId,
) -> Result<bool, ()> {
    let (Some(fixed), Some(other)) = (ext.set_domain(fixed_id), ext.set_domain(other_id)) else {
        return Err(());
    };
    if !is_fixed(&fixed) || definitely_ne(&fixed, &other) {
        return Ok(false);
    }
    // other ⊆ S is forced when lub == S; equality requires taking every element of S.
    if other.lub != fixed.glb {
        return Ok(false);
    }
    let undecided: Vec<i32> = fixed
        .glb
        .iter()
        .copied()
        .filter(|v| !other.glb.contains(v))
        .collect();
    if undecided.len() != 1 {
        return Ok(false);
    }
    let last = undecided[0];
    if other.card_max < fixed.glb.len() {
        return Ok(false);
    }
    let mut changed = ext.force_set_out(other_id, last);
    let new_card_max = fixed.glb.len().saturating_sub(1);
    if let Some(other) = ext.set_domain(other_id)
        && other.card_max > new_card_max
    {
        changed |= ext.tighten_set_cardinality(other_id, other.card_min, new_card_max);
    }
    Ok(changed)
}

impl Propagator for SetNePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (left_id, right_id) = (self.watched[0], self.watched[1]);
        let (left, right) = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id))
            else {
                return PropagationStatus::Failure;
            };
            if left.is_empty() || right.is_empty() {
                return PropagationStatus::Failure;
            }
            (left.clone(), right.clone())
        };

        if definitely_equal(&left, &right) {
            return PropagationStatus::Failure;
        }
        if definitely_ne(&left, &right) {
            return PropagationStatus::OkNoChange;
        }

        let mut changed = false;
        {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            match break_last_equalizer(ext, left_id, right_id) {
                Ok(c) => changed |= c,
                Err(()) => return PropagationStatus::Failure,
            }
            match break_last_equalizer(ext, right_id, left_id) {
                Ok(c) => changed |= c,
                Err(()) => return PropagationStatus::Failure,
            }

            let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id))
            else {
                return PropagationStatus::Failure;
            };
            if left.is_empty() || right.is_empty() || definitely_equal(&left, &right) {
                return PropagationStatus::Failure;
            }
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
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn fails_when_both_fixed_equal() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetNePropagator::new(a, b)));
        let status = engine.propagate_all().unwrap();
        assert!(status.is_failure());
    }

    #[test]
    fn forces_out_last_equalizer() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=2)
            .with_cardinality(2, 2)
            .force_in(1)
            .unwrap()
            .force_in(2)
            .unwrap();
        let right = SetIntervalDomain::universe(1..=2)
            .with_cardinality(0, 2)
            .force_in(1)
            .unwrap();
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetNePropagator::new(a, b)));
        engine.propagate_all().unwrap();
        assert!(!engine.domain(b).as_set().unwrap().lub().contains(&2));
    }

    #[test]
    fn accepts_already_unequal() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=3)
            .with_cardinality(1, 1)
            .force_in(1)
            .unwrap();
        let right = SetIntervalDomain::universe(2..=3).with_cardinality(0, 2);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetNePropagator::new(a, b)));
        engine.propagate_all().unwrap();
    }

    #[test]
    fn accepts_disjoint_cardinality() {
        let mut engine = Engine::new();
        let left = SetIntervalDomain::universe(1..=4).with_cardinality(0, 1);
        let right = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let a = engine.new_variable(AnyDomain::Set(left));
        let b = engine.new_variable(AnyDomain::Set(right));
        engine.add_propagator(Box::new(SetNePropagator::new(a, b)));
        let status = engine.propagate_all().unwrap();
        assert!(!status.is_failure());
    }
}
