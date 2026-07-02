use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates `inverse(forward, backward)`: forward[i] = j <=> backward[j] = i.
pub struct InversePropagator {
    forward: Vec<VariableId>,
    backward: Vec<VariableId>,
    watched: Vec<VariableId>,
}

impl InversePropagator {
    /// Creates an inverse constraint over equally sized arrays.
    #[must_use]
    pub fn new(forward: Vec<VariableId>, backward: Vec<VariableId>) -> Self {
        let mut watched = forward.clone();
        watched.extend(&backward);
        Self {
            forward,
            backward,
            watched,
        }
    }
}

impl Propagator for InversePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        20
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        let mut changed = false;
        loop {
            let mut round_changed = false;
            round_changed |= propagate_forward_to_backward(ctx, &self.forward, &self.backward);
            round_changed |= propagate_forward_to_backward(ctx, &self.backward, &self.forward);
            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if self
            .forward
            .iter()
            .chain(self.backward.iter())
            .any(|&var| ctx.domain(var).is_empty())
        {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

fn propagate_forward_to_backward(
    ctx: &mut dyn PropagationContext,
    forward: &[VariableId],
    backward: &[VariableId],
) -> bool {
    let mut changed = false;
    for (index, &forward_var) in forward.iter().enumerate() {
        let index_value = i32::try_from(index).expect("inverse index fits in i32");
        if let Some(target) = ctx.fixed_value(forward_var) {
            let target_index = usize::try_from(target).ok();
            if let Some(&backward_var) = target_index.and_then(|idx| backward.get(idx))
                && tighten_to_point(ctx, backward_var, index_value)
            {
                changed = true;
            }
            continue;
        }

        let mut unsupported = Vec::new();
        if let (Some(min), Some(max)) =
            (ctx.domain(forward_var).min(), ctx.domain(forward_var).max())
        {
            for value in min..=max {
                if !ctx.domain(forward_var).contains(value) {
                    continue;
                }
                let Ok(target_index) = usize::try_from(value) else {
                    unsupported.push(value);
                    continue;
                };
                let Some(&backward_var) = backward.get(target_index) else {
                    unsupported.push(value);
                    continue;
                };
                if !ctx.domain(backward_var).contains(index_value) {
                    unsupported.push(value);
                }
            }
        }
        for value in unsupported {
            if ctx.remove_value(forward_var, value) {
                changed = true;
            }
        }
    }
    changed
}

fn tighten_to_point(ctx: &mut dyn PropagationContext, var: VariableId, value: i32) -> bool {
    let mut changed = false;
    if ctx.remove_below(var, value) {
        changed = true;
    }
    if ctx.remove_above(var, value) {
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn fixed_forward_propagates_backward() {
        let mut engine = Engine::new();
        let f0 = engine.new_variable(IntervalDomain::new(0, 2));
        let f1 = engine.new_variable(IntervalDomain::new(0, 2));
        let f2 = engine.new_variable(IntervalDomain::new(0, 2));
        let t0 = engine.new_variable(IntervalDomain::new(0, 2));
        let t1 = engine.new_variable(IntervalDomain::new(0, 2));
        let t2 = engine.new_variable(IntervalDomain::new(0, 2));
        engine.add_propagator(Box::new(InversePropagator::new(
            vec![f0, f1, f2],
            vec![t0, t1, t2],
        )));
        engine.fix_variable(f0, 1).unwrap();
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(t1).fixed_value(), Some(0));
    }
}
