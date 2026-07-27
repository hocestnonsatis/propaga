use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `|set| = card` for a set variable and an integer cardinality variable.
#[derive(Clone, Debug)]
pub struct SetCardEqPropagator {
    watched: [VariableId; 2],
}

impl SetCardEqPropagator {
    #[must_use]
    pub fn new(set: VariableId, card: VariableId) -> Self {
        Self {
            watched: [set, card],
        }
    }
}

impl Propagator for SetCardEqPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let (set_id, card_id) = (self.watched[0], self.watched[1]);
        let set = {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            let Some(set) = ext.set_domain(set_id) else {
                return PropagationStatus::Failure;
            };
            if set.is_empty() {
                return PropagationStatus::Failure;
            }
            set.clone()
        };

        let card_domain = ctx.domain(card_id);
        let Some(card_min_i) = card_domain.min() else {
            return PropagationStatus::Failure;
        };
        let Some(card_max_i) = card_domain.max() else {
            return PropagationStatus::Failure;
        };
        if card_max_i < 0 {
            return PropagationStatus::Failure;
        }
        let int_lo = card_min_i.max(0) as usize;
        let int_hi = card_max_i as usize;

        let new_min = set.card_min.max(int_lo).max(set.glb.len());
        let new_max = set.card_max.min(int_hi).min(set.lub.len());
        if new_min > new_max {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        if new_min != set.card_min || new_max != set.card_max {
            let Some(ext) = ctx.as_extended() else {
                return PropagationStatus::OkNoChange;
            };
            changed |= ext.tighten_set_cardinality(set_id, new_min, new_max);
        }

        if ctx.remove_below(card_id, new_min as i32) {
            changed = true;
        }
        if ctx.remove_above(card_id, new_max as i32) {
            changed = true;
        }

        let set = {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            let Some(set) = ext.set_domain(set_id) else {
                return PropagationStatus::Failure;
            };
            if set.is_empty() {
                return PropagationStatus::Failure;
            }
            set.clone()
        };

        if set.glb.len() == set.card_max {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            for value in set.undecided() {
                changed |= ext.force_set_out(set_id, value);
            }
        } else if set.lub.len() == set.card_min {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            for value in set.undecided() {
                changed |= ext.force_set_in(set_id, value);
            }
        }

        let set_after = {
            let Some(ext) = ctx.as_extended() else {
                return if changed {
                    PropagationStatus::OkChanged
                } else {
                    PropagationStatus::OkNoChange
                };
            };
            ext.set_domain(set_id)
        };
        if set_after.as_ref().is_none_or(|d| d.is_empty()) {
            return PropagationStatus::Failure;
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
    use propaga_domains::{AnyDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn syncs_int_bounds_from_set_cardinality() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=4).with_cardinality(2, 3);
        let s = engine.new_variable(AnyDomain::Set(set));
        let k = engine.new_variable(IntervalDomain::new(0, 5));
        engine.add_propagator(Box::new(SetCardEqPropagator::new(s, k)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(k).min(), Some(2));
        assert_eq!(engine.hybrid_domain(k).max(), Some(3));
    }

    #[test]
    fn syncs_set_cardinality_from_int_bounds() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=4).with_cardinality(0, 4);
        let s = engine.new_variable(AnyDomain::Set(set));
        let k = engine.new_variable(IntervalDomain::new(2, 2));
        engine.add_propagator(Box::new(SetCardEqPropagator::new(s, k)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(s).as_set().unwrap().card_min(), 2);
        assert_eq!(engine.domain(s).as_set().unwrap().card_max(), 2);
    }

    #[test]
    fn forces_out_when_card_at_max() {
        let mut engine = Engine::new();
        let set = SetIntervalDomain::universe(1..=3)
            .with_cardinality(0, 3)
            .force_in(1)
            .unwrap();
        let s = engine.new_variable(AnyDomain::Set(set));
        let k = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(SetCardEqPropagator::new(s, k)));
        engine.propagate_all().unwrap();
        let domain = engine.domain(s).as_set().unwrap();
        assert!(!domain.lub().contains(&2));
        assert!(!domain.lub().contains(&3));
    }
}
