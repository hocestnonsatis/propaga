use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Rectangle specification for `diffn`.
#[derive(Clone, Copy, Debug)]
pub struct RectangleSpec {
    /// Left x coordinate.
    pub x: VariableId,
    /// Bottom y coordinate.
    pub y: VariableId,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

/// Propagates pairwise non-overlap among fixed-size rectangles.
pub struct DiffnPropagator {
    rectangles: Vec<RectangleSpec>,
    watched: Vec<VariableId>,
}

impl DiffnPropagator {
    /// Creates a diffn propagator over rectangles.
    #[must_use]
    pub fn new(rectangles: Vec<RectangleSpec>) -> Self {
        let mut watched = Vec::with_capacity(rectangles.len() * 2);
        for rect in &rectangles {
            watched.push(rect.x);
            watched.push(rect.y);
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

        if self
            .rectangles
            .iter()
            .any(|rect| ctx.domain(rect.x).is_empty() || ctx.domain(rect.y).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
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

    let left_right = lx_max + left.width <= rx_min;
    let right_left = rx_max + right.width <= lx_min;
    let left_above = ly_max + left.height <= ry_min;
    let right_above = ry_max + right.height <= ly_min;

    if left_right || right_left || left_above || right_above {
        return false;
    }

    if lx_min + left.width > rx_max && rx_min + right.width > lx_max {
        let required_y_gap = left.height.min(right.height);
        if ly_max + required_y_gap > ry_min && ctx.remove_below(left.y, ry_max - left.height + 1) {
            changed = true;
        }
        if ry_max + required_y_gap > ly_min && ctx.remove_below(right.y, ly_max - right.height + 1)
        {
            changed = true;
        }
    }

    if ly_min + left.height > ry_max && ry_min + right.height > ly_max {
        let required_x_gap = left.width.min(right.width);
        if lx_max + required_x_gap > rx_min && ctx.remove_below(left.x, rx_max - left.width + 1) {
            changed = true;
        }
        if rx_max + required_x_gap > lx_min && ctx.remove_below(right.x, lx_max - right.width + 1) {
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn separated_fixed_positions_do_not_fail() {
        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::fix(0));
        let y0 = engine.new_variable(IntervalDomain::fix(0));
        let x1 = engine.new_variable(IntervalDomain::fix(4));
        let y1 = engine.new_variable(IntervalDomain::fix(0));
        engine.add_propagator(Box::new(DiffnPropagator::new(vec![
            RectangleSpec {
                x: x0,
                y: y0,
                width: 3,
                height: 3,
            },
            RectangleSpec {
                x: x1,
                y: y1,
                width: 3,
                height: 3,
            },
        ])));
        let status = engine.propagate_all().unwrap();
        assert!(!status.is_failure());
    }
}
