use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `reif == 1 <=> left == right`.
#[derive(Clone)]
pub struct ReifiedEqualityPropagator {
    watched: [VariableId; 3],
}

impl ReifiedEqualityPropagator {
    /// Creates a reified equality propagator.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for ReifiedEqualityPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right, reif] = self.watched;
        let mut changed = false;

        match reif_literal(ctx, reif) {
            Some(1) => changed |= propagate_equal(ctx, left, right),
            Some(0) => changed |= propagate_not_equal(ctx, left, right),
            _ => {}
        }

        if let (Some(left_value), Some(right_value)) =
            (ctx.fixed_value(left), ctx.fixed_value(right))
        {
            let value = i32::from(left_value == right_value);
            changed |= tighten_reif(ctx, reif, value);
        } else if domains_disjoint(ctx, left, right) {
            changed |= tighten_reif(ctx, reif, 0);
        }

        if self.watched.iter().any(|&var| ctx.domain(var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif == 1 <=> left != right`.
#[derive(Clone)]
pub struct ReifiedNotEqualPropagator {
    watched: [VariableId; 3],
}

impl ReifiedNotEqualPropagator {
    /// Creates a reified disequality propagator.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for ReifiedNotEqualPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right, reif] = self.watched;
        let mut changed = false;

        match reif_literal(ctx, reif) {
            Some(1) => changed |= propagate_not_equal(ctx, left, right),
            Some(0) => changed |= propagate_equal(ctx, left, right),
            _ => {}
        }

        if let (Some(left_value), Some(right_value)) =
            (ctx.fixed_value(left), ctx.fixed_value(right))
        {
            let value = i32::from(left_value != right_value);
            changed |= tighten_reif(ctx, reif, value);
        }

        if self.watched.iter().any(|&var| ctx.domain(var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif == 1 <=> left <= right`.
#[derive(Clone)]
pub struct ReifiedLessEqualPropagator {
    watched: [VariableId; 3],
}

impl ReifiedLessEqualPropagator {
    /// Creates a reified `<=` propagator.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for ReifiedLessEqualPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right, reif] = self.watched;
        let mut changed = false;

        match reif_literal(ctx, reif) {
            Some(1) => changed |= propagate_less_equal(ctx, left, right),
            Some(0) => changed |= propagate_greater_than(ctx, left, right),
            _ => {}
        }

        if let (Some(left_value), Some(right_value)) =
            (ctx.fixed_value(left), ctx.fixed_value(right))
        {
            let value = i32::from(left_value <= right_value);
            changed |= tighten_reif(ctx, reif, value);
        } else if always_less_equal(ctx, left, right) {
            changed |= tighten_reif(ctx, reif, 1);
        } else if never_less_equal(ctx, left, right) {
            changed |= tighten_reif(ctx, reif, 0);
        }

        if self.watched.iter().any(|&var| ctx.domain(var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

/// Propagates `reif == 1 <=> left < right`.
#[derive(Clone)]
pub struct ReifiedLessThanPropagator {
    watched: [VariableId; 3],
}

impl ReifiedLessThanPropagator {
    /// Creates a reified `<` propagator.
    #[must_use]
    pub const fn new(left: VariableId, right: VariableId, reif: VariableId) -> Self {
        Self {
            watched: [left, right, reif],
        }
    }
}

impl Propagator for ReifiedLessThanPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        12
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let [left, right, reif] = self.watched;
        let mut changed = false;

        match reif_literal(ctx, reif) {
            Some(1) => changed |= propagate_less_than(ctx, left, right),
            Some(0) => changed |= propagate_greater_equal(ctx, left, right),
            _ => {}
        }

        if let (Some(left_value), Some(right_value)) =
            (ctx.fixed_value(left), ctx.fixed_value(right))
        {
            let value = i32::from(left_value < right_value);
            changed |= tighten_reif(ctx, reif, value);
        } else if always_less_than(ctx, left, right) {
            changed |= tighten_reif(ctx, reif, 1);
        } else if never_less_than(ctx, left, right) {
            changed |= tighten_reif(ctx, reif, 0);
        }

        if self.watched.iter().any(|&var| ctx.domain(var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn tighten_reif(ctx: &mut dyn PropagationContext, reif: VariableId, value: i32) -> bool {
    let mut changed = false;
    if ctx.remove_below(reif, value) {
        changed = true;
    }
    if ctx.remove_above(reif, value) {
        changed = true;
    }
    changed
}

pub(crate) fn reif_literal(ctx: &dyn PropagationContext, reif: VariableId) -> Option<i32> {
    if let Some(value) = ctx.fixed_value(reif) {
        return Some(value);
    }
    if ctx.domain(reif).size() == 1 {
        return ctx.domain(reif).min();
    }
    None
}

pub(crate) fn propagate_equal(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
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

    if let (Some(min), Some(max)) = (ctx.domain(left).min(), ctx.domain(left).max()) {
        if ctx.remove_below(right, min) {
            changed = true;
        }
        if ctx.remove_above(right, max) {
            changed = true;
        }
    }

    if let (Some(min), Some(max)) = (ctx.domain(right).min(), ctx.domain(right).max()) {
        if ctx.remove_below(left, min) {
            changed = true;
        }
        if ctx.remove_above(left, max) {
            changed = true;
        }
    }

    changed
}

fn propagate_not_equal(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(value) = ctx.fixed_value(right)
        && ctx.remove_value(left, value)
    {
        changed = true;
    }

    if let Some(value) = ctx.fixed_value(left)
        && ctx.remove_value(right, value)
    {
        changed = true;
    }

    changed
}

fn propagate_less_equal(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(max) = ctx.domain(left).max() {
        changed |= ctx.remove_below(right, max);
    }

    if let Some(min) = ctx.domain(right).min() {
        changed |= ctx.remove_above(left, min);
    }

    changed
}

fn propagate_greater_than(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(min) = ctx.domain(left).min() {
        changed |= ctx.remove_above(right, min - 1);
    }

    if let Some(max) = ctx.domain(right).max() {
        changed |= ctx.remove_below(left, max + 1);
    }

    changed
}

fn propagate_less_than(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(max) = ctx.domain(right).max()
        && ctx.remove_above(left, max - 1)
    {
        changed = true;
    }

    if let Some(min) = ctx.domain(left).min()
        && ctx.remove_below(right, min + 1)
    {
        changed = true;
    }

    changed
}

fn propagate_greater_equal(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(min) = ctx.domain(right).min()
        && ctx.remove_below(left, min)
    {
        changed = true;
    }

    if let Some(max) = ctx.domain(left).max()
        && ctx.remove_above(right, max)
    {
        changed = true;
    }

    changed
}

fn domains_disjoint(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    match (
        ctx.domain(left).max(),
        ctx.domain(left).min(),
        ctx.domain(right).max(),
        ctx.domain(right).min(),
    ) {
        (Some(left_max), Some(left_min), Some(right_max), Some(right_min)) => {
            left_max < right_min || right_max < left_min
        }
        _ => false,
    }
}

#[allow(dead_code)]
fn domains_equal(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    ctx.fixed_value(left).is_some()
        && ctx.fixed_value(left) == ctx.fixed_value(right)
        && ctx.domain(left).size() == 1
        && ctx.domain(right).size() == 1
}

fn always_less_equal(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    match (ctx.domain(left).max(), ctx.domain(right).min()) {
        (Some(left_max), Some(right_min)) => left_max <= right_min,
        _ => false,
    }
}

fn never_less_equal(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    match (ctx.domain(left).min(), ctx.domain(right).max()) {
        (Some(left_min), Some(right_max)) => left_min > right_max,
        _ => false,
    }
}

fn always_less_than(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    match (ctx.domain(left).max(), ctx.domain(right).min()) {
        (Some(left_max), Some(right_min)) => left_max < right_min,
        _ => false,
    }
}

fn never_less_than(ctx: &dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
    match (ctx.domain(left).min(), ctx.domain(right).max()) {
        (Some(left_min), Some(right_max)) => left_min >= right_max,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn reified_eq_singleton_false_propagates_not_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(left).contains(2));
    }

    #[test]
    fn reified_ne_singleton_true_propagates_not_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(left).contains(2));
    }

    #[test]
    fn reified_ne_singleton_false_propagates_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.fix_variable(left, 3).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(3));
    }

    #[test]
    fn reified_le_singleton_false_propagates_greater_than() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(
            engine.hybrid_domain(left).min().unwrap() > engine.hybrid_domain(right).max().unwrap()
        );
    }

    #[test]
    fn reified_lt_singleton_false_propagates_greater_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(
            engine.hybrid_domain(left).min().unwrap() >= engine.hybrid_domain(right).min().unwrap()
        );
    }

    #[test]
    fn reified_eq_fixed_right_equalizes_left() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::fix(3));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).fixed_value(), Some(3));
    }

    #[test]
    fn reified_eq_syncs_bounds_between_operands() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(2, 4));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).min(), Some(2));
        assert_eq!(engine.hybrid_domain(right).max(), Some(4));
    }

    #[test]
    fn reified_ne_fixed_left_prunes_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(2));
        let right = engine.new_variable(IntervalDomain::new(1, 3));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(right).contains(2));
    }

    #[test]
    fn reified_lt_fixed_right_tightens_left() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::fix(4));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).max(), Some(3));
    }

    #[test]
    fn reified_le_fixed_left_tightens_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(6));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).min(), Some(6));
    }

    #[test]
    fn reified_ge_via_lt_false_tightens_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.fix_variable(left, 5).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(right).min().unwrap() <= 5);
    }

    #[test]
    fn reified_eq_empty_domain_fails() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 0));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_eq_failure_on_empty_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 0));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_le_failure_on_empty_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 0));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_lt_failure_on_empty_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 0));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn reified_le_singleton_true_propagates() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.fix_variable(left, 4).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(right).min().unwrap() >= 4);
    }

    #[test]
    fn reified_lt_singleton_true_propagates() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.fix_variable(left, 4).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(right).min().unwrap() >= 5);
    }

    #[test]
    fn reified_ne_singleton_true_propagates() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(left).contains(2));
    }

    #[test]
    fn reified_eq_true_fixes_equal_values() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.fix_variable(left, 3).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(3));
    }

    #[test]
    fn reified_eq_false_removes_matching_value() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 0).unwrap();
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(left).contains(2));
    }

    #[test]
    fn reified_ne_true_prunes_equal_value() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.propagate_all().unwrap();
        assert!(!engine.hybrid_domain(left).contains(2));
    }

    #[test]
    fn reified_le_false_tightens_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 0).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(right).max().unwrap() <= 2);
        assert!(
            engine.hybrid_domain(left).min().unwrap() > engine.hybrid_domain(right).max().unwrap()
        );
    }

    #[test]
    fn reified_lt_true_tightens_upper_bound() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::fix(4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(left).max(), Some(3));
    }

    #[test]
    fn reified_eq_singleton_reif_propagates() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.fix_variable(left, 3).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(3));
    }

    #[test]
    fn reified_eq_infers_reif_true_when_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(4));
        let right = engine.new_variable(IntervalDomain::fix(4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_eq_infers_reif_false_when_disjoint() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(5, 7));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_eq_already_satisfied_no_change() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(2));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        assert_eq!(
            engine.propagate_all().unwrap(),
            PropagationStatus::OkNoChange
        );
    }

    #[test]
    fn reified_ne_reif_zero_syncs_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.fix_variable(left, 3).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(3));
    }

    #[test]
    fn reified_ne_infers_reif_false_when_equal() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(5));
        let right = engine.new_variable(IntervalDomain::fix(5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_le_true_propagates_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(6));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).min(), Some(6));
    }

    #[test]
    fn reified_le_infers_true_from_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(5, 7));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_le_infers_false_from_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(6, 8));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_lt_false_forces_ge() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(3, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 0).unwrap();
        engine.propagate_all().unwrap();
        assert!(
            engine.hybrid_domain(left).min().unwrap() >= engine.hybrid_domain(right).min().unwrap()
        );
    }

    #[test]
    fn reified_lt_infers_true_from_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(5, 7));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_lt_infers_false_from_bounds() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(5, 7));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_lt_fixed_left_tightens_right() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.propagate_all().unwrap();
        assert!(engine.hybrid_domain(right).min().unwrap() >= 4);
    }

    #[test]
    fn reified_eq_singleton_true_via_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.fix_variable(left, 4).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(right).fixed_value(), Some(4));
    }

    #[test]
    fn reified_ne_domains_equal_infers_false() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::fix(3));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn reified_le_singleton_false_via_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(6, 8));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        engine.add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert!(
            engine.hybrid_domain(left).min().unwrap() > engine.hybrid_domain(right).max().unwrap()
        );
    }

    #[test]
    fn reified_lt_singleton_true_via_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::new(5, 7));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_lt_fixed_operands_infer_reif() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(2));
        let right = engine.new_variable(IntervalDomain::fix(5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(1));
    }

    #[test]
    fn reified_lt_never_less_than_infers_false() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(5, 7));
        let right = engine.new_variable(IntervalDomain::new(1, 4));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(reif).fixed_value(), Some(0));
    }

    #[test]
    fn propagate_greater_than_tightens_bounds() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(5, 8));
        let right = engine.new_variable(IntervalDomain::new(1, 6));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_greater_than(&mut ctx, left, right));
        assert!(engine.hybrid_domain(right).max().unwrap() <= 4);
    }

    #[test]
    fn propagate_less_than_with_fixed_operands() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::fix(3));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_less_than(&mut ctx, left, right));
        assert!(engine.hybrid_domain(right).min().unwrap() >= 4);
    }

    #[test]
    fn propagate_greater_equal_tightens_bounds() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 10));
        let right = engine.new_variable(IntervalDomain::fix(6));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_greater_equal(&mut ctx, left, right));
        assert!(engine.hybrid_domain(left).min().unwrap() >= 6);
    }

    #[test]
    fn propagate_equal_syncs_bounds() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(2, 4));
        let right = engine.new_variable(IntervalDomain::new(1, 10));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(engine.hybrid_domain(right).min(), Some(2));
    }

    #[test]
    fn reified_eq_singleton_true_via_unfixed_reif_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        use crate::test_support::MutEngine;
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn reified_failure_on_empty_reif_domain() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(1, 0));
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        use crate::test_support::MutEngine;
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn reified_invalid_singleton_reif_is_ignored() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(2, 2));
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        use crate::test_support::MutEngine;
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn reified_not_equal_singleton_paths() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 3));
        let right = engine.new_variable(IntervalDomain::fix(2));
        let reif = engine.new_variable(IntervalDomain::new(1, 1));
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        use crate::test_support::MutEngine;
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
    }

    #[test]
    fn mock_singleton_reif_eq_propagates_equal() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
    }

    #[test]
    fn mock_singleton_reif_ne_propagates_not_equal() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![2])
            .with_domain(reif, vec![1])
            .with_fixed(right, 2);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert!(!ctx.domains[&left].values.borrow().contains(&2));
    }

    #[test]
    fn mock_singleton_reif_le_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
    }

    #[test]
    fn mock_singleton_reif_le_false_propagates_gt() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![1, 2, 3])
            .with_domain(reif, vec![0]);
        let mut prop = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
        assert!(ctx.domains[&left].min().unwrap() > ctx.domains[&right].max().unwrap());
    }

    #[test]
    fn mock_singleton_reif_lt_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedLessThanPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
    }

    #[test]
    fn mock_singleton_reif_lt_false_propagates_ge() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![1, 2, 3])
            .with_domain(reif, vec![0]);
        let mut prop = ReifiedLessThanPropagator::new(left, right, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
        assert!(ctx.domains[&left].min().unwrap() >= ctx.domains[&right].min().unwrap());
    }

    #[test]
    fn mock_reified_eq_domains_equal_infers_reif() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4])
            .with_domain(right, vec![4])
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 4)
            .with_fixed(right, 4);
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domains[&reif].values.borrow().as_slice(), &[1]);
    }

    #[test]
    fn mock_reified_le_infers_from_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![5, 6, 7])
            .with_domain(reif, vec![0, 1]);
        let mut prop = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domains[&reif].values.borrow().as_slice(), &[1]);
    }

    #[test]
    fn mock_reified_lt_fixed_operands_infer_reif() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![2])
            .with_domain(right, vec![5])
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 2)
            .with_fixed(right, 5);
        let mut prop = ReifiedLessThanPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domains[&reif].values.borrow().as_slice(), &[1]);
    }

    #[test]
    fn mock_propagate_helper_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![2, 3, 4, 10])
            .with_domain(right, vec![5, 6, 7]);
        assert!(propagate_less_equal(&mut ctx, left, right));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![1, 2, 3, 6]);
        assert!(propagate_greater_than(&mut ctx, left, right));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![2, 3, 4, 10])
            .with_domain(right, vec![5, 6, 7]);
        assert!(propagate_less_than(&mut ctx, left, right));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![2, 3, 4, 10]);
        assert!(propagate_greater_equal(&mut ctx, left, right));
    }

    #[test]
    fn mock_singleton_invalid_reif_value_ignored() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_domain(reif, vec![2]);
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn mock_domains_disjoint_infers_false() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6])
            .with_domain(reif, vec![0, 1]);
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domains[&reif].values.borrow().as_slice(), &[0]);
    }

    #[test]
    fn mock_singleton_reif_ne_propagates() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4])
            .with_domain(reif, vec![1]);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_ne!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_singleton_reif_le_true_and_false() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_true = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![4, 5, 6])
            .with_domain(reif, vec![1]);
        let mut le = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_ne!(le.propagate(&mut ctx_true), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_false = MockIntCtx::new()
            .with_domain(left, vec![8, 9, 10])
            .with_domain(right, vec![1, 2, 3])
            .with_domain(reif2, vec![0]);
        let mut le2 = ReifiedLessEqualPropagator::new(left, right, reif2);
        assert_ne!(le2.propagate(&mut ctx_false), PropagationStatus::Failure);
    }

    #[test]
    fn mock_singleton_reif_lt_true_and_false() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_true = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6])
            .with_domain(reif, vec![1]);
        let mut lt = ReifiedLessThanPropagator::new(left, right, reif);
        assert_ne!(lt.propagate(&mut ctx_true), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_false = MockIntCtx::new()
            .with_domain(left, vec![5, 6])
            .with_domain(right, vec![1, 2])
            .with_domain(reif2, vec![0]);
        let mut lt2 = ReifiedLessThanPropagator::new(left, right, reif2);
        assert_ne!(lt2.propagate(&mut ctx_false), PropagationStatus::Failure);
    }

    #[test]
    fn mock_reified_le_infers_true_and_false_from_bounds() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_true = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6])
            .with_domain(reif, vec![0, 1]);
        let mut prop = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx_true), PropagationStatus::OkChanged);
        assert_eq!(ctx_true.domains[&reif].values.borrow().as_slice(), &[1]);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx_false = MockIntCtx::new()
            .with_domain(left, vec![8, 9])
            .with_domain(right, vec![1, 2])
            .with_domain(reif2, vec![0, 1]);
        let mut prop2 = ReifiedLessEqualPropagator::new(left, right, reif2);
        assert_eq!(
            prop2.propagate(&mut ctx_false),
            PropagationStatus::OkChanged
        );
        assert_eq!(ctx_false.domains[&reif2].values.borrow().as_slice(), &[0]);
    }

    #[test]
    fn mock_propagate_less_than_fixed_operands() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(left, 3)
            .with_fixed(right, 5);
        assert!(propagate_less_than(&mut ctx, left, right));
        assert!(ctx.domains[&right].values.borrow().iter().all(|&v| v >= 4));
        assert!(ctx.domains[&left].values.borrow().iter().all(|&v| v <= 4));
    }

    #[test]
    fn mock_open_singleton_reif_invalid_value_branch() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_open_singleton(reif, 2);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkNoChange);
    }

    #[test]
    fn mock_reified_eq_domains_equal_tightens_reif() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_open_singleton(left, 4)
            .with_open_singleton(right, 4)
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 4)
            .with_fixed(right, 4);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domain_values(reif), vec![0]);
    }

    #[test]
    fn mock_reified_propagators_empty_domain_fail() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif, 1);
        let mut prop = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_reif_propagators() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![4, 5, 6])
            .with_open_singleton(reif, 1);
        let mut le = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_ne!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut le2 = ReifiedLessEqualPropagator::new(left, right, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(left, vec![8, 9])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif2, 0);
        assert_ne!(le2.propagate(&mut ctx2), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx3 = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6])
            .with_open_singleton(reif3, 1);
        let mut lt = ReifiedLessThanPropagator::new(left, right, reif3);
        assert_ne!(lt.propagate(&mut ctx3), PropagationStatus::Failure);

        let reif4 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx4 = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3, 4])
            .with_open_singleton(reif4, 1);
        let mut ne = ReifiedNotEqualPropagator::new(left, right, reif4);
        assert_ne!(ne.propagate(&mut ctx4), PropagationStatus::Failure);

        let reif5 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx5 = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_open_singleton(reif5, 1);
        let mut eq = ReifiedEqualityPropagator::new(left, right, reif5);
        assert_ne!(eq.propagate(&mut ctx5), PropagationStatus::Failure);
    }

    #[test]
    fn mock_domains_equal_and_disjoint_helpers() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let ctx_disjoint = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6]);
        assert!(domains_disjoint(&ctx_disjoint, left, right));
        assert!(always_less_than(&ctx_disjoint, left, right));
        assert!(!never_less_than(&ctx_disjoint, left, right));
        let ctx_equal = MockIntCtx::new()
            .with_domain(left, vec![4])
            .with_domain(right, vec![4])
            .with_fixed(left, 4)
            .with_fixed(right, 4);
        assert!(domains_equal(&ctx_equal, left, right));
        assert!(always_less_equal(
            &MockIntCtx::new()
                .with_domain(left, vec![1, 2])
                .with_domain(right, vec![3, 4]),
            left,
            right
        ));
        assert!(never_less_equal(
            &MockIntCtx::new()
                .with_domain(left, vec![5, 6])
                .with_domain(right, vec![1, 2]),
            left,
            right
        ));
    }

    #[test]
    fn mock_propagate_less_equal_remove_above_left_branch() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 17));
        let right = engine.new_variable(IntervalDomain::new(5, 18));
        let mut ctx = MutEngine(&mut engine);
        assert!(propagate_less_equal(&mut ctx, left, right));
    }

    #[test]
    fn mock_reified_eq_domains_equal_infers_false_via_open_singleton() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_open_singleton(left, 4)
            .with_open_singleton(right, 4)
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 4)
            .with_fixed(right, 4);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domain_values(reif), vec![0]);
    }

    #[test]
    fn mock_reified_eq_empty_operand_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif, 1);
        let mut prop = ReifiedEqualityPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_propagate_equal_and_ordering_helpers() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5, 6])
            .with_domain(right, vec![4, 5, 6, 10]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![4, 5, 6]);
        assert_eq!(ctx.domain_values(right), vec![4, 5, 6]);

        let mut ctx3 = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![1, 2, 3, 6]);
        assert!(propagate_greater_than(&mut ctx3, left, right));

        let mut ctx4 = MockIntCtx::new()
            .with_domain(left, vec![2, 3, 4, 10])
            .with_domain(right, vec![5, 6, 7]);
        assert!(propagate_less_than(&mut ctx4, left, right));

        let mut ctx5 = MockIntCtx::new()
            .with_domain(left, vec![5])
            .with_domain(right, vec![1, 2, 3, 4, 5, 6])
            .with_fixed(left, 5);
        assert!(propagate_less_than(&mut ctx5, left, right));
        assert_eq!(ctx5.domain_values(right), vec![6]);

        let mut ctx6 = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5, 6])
            .with_domain(right, vec![3])
            .with_fixed(right, 3);
        assert!(propagate_less_than(&mut ctx6, left, right));
        assert_eq!(ctx6.domain_values(left), vec![1, 2]);

        assert!(!domains_disjoint(
            &MockIntCtx::new()
                .with_domain(left, vec![])
                .with_domain(right, vec![1, 2]),
            left,
            right
        ));
        assert!(!always_less_than(
            &MockIntCtx::new()
                .with_domain(left, vec![])
                .with_domain(right, vec![1, 2]),
            left,
            right
        ));
        assert!(!never_less_than(
            &MockIntCtx::new()
                .with_domain(left, vec![])
                .with_domain(right, vec![1, 2]),
            left,
            right
        ));
    }

    #[test]
    fn mock_open_singleton_reif_le_ge_lt_ne_false_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));

        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![8, 9])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif, 0);
        let mut le = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_ne!(le.propagate(&mut ctx), PropagationStatus::Failure);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut lt = ReifiedLessThanPropagator::new(left, right, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(left, vec![8, 9])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif2, 0);
        assert_ne!(lt.propagate(&mut ctx2), PropagationStatus::Failure);

        let reif3 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ne = ReifiedNotEqualPropagator::new(left, right, reif3);
        let mut ctx3 = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_open_singleton(reif3, 0);
        assert_ne!(ne.propagate(&mut ctx3), PropagationStatus::Failure);

        let reif4 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut eq = ReifiedEqualityPropagator::new(left, right, reif4);
        let mut ctx4 = MockIntCtx::new()
            .with_domain(left, vec![1, 2])
            .with_domain(right, vec![5, 6])
            .with_open_singleton(reif4, 0);
        assert_ne!(eq.propagate(&mut ctx4), PropagationStatus::Failure);
    }

    #[test]
    fn mock_open_singleton_reif_inference_with_fixed_operands() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![5])
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 3)
            .with_fixed(right, 5);
        let mut le = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domain_values(reif), vec![1]);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx2 = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![5])
            .with_domain(reif2, vec![0, 1])
            .with_fixed(left, 3)
            .with_fixed(right, 5);
        let mut lt = ReifiedLessThanPropagator::new(left, right, reif2);
        assert_eq!(lt.propagate(&mut ctx2), PropagationStatus::OkChanged);
        assert_eq!(ctx2.domain_values(reif2), vec![1]);
    }

    #[test]
    fn mock_reified_not_equal_domains_equal_infers_false() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4])
            .with_domain(right, vec![4])
            .with_domain(reif, vec![0, 1])
            .with_fixed(left, 4)
            .with_fixed(right, 4);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::OkChanged);
        assert_eq!(ctx.domain_values(reif), vec![0]);
    }

    #[test]
    fn mock_reified_not_equal_empty_operand_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif, 1);
        let mut prop = ReifiedNotEqualPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }

    #[test]
    fn mock_propagate_equal_syncs_from_right_domain() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5, 6])
            .with_domain(right, vec![4, 5]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(right), vec![4, 5]);
    }

    #[test]
    fn mock_propagate_equal_tightens_right_above_from_left_max() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![4, 5])
            .with_domain(right, vec![4, 5, 6]);
        assert!(propagate_equal(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(right), vec![4, 5]);
    }

    #[test]
    fn mock_propagate_less_equal_remove_above_left_from_right_min() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![2, 3, 4, 10])
            .with_domain(right, vec![5, 6, 7]);
        assert!(propagate_less_equal(&mut ctx, left, right));
    }

    #[test]
    fn mock_propagate_greater_than_remove_below_left_from_right_max() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![2, 3, 4]);
        assert!(propagate_greater_than(&mut ctx, left, right));
    }

    #[test]
    fn mock_propagate_less_than_fixed_right_remove_above_left() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![3])
            .with_fixed(right, 3);
        assert!(propagate_less_than(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![1, 2]);
    }

    #[test]
    fn mock_propagate_less_equal_and_greater_than_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![5, 6, 7])
            .with_domain(right, vec![1, 2, 3, 6]);
        assert!(propagate_greater_than(&mut ctx, left, right));
        assert!(ctx.domain_values(left).iter().all(|&v| v >= 5));
    }

    #[test]
    fn mock_propagate_less_than_fixed_value_branches() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![3])
            .with_domain(right, vec![1, 2, 3, 4, 5])
            .with_fixed(left, 3);
        assert!(propagate_less_than(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(right), vec![4, 5]);

        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3, 4, 5])
            .with_domain(right, vec![3])
            .with_fixed(right, 3);
        assert!(propagate_less_than(&mut ctx, left, right));
        assert_eq!(ctx.domain_values(left), vec![1, 2]);
    }

    #[test]
    fn mock_invalid_open_singleton_reif_value_ignored() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_open_singleton(reif, 2);
        let mut le = ReifiedLessEqualPropagator::new(left, right, reif);
        assert_eq!(le.propagate(&mut ctx), PropagationStatus::OkNoChange);

        let reif2 = engine.new_variable(IntervalDomain::new(0, 0));
        let mut lt = ReifiedLessThanPropagator::new(left, right, reif2);
        let mut ctx2 = MockIntCtx::new()
            .with_domain(left, vec![1, 2, 3])
            .with_domain(right, vec![1, 2, 3])
            .with_open_singleton(reif2, 2);
        assert_eq!(lt.propagate(&mut ctx2), PropagationStatus::OkNoChange);
    }

    #[test]
    fn mock_reified_lt_empty_operand_fails() {
        use crate::test_support::MockIntCtx;

        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(0, 0));
        let right = engine.new_variable(IntervalDomain::new(0, 0));
        let reif = engine.new_variable(IntervalDomain::new(0, 0));
        let mut ctx = MockIntCtx::new()
            .with_domain(left, vec![])
            .with_domain(right, vec![1, 2])
            .with_open_singleton(reif, 1);
        let mut prop = ReifiedLessThanPropagator::new(left, right, reif);
        assert_eq!(prop.propagate(&mut ctx), PropagationStatus::Failure);
    }
}
