use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Direction for a single objective in a dominance cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DominanceCutDirection {
    /// Cut the weakly-dominated orthant of a minimization objective.
    Minimize,
    /// Cut the weakly-dominated orthant of a maximization objective.
    Maximize,
}

/// One typed objective threshold in a dominance cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DominanceCutTarget {
    /// Integer objective variable and incumbent value.
    Int {
        /// Objective variable.
        var: VariableId,
        /// Optimization direction.
        direction: DominanceCutDirection,
        /// Incumbent objective value.
        value: i32,
    },
    /// Floating-point objective variable and incumbent value.
    Float {
        /// Objective variable.
        var: VariableId,
        /// Optimization direction.
        direction: DominanceCutDirection,
        /// Incumbent objective value.
        value: f64,
    },
    /// Set-cardinality objective and incumbent cardinality.
    SetCardinality {
        /// Set variable optimized by cardinality.
        var: VariableId,
        /// Optimization direction.
        direction: DominanceCutDirection,
        /// Incumbent cardinality.
        value: usize,
    },
}

impl DominanceCutTarget {
    fn var(self) -> VariableId {
        match self {
            Self::Int { var, .. } | Self::Float { var, .. } | Self::SetCardinality { var, .. } => {
                var
            }
        }
    }
}

/// Prunes assignments weakly dominated by a previously found objective vector.
///
/// When only one objective can still improve, that objective is forced to improve.
#[derive(Clone, Debug)]
pub struct DominanceCutPropagator {
    watched: Vec<VariableId>,
    cuts: Vec<DominanceCutTarget>,
}

impl DominanceCutPropagator {
    /// Creates a dominance cut over typed objective thresholds.
    #[must_use]
    pub fn new(cuts: Vec<DominanceCutTarget>) -> Self {
        let mut watched = Vec::with_capacity(cuts.len());
        for cut in &cuts {
            let var = cut.var();
            if !watched.contains(&var) {
                watched.push(var);
            }
        }
        Self { watched, cuts }
    }
}

/// Integer-only dominance cut (compat wrapper).
#[derive(Clone, Debug)]
pub struct IntDominanceCutPropagator {
    inner: DominanceCutPropagator,
}

impl IntDominanceCutPropagator {
    /// Creates a dominance cut over integer objectives and their incumbent values.
    #[must_use]
    pub fn new(cuts: Vec<(VariableId, DominanceCutDirection, i32)>) -> Self {
        let cuts = cuts
            .into_iter()
            .map(|(var, direction, value)| DominanceCutTarget::Int {
                var,
                direction,
                value,
            })
            .collect();
        Self {
            inner: DominanceCutPropagator::new(cuts),
        }
    }
}

impl Propagator for IntDominanceCutPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        self.inner.watched_variables()
    }

    fn priority(&self) -> u32 {
        self.inner.priority()
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        self.inner.propagate(ctx)
    }
}

impl Propagator for DominanceCutPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        5
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut improvable = Vec::new();
        for cut in &self.cuts {
            if can_improve(ctx, *cut) {
                improvable.push(*cut);
            }
        }

        if improvable.is_empty() {
            return PropagationStatus::Failure;
        }

        if improvable.len() == 1 {
            let cut = improvable[0];
            if !force_improve(ctx, cut) {
                return PropagationStatus::Failure;
            }
            return PropagationStatus::OkChanged;
        }

        PropagationStatus::OkNoChange
    }
}

fn can_improve(ctx: &mut dyn PropagationContext, cut: DominanceCutTarget) -> bool {
    match cut {
        DominanceCutTarget::Int {
            var,
            direction,
            value,
        } => match direction {
            DominanceCutDirection::Minimize => ctx.domain(var).min().is_some_and(|min| min < value),
            DominanceCutDirection::Maximize => ctx.domain(var).max().is_some_and(|max| max > value),
        },
        DominanceCutTarget::Float {
            var,
            direction,
            value,
        } => {
            let Some(ext) = ctx.as_extended() else {
                return false;
            };
            let Some(domain) = ext.float_domain(var) else {
                return false;
            };
            match direction {
                DominanceCutDirection::Minimize => domain.min < value,
                DominanceCutDirection::Maximize => domain.max > value,
            }
        }
        DominanceCutTarget::SetCardinality {
            var,
            direction,
            value,
        } => {
            let Some(ext) = ctx.as_extended() else {
                return false;
            };
            let Some(domain) = ext.set_domain(var) else {
                return false;
            };
            match direction {
                DominanceCutDirection::Minimize => domain.card_min < value,
                DominanceCutDirection::Maximize => domain.card_max > value,
            }
        }
    }
}

