use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `reif == 1 <=> left == right`.
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

        match ctx.fixed_value(reif) {
            Some(1) => changed |= propagate_equal(ctx, left, right),
            Some(0) => changed |= propagate_not_equal(ctx, left, right),
            _ => {}
        }

        if ctx.domain(reif).size() == 1 {
            match ctx.domain(reif).min() {
                Some(1) => changed |= propagate_equal(ctx, left, right),
                Some(0) => changed |= propagate_not_equal(ctx, left, right),
                _ => {}
            }
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

        match ctx.fixed_value(reif) {
            Some(1) => changed |= propagate_not_equal(ctx, left, right),
            Some(0) => changed |= propagate_equal(ctx, left, right),
            _ => {}
        }

        if ctx.domain(reif).size() == 1 {
            match ctx.domain(reif).min() {
                Some(1) => changed |= propagate_not_equal(ctx, left, right),
                Some(0) => changed |= propagate_equal(ctx, left, right),
                _ => {}
            }
        }

        if let (Some(left_value), Some(right_value)) =
            (ctx.fixed_value(left), ctx.fixed_value(right))
        {
            let value = i32::from(left_value != right_value);
            changed |= tighten_reif(ctx, reif, value);
        } else if domains_equal(ctx, left, right) {
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

/// Propagates `reif == 1 <=> left <= right`.
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

        match ctx.fixed_value(reif) {
            Some(1) => changed |= propagate_less_equal(ctx, left, right),
            Some(0) => changed |= propagate_greater_than(ctx, left, right),
            _ => {}
        }

        if ctx.domain(reif).size() == 1 {
            match ctx.domain(reif).min() {
                Some(1) => changed |= propagate_less_equal(ctx, left, right),
                Some(0) => changed |= propagate_greater_than(ctx, left, right),
                _ => {}
            }
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

        match ctx.fixed_value(reif) {
            Some(1) => changed |= propagate_less_than(ctx, left, right),
            Some(0) => changed |= propagate_greater_equal(ctx, left, right),
            _ => {}
        }

        if ctx.domain(reif).size() == 1 {
            match ctx.domain(reif).min() {
                Some(1) => changed |= propagate_less_than(ctx, left, right),
                Some(0) => changed |= propagate_greater_equal(ctx, left, right),
                _ => {}
            }
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

fn propagate_equal(ctx: &mut dyn PropagationContext, left: VariableId, right: VariableId) -> bool {
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

    if let Some(value) = ctx.fixed_value(right) {
        if ctx.remove_value(left, value) {
            changed = true;
        }
    }

    if let Some(value) = ctx.fixed_value(left) {
        if ctx.remove_value(right, value) {
            changed = true;
        }
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
        if ctx.remove_below(right, max) {
            changed = true;
        }
    }

    if let Some(min) = ctx.domain(right).min() {
        if ctx.remove_above(left, min) {
            changed = true;
        }
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
        if ctx.remove_above(right, min - 1) {
            changed = true;
        }
    }

    if let Some(max) = ctx.domain(right).max() {
        if ctx.remove_below(left, max + 1) {
            changed = true;
        }
    }

    changed
}

fn propagate_less_than(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(max) = ctx.domain(right).max() {
        if ctx.remove_above(left, max - 1) {
            changed = true;
        }
    }

    if let Some(min) = ctx.domain(left).min() {
        if ctx.remove_below(right, min + 1) {
            changed = true;
        }
    }

    if let Some(value) = ctx.fixed_value(left) {
        if ctx.remove_below(right, value + 1) {
            changed = true;
        }
    }

    if let Some(value) = ctx.fixed_value(right) {
        if ctx.remove_above(left, value - 1) {
            changed = true;
        }
    }

    changed
}

fn propagate_greater_equal(
    ctx: &mut dyn PropagationContext,
    left: VariableId,
    right: VariableId,
) -> bool {
    let mut changed = false;

    if let Some(min) = ctx.domain(right).min() {
        if ctx.remove_below(left, min) {
            changed = true;
        }
    }

    if let Some(max) = ctx.domain(left).max() {
        if ctx.remove_above(right, max) {
            changed = true;
        }
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
    fn reified_eq_true_fixes_equal_values() {
        let mut engine = Engine::new();
        let left = engine.new_variable(IntervalDomain::new(1, 5));
        let right = engine.new_variable(IntervalDomain::new(1, 5));
        let reif = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
        engine.fix_variable(reif, 1).unwrap();
        engine.fix_variable(left, 3).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(right).fixed_value(), Some(3));
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
        assert!(!engine.domain(left).contains(2));
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
        assert!(!engine.domain(left).contains(2));
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
        assert!(engine.domain(right).max().unwrap() <= 2);
        assert!(engine.domain(left).min().unwrap() > engine.domain(right).max().unwrap());
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
        assert_eq!(engine.domain(left).max(), Some(3));
    }
}
