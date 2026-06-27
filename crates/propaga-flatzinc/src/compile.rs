use crate::error::FlatZincError;
use crate::parse::{
    Constraint, DurationSpec, Expr, FlatZincProgram, IntSearchAnnotation, OutputDirective,
    ParamDecl, RestartKind, SearchAnnotations, SolveGoal, VarDecl,
};
use propaga_core::VariableId;
use propaga_model::Model;
use propaga_propagators::{CardinalityBound, DisjunctiveTask, TaskSpec};
use propaga_search::{RestartPolicy, ValueOrdering, VariableOrdering};
use std::collections::HashMap;

use propaga_search::ObjectiveDirection;

/// Search configuration extracted from FlatZinc annotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationSearchConfig {
    /// Variable ordering from `int_search`.
    pub variable_ordering: VariableOrdering,
    /// Value ordering from `int_search`.
    pub value_ordering: ValueOrdering,
    /// Restart policy from `restart_*`.
    pub restart_policy: RestartPolicy,
}

/// Objective specification extracted from a FlatZinc solve directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveSpec {
    /// Objective variable to optimize.
    pub var: VariableId,
    /// Optimization direction.
    pub direction: ObjectiveDirection,
}

/// A compiled FlatZinc instance ready for search.
pub struct CompiledInstance {
    /// Underlying model with posted constraints.
    pub model: Model,
    /// Decision variables in solve order.
    pub solve_vars: Vec<VariableId>,
    /// Human-readable variable names for output.
    pub names: HashMap<VariableId, String>,
    /// Parsed output directives.
    pub outputs: Vec<OutputDirective>,
    /// Optional optimization objective.
    pub objective: Option<ObjectiveSpec>,
    /// Optional search configuration from FlatZinc annotations.
    pub annotation_search: Option<AnnotationSearchConfig>,
}

/// Compiles a parsed FlatZinc program into a Propaga model.
pub fn compile(program: FlatZincProgram) -> Result<CompiledInstance, FlatZincError> {
    let mut model = Model::new();
    let mut env: HashMap<String, Binding> = HashMap::new();
    let mut names = HashMap::new();

    for param in program.params {
        match param {
            ParamDecl::Int { name, value } => {
                env.insert(name, Binding::Param(value));
            }
            ParamDecl::IntArray { name, values } => {
                env.insert(name, Binding::ParamArray(values));
            }
        }
    }

    for decl in program.variables {
        match decl {
            VarDecl::IntVar { name, low, high } => {
                let var = if low == high {
                    model.int_var_fixed(low)
                } else {
                    model.int_var(low, high)
                };
                names.insert(var, name.clone());
                env.insert(name, Binding::Var(var));
            }
            VarDecl::Array {
                name,
                index_low,
                index_high,
                low,
                high,
            } => {
                let mut elements = HashMap::new();
                for index in index_low..=index_high {
                    let var = model.int_var(low, high);
                    names.insert(var, format!("{name}[{index}]"));
                    elements.insert(index, var);
                }
                env.insert(name, Binding::Array(elements));
            }
            VarDecl::BoolVar { name, fixed } => {
                let var = match fixed {
                    Some(value) => model.int_var_fixed(value),
                    None => model.int_var(0, 1),
                };
                names.insert(var, name.clone());
                env.insert(name, Binding::Var(var));
            }
            VarDecl::BoolArray {
                name,
                index_low,
                index_high,
            } => {
                let mut elements = HashMap::new();
                for index in index_low..=index_high {
                    let var = model.int_var(0, 1);
                    names.insert(var, format!("{name}[{index}]"));
                    elements.insert(index, var);
                }
                env.insert(name, Binding::Array(elements));
            }
        }
    }

    for constraint in program.constraints {
        post_constraint(&mut model, &env, constraint)?;
    }

    let annotation_search = compile_search_config(&program.solve.annotations)?;
    let solve_vars =
        resolve_search_vars(&env, program.solve.annotations.int_search.as_ref(), &model)?;

    let objective = match program.solve.goal {
        SolveGoal::Satisfy => None,
        SolveGoal::Minimize(expr) => {
            let var = resolve_var(&env, expr)?;
            Some(ObjectiveSpec {
                var,
                direction: ObjectiveDirection::Minimize,
            })
        }
        SolveGoal::Maximize(expr) => {
            let var = resolve_var(&env, expr)?;
            Some(ObjectiveSpec {
                var,
                direction: ObjectiveDirection::Maximize,
            })
        }
    };

    Ok(CompiledInstance {
        model,
        solve_vars,
        names,
        outputs: program.outputs,
        objective,
        annotation_search,
    })
}

