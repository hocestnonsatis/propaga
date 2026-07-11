use propaga_core::{PropagationStatus, VariableId};
use propaga_domains::{AnyDomain, FloatDomain, HybridDomain, IntervalDomain, SetIntervalDomain};
use propaga_engine::Engine;
use propaga_propagators::{
    AllDifferentPropagator, CardinalityBound, CircuitPropagator, CumulativePropagator,
    DiffnPropagator, DisjunctivePropagator, DisjunctiveTask, ElementPropagator, EqualityPropagator,
    FloatEqPropagator, FloatLePropagator, FloatTimesPropagator, GlobalCardinalityPropagator,
    InversePropagator, LessEqualPropagator, LessThanPropagator, LinearEqPropagator,
    LinearScalarGePropagator, LinearScalarLePropagator, NotEqualOffsetPropagator, RectangleSpec,
    RegularPropagator, ReifiedEqualityPropagator, ReifiedLessEqualPropagator,
    ReifiedLessThanPropagator, ReifiedNotEqualPropagator, ReifiedScalarEqPropagator,
    ReifiedScalarGePropagator, ReifiedScalarLePropagator, SetCardPropagator,
    SetIntersectPropagator, SetSubsetPropagator, SetUnionPropagator, TablePropagator, TaskSpec,
};
use propaga_search::{
    DepthFirstSearch, LexicographicOptimization, LexicographicResult, Objective,
    ObjectiveDirection, ParetoOptimization, ParetoResult, PortfolioConfig, PortfolioSearch,
    SearchConfig, SearchStats, Solution,
};

/// High-level modeling facade over the Propaga engine.
pub struct Model {
    engine: Engine,
    variables: Vec<VariableId>,
    search_config: SearchConfig,
}

impl Model {
    /// Creates an empty model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            variables: Vec::new(),
            search_config: SearchConfig::default(),
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

    /// Returns the active search configuration.
    #[must_use]
    pub fn search_config(&self) -> SearchConfig {
        self.search_config
    }

    /// Returns all decision variables declared through the modeling API.
    #[must_use]
    pub fn decision_variables(&self) -> &[VariableId] {
        &self.variables
    }

    /// Declares an integer variable with inclusive bounds and returns its handle.
    pub fn int_var(&mut self, min: i32, max: i32) -> VariableId {
        let var = self.engine.new_variable(HybridDomain::new(min, max));
        self.variables.push(var);
        var
    }

    /// Declares a fixed integer variable.
    pub fn int_var_fixed(&mut self, value: i32) -> VariableId {
        let var = self.engine.new_variable(HybridDomain::fix(value));
        self.variables.push(var);
        var
    }

    /// Declares a variable from an explicit interval domain.
    pub fn int_var_domain(&mut self, domain: IntervalDomain) -> VariableId {
        let var = self.engine.new_variable(domain);
        self.variables.push(var);
        var
    }

    /// Declares a set variable over `[low, high]` with cardinality bounds.
    pub fn set_var(&mut self, low: i32, high: i32, card_min: usize, card_max: usize) -> VariableId {
        let domain = SetIntervalDomain::universe(low..=high).with_cardinality(card_min, card_max);
        let var = self.engine.new_variable(AnyDomain::Set(domain));
        self.variables.push(var);
        var
    }

    /// Declares a float variable with inclusive bounds.
    pub fn float_var(&mut self, min: f64, max: f64) -> VariableId {
        let var = self
            .engine
            .new_variable(AnyDomain::Float(FloatDomain::new(min, max)));
        self.variables.push(var);
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

    /// Posts `subset ⊆ superset`.
    pub fn set_subset(&mut self, subset: VariableId, superset: VariableId) {
        self.engine
            .add_propagator(Box::new(SetSubsetPropagator::new(subset, superset)));
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

    /// Posts `c = a * b` for float variables.
    pub fn float_times(&mut self, a: VariableId, b: VariableId, c: VariableId) {
        self.engine
            .add_propagator(Box::new(FloatTimesPropagator::new(a, b, c)));
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

    /// Posts a cumulative scheduling constraint over `tasks`.
    pub fn cumulative(&mut self, tasks: impl Into<Vec<TaskSpec>>, capacity: i32) {
        self.engine
            .add_propagator(Box::new(CumulativePropagator::new(tasks, capacity)));
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
        let mut search = DepthFirstSearch::with_config(self.variables.clone(), self.search_config);
        search.solve(&mut self.engine)
    }

    /// Solves while tracking only the provided decision variables.
    pub fn solve_subset(&mut self, variables: impl Into<Vec<VariableId>>) -> Option<Solution> {
        let mut search = DepthFirstSearch::with_config(variables, self.search_config);
        search.solve(&mut self.engine)
    }

    /// Solves and returns search statistics.
    pub fn solve_with_stats(&mut self) -> (Option<Solution>, SearchStats) {
        let mut search = DepthFirstSearch::with_config(self.variables.clone(), self.search_config);
        let solution = search.solve(&mut self.engine);
        (solution, search.stats())
    }

    /// Solves a variable subset and returns search statistics.
    pub fn solve_subset_with_stats(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
    ) -> (Option<Solution>, SearchStats) {
        let mut search = DepthFirstSearch::with_config(variables, self.search_config);
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
        );
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
        );
        let solutions = search.solve_all_limited(&mut self.engine, limit);
        (solutions, search.stats())
    }

    /// Optimizes a single integer objective using branch-and-bound.
    pub fn optimize(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objective: VariableId,
        direction: propaga_search::ObjectiveDirection,
    ) -> (Option<Solution>, Option<i32>, SearchStats, u32) {
        let mut search = propaga_search::OptimizationSearch::new(
            variables,
            objective,
            direction,
            self.search_config,
        );
        let result = search.optimize(&mut self.engine);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
    }

    /// Solves using a portfolio of search configurations.
    pub fn solve_portfolio(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        portfolio: PortfolioConfig,
    ) -> (Option<Solution>, SearchStats) {
        let search = PortfolioSearch::new(variables, self.search_config, portfolio);
        search.solve(&mut self.engine)
    }

    /// Optimizes multiple objectives lexicographically.
    pub fn optimize_lexicographic(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<Objective>,
    ) -> LexicographicResult {
        let mut search = LexicographicOptimization::new(variables, objectives, self.search_config);
        search.optimize(&mut self.engine)
    }

    /// Enumerates the Pareto front for multiple objectives.
    pub fn pareto_optimize(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(VariableId, ObjectiveDirection)>,
    ) -> ParetoResult {
        let _ = self.propagate();
        let mut search = ParetoOptimization::new(variables, objectives, self.search_config);
        search.optimize(&mut self.engine)
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
}
