use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Rectangle specification for `diffn`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectangleSpec {
    /// Left x coordinate.
    pub x: VariableId,
    /// Bottom y coordinate.
    pub y: VariableId,
    /// Fixed width (used when [`Self::width_var`] is `None`).
    pub width: i32,
    /// Fixed height (used when [`Self::height_var`] is `None`).
    pub height: i32,
    /// Optional variable width.
    pub width_var: Option<VariableId>,
    /// Optional variable height.
    pub height_var: Option<VariableId>,
}

impl RectangleSpec {
    /// Creates a fixed-size rectangle.
    #[must_use]
    pub fn new(x: VariableId, y: VariableId, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            width_var: None,
            height_var: None,
        }
    }

    /// Creates a rectangle with optional variable width/height.
    #[must_use]
    pub fn with_variable_size(
        x: VariableId,
        y: VariableId,
        width: i32,
        width_var: Option<VariableId>,
        height: i32,
        height_var: Option<VariableId>,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            width_var,
            height_var,
        }
    }
}

/// Propagates pairwise non-overlap among rectangles (fixed or variable size).
#[derive(Clone)]
pub struct DiffnPropagator {
    rectangles: Vec<RectangleSpec>,
    watched: Vec<VariableId>,
}

impl DiffnPropagator {
    /// Creates a diffn propagator over rectangles.
    #[must_use]
    pub fn new(rectangles: Vec<RectangleSpec>) -> Self {
        let mut watched = Vec::with_capacity(rectangles.len() * 4);
        for rect in &rectangles {
            watched.push(rect.x);
            watched.push(rect.y);
            if let Some(width) = rect.width_var {
                watched.push(width);
            }
            if let Some(height) = rect.height_var {
                watched.push(height);
            }
        }
        Self {
            rectangles,
            watched,
        }
    }
}

impl Propagator for DiffnPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        24
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;
        let count = self.rectangles.len();
        for left in 0..count {
            for right in left + 1..count {
                if propagate_pair(ctx, self.rectangles[left], self.rectangles[right]) {
                    changed = true;
                }
            }
        }

        if self.rectangles.iter().any(|rect| {
            ctx.domain(rect.x).is_empty()
                || ctx.domain(rect.y).is_empty()
                || rect.width_var.is_some_and(|var| ctx.domain(var).is_empty())
                || rect
                    .height_var
                    .is_some_and(|var| ctx.domain(var).is_empty())
        }) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn size_bounds(
    ctx: &dyn PropagationContext,
    fixed: i32,
    var: Option<VariableId>,
) -> Option<(i32, i32)> {
    match var {
        Some(var) => Some((ctx.domain(var).min()?, ctx.domain(var).max()?)),
        None => Some((fixed, fixed)),
    }
}

