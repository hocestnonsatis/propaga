use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Direction for a single objective in an integer dominance cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DominanceCutDirection {
    /// Cut the weakly-dominated orthant of a minimization objective.
    Minimize,
    /// Cut the weakly-dominated orthant of a maximization objective.
    Maximize,
}

/// Prunes assignments weakly dominated by a previously found objective vector.
///
/// For minimize thresholds `v`, any assignment with all `obj_i >= v_i` is forbidden.
/// Propagation forces improvement on the last open objective when the others cannot improve.
#[derive(Clone, Debug)]
pub struct IntDominanceCutPropagator {
    watched: Vec<VariableId>,
    cuts: Vec<(VariableId, DominanceCutDirection, i32)>,
}

impl IntDominanceCutPropagator {
    /// Creates a dominance cut over integer objectives and their incumbent values.
    #[must_use]
    pub fn new(cuts: Vec<(VariableId, DominanceCutDirection, i32)>) -> Self {
        let mut watched = Vec::with_capacity(cuts.len());
        for (var, _, _) in &cuts {
            if !watched.contains(var) {
                watched.push(*var);
            }
        }
        Self { watched, cuts }
    }
}

impl Propagator for IntDominanceCutPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        5
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut improvable = Vec::new();
        for &(var, direction, value) in &self.cuts {
            if can_improve(ctx, var, direction, value) {
                improvable.push((var, direction, value));
            }
        }

        if improvable.is_empty() {
            return PropagationStatus::Failure;
        }

        if improvable.len() == 1 {
            let (var, direction, value) = improvable[0];
            if !force_improve(ctx, var, direction, value) {
                return PropagationStatus::Failure;
            }
            if ctx.domain(var).is_empty() {
                return PropagationStatus::Failure;
            }
            return PropagationStatus::OkChanged;
        }

        PropagationStatus::OkNoChange
    }
}

fn can_improve(
    ctx: &dyn PropagationContext,
    var: VariableId,
    direction: DominanceCutDirection,
    value: i32,
) -> bool {
    match direction {
        DominanceCutDirection::Minimize => ctx.domain(var).min().is_some_and(|min| min < value),
        DominanceCutDirection::Maximize => ctx.domain(var).max().is_some_and(|max| max > value),
    }
}

fn force_improve(
    ctx: &mut dyn PropagationContext,
    var: VariableId,
    direction: DominanceCutDirection,
    value: i32,
) -> bool {
    match direction {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear_eq::LinearEqPropagator;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
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
}