enum Binding {
    Param(i32),
    ParamArray(Vec<i32>),
    Var(VariableId),
    Array(HashMap<i32, VariableId>),
}

fn compile_search_config(
    annotations: &SearchAnnotations,
) -> Result<Option<AnnotationSearchConfig>, FlatZincError> {
    if annotations.int_search.is_none() && annotations.restart.is_none() {
        return Ok(None);
    }

    let (variable_ordering, value_ordering) = if let Some(int_search) = &annotations.int_search {
        if !int_search.complete {
            return Err(FlatZincError::Unsupported(
                "incomplete int_search is not supported".to_string(),
            ));
        }
        (
            map_var_choice(&int_search.var_choice)?,
            map_value_choice(&int_search.value_choice)?,
        )
    } else {
        (VariableOrdering::default(), ValueOrdering::default())
    };

    let restart_policy = match annotations.restart.as_ref().map(|restart| &restart.kind) {
        Some(RestartKind::Constant { scale }) => RestartPolicy::Constant { scale: *scale },
        Some(RestartKind::Geometric { base, scale }) => RestartPolicy::Geometric {
            base: parse_geometric_restart_base(base)?,
            scale: *scale,
        },
        Some(RestartKind::Luby { base }) => RestartPolicy::Luby { base: *base },
        Some(RestartKind::None) => RestartPolicy::None,
        None => RestartPolicy::default(),
    };

    Ok(Some(AnnotationSearchConfig {
        variable_ordering,
        value_ordering,
        restart_policy,
    }))
}

fn parse_geometric_restart_base(base: &str) -> Result<f64, FlatZincError> {
    let parsed = base.parse::<f64>().map_err(|_| {
        FlatZincError::Unsupported(format!("invalid restart_geometric base `{base}`"))
    })?;
    if parsed <= 0.0 {
        return Err(FlatZincError::Unsupported(
            "restart_geometric base must be positive".to_string(),
        ));
    }
    Ok(parsed)
}

fn map_var_choice(choice: &str) -> Result<VariableOrdering, FlatZincError> {
    match choice {
        "input_order" => Ok(VariableOrdering::InputOrder),
        "first_fail" => Ok(VariableOrdering::Mrv),
        "smallest" | "occurrence" | "degree" | "anti_first_fail" => Ok(VariableOrdering::Dom),
        "largest" => Ok(VariableOrdering::DomWdeg),
        other => Err(FlatZincError::Unsupported(format!(
            "unsupported variable selection `{other}`"
        ))),
    }
}

fn map_value_choice(choice: &str) -> Result<ValueOrdering, FlatZincError> {
    match choice {
        "indomain_min" => Ok(ValueOrdering::Ascending),
        "indomain_max" => Ok(ValueOrdering::Descending),
        "indomain_split" | "indomain_median" => Ok(ValueOrdering::Ascending),
        other => Err(FlatZincError::Unsupported(format!(
            "unsupported value selection `{other}`"
        ))),
    }
}

fn resolve_search_vars(
    env: &HashMap<String, Binding>,
    int_search: Option<&IntSearchAnnotation>,
    model: &Model,
) -> Result<Vec<VariableId>, FlatZincError> {
    if let Some(int_search) = int_search {
        let vars = resolve_var_list(env, Expr::List(int_search.vars.clone()))?;
        if vars.is_empty() {
            return Err(FlatZincError::Unsupported(
                "int_search has no variables".to_string(),
            ));
        }
        Ok(vars)
    } else {
        Ok(model.decision_variables().to_vec())
    }
}

