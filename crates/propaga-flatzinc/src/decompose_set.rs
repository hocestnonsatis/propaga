use propaga_core::VariableId;
use propaga_model::Model;

/// Posts `left == right` for set variables.
pub fn set_eq(model: &mut Model, left: VariableId, right: VariableId) {
    model.set_subset(left, right);
    model.set_subset(right, left);
}

/// Posts `left != right`.
pub fn set_ne(model: &mut Model, left: VariableId, right: VariableId) {
    let reif = model.int_var_aux(0, 1);
    model.set_eq_reif(left, right, reif);
    let zero = model.int_var_fixed(0);
    model.equal(reif, zero);
}

/// Posts `superset ⊇ subset`.
pub fn set_superset(model: &mut Model, superset: VariableId, subset: VariableId) {
    model.set_subset(subset, superset);
}

/// Posts `value ∈ set`.
pub fn set_in(model: &mut Model, value: VariableId, set: VariableId) {
    model.set_member(value, set);
}

/// Posts `left <= right` as subset.
pub fn set_le(model: &mut Model, left: VariableId, right: VariableId) {
    model.set_subset(left, right);
}

/// Posts `left < right`.
pub fn set_lt(model: &mut Model, left: VariableId, right: VariableId) {
    set_le(model, left, right);
    set_ne(model, left, right);
}

/// Posts `result = left \\ right`.
pub fn set_diff(model: &mut Model, left: VariableId, right: VariableId, result: VariableId) {
    model.set_diff(left, right, result);
}

/// Posts `result = left △ right`.
pub fn set_symdiff(model: &mut Model, left: VariableId, right: VariableId, result: VariableId) {
    model.set_symdiff(left, right, result);
}

/// Posts `reif <=> left == right`.
pub fn set_eq_reif(model: &mut Model, left: VariableId, right: VariableId, reif: VariableId) {
    model.set_eq_reif(left, right, reif);
}

/// Posts `reif <=> left != right`.
pub fn set_ne_reif(model: &mut Model, left: VariableId, right: VariableId, reif: VariableId) {
    let eq = model.int_var_aux(0, 1);
    model.set_eq_reif(left, right, eq);
    let zero = model.int_var_fixed(0);
    model.reified_equal(eq, zero, reif);
}

/// Posts `reif <=> value ∈ set`.
pub fn set_in_reif(model: &mut Model, value: VariableId, set: VariableId, reif: VariableId) {
    model.set_member_reif(value, set, reif);
}

/// Posts `reif <=> subset ⊆ superset`.
pub fn set_subset_reif(
    model: &mut Model,
    subset: VariableId,
    superset: VariableId,
    reif: VariableId,
) {
    model.set_subset_reif(subset, superset, reif);
}

/// Posts `reif <=> superset ⊇ subset`.
pub fn set_superset_reif(
    model: &mut Model,
    superset: VariableId,
    subset: VariableId,
    reif: VariableId,
) {
    set_subset_reif(model, subset, superset, reif);
}

/// Posts `reif <=> left <= right`.
pub fn set_le_reif(model: &mut Model, left: VariableId, right: VariableId, reif: VariableId) {
    set_subset_reif(model, left, right, reif);
}

/// Posts `reif <=> left < right`.
pub fn set_lt_reif(model: &mut Model, left: VariableId, right: VariableId, reif: VariableId) {
    let le_reif = model.int_var_aux(0, 1);
    set_subset_reif(model, left, right, le_reif);
    let eq = model.int_var_aux(0, 1);
    model.set_eq_reif(left, right, eq);
    let ne_reif = model.int_var_aux(0, 1);
    let zero = model.int_var_fixed(0);
    model.reified_equal(eq, zero, ne_reif);
    crate::decompose::bool_and(model, le_reif, ne_reif, reif);
}
