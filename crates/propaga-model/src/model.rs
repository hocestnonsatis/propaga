use propaga_core::{PropagationStatus, VariableId};
use propaga_domains::{AnyDomain, FloatDomain, HybridDomain, IntervalDomain, SetIntervalDomain};
use propaga_engine::Engine;
use propaga_propagators::{
    AllDifferentPropagator, CardinalityBound, CircuitPropagator, CumulativePropagator,
    DiffnPropagator, DisjunctivePropagator, DisjunctiveTask, ElementPropagator, EqualityPropagator,
    FloatBinaryOp, FloatBinaryPropagator, FloatElementPropagator, FloatEqPropagator,
    FloatEqReifPropagator, FloatLePropagator, FloatLeReifPropagator, FloatLinearEqPropagator,
    FloatLinearGePropagator, FloatLinearLePropagator, FloatLinearNePropagator,
    FloatLtReifPropagator, FloatMinMaxOp, FloatMinMaxPropagator, FloatNePropagator,
    FloatTimesPropagator, FloatUnaryOp, FloatUnaryPropagator, GlobalCardinalityPropagator,
    Int2FloatPropagator, IntAbsPropagator, IntDivPropagator, IntMinMaxOp, IntMinMaxPropagator,
    IntModPropagator, IntTimesPropagator, InversePropagator, LessEqualPropagator,
    LessThanPropagator, LinearEqPropagator, LinearScalarGePropagator, LinearScalarLePropagator,
    NotEqualOffsetPropagator, RectangleSpec, RegularPropagator, ReifiedEqualityPropagator,
    ReifiedFloatLinearEqPropagator, ReifiedFloatLinearGePropagator, ReifiedFloatLinearLePropagator,
    ReifiedLessEqualPropagator, ReifiedLessThanPropagator, ReifiedNotEqualPropagator,
    ReifiedScalarEqPropagator, ReifiedScalarGePropagator, ReifiedScalarLePropagator,
    SetCardEqPropagator, SetCardPropagator, SetDiffPropagator, SetEqPropagator,
    SetEqReifPropagator, SetInPropagator, SetInReifPropagator, SetIntersectPropagator, SetLexOp,
    SetLexPropagator, SetLexReifPropagator, SetLtPropagator, SetNePropagator, SetSubsetPropagator,
    SetSubsetReifPropagator, SetSymDiffPropagator, SetUnionPropagator, TablePropagator, TaskSpec,
};
use propaga_search::{
    DepthFirstSearch, LargeNeighborhoodSearch, LexicographicOptimization, LexicographicResult,
    LnsConfig, Objective, ObjectiveDirection, OptimizationSearch, OptimizationTarget,
    ParetoOptimization, ParetoResult, PortfolioConfig, PortfolioSearch, SearchConfig, SearchPhase,
    SearchStats, Solution,
};

/// High-level modeling facade over the Propaga engine.
pub struct Model {
    engine: Engine,
    variables: Vec<VariableId>,
    search_config: SearchConfig,
    search_phases: Vec<SearchPhase>,
}