fn post_constraint(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    constraint: Constraint,
) -> Result<(), FlatZincError> {
    match constraint {
        Constraint::AllDifferent(vars) => {
            let vars = resolve_var_list(env, Expr::List(vars))?;
            model.all_different(vars);
        }
        Constraint::IntEq(left, right) => {
            let left_var = resolve_var(env, left)?;
            match right {
                Expr::Int(value) => {
                    model
                        .engine_mut()
                        .fix_variable(left_var, value)
                        .map_err(|_| {
                            FlatZincError::Unsupported("failed to fix variable".to_string())
                        })?;
                }
                other => {
                    let right_var = resolve_var(env, other)?;
                    model.equal(left_var, right_var);
                }
            }
        }
        Constraint::IntLinEq { coeffs, vars, rhs } => {
            post_linear_eq(model, env, &coeffs, vars, rhs)?;
        }
        Constraint::IntLinLe { coeffs, vars, rhs } => {
            post_linear_le(model, env, &coeffs, vars, rhs)?;
        }
        Constraint::IntLinGe { coeffs, vars, rhs } => {
            post_linear_ge(model, env, &coeffs, vars, rhs)?;
        }
        Constraint::IntLinLeReif {
            coeffs,
            vars,
            rhs,
            reif,
        } => {
            post_linear_le_reif(model, env, &coeffs, vars, rhs, reif)?;
        }
        Constraint::IntLinGeReif {
            coeffs,
            vars,
            rhs,
            reif,
        } => {
            post_linear_ge_reif(model, env, &coeffs, vars, rhs, reif)?;
        }
        Constraint::IntLinEqReif {
            coeffs,
            vars,
            rhs,
            reif,
        } => {
            post_linear_eq_reif(model, env, &coeffs, vars, rhs, reif)?;
        }
        Constraint::IntNe(left, right) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            model.not_equal_offset(left_var, right_var, 0);
        }
        Constraint::IntLe(left, right) => {
            post_int_le(model, env, left, right)?;
        }
        Constraint::IntLt(left, right) => {
            post_int_lt(model, env, left, right)?;
        }
        Constraint::IntGe(left, right) => {
            post_int_le(model, env, right, left)?;
        }
        Constraint::IntGt(left, right) => {
            post_int_lt(model, env, right, left)?;
        }
        Constraint::IntEqReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_equal(left_var, right_var, reif_var);
        }
        Constraint::IntNeReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_not_equal(left_var, right_var, reif_var);
        }
        Constraint::IntLeReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_less_equal(left_var, right_var, reif_var);
        }
        Constraint::IntLtReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_less_than(left_var, right_var, reif_var);
        }
        Constraint::IntGeReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_less_equal(right_var, left_var, reif_var);
        }
        Constraint::IntGtReif(left, right, reif) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            let reif_var = resolve_var(env, reif)?;
            model.reified_less_than(right_var, left_var, reif_var);
        }
        Constraint::Element {
            array,
            index,
            value,
        } => {
            let array_vars = resolve_var_list(env, array)?;
            let index_var = resolve_var(env, index)?;
            let value_var = resolve_var(env, value)?;
            model.element(index_var, array_vars, value_var);
        }
        Constraint::Cumulative {
            starts,
            durations,
            ends,
            heights,
            capacity,
        } => {
            post_cumulative(model, env, starts, durations, ends, heights, capacity)?;
        }
        Constraint::Disjunctive { starts, durations } => {
            post_disjunctive(model, env, starts, durations)?;
        }
        Constraint::GlobalCardinality {
            vars,
            cover,
            lbound,
            ubound,
        } => {
            post_global_cardinality(model, env, vars, cover, lbound, ubound)?;
        }
        Constraint::Table { vars, tuples } => {
            post_table(model, env, vars, tuples)?;
        }
        Constraint::BoolEq(left, right) => {
            let left_var = resolve_var(env, left)?;
            match right {
                Expr::Int(value) => {
                    model
                        .engine_mut()
                        .fix_variable(left_var, value)
                        .map_err(|_| {
                            FlatZincError::Unsupported("failed to fix variable".to_string())
                        })?;
                }
                other => {
                    let right_var = resolve_var(env, other)?;
                    model.equal(left_var, right_var);
                }
            }
        }
        Constraint::Bool2Int(bool_expr, int_expr) => {
            let bool_var = resolve_var(env, bool_expr)?;
            let int_var = resolve_var(env, int_expr)?;
            model.equal(bool_var, int_var);
        }
    }
    Ok(())
}

