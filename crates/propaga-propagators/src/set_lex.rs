use propaga_core::{
    ExtendedPropagationContext, PropagationContext, PropagationStatus, Propagator,
    SetDomainSnapshot, VariableId,
};

/// MiniZinc / FlatZinc set ordering: compare sorted element lists lexicographically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetLexOp {
    /// `left ≤_lex right`
    Le,
    /// `left <_lex right`
    Lt,
}

/// Propagates lexicographic set comparison (`set_le` / `set_lt` in FlatZinc).
#[derive(Clone, Debug)]
pub struct SetLexPropagator {
    watched: [VariableId; 2],
    op: SetLexOp,
}

impl SetLexPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, op: SetLexOp) -> Self {
        Self {
            watched: [left, right],
            op,
        }
    }
}

/// Propagates `reif <=> (left ∘_lex right)` for FlatZinc `set_le_reif` / `set_lt_reif`.
#[derive(Clone, Debug)]
pub struct SetLexReifPropagator {
    watched: [VariableId; 3],
    op: SetLexOp,
}

impl SetLexReifPropagator {
    #[must_use]
    pub fn new(left: VariableId, right: VariableId, reif: VariableId, op: SetLexOp) -> Self {
        Self {
            watched: [left, right, reif],
            op,
        }
    }
}

impl Propagator for SetLexPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (left_id, right_id) = (self.watched[0], self.watched[1]);
        let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id)) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() {
            return PropagationStatus::Failure;
        }

        if !feasible_bounds(&left, &right, self.op) {
            return PropagationStatus::Failure;
        }

        let left_fixed = is_fixed_set(&left);
        let right_fixed = is_fixed_set(&right);
        if left_fixed && right_fixed {
            let holds = match self.op {
                SetLexOp::Le => sorted_lex_le(&left.glb, &right.glb),
                SetLexOp::Lt => sorted_lex_lt(&left.glb, &right.glb),
            };
            return if holds {
                PropagationStatus::OkNoChange
            } else {
                PropagationStatus::Failure
            };
        }

        let mut changed = false;
        if left_fixed {
            changed |= force_right_to_dominate(ext, right_id, &left.glb, self.op);
        }
        if right_fixed {
            changed |= force_left_not_to_exceed(ext, left_id, &right.glb, self.op);
        }

        let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id)) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() || !feasible_bounds(&left, &right, self.op) {
            return PropagationStatus::Failure;
        }

        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

impl Propagator for SetLexReifPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let Some(ext) = ctx.as_extended() else {
            return PropagationStatus::OkNoChange;
        };
        let (left_id, right_id, reif) = (self.watched[0], self.watched[1], self.watched[2]);
        let (Some(left), Some(right)) = (ext.set_domain(left_id), ext.set_domain(right_id)) else {
            return PropagationStatus::Failure;
        };
        if left.is_empty() || right.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        let reif_fixed = ctx.fixed_value(reif);

        let inevitable = match (max_lex_set(&left), min_lex_set(&right)) {
            (Some(max_left), Some(min_right)) => relation_holds(&max_left, &min_right, self.op),
            _ => false,
        };
        let impossible = !feasible_bounds(&left, &right, self.op);

        if inevitable {
            if reif_fixed == Some(0) {
                return PropagationStatus::Failure;
            }
            changed |= ctx.remove_below(reif, 1);
            changed |= ctx.remove_above(reif, 1);
        } else if impossible {
            if reif_fixed == Some(1) {
                return PropagationStatus::Failure;
            }
            changed |= ctx.remove_below(reif, 0);
            changed |= ctx.remove_above(reif, 0);
        }

        if ctx.fixed_value(reif) == Some(1) {
            let mut inner = SetLexPropagator::new(left_id, right_id, self.op);
            return match inner.propagate(ctx) {
                PropagationStatus::Failure => PropagationStatus::Failure,
                PropagationStatus::OkChanged => PropagationStatus::OkChanged,
                PropagationStatus::OkNoChange if changed => PropagationStatus::OkChanged,
                other => other,
            };
        }

        if ctx.fixed_value(reif) == Some(0) {
            // ¬(A ≤ B) ≡ B < A; ¬(A < B) ≡ B ≤ A.
            let negated = match self.op {
                SetLexOp::Le => SetLexOp::Lt,
                SetLexOp::Lt => SetLexOp::Le,
            };
            let mut inner = SetLexPropagator::new(right_id, left_id, negated);
            return match inner.propagate(ctx) {
                PropagationStatus::Failure => PropagationStatus::Failure,
                PropagationStatus::OkChanged => PropagationStatus::OkChanged,
                PropagationStatus::OkNoChange if changed => PropagationStatus::OkChanged,
                other => other,
            };
        }

        if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn is_fixed_set(domain: &SetDomainSnapshot) -> bool {
    domain.glb.len() == domain.lub.len()
        && domain.glb.len() == domain.card_min
        && domain.card_min == domain.card_max
}