impl Model {
    /// Creates an empty model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            variables: Vec::new(),
            search_config: SearchConfig::default(),
            search_phases: Vec::new(),
        }
    }

    /// Returns the underlying engine for advanced use.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Returns a mutable reference to the underlying engine.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Sets the search configuration used by [`Self::solve`] helpers.
    pub fn set_search_config(&mut self, config: SearchConfig) {
        self.search_config = config;
    }

    /// Sets sequenced search phases used by DFS (`seq_search` groups).
    pub fn set_search_phases(&mut self, phases: impl Into<Vec<SearchPhase>>) {
        self.search_phases = phases.into();
    }

    /// Returns the active search configuration.
    #[must_use]
    pub fn search_config(&self) -> SearchConfig {
        self.search_config
    }

    /// Returns sequenced search phases, if any.
    #[must_use]
    pub fn search_phases(&self) -> &[SearchPhase] {
        &self.search_phases
    }

    fn dfs(&self, variables: impl Into<Vec<VariableId>>) -> DepthFirstSearch {
        DepthFirstSearch::with_config(variables, self.search_config)
            .with_search_phases(self.search_phases.clone())
    }

    /// Returns all decision variables declared through the modeling API.
    #[must_use]
    pub fn decision_variables(&self) -> &[VariableId] {
        &self.variables
    }

    /// Declares an integer variable with inclusive bounds and returns its handle.
    pub fn int_var(&mut self, min: i32, max: i32) -> VariableId {
        self.declare_int_var(min, max, true)
    }

    /// Declares an auxiliary integer variable that is not a search decision variable.
    pub fn int_var_aux(&mut self, min: i32, max: i32) -> VariableId {
        self.declare_int_var(min, max, false)
    }

    fn declare_int_var(&mut self, min: i32, max: i32, decision: bool) -> VariableId {
        let var = self.engine.new_variable(HybridDomain::new(min, max));
        if decision {
            self.variables.push(var);
        }
        var
    }

    /// Declares a fixed integer variable.
    ///
    /// Fixed values participate in the engine but are not search decisions.
    pub fn int_var_fixed(&mut self, value: i32) -> VariableId {
        self.engine.new_variable(HybridDomain::fix(value))
    }

    /// Declares a variable from an explicit interval domain.
    pub fn int_var_domain(&mut self, domain: IntervalDomain) -> VariableId {
        let var = self.engine.new_variable(domain);
        self.variables.push(var);
        var
    }

    /// Declares a set variable over `[low, high]` with cardinality bounds.
    pub fn set_var(&mut self, low: i32, high: i32, card_min: usize, card_max: usize) -> VariableId {
        self.declare_set_var(low, high, card_min, card_max, true)
    }

    /// Declares an auxiliary set variable that is not a search decision variable.
    ///
    /// Used by FlatZinc decompositions (`set_diff`, `set_symdiff`, …) so cover/empty
    /// auxiliaries are fixed by propagation rather than branched on.
    pub fn set_var_aux(
        &mut self,
        low: i32,
        high: i32,
        card_min: usize,
        card_max: usize,
    ) -> VariableId {
        self.declare_set_var(low, high, card_min, card_max, false)
    }

    fn declare_set_var(
        &mut self,
        low: i32,
        high: i32,
        card_min: usize,
        card_max: usize,
        decision: bool,
    ) -> VariableId {
        let domain = SetIntervalDomain::universe(low..=high).with_cardinality(card_min, card_max);
        let var = self.engine.new_variable(AnyDomain::Set(domain));
        if decision {
            self.variables.push(var);
        }
        var
    }

    /// Declares a float variable with inclusive bounds.
    pub fn float_var(&mut self, min: f64, max: f64) -> VariableId {
        self.declare_float_var(min, max, true)
    }

    /// Declares an auxiliary float variable that is not a search decision variable.
    pub fn float_var_aux(&mut self, min: f64, max: f64) -> VariableId {
        self.declare_float_var(min, max, false)
    }

    fn declare_float_var(&mut self, min: f64, max: f64, decision: bool) -> VariableId {
        let var = self
            .engine
            .new_variable(AnyDomain::Float(FloatDomain::new(min, max)));
        if decision {
            self.variables.push(var);
        }
        var
    }

    /// Posts set cardinality propagation on `set`.
    pub fn set_card(&mut self, set: VariableId) {
        self.engine
            .add_propagator(Box::new(SetCardPropagator::new(set)));
    }

    /// Tightens set cardinality bounds before posting propagation.
    pub fn constrain_set_cardinality(&mut self, set: VariableId, card_min: usize, card_max: usize) {
        if let Some(domain) = self.engine.domain(set).as_set().cloned() {
            let updated = domain.with_cardinality(card_min, card_max);
            self.engine.set_domain(set, AnyDomain::Set(updated));
        }
        self.set_card(set);
    }

    /// Posts `|set| = card` for a set variable and an integer cardinality variable.
    pub fn set_card_eq(&mut self, set: VariableId, card: VariableId) {
        self.engine
            .add_propagator(Box::new(SetCardEqPropagator::new(set, card)));
    }

    /// Posts `subset ⊆ superset`.
    pub fn set_subset(&mut self, subset: VariableId, superset: VariableId) {
        self.engine
            .add_propagator(Box::new(SetSubsetPropagator::new(subset, superset)));
    }

    /// Posts `left == right` for set variables.
    pub fn set_eq(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(SetEqPropagator::new(left, right)));
    }

    /// Posts `left != right` for set variables.
    pub fn set_ne(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(SetNePropagator::new(left, right)));
    }

    /// Posts `left ⊂ right` (proper subset) for set variables.
    pub fn set_lt(&mut self, left: VariableId, right: VariableId) {
        self.set_subset(left, right);
        self.engine
            .add_propagator(Box::new(SetLtPropagator::new(left, right)));
    }

    /// Posts MiniZinc/FlatZinc `set_le`: sorted-list lexicographic `left ≤ right`.
    pub fn set_lex_le(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Le)));
    }

    /// Posts MiniZinc/FlatZinc `set_lt`: sorted-list lexicographic `left < right`.
    pub fn set_lex_lt(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(SetLexPropagator::new(left, right, SetLexOp::Lt)));
    }

    /// Posts `reif <=> left ≤_lex right`.
    pub fn set_lex_le_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(SetLexReifPropagator::new(
                left,
                right,
                reif,
                SetLexOp::Le,
            )));
    }

    /// Posts `reif <=> left <_lex right`.
    pub fn set_lex_lt_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(SetLexReifPropagator::new(
                left,
                right,
                reif,
                SetLexOp::Lt,
            )));
    }

    /// Posts `value ∈ set`.
    pub fn set_member(&mut self, value: VariableId, set: VariableId) {
        self.engine
            .add_propagator(Box::new(SetInPropagator::new(value, set)));
    }

    /// Declares a fixed set variable with exactly `values`.
    pub fn set_var_fixed_values(&mut self, values: &[i32]) -> VariableId {
        if values.is_empty() {
            return self.set_var(0, 0, 0, 0);
        }
        let low = *values.iter().min().unwrap();
        let high = *values.iter().max().unwrap();
        let var = self.set_var(low, high, values.len(), values.len());
        for &value in values {
            let _ = self.engine.force_set_in(var, value);
        }
        var
    }

    /// Posts `reif <=> value ∈ set`.
    pub fn set_member_reif(&mut self, value: VariableId, set: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(SetInReifPropagator::new(value, set, reif)));
    }

    /// Posts `reif <=> left == right` for set variables.
    pub fn set_eq_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(SetEqReifPropagator::new(left, right, reif)));
    }

    /// Posts `reif <=> subset ⊆ superset`.
    pub fn set_subset_reif(&mut self, subset: VariableId, superset: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(SetSubsetReifPropagator::new(
                subset, superset, reif,
            )));
    }

    /// Posts `left <= right` for float variables.
    pub fn float_le(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatLePropagator::new(left, right)));
    }

    /// Posts `left == right` for float variables.
    pub fn float_eq(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatEqPropagator::new(left, right)));
    }

    /// Posts `c = min(a, b)` for float variables.
    pub fn float_min(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatMinMaxPropagator::new(
                a,
                b,
                c,
                FloatMinMaxOp::Min,
            )));
    }

    /// Posts `c = max(a, b)` for float variables.
    pub fn float_max(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatMinMaxPropagator::new(
                a,
                b,
                c,
                FloatMinMaxOp::Max,
            )));
    }

    /// Posts `left != right` for float variables.
    pub fn float_ne(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatNePropagator::new(left, right)));
    }

    /// Posts `left < right` for float variables (strict).
    pub fn float_lt(&mut self, left: VariableId, right: VariableId) {
        let zero = self.int_var_fixed(0);
        // ¬(right ≤ left) ⇔ left < right
        self.float_le_reif(right, left, zero);
    }

    /// Posts `reif <=> left == right` for float variables.
    pub fn float_eq_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatEqReifPropagator::new(left, right, reif)));
    }

    /// Posts `reif <=> left <= right` for float variables.
    pub fn float_le_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatLeReifPropagator::new(left, right, reif)));
    }

    /// Posts `reif <=> left < right` for float variables.
    pub fn float_lt_reif(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatLtReifPropagator::new(left, right, reif)));
    }

    /// Posts `result = left ∪ right`.
    pub fn set_union(&mut self, left: VariableId, right: VariableId, result: VariableId) {
        self.engine
            .add_propagator(Box::new(SetUnionPropagator::new(left, right, result)));
    }

    /// Posts `result = left ∩ right`.
    pub fn set_intersect(&mut self, left: VariableId, right: VariableId, result: VariableId) {
        self.engine
            .add_propagator(Box::new(SetIntersectPropagator::new(left, right, result)));
    }

    /// Posts `result = left \\ right`.
    pub fn set_diff(&mut self, left: VariableId, right: VariableId, result: VariableId) {
        self.engine
            .add_propagator(Box::new(SetDiffPropagator::new(left, right, result)));
    }

    /// Posts `result = left △ right`.
    pub fn set_symdiff(&mut self, left: VariableId, right: VariableId, result: VariableId) {
        self.engine
            .add_propagator(Box::new(SetSymDiffPropagator::new(left, right, result)));
    }

    /// Posts `c = a * b` for float variables.
    pub fn float_times(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
    }

    /// Posts `c = a + b` for float variables.
    pub fn float_plus(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatBinaryPropagator::new(
                a,
                b,
                c,
                FloatBinaryOp::Plus,
            )));
    }

    /// Posts `c = a / b` for float variables.
    pub fn float_div(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatBinaryPropagator::new(
                a,
                b,
                c,
                FloatBinaryOp::Div,
            )));
    }

    /// Posts `b = unary(a)` for float variables.
    pub fn float_unary(&mut self, input: VariableId, output: VariableId, op: FloatUnaryOp) {
        self.engine
            .add_propagator(Box::new(FloatUnaryPropagator::new(input, output, op)));
    }

    /// Posts `float = int` channeling.
    pub fn int2float(&mut self, int_var: VariableId, float_var: VariableId) {
        self.engine
            .add_propagator(Box::new(Int2FloatPropagator::new(int_var, float_var)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) <= rhs` for float variables.
    pub fn float_scalar_le(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
    ) {
        self.engine
            .add_propagator(Box::new(FloatLinearLePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) >= rhs` for float variables.
    pub fn float_scalar_ge(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
    ) {
        self.engine
            .add_propagator(Box::new(FloatLinearGePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) != rhs` for float variables.
    pub fn float_scalar_ne(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
    ) {
        self.engine
            .add_propagator(Box::new(FloatLinearNePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) == rhs` for float variables.
    pub fn float_scalar_eq(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
    ) {
        self.engine
            .add_propagator(Box::new(FloatLinearEqPropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `reif <=> sum(coeffs[i] * vars[i]) <= rhs` for float variables.
    pub fn reified_float_scalar_le(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedFloatLinearLePropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts `reif <=> sum(coeffs[i] * vars[i]) == rhs` for float variables.
    pub fn reified_float_scalar_eq(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedFloatLinearEqPropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts `reif <=> sum(coeffs[i] * vars[i]) >= rhs` for float variables.
    pub fn reified_float_scalar_ge(
        &mut self,
        coeffs: impl Into<Vec<f64>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: f64,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedFloatLinearGePropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts `left == right`.
    pub fn equal(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(EqualityPropagator::new(left, right)));
    }

    /// Posts `left + right == result`.
    pub fn linear_eq(&mut self, left: VariableId, right: VariableId, result: VariableId) {
        self.engine
            .add_propagator(Box::new(LinearEqPropagator::new(left, right, result)));
    }

    /// Posts `left <= right`.
    pub fn less_equal(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(LessEqualPropagator::new(left, right)));
    }

    /// Posts `c = min(a, b)` for integer variables.
    pub fn int_min(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(IntMinMaxPropagator::new(
                a,
                b,
                c,
                IntMinMaxOp::Min,
            )));
    }

    /// Posts `c = max(a, b)` for integer variables.
    pub fn int_max(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(IntMinMaxPropagator::new(
                a,
                b,
                c,
                IntMinMaxOp::Max,
            )));
    }

    /// Posts `b = |a|` for integer variables.
    pub fn int_abs(&mut self, a: VariableId, b: VariableId) {
        self.engine
            .add_propagator(Box::new(IntAbsPropagator::new(a, b)));
    }

    /// Posts `c = a * b` for integer variables.
    pub fn int_times(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(IntTimesPropagator::new(a, b, c)));
    }

    /// Posts `c = a / b` (trunc toward zero) for integer variables; excludes `b = 0`.
    pub fn int_div(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(IntDivPropagator::new(a, b, c)));
    }

    /// Posts `c = a mod b` (truncating remainder) for integer variables; excludes `b = 0`.
    pub fn int_mod(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(IntModPropagator::new(a, b, c)));
    }

    /// Posts `left < right`.
    pub fn less_than(&mut self, left: VariableId, right: VariableId) {
        self.engine
            .add_propagator(Box::new(LessThanPropagator::new(left, right)));
    }

    /// Posts `left >= right`.
    pub fn greater_equal(&mut self, left: VariableId, right: VariableId) {
        self.less_equal(right, left);
    }

    /// Posts `left > right`.
    pub fn greater_than(&mut self, left: VariableId, right: VariableId) {
        self.less_than(right, left);
    }

    /// Posts `left != right + offset`.
    pub fn not_equal_offset(&mut self, left: VariableId, right: VariableId, offset: i32) {
        self.engine
            .add_propagator(Box::new(NotEqualOffsetPropagator::new(left, right, offset)));
    }

    /// Posts `reif == 1 <=> left == right`.
    pub fn reified_equal(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(ReifiedEqualityPropagator::new(left, right, reif)));
    }

    /// Posts `reif == 1 <=> left != right`.
    pub fn reified_not_equal(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(ReifiedNotEqualPropagator::new(left, right, reif)));
    }

    /// Posts `reif == 1 <=> left <= right`.
    pub fn reified_less_equal(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(ReifiedLessEqualPropagator::new(left, right, reif)));
    }

    /// Posts `reif == 1 <=> left < right`.
    pub fn reified_less_than(&mut self, left: VariableId, right: VariableId, reif: VariableId) {
        self.engine
            .add_propagator(Box::new(ReifiedLessThanPropagator::new(left, right, reif)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) <= rhs`.
    pub fn scalar_le(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
    ) {
        self.engine
            .add_propagator(Box::new(LinearScalarLePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) >= rhs`.
    pub fn scalar_ge(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
    ) {
        self.engine
            .add_propagator(Box::new(LinearScalarGePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `sum(coeffs[i] * vars[i]) == rhs`.
    pub fn scalar_eq(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
    ) {
        let coeffs = coeffs.into();
        let vars = vars.into();
        self.engine
            .add_propagator(Box::new(LinearScalarLePropagator::new(
                coeffs.clone(),
                vars.clone(),
                rhs,
            )));
        self.engine
            .add_propagator(Box::new(LinearScalarGePropagator::new(coeffs, vars, rhs)));
    }

    /// Posts `reif == 1 <=> sum(coeffs[i] * vars[i]) <= rhs`.
    pub fn reified_scalar_le(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedScalarLePropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts `reif == 1 <=> sum(coeffs[i] * vars[i]) >= rhs`.
    pub fn reified_scalar_ge(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedScalarGePropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts `reif == 1 <=> sum(coeffs[i] * vars[i]) == rhs`.
    pub fn reified_scalar_eq(
        &mut self,
        coeffs: impl Into<Vec<i32>>,
        vars: impl Into<Vec<VariableId>>,
        rhs: i32,
        reif: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ReifiedScalarEqPropagator::new(
                coeffs, vars, rhs, reif,
            )));
    }

    /// Posts an all-different constraint over `variables`.
    pub fn all_different(&mut self, variables: impl Into<Vec<VariableId>>) {
        self.engine
            .add_propagator(Box::new(AllDifferentPropagator::new(variables)));
    }

    /// Posts a global cardinality constraint with per-value bounds.
    pub fn gcc(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        cards: impl IntoIterator<Item = (i32, CardinalityBound)>,
    ) {
        self.engine
            .add_propagator(Box::new(GlobalCardinalityPropagator::new(variables, cards)));
    }

    /// Posts a table constraint allowing only the given `tuples`.
    pub fn table(&mut self, variables: impl Into<Vec<VariableId>>, tuples: Vec<Vec<i32>>) {
        self.engine
            .add_propagator(Box::new(TablePropagator::new(variables, tuples)));
    }

    /// Posts `value == array[index]`.
    pub fn element(
        &mut self,
        index: VariableId,
        array: impl Into<Vec<VariableId>>,
        value: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(ElementPropagator::new(index, array, value)));
    }

    /// Posts `value == array[index]` for float variables (0-based index).
    pub fn float_element(
        &mut self,
        index: VariableId,
        array: impl Into<Vec<VariableId>>,
        value: VariableId,
    ) {
        self.engine
            .add_propagator(Box::new(FloatElementPropagator::new(index, array, value)));
    }

    /// Posts a cumulative scheduling constraint over `tasks`.
    pub fn cumulative(&mut self, tasks: impl Into<Vec<TaskSpec>>, capacity: i32) {
        self.engine
            .add_propagator(Box::new(CumulativePropagator::new(tasks, capacity)));
    }

    /// Posts a cumulative constraint with a variable resource capacity.
    pub fn cumulative_var(&mut self, tasks: impl Into<Vec<TaskSpec>>, capacity: VariableId) {
        self.engine
            .add_propagator(Box::new(CumulativePropagator::with_capacity_var(
                tasks, capacity,
            )));
    }

    /// Posts a disjunctive (single-machine) constraint over `tasks`.
    pub fn disjunctive(&mut self, tasks: impl Into<Vec<DisjunctiveTask>>) {
        self.engine
            .add_propagator(Box::new(DisjunctivePropagator::new(tasks)));
    }

    /// Posts a Hamiltonian circuit over successor variables.
    pub fn circuit(&mut self, successors: impl Into<Vec<VariableId>>) {
        self.engine
            .add_propagator(Box::new(CircuitPropagator::new(successors.into())));
    }

    /// Posts `inverse(forward, backward)`.
    pub fn inverse(
        &mut self,
        forward: impl Into<Vec<VariableId>>,
        backward: impl Into<Vec<VariableId>>,
    ) {
        self.engine.add_propagator(Box::new(InversePropagator::new(
            forward.into(),
            backward.into(),
        )));
    }

    /// Posts a `diffn` non-overlap constraint over rectangles.
    pub fn diffn(&mut self, rectangles: impl Into<Vec<RectangleSpec>>) {
        self.engine
            .add_propagator(Box::new(DiffnPropagator::new(rectangles.into())));
    }

    /// Posts a `regular` sequence constraint.
    pub fn regular(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        num_states: usize,
        transitions: Vec<Vec<i32>>,
        start_state: i32,
        accepting: impl Into<Vec<i32>>,
    ) {
        self.engine.add_propagator(Box::new(RegularPropagator::new(
            variables.into(),
            num_states,
            transitions,
            start_state,
            &accepting.into(),
        )));
    }

    /// Runs propagation to fixpoint.
    pub fn propagate(&mut self) -> Result<PropagationStatus, propaga_core::PropagaError> {
        self.engine.propagate_all()
    }

    /// Solves the model using depth-first search with MRV.
    pub fn solve(&mut self) -> Option<Solution> {
        let mut search = self.dfs(self.variables.clone());
        search.solve(&mut self.engine)
    }

    /// Solves while tracking only the provided decision variables.
    pub fn solve_subset(&mut self, variables: impl Into<Vec<VariableId>>) -> Option<Solution> {
        let mut search = self.dfs(variables);
        search.solve(&mut self.engine)
    }

    /// Solves and returns search statistics.
    pub fn solve_with_stats(&mut self) -> (Option<Solution>, SearchStats) {
        let mut search = self.dfs(self.variables.clone());
        let solution = search.solve(&mut self.engine);
        (solution, search.stats())
    }

    /// Solves a variable subset and returns search statistics.
    pub fn solve_subset_with_stats(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
    ) -> (Option<Solution>, SearchStats) {
        let mut search = self.dfs(variables);
        let solution = search.solve(&mut self.engine);
        (solution, search.stats())
    }

    /// Returns all solutions using exhaustive DFS.
    pub fn solve_all(&mut self, variables: impl Into<Vec<VariableId>>) -> Vec<Solution> {
        self.solve_all_limited(variables, None)
    }

    /// Returns up to `limit` solutions using exhaustive DFS.
    pub fn solve_all_limited(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        limit: Option<usize>,
    ) -> Vec<Solution> {
        let mut search = DepthFirstSearch::with_config(
            variables,
            SearchConfig {
                restart_policy: propaga_search::RestartPolicy::None,
                ..self.search_config
            },
        )
        .with_search_phases(self.search_phases.clone());
        search.solve_all_limited(&mut self.engine, limit)
    }

    /// Returns all solutions with search statistics.
    pub fn solve_all_with_stats(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
    ) -> (Vec<Solution>, SearchStats) {
        self.solve_all_with_stats_limited(variables, None)
    }

    /// Returns up to `limit` solutions with search statistics.
    pub fn solve_all_with_stats_limited(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        limit: Option<usize>,
    ) -> (Vec<Solution>, SearchStats) {
        let mut search = DepthFirstSearch::with_config(
            variables,
            SearchConfig {
                restart_policy: propaga_search::RestartPolicy::None,
                ..self.search_config
            },
        )
        .with_search_phases(self.search_phases.clone());
        let solutions = search.solve_all_limited(&mut self.engine, limit);
        (solutions, search.stats())
    }

    /// Optimizes a single objective using branch-and-bound.
    pub fn optimize_objective(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
    ) -> (
        Option<Solution>,
        Option<propaga_search::ObjectiveValue>,
        SearchStats,
        u32,
    ) {
        let mut search =
            OptimizationSearch::with_target(variables, target, direction, self.search_config)
                .with_search_phases(self.search_phases.clone());
        let result = search.optimize(&mut self.engine);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
    }

    /// Branch-and-bound seeded with a warm-start assignment when feasible.
    pub fn optimize_objective_with_hint(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
        hint: Solution,
    ) -> (
        Option<Solution>,
        Option<propaga_search::ObjectiveValue>,
        SearchStats,
        u32,
    ) {
        let mut search =
            OptimizationSearch::with_target(variables, target, direction, self.search_config)
                .with_search_phases(self.search_phases.clone())
                .with_hint(hint);
        let result = search.optimize(&mut self.engine);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
    }

    /// Large-neighborhood search for a single objective (optional warm-start hint).
    pub fn optimize_objective_lns(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        target: OptimizationTarget,
        direction: ObjectiveDirection,
        lns: LnsConfig,
        hint: Option<Solution>,
    ) -> (
        Option<Solution>,
        Option<propaga_search::ObjectiveValue>,
        SearchStats,
        u32,
    ) {
        let mut search =
            LargeNeighborhoodSearch::new(variables, target, direction, self.search_config, lns)
                .with_search_phases(self.search_phases.clone());
        if let Some(hint) = hint {
            search = search.with_hint(hint);
        }
        let result = search.optimize(&mut self.engine);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
    }

    /// Portfolio branch-and-bound over diversified search configurations.
    pub fn optimize_objective_portfolio(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        target: propaga_search::OptimizationTarget,
        direction: propaga_search::ObjectiveDirection,
        portfolio: PortfolioConfig,
    ) -> (
        Option<Solution>,
        Option<propaga_search::ObjectiveValue>,
        SearchStats,
        u32,
    ) {
        let search = PortfolioSearch::new(variables, self.search_config, portfolio)
            .with_search_phases(self.search_phases.clone());
        let result = search.optimize(&mut self.engine, target, direction);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
    }

    /// Optimizes a single integer objective using branch-and-bound.
    pub fn optimize(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objective: VariableId,
        direction: propaga_search::ObjectiveDirection,
    ) -> (Option<Solution>, Option<i32>, SearchStats, u32) {
        let (solution, value, stats, solutions_found) = self.optimize_objective(
            variables,
            propaga_search::OptimizationTarget::Int(objective),
            direction,
        );
        (
            solution,
            value.and_then(|value| value.as_int()),
            stats,
            solutions_found,
        )
    }

    /// Solves using a portfolio of search configurations.
    pub fn solve_portfolio(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        portfolio: PortfolioConfig,
    ) -> (Option<Solution>, SearchStats) {
        let search = PortfolioSearch::new(variables, self.search_config, portfolio)
            .with_search_phases(self.search_phases.clone());
        search.solve(&mut self.engine)
    }

    /// Optimizes multiple objectives lexicographically.
    pub fn optimize_lexicographic(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<Objective>,
    ) -> LexicographicResult {
        let mut search = LexicographicOptimization::new(variables, objectives, self.search_config)
            .with_search_phases(self.search_phases.clone());
        search.optimize(&mut self.engine)
    }

    /// Portfolio lexicographic optimization over diversified search configurations.
    pub fn optimize_lexicographic_portfolio(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<Objective>,
        portfolio: PortfolioConfig,
    ) -> LexicographicResult {
        let search = PortfolioSearch::new(variables, self.search_config, portfolio)
            .with_search_phases(self.search_phases.clone());
        search.optimize_lexicographic(&mut self.engine, objectives)
    }

    /// Enumerates the Pareto front for multiple objectives.
    pub fn pareto_optimize(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(propaga_search::OptimizationTarget, ObjectiveDirection)>,
    ) -> ParetoResult {
        let _ = self.propagate();
        let mut search = ParetoOptimization::new(variables, objectives, self.search_config)
            .with_search_phases(self.search_phases.clone());
        search.optimize(&mut self.engine)
    }

    /// Portfolio Pareto enumeration; worker fronts are merged with dominance filtering.
    pub fn pareto_optimize_portfolio(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(propaga_search::OptimizationTarget, ObjectiveDirection)>,
        portfolio: PortfolioConfig,
    ) -> ParetoResult {
        let search = PortfolioSearch::new(variables, self.search_config, portfolio)
            .with_search_phases(self.search_phases.clone());
        search.optimize_pareto(&mut self.engine, objectives)
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_search::{AssignmentValue, ObjectiveValue};

    #[test]
    fn solves_simple_equality() {
        let mut model = Model::new();
        let left = model.int_var(1, 5);
        let right = model.int_var(1, 10);
        model.equal(left, right);
        model.engine_mut().fix_variable(left, 3).unwrap();
        model.propagate().unwrap();
        assert_eq!(model.engine().hybrid_domain(right).fixed_value(), Some(3));
    }

    #[test]
    fn optimize_with_hint_reaches_optimum() {
        let mut model = Model::new();
        let x = model.int_var(0, 10);
        let y = model.int_var(0, 10);
        model.scalar_le([1, 1], [x, y], 10);
        let hint = vec![(x, AssignmentValue::Int(3)), (y, AssignmentValue::Int(0))];
        let (_sol, value, _stats, found) = model.optimize_objective_with_hint(
            vec![x, y],
            OptimizationTarget::Int(x),
            ObjectiveDirection::Maximize,
            hint,
        );
        assert!(found >= 1);
        assert_eq!(value, Some(ObjectiveValue::Int(10)));
    }

    #[test]
    fn optimize_lns_from_hint_reaches_optimum() {
        let mut model = Model::new();
        let x = model.int_var(0, 10);
        let y = model.int_var(0, 10);
        model.scalar_le([1, 1], [x, y], 10);
        let hint = vec![(x, AssignmentValue::Int(3)), (y, AssignmentValue::Int(0))];
        let (_sol, value, _stats, _) = model.optimize_objective_lns(
            vec![x, y],
            OptimizationTarget::Int(x),
            ObjectiveDirection::Maximize,
            LnsConfig {
                iterations: 8,
                destroy_fraction: 0.5,
                seed: 7,
            },
            Some(hint),
        );
        assert_eq!(value, Some(ObjectiveValue::Int(10)));
    }
}
