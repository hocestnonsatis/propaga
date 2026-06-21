use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagates a positive table constraint to generalized arc consistency.
pub struct TablePropagator {
    variables: Vec<VariableId>,
    tuples: Vec<Vec<i32>>,
}

impl TablePropagator {
    /// Creates a table propagator over `variables` with allowed `tuples`.
    #[must_use]
    pub fn new(variables: impl Into<Vec<VariableId>>, tuples: impl Into<Vec<Vec<i32>>>) -> Self {
        Self {
            variables: variables.into(),
            tuples: tuples.into(),
        }
    }
}

impl Propagator for TablePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.variables
    }

    fn priority(&self) -> u32 {
        20
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        if self.variables.is_empty() {
            return PropagationStatus::OkNoChange;
        }

        if self.tuples.is_empty() {
            return PropagationStatus::Failure;
        }

        let mut changed = false;
        loop {
            let mut round_changed = false;
            for (index, &var) in self.variables.iter().enumerate() {
                let supported: Vec<i32> = collect_values(ctx, var)
                    .into_iter()
                    .filter(|value| self.has_support(ctx, index, *value))
                    .collect();

                for value in collect_values(ctx, var) {
                    if !supported.contains(&value) && ctx.remove_value(var, value) {
                        round_changed = true;
                    }
                }
            }

            changed |= round_changed;
            if !round_changed {
                break;
            }
        }

        if self.variables.iter().any(|var| ctx.domain(*var).is_empty()) {
            PropagationStatus::Failure
        } else if changed {
            PropagationStatus::OkChanged
        } else {
            PropagationStatus::OkNoChange
        }
    }
}

impl TablePropagator {
    fn has_support(&self, ctx: &dyn PropagationContext, var_index: usize, value: i32) -> bool {
        self.tuples.iter().any(|tuple| {
            if tuple.get(var_index).copied() != Some(value) {
                return false;
            }
            self.variables.iter().enumerate().all(|(index, &var)| {
                if index == var_index {
                    return true;
                }
                tuple
                    .get(index)
                    .is_some_and(|candidate| ctx.domain(var).contains(*candidate))
            })
        })
    }
}

fn collect_values(ctx: &dyn PropagationContext, var: VariableId) -> Vec<i32> {
    let domain = ctx.domain(var);
    let mut values = Vec::new();
    if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
        for value in min..=max {
            if domain.contains(value) {
                values.push(value);
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn removes_unsupported_values() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 3));
        let y = engine.new_variable(IntervalDomain::new(1, 3));
        let tuples = vec![vec![1, 2], vec![2, 3], vec![3, 1]];
        engine.add_propagator(Box::new(TablePropagator::new(vec![x, y], tuples)));

        engine.propagate_all().unwrap();
        assert!(!engine.domain(x).contains(2) || engine.domain(x).contains(1));
        assert!(!engine.domain(y).contains(2) || engine.domain(y).contains(2));
    }

    #[test]
    fn empty_table_is_inconsistent() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(TablePropagator::new(
            vec![x],
            Vec::<Vec<i32>>::new(),
        )));

        let status = engine.propagate_all().unwrap();
        assert_eq!(status, PropagationStatus::Failure);
    }
}