fn feasible_bounds(left: &SetDomainSnapshot, right: &SetDomainSnapshot, op: SetLexOp) -> bool {
    let (Some(min_left), Some(max_right)) = (min_lex_set(left), max_lex_set(right)) else {
        return false;
    };
    match op {
        SetLexOp::Le => sorted_lex_le(&min_left, &max_right),
        SetLexOp::Lt => sorted_lex_lt(&min_left, &max_right),
    }
}

fn relation_holds(left: &[i32], right: &[i32], op: SetLexOp) -> bool {
    match op {
        SetLexOp::Le => sorted_lex_le(left, right),
        SetLexOp::Lt => sorted_lex_lt(left, right),
    }
}

fn sorted_lex_le(left: &[i32], right: &[i32]) -> bool {
    let n = left.len().min(right.len());
    for i in 0..n {
        if left[i] < right[i] {
            return true;
        }
        if left[i] > right[i] {
            return false;
        }
    }
    left.len() <= right.len()
}

fn sorted_lex_lt(left: &[i32], right: &[i32]) -> bool {
    sorted_lex_le(left, right) && left != right
}

fn min_lex_set(domain: &SetDomainSnapshot) -> Option<Vec<i32>> {
    let card_min = domain.card_min.max(domain.glb.len());
    let card_max = domain.card_max.min(domain.lub.len());
    if card_min > card_max || !domain.glb.iter().all(|v| domain.lub.contains(v)) {
        return None;
    }
    let mut result = domain.glb.clone();
    result.sort_unstable();
    let mut pool: Vec<i32> = domain
        .lub
        .iter()
        .copied()
        .filter(|value| !domain.glb.contains(value))
        .collect();
    pool.sort_unstable();
    let need = card_min.saturating_sub(result.len());
    if pool.len() < need {
        return None;
    }
    result.extend(pool.into_iter().take(need));
    result.sort_unstable();
    Some(result)
}

fn max_lex_set(domain: &SetDomainSnapshot) -> Option<Vec<i32>> {
    let card_min = domain.card_min.max(domain.glb.len());
    let card_max = domain.card_max.min(domain.lub.len());
    if card_min > card_max || !domain.glb.iter().all(|v| domain.lub.contains(v)) {
        return None;
    }
    let pool: Vec<i32> = domain
        .lub
        .iter()
        .copied()
        .filter(|value| !domain.glb.contains(value))
        .collect();
    let mut best: Option<Vec<i32>> = None;
    for size in card_min..=card_max {
        let extra = size.saturating_sub(domain.glb.len());
        if extra > pool.len() {
            continue;
        }
        let mut candidate = domain.glb.clone();
        let mut sorted_pool = pool.clone();
        sorted_pool.sort_unstable();
        candidate.extend(sorted_pool.into_iter().rev().take(extra));
        candidate.sort_unstable();
        if best
            .as_ref()
            .is_none_or(|cur| sorted_lex_lt(cur, &candidate))
        {
            best = Some(candidate);
        }
    }
    best
}

fn without_value(domain: &SetDomainSnapshot, value: i32) -> SetDomainSnapshot {
    SetDomainSnapshot {
        glb: domain.glb.clone(),
        lub: domain
            .lub
            .iter()
            .copied()
            .filter(|&candidate| candidate != value)
            .collect(),
        card_min: domain.card_min,
        card_max: domain.card_max.min(domain.lub.len().saturating_sub(1)),
    }
}

fn with_value(domain: &SetDomainSnapshot, value: i32) -> SetDomainSnapshot {
    let mut glb = domain.glb.clone();
    glb.push(value);
    glb.sort_unstable();
    glb.dedup();
    SetDomainSnapshot {
        glb: glb.clone(),
        lub: domain.lub.clone(),
        card_min: domain.card_min.max(glb.len()),
        card_max: domain.card_max,
    }
}

/// When `left` is fixed, force membership on `right` required for `left ∘ right`.
fn force_right_to_dominate(
    ext: &mut dyn ExtendedPropagationContext,
    right: VariableId,
    left: &[i32],
    op: SetLexOp,
) -> bool {
    let Some(dom) = ext.set_domain(right) else {
        return false;
    };
    let mut changed = false;
    for &value in &dom.undecided() {
        let without = without_value(&dom, value);
        let feasible_without =
            max_lex_set(&without).is_some_and(|max_right| relation_holds(left, &max_right, op));
        if !feasible_without {
            changed |= ext.force_set_in(right, value);
        }
    }
    let Some(dom) = ext.set_domain(right) else {
        return changed;
    };
    for &value in &dom.undecided() {
        let with = with_value(&dom, value);
        let feasible_with =
            max_lex_set(&with).is_some_and(|max_right| relation_holds(left, &max_right, op));
        if !feasible_with {
            changed |= ext.force_set_out(right, value);
        }
    }
    changed
}

