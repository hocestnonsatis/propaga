use propaga_core::VariableId;
use std::collections::HashMap;

/// Typed assignment value for mixed-domain models.
#[derive(Clone, Debug, PartialEq)]
pub enum AssignmentValue {
    Int(i32),
    Set(Vec<i32>),
    Float(f64),
}

/// Assignment mapping variables to typed values.
pub type Solution = Vec<(VariableId, AssignmentValue)>;

/// Returns the integer assignment for `var` when present.
#[must_use]
pub fn assignment_int(solution: &Solution, var: VariableId) -> Option<i32> {
    solution.iter().find_map(|(candidate, value)| {
        if *candidate == var {
            match value {
                AssignmentValue::Int(value) => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Collects integer assignments from a mixed solution.
#[must_use]
pub fn solution_int_map(solution: &Solution) -> HashMap<VariableId, i32> {
    solution
        .iter()
        .filter_map(|(var, value)| match value {
            AssignmentValue::Int(value) => Some((*var, *value)),
            _ => None,
        })
        .collect()
}

/// Returns integer columns from a solution in iteration order (int-only).
#[must_use]
pub fn solution_int_values(solution: &Solution) -> Vec<i32> {
    solution
        .iter()
        .filter_map(|(_, value)| match value {
            AssignmentValue::Int(value) => Some(*value),
            _ => None,
        })
        .collect()
}