fn propagate_pair(
    ctx: &mut dyn PropagationContext,
    left: RectangleSpec,
    right: RectangleSpec,
) -> bool {
    let mut changed = false;
    let left_x = ctx.domain(left.x);
    let left_y = ctx.domain(left.y);
    let right_x = ctx.domain(right.x);
    let right_y = ctx.domain(right.y);

    let (Some(lx_min), Some(lx_max), Some(ly_min), Some(ly_max)) =
        (left_x.min(), left_x.max(), left_y.min(), left_y.max())
    else {
        return false;
    };
    let (Some(rx_min), Some(rx_max), Some(ry_min), Some(ry_max)) =
        (right_x.min(), right_x.max(), right_y.min(), right_y.max())
    else {
        return false;
    };

    let Some((lw_min, lw_max)) = size_bounds(ctx, left.width, left.width_var) else {
        return false;
    };
    let Some((lh_min, lh_max)) = size_bounds(ctx, left.height, left.height_var) else {
        return false;
    };
    let Some((rw_min, rw_max)) = size_bounds(ctx, right.width, right.width_var) else {
        return false;
    };
    let Some((rh_min, rh_max)) = size_bounds(ctx, right.height, right.height_var) else {
        return false;
    };

    // Definitely separated using maximum extents.
    let left_right = lx_max + lw_max <= rx_min;
    let right_left = rx_max + rw_max <= lx_min;
    let left_above = ly_max + lh_max <= ry_min;
    let right_above = ry_max + rh_max <= ly_min;

    if left_right || right_left || left_above || right_above {
        return false;
    }

    // Forced X overlap using minimum extents → push Y apart (same prune as fixed-size).
    if lx_min + lw_min > rx_max && rx_min + rw_min > lx_max {
        let required_y_gap = lh_min.min(rh_min);
        if ly_max + required_y_gap > ry_min
            && ctx.remove_below(left.y, ry_max.saturating_sub(lh_max).saturating_add(1))
        {
            changed = true;
        }
        if ry_max + required_y_gap > ly_min
            && ctx.remove_below(right.y, ly_max.saturating_sub(rh_max).saturating_add(1))
        {
            changed = true;
        }
    }

    // Forced Y overlap using minimum extents → push X apart.
    if ly_min + lh_min > ry_max && ry_min + rh_min > ly_max {
        let required_x_gap = lw_min.min(rw_min);
        if lx_max + required_x_gap > rx_min
            && ctx.remove_below(left.x, rx_max.saturating_sub(lw_max).saturating_add(1))
        {
            changed = true;
        }
        if rx_max + required_x_gap > lx_min
            && ctx.remove_below(right.x, lx_max.saturating_sub(rw_max).saturating_add(1))
        {
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn empty_coordinate_domain_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(1, 0));
        let y0 = engine.new_variable(IntervalDomain::new(0, 5));
        let x1 = engine.new_variable(IntervalDomain::new(0, 5));
        let y1 = engine.new_variable(IntervalDomain::new(0, 5));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::new(x0, y0, 2, 2),
            RectangleSpec::new(x1, y1, 2, 2),
        ])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn missing_y_bounds_returns_failure() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 1));
        let y0 = engine.new_variable(IntervalDomain::new(1, 0));
        let x1 = engine.new_variable(IntervalDomain::new(0, 1));
        let y1 = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::new(x0, y0, 2, 2),
            RectangleSpec::new(x1, y1, 2, 2),
        ])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }

    #[test]
    fn separated_fixed_positions_do_not_fail() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(0));
        let y0 = engine.new_variable(IntervalDomain::fix(0));
        let x1 = engine.new_variable(IntervalDomain::fix(4));
        let y1 = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::new(x0, y0, 3, 3),
            RectangleSpec::new(x1, y1, 3, 3),
        ])));
        let status = engine.propagate_all().unwrap();
        assert!(!status.is_failure());
    }

    #[test]
    fn overlapping_x_forces_y_separation() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 1));
        let y0 = engine.new_variable(IntervalDomain::new(0, 10));
        let x1 = engine.new_variable(IntervalDomain::new(0, 1));
        let y1 = engine.new_variable(IntervalDomain::new(0, 10));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::new(x0, y0, 2, 2),
            RectangleSpec::new(x1, y1, 2, 2),
        ])));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(y0).min(), Some(9));
        assert_eq!(engine.hybrid_domain(y1).min(), Some(9));
    }

    #[test]
    fn overlapping_y_forces_x_separation() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 10));
        let y0 = engine.new_variable(IntervalDomain::new(0, 1));
        let x1 = engine.new_variable(IntervalDomain::new(0, 10));
        let y1 = engine.new_variable(IntervalDomain::new(0, 1));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::new(x0, y0, 2, 2),
            RectangleSpec::new(x1, y1, 2, 2),
        ])));

        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(x0).min(), Some(9));
        assert_eq!(engine.hybrid_domain(x1).min(), Some(9));
    }

    #[test]
    fn variable_width_forces_y_when_min_widths_overlap() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(0));
        let y0 = engine.new_variable(IntervalDomain::new(0, 10));
        let x1 = engine.new_variable(IntervalDomain::fix(0));
        let y1 = engine.new_variable(IntervalDomain::new(0, 10));
        let w0 = engine.new_variable(IntervalDomain::new(2, 4));
        let w1 = engine.new_variable(IntervalDomain::new(2, 4));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::with_variable_size(x0, y0, 2, Some(w0), 2, None),
            RectangleSpec::with_variable_size(x1, y1, 2, Some(w1), 2, None),
        ])));

        assert!(!engine.commit_initial_propagation().unwrap().is_failure());
        assert_eq!(engine.hybrid_domain(y0).min(), Some(9));
        assert_eq!(engine.hybrid_domain(y1).min(), Some(9));
    }

    #[test]
    fn variable_width_allows_side_by_side_when_min_fits() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(0));
        let y0 = engine.new_variable(IntervalDomain::fix(0));
        let x1 = engine.new_variable(IntervalDomain::new(1, 5));
        let y1 = engine.new_variable(IntervalDomain::fix(0));
        let w0 = engine.new_variable(IntervalDomain::new(1, 3));
        let w1 = engine.new_variable(IntervalDomain::fix(1));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::with_variable_size(x0, y0, 1, Some(w0), 1, None),
            RectangleSpec::with_variable_size(x1, y1, 1, Some(w1), 1, None),
        ])));

        assert!(!engine.commit_initial_propagation().unwrap().is_failure());
        // Min width 1 allows x1=1; do not force y separation.
        assert_eq!(engine.hybrid_domain(y0).min(), Some(0));
        assert_eq!(engine.hybrid_domain(y1).min(), Some(0));
    }

    #[test]
    fn empty_size_domain_fails() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 5));
        let y0 = engine.new_variable(IntervalDomain::new(0, 5));
        let x1 = engine.new_variable(IntervalDomain::new(0, 5));
        let y1 = engine.new_variable(IntervalDomain::new(0, 5));
        let w0 = engine.new_variable(IntervalDomain::new(2, 1));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec::with_variable_size(x0, y0, 1, Some(w0), 1, None),
            RectangleSpec::new(x1, y1, 1, 1),
        ])));
        assert_eq!(engine.propagate_all().unwrap(), PropagationStatus::Failure);
    }
}