fn post_cumulative(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    starts: Expr,
    durations: DurationSpec,
    ends: Expr,
    heights: Option<DurationSpec>,
    capacity: i32,
) -> Result<(), FlatZincError> {
    let start_vars = resolve_var_list(env, starts)?;
    let end_vars = resolve_var_list(env, ends)?;
    if start_vars.len() != end_vars.len() {
        return Err(FlatZincError::Unsupported(
            "cumulative start/end length mismatch".to_string(),
        ));
    }

    let duration_values = resolve_duration_values(env, durations)?;
    let height_values = match heights {
        Some(spec) => resolve_duration_values(env, spec)?,
        None => vec![1; start_vars.len()],
    };

    if duration_values.len() != start_vars.len() {
        return Err(FlatZincError::Unsupported(
            "cumulative duration length mismatch".to_string(),
        ));
    }
    if height_values.len() != start_vars.len() {
        return Err(FlatZincError::Unsupported(
            "cumulative height length mismatch".to_string(),
        ));
    }

    let tasks: Vec<TaskSpec> = start_vars
        .into_iter()
        .zip(end_vars)
        .zip(duration_values)
        .zip(height_values)
        .map(|(((start, end), duration), demand)| {
            TaskSpec::with_demand(start, duration, end, demand)
        })
        .collect();
    model.cumulative(tasks, capacity);
    Ok(())
}