fn force_improve(ctx: &mut dyn PropagationContext, cut: DominanceCutTarget) -> bool {
    match cut {
        DominanceCutTarget::Int {
            var,
            direction,
            value,
        } => match direction {
            DominanceCutDirection::Minimize => match value.checked_sub(1) {
                Some(bound) => {
                    let _ = ctx.remove_above(var, bound);
                    ctx.domain(var).max().is_some_and(|max| max <= bound)
                }
                None => false,
            },
            DominanceCutDirection::Maximize => match value.checked_add(1) {
                Some(bound) => {
                    let _ = ctx.remove_below(var, bound);
                    ctx.domain(var).min().is_some_and(|min| min >= bound)
                }
                None => false,
            },
        },
        DominanceCutTarget::Float {
            var,
            direction,
            value,
        } => {
            let Some(ext) = ctx.as_extended() else {
                return false;
            };
            match direction {
                DominanceCutDirection::Minimize => {
                    let bound = next_float_down(value);
                    if !(bound < value) {
                        return false;
                    }
                    let _ = ext.tighten_float_above(var, bound);
                    ext.float_domain(var)
                        .is_some_and(|domain| !domain.is_empty() && domain.max <= bound)
                }
                DominanceCutDirection::Maximize => {
                    let bound = next_float_up(value);
                    if !(bound > value) {
                        return false;
                    }
                    let _ = ext.tighten_float_below(var, bound);
                    ext.float_domain(var)
                        .is_some_and(|domain| !domain.is_empty() && domain.min >= bound)
                }
            }
        }
        DominanceCutTarget::SetCardinality {
            var,
            direction,
            value,
        } => {
            let Some(ext) = ctx.as_extended() else {
                return false;
            };
            let Some(domain) = ext.set_domain(var) else {
                return false;
            };
            let (card_min, card_max) = match direction {
                DominanceCutDirection::Minimize => {
                    if value == 0 {
                        return false;
                    }
                    (domain.card_min, value - 1)
                }
                DominanceCutDirection::Maximize => (value + 1, domain.card_max),
            };
            if card_min > card_max {
                return false;
            }
            let _ = ext.tighten_set_cardinality(var, card_min, card_max);
            ext.set_domain(var).is_some_and(|domain| {
                !domain.is_empty() && domain.card_min >= card_min && domain.card_max <= card_max
            })
        }
    }
}

fn next_float_up(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_positive() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_add(1))
    }
}

fn next_float_down(value: f64) -> f64 {
    if value.is_infinite() && value.is_sign_negative() {
        value
    } else {
        f64::from_bits(value.to_bits().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear_eq::LinearEqPropagator;
    use crate::set_card::SetCardPropagator;
    use propaga_core::DomainView;
    use propaga_domains::{AnyDomain, FloatDomain, IntervalDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn forces_improvement_when_one_objective_is_stuck() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(IntDominanceCutPropagator::new(vec![
            (x, DominanceCutDirection::Minimize, 1),
            (y, DominanceCutDirection::Minimize, 3),
        ])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        assert_eq!(engine.domain(y).as_int().unwrap().max(), Some(2));
        assert_eq!(engine.domain(x).as_int().unwrap().min(), Some(1));
    }

    #[test]
    fn cut_leaves_other_pareto_points() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let sum = engine.new_variable(IntervalDomain::fix(4));
        engine.add_propagator(Box::new(LinearEqPropagator::new(x, y, sum)));
        engine.add_propagator(Box::new(IntDominanceCutPropagator::new(vec![
            (x, DominanceCutDirection::Minimize, 1),
            (y, DominanceCutDirection::Minimize, 3),
        ])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        assert!(engine.domain(y).as_int().unwrap().max().unwrap() <= 2);
        assert!(engine.domain(x).as_int().unwrap().min().unwrap() >= 2);
    }

    #[test]
    fn float_cut_forces_improvement() {
        let mut engine = Engine::new();
        let x = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        engine.add_propagator(Box::new(DominanceCutPropagator::new(vec![
            DominanceCutTarget::Float {
                var: x,
                direction: DominanceCutDirection::Minimize,
                value: 0.0,
            },
            DominanceCutTarget::Float {
                var: y,
                direction: DominanceCutDirection::Minimize,
                value: 2.0,
            },
        ])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        let y_dom = engine.domain(y).as_float().unwrap();
        assert!(y_dom.upper_bound() < 2.0);
    }

    #[test]
    fn set_card_cut_forces_improvement() {
        let mut engine = Engine::new();
        let a = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3).with_cardinality(1, 3),
        ));
        let b = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3).with_cardinality(1, 3),
        ));
        engine.add_propagator(Box::new(SetCardPropagator::new(a)));
        engine.add_propagator(Box::new(SetCardPropagator::new(b)));
        engine.add_propagator(Box::new(DominanceCutPropagator::new(vec![
            DominanceCutTarget::SetCardinality {
                var: a,
                direction: DominanceCutDirection::Minimize,
                value: 1,
            },
            DominanceCutTarget::SetCardinality {
                var: b,
                direction: DominanceCutDirection::Minimize,
                value: 3,
            },
        ])));
        let status = engine.commit_initial_propagation().unwrap();
        assert!(!status.is_failure());
        assert!(engine.domain(b).as_set().unwrap().card_max() <= 2);
    }
}