/// When `right` is fixed, exclude left memberships that always violate `left ∘ right`.
fn force_left_not_to_exceed(
    ext: &mut dyn ExtendedPropagationContext,
    left: VariableId,
    right: &[i32],
    op: SetLexOp,
) -> bool {
    let Some(dom) = ext.set_domain(left) else {
        return false;
    };
    let mut changed = false;
    for &value in &dom.undecided() {
        let with = with_value(&dom, value);
        let feasible_with =
            min_lex_set(&with).is_some_and(|min_left| relation_holds(&min_left, right, op));
        if !feasible_with {
            changed |= ext.force_set_out(left, value);
        }
    }
    let Some(dom) = ext.set_domain(left) else {
        return changed;
    };
    for &value in &dom.undecided() {
        let without = without_value(&dom, value);
        let feasible_without =
            min_lex_set(&without).is_some_and(|min_left| relation_holds(&min_left, right, op));
        if !feasible_without {
            changed |= ext.force_set_in(left, value);
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::{AnyDomain, SetIntervalDomain};
    use propaga_engine::Engine;

    #[test]
    fn sorted_lex_helpers() {
        assert!(sorted_lex_le(&[1], &[1, 2]));
        assert!(sorted_lex_lt(&[1], &[1, 2]));
        assert!(!sorted_lex_le(&[1, 3], &[1, 2]));
        assert!(!sorted_lex_le(&[2], &[1, 2]));
        assert!(sorted_lex_le(&[], &[1]));
        assert!(sorted_lex_le(&[1, 2], &[1, 2]));
        assert!(!sorted_lex_lt(&[1, 2], &[1, 2]));
    }

    #[test]
    fn fails_when_fixed_left_lex_greater() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(3)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
        assert!(engine.propagate_all().unwrap().is_failure());
    }

    #[test]
    fn accepts_prefix_lex_order() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(1)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
        engine.propagate_all().unwrap();
    }

    #[test]
    fn rejects_singleton_two_against_one_two_for_le() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(2)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
        assert!(engine.propagate_all().unwrap().is_failure());
    }

    #[test]
    fn fixed_right_prunes_too_large_left_optional() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3).with_cardinality(1, 1),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(1)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
        engine.propagate_all().unwrap();
        // {2} and {3} are >lex {1}; only {1} remains.
        assert!(engine.domain(left).as_set().unwrap().lub().contains(&1));
        assert!(!engine.domain(left).as_set().unwrap().lub().contains(&2));
        assert!(!engine.domain(left).as_set().unwrap().lub().contains(&3));
    }

    #[test]
    fn reif_true_enforces_lex_le() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(2)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        let reif = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(SetLexReifPropagator::new(
            left,
            right,
            reif,
            SetLexOp::Le,
        )));
        assert!(engine.propagate_all().unwrap().is_failure());
    }

    #[test]
    fn reif_false_enforces_negated_lex_le() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        // {1} ≤ {1,2} is inevitable, so reif=false must fail.
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(1)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        let reif = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(SetLexReifPropagator::new(
            left,
            right,
            reif,
            SetLexOp::Le,
        )));
        assert!(engine.propagate_all().unwrap().is_failure());
    }

    #[test]
    fn reif_false_prunes_when_negation_requires_it() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=3).with_cardinality(1, 1),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(1)
                .unwrap(),
        ));
        let reif = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(SetLexReifPropagator::new(
            left,
            right,
            reif,
            SetLexOp::Le,
        )));
        engine.propagate_all().unwrap();
        // Need A > {1}, so A cannot be {1}; must be {2} or {3}.
        assert!(!engine.domain(left).as_set().unwrap().lub().contains(&1));
    }

    #[test]
    fn unfixed_reif_assigned_when_lex_decided() {
        use propaga_domains::IntervalDomain;

        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(2)
                .unwrap(),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(2, 2)
                .force_in(1)
                .unwrap()
                .force_in(2)
                .unwrap(),
        ));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(SetLexReifPropagator::new(
            left,
            right,
            reif,
            SetLexOp::Le,
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(reif).as_int().unwrap().fixed_value(), Some(0));
    }

    #[test]
    fn fixed_right_forces_required_left_element_for_lex_le() {
        let mut engine = Engine::new();
        let left = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2).with_cardinality(1, 1),
        ));
        let right = engine.new_variable(AnyDomain::Set(
            SetIntervalDomain::universe(1..=2)
                .with_cardinality(1, 1)
                .force_in(1)
                .unwrap(),
        ));
        engine.add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
        assert_ne!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
        let dom = engine.domain(left).as_set().unwrap();
        assert!(dom.glb().contains(&1));
        assert!(!dom.lub().contains(&2));
    }
}