fn post_linear_eq(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    if coeffs.iter().all(|&coeff| coeff == 1) {
        let resolved = resolve_var_list(env, Expr::List(vars))?;
        return post_unit_sum(model, &resolved, rhs);
    }

    if coeffs.len() == 2 && coeffs[0] == 1 && coeffs[1] == 1 {
        let left = resolve_var(env, vars[0].clone())?;
        let right = resolve_var(env, vars[1].clone())?;
        let sum = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(left, right, sum);
        model
            .engine_mut()
            .fix_variable(sum, rhs)
            .map_err(|_| FlatZincError::Unsupported("failed to fix sum variable".to_string()))?;
        return Ok(());
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    model.scalar_eq(coeffs.to_vec(), resolved, rhs);
    Ok(())
}

fn post_linear_le(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    if coeffs.iter().all(|&coeff| coeff == 1) {
        let resolved = resolve_var_list(env, Expr::List(vars))?;
        return post_unit_sum_le(model, &resolved, rhs);
    }

    if coeffs.len() == 2 && coeffs[0] == 1 && coeffs[1] == 1 {
        let left = resolve_var(env, vars[0].clone())?;
        let right = resolve_var(env, vars[1].clone())?;
        let sum = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(left, right, sum);
        let bound = model.int_var_fixed(rhs);
        model.less_equal(sum, bound);
        return Ok(());
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    model.scalar_le(coeffs.to_vec(), resolved, rhs);
    Ok(())
}

fn post_linear_ge(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    if coeffs.iter().all(|&coeff| coeff == 1) {
        let resolved = resolve_var_list(env, Expr::List(vars))?;
        return post_unit_sum_ge(model, &resolved, rhs);
    }

    if coeffs.len() == 2 && coeffs[0] == 1 && coeffs[1] == 1 {
        let left = resolve_var(env, vars[0].clone())?;
        let right = resolve_var(env, vars[1].clone())?;
        let sum = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(left, right, sum);
        let bound = model.int_var_fixed(rhs);
        model.greater_equal(sum, bound);
        return Ok(());
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    model.scalar_ge(coeffs.to_vec(), resolved, rhs);
    Ok(())
}

fn post_linear_le_reif(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
    reif: Expr,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    let reif_var = resolve_var(env, reif)?;
    model.reified_scalar_le(coeffs.to_vec(), resolved, rhs, reif_var);
    Ok(())
}

fn post_linear_ge_reif(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
    reif: Expr,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    let reif_var = resolve_var(env, reif)?;
    model.reified_scalar_ge(coeffs.to_vec(), resolved, rhs, reif_var);
    Ok(())
}

fn post_linear_eq_reif(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: &[i32],
    vars: Vec<Expr>,
    rhs: i32,
    reif: Expr,
) -> Result<(), FlatZincError> {
    if coeffs.len() != vars.len() {
        return Err(FlatZincError::Unsupported(
            "coefficient and variable length mismatch".to_string(),
        ));
    }

    let resolved = resolve_var_list(env, Expr::List(vars))?;
    let reif_var = resolve_var(env, reif)?;
    model.reified_scalar_eq(coeffs.to_vec(), resolved, rhs, reif_var);
    Ok(())
}

fn post_unit_sum_le(model: &mut Model, vars: &[VariableId], rhs: i32) -> Result<(), FlatZincError> {
    if vars.is_empty() {
        return if rhs >= 0 {
            Ok(())
        } else {
            Err(FlatZincError::Unsupported(
                "empty linear sum exceeds rhs".to_string(),
            ))
        };
    }

    if vars.len() == 1 {
        let bound = model.int_var_fixed(rhs);
        model.less_equal(vars[0], bound);
        return Ok(());
    }

    let mut running = vars[0];
    for &next in &vars[1..vars.len() - 1] {
        let partial = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(running, next, partial);
        running = partial;
    }
    let last = *vars.last().expect("len >= 2");
    let total = model.int_var(i32::MIN / 4, i32::MAX / 4);
    model.linear_eq(running, last, total);
    let bound = model.int_var_fixed(rhs);
    model.less_equal(total, bound);
    Ok(())
}

fn post_unit_sum_ge(model: &mut Model, vars: &[VariableId], rhs: i32) -> Result<(), FlatZincError> {
    if vars.is_empty() {
        return if rhs <= 0 {
            Ok(())
        } else {
            Err(FlatZincError::Unsupported(
                "empty linear sum below rhs".to_string(),
            ))
        };
    }

    if vars.len() == 1 {
        let bound = model.int_var_fixed(rhs);
        model.greater_equal(vars[0], bound);
        return Ok(());
    }

    let mut running = vars[0];
    for &next in &vars[1..vars.len() - 1] {
        let partial = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(running, next, partial);
        running = partial;
    }
    let last = *vars.last().expect("len >= 2");
    let total = model.int_var(i32::MIN / 4, i32::MAX / 4);
    model.linear_eq(running, last, total);
    let bound = model.int_var_fixed(rhs);
    model.greater_equal(total, bound);
    Ok(())
}

fn post_unit_sum(model: &mut Model, vars: &[VariableId], rhs: i32) -> Result<(), FlatZincError> {
    if vars.is_empty() {
        return if rhs == 0 {
            Ok(())
        } else {
            Err(FlatZincError::Unsupported("empty linear sum".to_string()))
        };
    }

    if vars.len() == 1 {
        model
            .engine_mut()
            .fix_variable(vars[0], rhs)
            .map_err(|_| FlatZincError::Unsupported("failed to fix variable".to_string()))?;
        return Ok(());
    }

    let mut running = vars[0];
    for &next in &vars[1..vars.len() - 1] {
        let partial = model.int_var(i32::MIN / 4, i32::MAX / 4);
        model.linear_eq(running, next, partial);
        running = partial;
    }
    let last = *vars.last().expect("len >= 2");
    let total = model.int_var(i32::MIN / 4, i32::MAX / 4);
    model.linear_eq(running, last, total);
    model
        .engine_mut()
        .fix_variable(total, rhs)
        .map_err(|_| FlatZincError::Unsupported("failed to fix sum variable".to_string()))?;
    Ok(())
}

fn post_int_lt(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    left: Expr,
    right: Expr,
) -> Result<(), FlatZincError> {
    match (left, right) {
        (Expr::Int(lvalue), right) => {
            let left_var = model.int_var_fixed(lvalue);
            let right_var = resolve_var(env, right)?;
            model.less_than(left_var, right_var);
        }
        (left, Expr::Int(rvalue)) => {
            let left_var = resolve_var(env, left)?;
            let right_var = model.int_var_fixed(rvalue);
            model.less_than(left_var, right_var);
        }
        (left, right) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            model.less_than(left_var, right_var);
        }
    }
    Ok(())
}

fn post_disjunctive(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    starts: Expr,
    durations: DurationSpec,
) -> Result<(), FlatZincError> {
    let start_vars = resolve_var_list(env, starts)?;
    let duration_values = resolve_duration_values(env, durations)?;
    if duration_values.len() != start_vars.len() {
        return Err(FlatZincError::Unsupported(
            "disjunctive start/duration length mismatch".to_string(),
        ));
    }
    if start_vars.len() < 2 {
        return Err(FlatZincError::Unsupported(
            "disjunctive requires at least two tasks".to_string(),
        ));
    }

    let tasks: Vec<DisjunctiveTask> = start_vars
        .into_iter()
        .zip(duration_values)
        .map(|(start, duration)| DisjunctiveTask { start, duration })
        .collect();
    model.disjunctive(tasks);
    Ok(())
}

fn post_global_cardinality(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    vars: Expr,
    cover: Expr,
    lbound: Option<Expr>,
    ubound: Option<Expr>,
) -> Result<(), FlatZincError> {
    let var_list = resolve_var_list(env, vars)?;
    let cover_values = resolve_int_array(env, cover)?;
    let lb_values = match lbound {
        Some(expr) => resolve_int_array(env, expr)?,
        None => vec![1; cover_values.len()],
    };
    let ub_values = match ubound {
        Some(expr) => resolve_int_array(env, expr)?,
        None => vec![1; cover_values.len()],
    };

    if cover_values.len() != lb_values.len() || cover_values.len() != ub_values.len() {
        return Err(FlatZincError::Unsupported(
            "global_cardinality cover/bounds length mismatch".to_string(),
        ));
    }

    let cards: Vec<(i32, CardinalityBound)> = cover_values
        .into_iter()
        .zip(lb_values)
        .zip(ub_values)
        .map(|((value, min), max)| (value, CardinalityBound { min, max }))
        .collect();
    model.gcc(var_list, cards);
    Ok(())
}

fn post_table(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    vars: Expr,
    flat_tuples: Vec<i32>,
) -> Result<(), FlatZincError> {
    let var_list = resolve_var_list(env, vars)?;
    if var_list.is_empty() {
        return Err(FlatZincError::Unsupported(
            "table constraint requires at least one variable".to_string(),
        ));
    }
    if !flat_tuples.is_empty() && !flat_tuples.len().is_multiple_of(var_list.len()) {
        return Err(FlatZincError::Unsupported(
            "table tuple length does not match variable count".to_string(),
        ));
    }

    let width = var_list.len();
    let tuples: Vec<Vec<i32>> = flat_tuples
        .chunks(width)
        .map(|chunk| chunk.to_vec())
        .collect();
    model.table(var_list, tuples);
    Ok(())
}

fn resolve_int_array(
    env: &HashMap<String, Binding>,
    expr: Expr,
) -> Result<Vec<i32>, FlatZincError> {
    match expr {
        Expr::List(items) => {
            let mut values = Vec::new();
            for item in items {
                values.push(resolve_int(env, item)?);
            }
            Ok(values)
        }
        Expr::Name(name) => match env.get(&name) {
            Some(Binding::ParamArray(values)) => Ok(values.clone()),
            Some(Binding::Param(value)) => Ok(vec![*value]),
            Some(Binding::Var(_)) | Some(Binding::Array(_)) => Err(FlatZincError::Unsupported(
                format!("variable `{name}` used as integer array"),
            )),
            None => Err(FlatZincError::UnknownIdentifier(name)),
        },
        Expr::Int(value) => Ok(vec![value]),
        Expr::Index { .. } => Err(FlatZincError::Unsupported(
            "indexed expression used as integer array".to_string(),
        )),
    }
}

fn resolve_duration_values(
    env: &HashMap<String, Binding>,
    durations: DurationSpec,
) -> Result<Vec<i32>, FlatZincError> {
    match durations {
        DurationSpec::Inline(values) => Ok(values),
        DurationSpec::Name(name) => match env.get(&name) {
            Some(Binding::ParamArray(values)) => Ok(values.clone()),
            Some(Binding::Param(_)) => Err(FlatZincError::Unsupported(format!(
                "scalar `{name}` used as duration array"
            ))),
            Some(Binding::Var(_)) | Some(Binding::Array(_)) => Err(FlatZincError::Unsupported(
                format!("variable `{name}` used as duration array"),
            )),
            None => Err(FlatZincError::UnknownIdentifier(name)),
        },
    }
}

fn post_int_le(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    left: Expr,
    right: Expr,
) -> Result<(), FlatZincError> {
    match (left, right) {
        (Expr::Int(lvalue), right) => {
            let left_var = model.int_var_fixed(lvalue);
            let right_var = resolve_var(env, right)?;
            model.less_equal(left_var, right_var);
        }
        (left, Expr::Int(rvalue)) => {
            let left_var = resolve_var(env, left)?;
            let right_var = model.int_var_fixed(rvalue);
            model.less_equal(left_var, right_var);
        }
        (left, right) => {
            let left_var = resolve_var(env, left)?;
            let right_var = resolve_var(env, right)?;
            model.less_equal(left_var, right_var);
        }
    }
    Ok(())
}

fn resolve_var_list(
    env: &HashMap<String, Binding>,
    expr: Expr,
) -> Result<Vec<VariableId>, FlatZincError> {
    match expr {
        Expr::List(items) => {
            let mut vars = Vec::new();
            for item in items {
                vars.extend(resolve_var_list(env, item)?);
            }
            Ok(vars)
        }
        Expr::Name(name) => match env.get(&name) {
            Some(Binding::Array(elements)) => {
                let mut indices: Vec<_> = elements.keys().copied().collect();
                indices.sort_unstable();
                Ok(indices.into_iter().map(|index| elements[&index]).collect())
            }
            Some(Binding::Var(var)) => Ok(vec![*var]),
            Some(Binding::Param(_)) | Some(Binding::ParamArray(_)) => Err(
                FlatZincError::Unsupported(format!("parameter `{name}` used as decision variable")),
            ),
            None => Err(FlatZincError::UnknownIdentifier(name)),
        },
        other => resolve_var(env, other).map(|var| vec![var]),
    }
}

fn resolve_var(env: &HashMap<String, Binding>, expr: Expr) -> Result<VariableId, FlatZincError> {
    match expr {
        Expr::Name(name) => match env.get(&name) {
            Some(Binding::Var(var)) => Ok(*var),
            Some(Binding::Param(value)) => Err(FlatZincError::Unsupported(format!(
                "parameter `{name}`={value} used as variable"
            ))),
            Some(Binding::ParamArray(_)) => Err(FlatZincError::Unsupported(format!(
                "array parameter `{name}` used as variable"
            ))),
            Some(Binding::Array(_)) => Err(FlatZincError::Unsupported(format!(
                "array `{name}` requires an index"
            ))),
            None => Err(FlatZincError::UnknownIdentifier(name)),
        },
        Expr::Index { name, index } => {
            let Binding::Array(elements) = env
                .get(&name)
                .ok_or_else(|| FlatZincError::UnknownIdentifier(name.clone()))?
            else {
                return Err(FlatZincError::Unsupported(format!(
                    "`{name}` is not an array"
                )));
            };
            let index_value = resolve_int(env, *index)?;
            elements.get(&index_value).copied().ok_or_else(|| {
                FlatZincError::Unsupported(format!("index {index_value} out of range"))
            })
        }
        Expr::Int(value) => Err(FlatZincError::Unsupported(format!(
            "integer literal `{value}` used as variable"
        ))),
        Expr::List(_) => Err(FlatZincError::Unsupported(
            "list expression used as scalar variable".to_string(),
        )),
    }
}

fn resolve_int(env: &HashMap<String, Binding>, expr: Expr) -> Result<i32, FlatZincError> {
    match expr {
        Expr::Int(value) => Ok(value),
        Expr::Name(name) => match env.get(&name) {
            Some(Binding::Param(value)) => Ok(*value),
            Some(Binding::ParamArray(_)) => Err(FlatZincError::Unsupported(format!(
                "array parameter `{name}` used as index"
            ))),
            Some(Binding::Var(_)) => Err(FlatZincError::Unsupported(format!(
                "variable `{name}` used as index"
            ))),
            Some(Binding::Array(_)) => Err(FlatZincError::Unsupported(format!(
                "array `{name}` used as index"
            ))),
            None => Err(FlatZincError::UnknownIdentifier(name)),
        },
        _ => Err(FlatZincError::Unsupported(
            "complex index expression".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn compiles_reified_constraints() {
        let source = r#"
            var 1..3: x;
            var 1..3: y;
            var 0..1: b;
            constraint int_eq_reif(x, y, b);
            constraint int_ne_reif(x, y, b);
            constraint int_le_reif(x, y, b);
            constraint int_lt_reif(x, y, b);
            constraint int_ge_reif(x, y, b);
            constraint int_gt_reif(x, y, b);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_cumulative_with_heights() {
        let source = r#"
            array [1..2] of int: duration = [2, 2];
            array [1..2] of int: height = [2, 1];
            array [1..2] of var 0..10: s;
            array [1..2] of var 0..20: e;
            constraint cumulative(s, duration, e, height, 2);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_reified_linear_constraints() {
        let source = r#"
            var 1..3: x;
            var 1..3: y;
            var 1..3: z;
            var 0..1: b;
            constraint int_lin_le_reif([1, 1, 1], [x, y, z], 6, b);
            constraint int_lin_ge_reif([1, 1, 1], [x, y, z], 4, b);
            constraint int_lin_eq_reif([1, 1, 1], [x, y, z], 5, b);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_weighted_int_lin_le() {
        let source = r#"
            array [1..3] of var 0..4: x;
            constraint int_lin_le([2, 1, 1], [x[1], x[2], x[3]], 6);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_int_lin_le_sum() {
        let source = r#"
            array [1..3] of var 1..4: x;
            constraint int_lin_le([1, 1, 1], [x[1], x[2], x[3]], 8);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_weighted_int_lin_ge() {
        let source = r#"
            array [1..3] of var 0..4: x;
            constraint int_lin_ge([2, 1, 1], [x[1], x[2], x[3]], 4);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_int_lin_ge_sum() {
        let source = r#"
            array [1..3] of var 1..4: x;
            constraint int_lin_ge([1, 1, 1], [x[1], x[2], x[3]], 6);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_disjunctive_and_strict_ordering() {
        let source = r#"
            array [1..2] of int: duration = [3, 2];
            array [1..2] of var 0..10: s;
            constraint disjunctive(s, duration);
            array [1..3] of var 1..3: x;
            constraint int_lt(x[1], x[2]);
            constraint int_ge(x[3], x[2]);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_int_le_chain() {
        let source = r#"
            array [1..3] of var 1..3: x;
            constraint all_different(x);
            constraint int_le(x[1], x[2]);
            constraint int_le(x[2], x[3]);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(instance.solve_vars.len(), 3);
    }

    #[test]
    fn compiles_all_different_array() {
        let source = r#"
            array [1..3] of var 1..3: x;
            constraint all_different(x);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(instance.solve_vars.len(), 3);
    }

    #[test]
    fn compiles_global_cardinality_two_arg() {
        let source = r#"
            array [1..3] of int: cards = [1, 2, 3];
            array [1..3] of var 1..3: x;
            constraint global_cardinality(cards, x);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_global_cardinality_with_bounds() {
        let source = r#"
            array [1..3] of var 1..3: x;
            array [1..3] of int: cover = [1, 2, 3];
            array [1..3] of int: lb = [1, 1, 1];
            array [1..3] of int: ub = [1, 1, 1];
            constraint global_cardinality(x, cover, lb, ub);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_table_constraint() {
        let source = r#"
            var 1..5: x;
            var 1..5: y;
            constraint table([x, y], {1, 2, 3, 4});
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        compile(program).unwrap();
    }

    #[test]
    fn compiles_minimize_and_maximize() {
        let minimize = r#"
            var 0..10: x;
            solve minimize x;
        "#;
        compile(parse(minimize).unwrap()).unwrap();

        let maximize = r#"
            var 0..10: x;
            solve maximize x;
        "#;
        compile(parse(maximize).unwrap()).unwrap();
    }

    #[test]
    fn compiles_bool_constraints() {
        let source = r#"
            var bool: b;
            var 0..5: x;
            constraint bool2int(b, x);
            constraint int_eq(x, 1);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        let mut instance = compile(program).unwrap();
        let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
        assert!(solution.is_some());
    }

    #[test]
    fn compiles_int_search_variable_order() {
        let source = r#"
            array [1..3] of var 1..3: x;
            constraint all_different(x);
            solve :: int_search([x[3], x[1], x[2]], input_order, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(instance.solve_vars.len(), 3);
        assert_eq!(
            instance
                .names
                .get(&instance.solve_vars[0])
                .map(String::as_str),
            Some("x[3]")
        );
        assert_eq!(
            instance
                .names
                .get(&instance.solve_vars[1])
                .map(String::as_str),
            Some("x[1]")
        );
        assert_eq!(
            instance.annotation_search,
            Some(AnnotationSearchConfig {
                variable_ordering: VariableOrdering::InputOrder,
                value_ordering: ValueOrdering::Ascending,
                restart_policy: RestartPolicy::default(),
            })
        );
    }

    #[test]
    fn compiles_restart_none_annotation() {
        let source = r#"
            var 1..3: x;
            solve :: restart_none :: int_search([x], first_fail, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(
            instance.annotation_search.unwrap().restart_policy,
            RestartPolicy::None
        );
    }

    #[test]
    fn compiles_constant_and_geometric_restart_annotations() {
        let constant = r#"
            var 1..3: x;
            solve :: restart_constant(100) satisfy;
        "#;
        let program = parse(constant).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(
            instance.annotation_search.unwrap().restart_policy,
            RestartPolicy::Constant { scale: 100 }
        );

        let geometric = r#"
            var 1..3: x;
            solve :: restart_geometric(1.5, 100) satisfy;
        "#;
        let program = parse(geometric).unwrap();
        let instance = compile(program).unwrap();
        assert_eq!(
            instance.annotation_search.unwrap().restart_policy,
            RestartPolicy::Geometric {
                base: 1.5,
                scale: 100
            }
        );
    }
}
