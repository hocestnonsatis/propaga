use propaga_core::{PropagationStatus, VariableId};
use propaga_domains::{HybridDomain, IntervalDomain};
use propaga_engine::Engine;
use propaga_propagators::{
    AllDifferentPropagator, CardinalityBound, CumulativePropagator, DisjunctivePropagator,
    DisjunctiveTask, ElementPropagator, EqualityPropagator, GlobalCardinalityPropagator,
    LessEqualPropagator, LessThanPropagator, LinearEqPropagator, LinearScalarGePropagator,
    LinearScalarLePropagator, NotEqualOffsetPropagator, ReifiedEqualityPropagator,
    ReifiedLessEqualPropagator, ReifiedLessThanPropagator, ReifiedNotEqualPropagator,
    ReifiedScalarEqPropagator, ReifiedScalarGePropagator, ReifiedScalarLePropagator,
    TablePropagator, TaskSpec,
};
use propaga_search::{
    DepthFirstSearch, ObjectiveDirection, OptimizationSearch, SearchConfig, SearchStats, Solution,
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
        direction: ObjectiveDirection,
    ) -> (Option<Solution>, Option<i32>, SearchStats, u32) {
        let mut search =
            OptimizationSearch::new(variables, objective, direction, self.search_config);
        let result = search.optimize(&mut self.engine);
        (
            result.solution,
            result.objective_value,
            result.stats,
            result.solutions_found,
        )
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
        assert_eq!(model.engine().domain(right).fixed_value(), Some(3));
    }
}
