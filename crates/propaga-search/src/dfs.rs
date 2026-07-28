use crate::config::{SearchConfig, SearchPhase};
use crate::conflict::{ConflictAnalyzer, NogoodStore};
use crate::lcg::ClauseStore;
use crate::optimize::{next_float_down, next_float_up};
use crate::stats::{SearchStats, branch_assignments_from_explanation};
use crate::value::{AssignmentValue, Solution};
use propaga_core::{DomainView, NogoodLiteral, PropagationStatus, VariableId};
use propaga_domains::DomainKind;
use propaga_engine::Engine;
use propaga_propagators::ClausePropagator;
use propaga_propagators::NogoodPropagator;
use std::collections::HashMap;
use std::time::Instant;

/// Depth-first search with MRV, nogood learning, and optional restarts.
pub struct DepthFirstSearch {
    variables: Vec<VariableId>,
    config: SearchConfig,
    search_phases: Vec<SearchPhase>,
    nogoods: NogoodStore,
    clauses: ClauseStore,
    stats: SearchStats,
    nodes_since_restart: u64,
    restart_index: u32,
    phases: HashMap<VariableId, i32>,
    weights: HashMap<VariableId, u32>,
    activities: HashMap<VariableId, u32>,
    pending_solution_restart: bool,
    deadline: Option<Instant>,
    /// IEEE points to prefer splitting around when branching on float domains.
    float_holes: HashMap<VariableId, Vec<f64>>,
}

impl DepthFirstSearch {
    /// Creates a DFS over the given decision variables with default config.
    #[must_use]
    pub fn new(variables: impl Into<Vec<VariableId>>) -> Self {
        Self::with_config(variables, SearchConfig::default())
    }

    /// Creates a DFS with explicit search configuration.
    #[must_use]
    pub fn with_config(variables: impl Into<Vec<VariableId>>, config: SearchConfig) -> Self {
        Self {
            variables: variables.into(),
            config,
            search_phases: Vec::new(),
            nogoods: NogoodStore::new(),
            clauses: ClauseStore::new(),
            stats: SearchStats::default(),
            nodes_since_restart: 0,
            restart_index: 0,
            phases: HashMap::new(),
            weights: HashMap::new(),
            activities: HashMap::new(),
            pending_solution_restart: false,
            deadline: None,
            float_holes: HashMap::new(),
        }
    }

    /// Creates a DFS with optional nogood learning and no restarts.
    #[must_use]
    pub fn with_learning(variables: impl Into<Vec<VariableId>>, learning: bool) -> Self {
        Self::with_config(
            variables,
            SearchConfig {
                learning,
                restart_policy: crate::config::RestartPolicy::None,
                ..SearchConfig::default()
            },
        )
    }

    /// Attaches sequenced search phases (`seq_search` groups).
    ///
    /// While any variable in an earlier phase is unfixed, later phases are ignored and
    /// that phase's variable/value orderings are used.
    #[must_use]
    pub fn with_search_phases(mut self, search_phases: impl Into<Vec<SearchPhase>>) -> Self {
        self.search_phases = search_phases.into();
        self
    }

    /// Registers IEEE points that float branching should split around when interior.
    #[must_use]
    pub fn with_float_holes(mut self, float_holes: HashMap<VariableId, Vec<f64>>) -> Self {
        self.float_holes = float_holes;
        self
    }

    /// Records a blocked float value to prefer as a split point.
    pub fn register_float_hole(&mut self, var: VariableId, value: f64) {
        let holes = self.float_holes.entry(var).or_default();
        if !holes
            .iter()
            .any(|hole| (*hole - value).abs() <= f64::EPSILON)
        {
            holes.push(value);
        }
    }

    /// Returns statistics from the most recent search.
    #[must_use]
    pub fn stats(&self) -> SearchStats {
        self.stats
    }

    /// Returns the number of learned nogoods.
    #[must_use]
    pub fn nogood_count(&self) -> usize {
        self.nogoods.len()
    }

    /// Returns the number of learned clauses.
    #[must_use]
    pub fn clause_count(&self) -> usize {
        self.clauses.clauses().len()
    }

    /// Searches for a solution, returning the first one found.
    pub fn solve(&mut self, engine: &mut Engine) -> Option<Solution> {
        self.begin_search();

        if !self.propagate_root(engine) {
            return None;
        }

        loop {
            if self.check_timeout() {
                return None;
            }

            if let Some(solution) = self.search(engine) {
                return Some(solution);
            }

            if self.stats.timed_out || !self.should_restart() {
                return None;
            }

            if matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            ) {
                return None;
            }

            self.perform_restart(engine);
            if !self.propagate_root(engine) {
                return None;
            }
        }
    }

    /// Returns all solutions found by exhaustive DFS, stopping after `limit` solutions.
    pub fn solve_all(&mut self, engine: &mut Engine) -> Vec<Solution> {
        self.solve_all_limited(engine, None)
    }

    /// Invokes `on_solution` for each feasible solution without retaining them all.
    ///
    /// Returns early when `on_solution` returns `false`.
    pub fn solve_each(
        &mut self,
        engine: &mut Engine,
        mut on_solution: impl FnMut(&Solution) -> bool,
    ) {
        self.begin_search();
        if self.propagate_root(engine) {
            self.collect_each(engine, &mut on_solution);
        }
    }

    /// Returns up to `limit` solutions found by exhaustive DFS.
    pub fn solve_all_limited(
        &mut self,
        engine: &mut Engine,
        limit: Option<usize>,
    ) -> Vec<Solution> {
        let mut solutions = Vec::new();
        self.solve_each(engine, |solution| {
            solutions.push(solution.clone());
            limit.is_none_or(|max| solutions.len() < max)
        });
        solutions
    }

    fn begin_search(&mut self) {
        self.stats = SearchStats::default();
        self.nodes_since_restart = 0;
        self.restart_index = 0;
        self.pending_solution_restart = false;
        self.deadline = self.config.time_limit.map(|limit| Instant::now() + limit);
    }

    fn check_timeout(&mut self) -> bool {
        if self.stats.timed_out {
            return true;
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.stats.timed_out = true;
            true
        } else {
            false
        }
    }

    fn propagate_root(&mut self, engine: &mut Engine) -> bool {
        match engine.commit_initial_propagation() {
            Ok(status) => !status.is_failure(),
            Err(_) => false,
        }
    }

    /// Searches without running root propagation (caller must commit root state first).
    pub fn solve_without_root_propagation(&mut self, engine: &mut Engine) -> Option<Solution> {
        self.begin_search();
        loop {
            if self.check_timeout() {
                return None;
            }

            if let Some(solution) = self.search(engine) {
                return Some(solution);
            }

            if self.stats.timed_out || !self.should_restart() {
                return None;
            }

            if matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            ) {
                return None;
            }

            self.perform_restart(engine);
        }
    }

    #[cfg(test)]
    fn solve_without_initial_propagation(&mut self, engine: &mut Engine) -> Option<Solution> {
        self.search(engine)
    }

    fn search(&mut self, engine: &mut Engine) -> Option<Solution> {
        if self.check_timeout() {
            return None;
        }

        if engine.is_solved() {
            if matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            ) {
                self.pending_solution_restart = true;
            }
            return Some(self.collect_solution(engine));
        }

        let assignment = branch_assignments_from_explanation(engine.explanation());
        if self.is_pruned(&assignment) {
            return None;
        }

        let var = self.select_variable(engine)?;
        if let Some(solution) = self.explore_variable(engine, var, &assignment) {
            return Some(solution);
        }

        None
    }

    fn explore_variable(
        &mut self,
        engine: &mut Engine,
        var: VariableId,
        assignment: &[(VariableId, i32)],
    ) -> Option<Solution> {
        match engine.domain(var).kind() {
            DomainKind::Int => self.explore_int(engine, var, assignment),
            DomainKind::Set => self.explore_set(engine, var),
            DomainKind::Float => self.explore_float(engine, var),
        }
    }

    fn explore_int(
        &mut self,
        engine: &mut Engine,
        var: VariableId,
        assignment: &[(VariableId, i32)],
    ) -> Option<Solution> {
        let values = self.ordered_values(engine, var);
        for value in values {
            if self.would_prune(assignment, var, value) {
                continue;
            }

            self.record_branch();
            let level = engine.trail_mark();
            self.record_phase(var, value);
            match engine.fix_variable(var, value) {
                Ok(PropagationStatus::Failure) => {
                    let jumped = self.handle_failure(engine, level);
                    if jumped {
                        return None;
                    }
                }
                Ok(_) => {
                    if let Some(solution) = self.search(engine) {
                        return Some(solution);
                    }
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }
        None
    }

    fn explore_set(&mut self, engine: &mut Engine, var: VariableId) -> Option<Solution> {
        let undecided = engine.domain(var).as_set()?.undecided();
        if undecided.is_empty() {
            return self.search(engine);
        }
        let ordering = self.active_value_ordering(engine);
        let value = choose_set_branch_element(&undecided, var, ordering);
        for force_in in set_membership_branch_order(ordering) {
            self.record_branch();
            let level = engine.trail_mark();
            let status = if force_in {
                engine.force_set_in(var, value)
            } else {
                engine.force_set_out(var, value)
            };
            match status {
                Ok(PropagationStatus::Failure) => {
                    let jumped = self.handle_failure(engine, level);
                    if jumped {
                        return None;
                    }
                }
                Ok(_) => {
                    if let Some(solution) = self.search(engine) {
                        return Some(solution);
                    }
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }

        None
    }

    fn explore_float(&mut self, engine: &mut Engine, var: VariableId) -> Option<Solution> {
        let float = engine.domain(var).as_float().cloned()?;
        if float.is_fixed() {
            return self.search(engine);
        }

        let width = float.upper_bound() - float.lower_bound();
        let precision = self.config.float_precision.max(f64::EPSILON);
        if width <= precision {
            self.record_branch();
            let level = engine.trail_mark();
            let value = float.lower_bound() + width / 2.0;
            match engine.fix_float(var, value) {
                Ok(PropagationStatus::Failure) => {
                    let jumped = self.handle_failure(engine, level);
                    if jumped {
                        return None;
                    }
                }
                Ok(_) => return self.search(engine),
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
            return None;
        }

        for (side, bound) in
            self.float_branch_cuts(engine, var, float.lower_bound(), float.upper_bound())
        {
            self.record_branch();
            let level = engine.trail_mark();
            let status = match side {
                FloatBranchSide::Above => engine.tighten_float_above(var, bound),
                FloatBranchSide::Below => engine.tighten_float_below(var, bound),
            };
            match status {
                Ok(PropagationStatus::Failure) => {
                    let jumped = self.handle_failure(engine, level);
                    if jumped {
                        return None;
                    }
                }
                Ok(_) => {
                    if let Some(solution) = self.search(engine) {
                        return Some(solution);
                    }
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }

        None
    }

    fn collect_each(
        &mut self,
        engine: &mut Engine,
        on_solution: &mut dyn FnMut(&Solution) -> bool,
    ) -> bool {
        if self.check_timeout() {
            return false;
        }

        if engine.is_solved() {
            let solution = self.collect_solution(engine);
            if matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            ) {
                self.pending_solution_restart = true;
            }
            return on_solution(&solution);
        }

        let assignment = branch_assignments_from_explanation(engine.explanation());
        if self.is_pruned(&assignment) {
            return true;
        }

        let Some(var) = self.select_variable(engine) else {
            return true;
        };

        match engine.domain(var).kind() {
            DomainKind::Int => self.collect_int_branches(engine, var, &assignment, on_solution),
            DomainKind::Set => self.collect_set_branches(engine, var, on_solution),
            DomainKind::Float => self.collect_float_branches(engine, var, on_solution),
        }
    }

    fn collect_int_branches(
        &mut self,
        engine: &mut Engine,
        var: VariableId,
        assignment: &[(VariableId, i32)],
        on_solution: &mut dyn FnMut(&Solution) -> bool,
    ) -> bool {
        for value in self.ordered_values(engine, var) {
            if self.would_prune(assignment, var, value) {
                continue;
            }
            self.record_branch();
            let level = engine.trail_mark();
            self.record_phase(var, value);
            match engine.fix_variable(var, value) {
                Ok(PropagationStatus::Failure) => {
                    let _ = self.handle_failure(engine, level);
                }
                Ok(_) => {
                    let cont = self.collect_each(engine, on_solution);
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                    if !cont {
                        return false;
                    }
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }
        true
    }

    fn collect_set_branches(
        &mut self,
        engine: &mut Engine,
        var: VariableId,
        on_solution: &mut dyn FnMut(&Solution) -> bool,
    ) -> bool {
        let Some(undecided) = engine.domain(var).as_set().map(|set| set.undecided()) else {
            return true;
        };
        if undecided.is_empty() {
            return self.collect_each(engine, on_solution);
        }
        let ordering = self.active_value_ordering(engine);
        let value = choose_set_branch_element(&undecided, var, ordering);
        for force_in in set_membership_branch_order(ordering) {
            self.record_branch();
            let level = engine.trail_mark();
            let status = if force_in {
                engine.force_set_in(var, value)
            } else {
                engine.force_set_out(var, value)
            };
            match status {
                Ok(PropagationStatus::Failure) => {
                    let _ = self.handle_failure(engine, level);
                }
                Ok(_) => {
                    let cont = self.collect_each(engine, on_solution);
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                    if !cont {
                        return false;
                    }
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }
        true
    }

    fn collect_float_branches(
        &mut self,
        engine: &mut Engine,
        var: VariableId,
        on_solution: &mut dyn FnMut(&Solution) -> bool,
    ) -> bool {
        let Some(float) = engine.domain(var).as_float().cloned() else {
            return true;
        };
        if float.is_fixed() {
            return self.collect_each(engine, on_solution);
        }
        let width = float.upper_bound() - float.lower_bound();
        let precision = self.config.float_precision.max(f64::EPSILON);
        if width <= precision {
            self.record_branch();
            let level = engine.trail_mark();
            let value = float.lower_bound() + width / 2.0;
            if let Ok(PropagationStatus::Failure) = engine.fix_float(var, value) {
                let _ = self.handle_failure(engine, level);
            } else {
                let cont = self.collect_each(engine, on_solution);
                engine.trail_backtrack(level);
                if !cont {
                    return false;
                }
            }
            return true;
        }
        for (side, bound) in
            self.float_branch_cuts(engine, var, float.lower_bound(), float.upper_bound())
        {
            self.record_branch();
            let level = engine.trail_mark();
            let status = match side {
                FloatBranchSide::Above => engine.tighten_float_above(var, bound),
                FloatBranchSide::Below => engine.tighten_float_below(var, bound),
            };
            match status {
                Ok(PropagationStatus::Failure) => {
                    let _ = self.handle_failure(engine, level);
                }
                Ok(_) => {
                    let cont = self.collect_each(engine, on_solution);
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                    if !cont {
                        return false;
                    }
                }
                Err(_) => {
                    self.stats.record_backtrack();
                    engine.trail_backtrack(level);
                }
            }
        }
        true
    }

    /// Branch cuts for an open float interval: `(side, bound)` pairs applied in order.
    ///
    /// When a registered or domain hole lies strictly inside `(lo, hi)`, splits as
    /// `x <= next_down(hole)` and `x >= next_up(hole)`. Otherwise bisects at the midpoint.
    fn float_branch_cuts(
        &self,
        engine: &Engine,
        var: VariableId,
        lo: f64,
        hi: f64,
    ) -> [(FloatBranchSide, f64); 2] {
        let cuts = if let Some(hole) = self.best_interior_float_hole(engine, var, lo, hi) {
            [
                (FloatBranchSide::Above, next_float_down(hole)),
                (FloatBranchSide::Below, next_float_up(hole)),
            ]
        } else {
            let mid = lo + (hi - lo) / 2.0;
            [(FloatBranchSide::Above, mid), (FloatBranchSide::Below, mid)]
        };
        match self.active_value_ordering(engine) {
            crate::config::ValueOrdering::Descending
            | crate::config::ValueOrdering::ReverseSplit => [cuts[1], cuts[0]],
            _ => cuts,
        }
    }

    fn best_interior_float_hole(
        &self,
        engine: &Engine,
        var: VariableId,
        lo: f64,
        hi: f64,
    ) -> Option<f64> {
        let mid = lo + (hi - lo) / 2.0;
        let mut candidates: Vec<f64> = self
            .float_holes
            .get(&var)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&hole| hole > lo && hole < hi)
            .collect();
        if let Some(domain) = engine.domain(var).as_float() {
            candidates.extend(
                domain
                    .holes()
                    .iter()
                    .copied()
                    .filter(|&hole| hole > lo && hole < hi),
            );
        }
        candidates
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        candidates.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
        candidates.into_iter().min_by(|left, right| {
            let left_dist = (left - mid).abs();
            let right_dist = (right - mid).abs();
            left_dist
                .partial_cmp(&right_dist)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn handle_failure(&mut self, engine: &mut Engine, level: usize) -> bool {
        self.stats.record_backtrack();
        self.stats.record_conflict();

        // Nogood/clause propagators are int-literal based; skip learning on set/float conflicts.
        if self.config.learning
            && let Some(conflict) = engine.last_conflict()
            && matches!(
                engine.domain(conflict.variable).kind(),
                propaga_domains::DomainKind::Int
            )
        {
            self.bump_weights(&conflict.explanation.unique_branch_literals());
            self.bump_activities(&conflict.explanation.unique_branch_literals());
            let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
            let branch_order: Vec<NogoodLiteral> = conflict.explanation.unique_branch_literals();
            let learned = self.nogoods.learn(nogood.clone());
            if learned {
                engine.add_propagator(Box::new(NogoodPropagator::new(nogood.literals().to_vec())));
                self.stats.record_nogood();
            }
            if self.config.clause_learning && self.clauses.learn_from_nogood(&nogood) {
                engine.add_propagator(Box::new(ClausePropagator::new(nogood.literals().to_vec())));
            }
            if learned && let Some(learned_nogood) = self.nogoods.last() {
                let backjump = ConflictAnalyzer::backjump_level(learned_nogood, &branch_order);
                let target = backjump.min(level);
                engine.trail_backtrack(target);
                return target < level;
            }
        }

        engine.trail_backtrack(level);
        false
    }

    fn should_restart(&self) -> bool {
        if self.pending_solution_restart
            && matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            )
        {
            return true;
        }
        self.config
            .restart_policy
            .node_limit(self.restart_index)
            .is_some_and(|limit| limit > 0 && self.nodes_since_restart >= limit)
    }

    fn perform_restart(&mut self, engine: &mut Engine) {
        if engine.trail_depth() > 0 {
            engine.trail_backtrack(0);
        }
        self.stats.record_restart();
        self.restart_index += 1;
        self.nodes_since_restart = 0;
        self.pending_solution_restart = false;
    }

    fn record_branch(&mut self) {
        self.stats.record_node();
        self.nodes_since_restart += 1;
    }

    fn record_phase(&mut self, var: VariableId, value: i32) {
        if self.config.phase_saving {
            self.phases.insert(var, value);
        }
    }

    fn is_pruned(&self, assignment: &[(VariableId, i32)]) -> bool {
        (self.config.learning && self.nogoods.is_violated(assignment))
            || (self.config.clause_learning && self.clauses.is_violated(assignment))
    }

    fn would_prune(&self, assignment: &[(VariableId, i32)], var: VariableId, value: i32) -> bool {
        (self.config.learning && self.nogoods.would_violate(assignment, var, value))
            || (self.config.clause_learning && self.clauses.would_violate(assignment, var, value))
    }

    fn bump_weights(&mut self, literals: &[NogoodLiteral]) {
        if !matches!(
            self.config.variable_ordering,
            crate::config::VariableOrdering::DomWdeg
        ) {
            return;
        }
        for literal in literals {
            *self.weights.entry(literal.variable).or_insert(1) += 1;
        }
    }

    fn bump_activities(&mut self, literals: &[NogoodLiteral]) {
        if !matches!(
            self.config.variable_ordering,
            crate::config::VariableOrdering::Activity
        ) {
            return;
        }
        for literal in literals {
            let entry = self.activities.entry(literal.variable).or_insert(1);
            *entry = entry.saturating_add(1);
        }
    }

    fn active_value_ordering(&self, engine: &Engine) -> crate::config::ValueOrdering {
        self.active_search_phase(engine)
            .map(|phase| phase.value_ordering)
            .unwrap_or(self.config.value_ordering)
    }

    fn select_variable(&self, engine: &Engine) -> Option<VariableId> {
        if let Some(phase) = self.active_search_phase(engine) {
            return self.select_variable_among(engine, &phase.variables, phase.variable_ordering);
        }
        self.select_variable_among(engine, &self.variables, self.config.variable_ordering)
    }

    fn active_search_phase(&self, engine: &Engine) -> Option<&SearchPhase> {
        self.search_phases.iter().find(|phase| {
            phase
                .variables
                .iter()
                .any(|&var| !engine.domain(var).is_fixed())
        })
    }

    fn select_variable_among(
        &self,
        engine: &Engine,
        order: &[VariableId],
        ordering: crate::config::VariableOrdering,
    ) -> Option<VariableId> {
        let candidates: Vec<VariableId> = order
            .iter()
            .copied()
            .filter(|&var| !engine.domain(var).is_fixed())
            .collect();

        match ordering {
            crate::config::VariableOrdering::Mrv => candidates
                .into_iter()
                .min_by_key(|&var| engine.domain(var).size()),
            crate::config::VariableOrdering::Dom => {
                candidates.into_iter().max_by(|&left, &right| {
                    let left_size = engine.domain(left).size();
                    let right_size = engine.domain(right).size();
                    left_size.cmp(&right_size).then_with(|| {
                        // Stable: earlier in `order` wins on ties.
                        variable_index(order, right).cmp(&variable_index(order, left))
                    })
                })
            }
            crate::config::VariableOrdering::DomWdeg => {
                candidates.into_iter().min_by(|&left, &right| {
                    let left_score = weighted_score(engine, left, self.weights.get(&left).copied());
                    let right_score =
                        weighted_score(engine, right, self.weights.get(&right).copied());
                    left_score.cmp(&right_score).then_with(|| {
                        variable_index(order, left).cmp(&variable_index(order, right))
                    })
                })
            }
            crate::config::VariableOrdering::InputOrder => order
                .iter()
                .copied()
                .find(|&var| !engine.domain(var).is_fixed()),
            crate::config::VariableOrdering::Activity => {
                candidates.into_iter().max_by(|&left, &right| {
                    let left_activity = self.activities.get(&left).copied().unwrap_or(1);
                    let right_activity = self.activities.get(&right).copied().unwrap_or(1);
                    left_activity
                        .cmp(&right_activity)
                        .then_with(|| engine.domain(left).size().cmp(&engine.domain(right).size()))
                        .then_with(|| {
                            variable_index(order, left).cmp(&variable_index(order, right))
                        })
                })
            }
            crate::config::VariableOrdering::SmallestMin => {
                candidates.into_iter().min_by(|&left, &right| {
                    domain_min_key(engine, left)
                        .cmp(&domain_min_key(engine, right))
                        .then_with(|| {
                            variable_index(order, left).cmp(&variable_index(order, right))
                        })
                })
            }
            crate::config::VariableOrdering::LargestMax => {
                candidates.into_iter().max_by(|&left, &right| {
                    domain_max_key(engine, left)
                        .cmp(&domain_max_key(engine, right))
                        .then_with(|| {
                            variable_index(order, right).cmp(&variable_index(order, left))
                        })
                })
            }
            crate::config::VariableOrdering::MaxRegret => {
                candidates.into_iter().max_by(|&left, &right| {
                    max_regret_score(engine, left)
                        .cmp(&max_regret_score(engine, right))
                        .then_with(|| {
                            variable_index(order, right).cmp(&variable_index(order, left))
                        })
                })
            }
        }
    }

    fn ordered_values(&self, engine: &Engine, var: VariableId) -> Vec<i32> {
        let domain = engine.int_domain(var).expect("int search variable");
        let mut values = Vec::new();

        if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
            for value in min..=max {
                if domain.contains(value) {
                    values.push(value);
                }
            }
        }

        let value_ordering = self.active_value_ordering(engine);

        match value_ordering {
            crate::config::ValueOrdering::Ascending => {}
            crate::config::ValueOrdering::Descending => values.reverse(),
            crate::config::ValueOrdering::Lcv => {
                values.sort_by_key(|value| {
                    self.variables
                        .iter()
                        .filter(|&&other| {
                            other != var
                                && engine
                                    .int_domain(other)
                                    .is_some_and(|domain| domain.contains(*value))
                        })
                        .count()
                });
            }
            crate::config::ValueOrdering::Split => {
                if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
                    let midpoint = min + (max - min) / 2;
                    values.sort_by_key(|value| {
                        let distance = value.abs_diff(midpoint);
                        (distance, *value)
                    });
                }
            }
            crate::config::ValueOrdering::ReverseSplit => {
                if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
                    let midpoint = min + (max - min) / 2;
                    values.sort_by_key(|value| {
                        let upper_first = if *value > midpoint { 0u8 } else { 1 };
                        let distance = value.abs_diff(midpoint);
                        (upper_first, distance, *value)
                    });
                }
            }
            crate::config::ValueOrdering::Median => {
                if !values.is_empty() {
                    let median = values[values.len() / 2];
                    values.retain(|&value| value != median);
                    values.sort_unstable();
                    values.insert(0, median);
                }
            }
            crate::config::ValueOrdering::Middle => {
                if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
                    order_middle(&mut values, min, max);
                }
            }
            crate::config::ValueOrdering::Random => {
                shuffle_values_deterministic(&mut values, var);
            }
            crate::config::ValueOrdering::Interval => {
                order_first_interval_or_split(&mut values, domain.min(), domain.max());
            }
        }

        if self.config.phase_saving
            && let Some(&phase) = self.phases.get(&var)
            && let Some(pos) = values.iter().position(|&value| value == phase)
        {
            values.remove(pos);
            values.insert(0, phase);
        }

        values
    }

    fn collect_solution(&self, engine: &Engine) -> Solution {
        self.variables
            .iter()
            .filter_map(|&var| {
                if !engine.domain(var).is_fixed() {
                    return None;
                }
                let value = match engine.domain(var) {
                    propaga_domains::AnyDomain::Int(domain) => AssignmentValue::Int(domain.min()?),
                    propaga_domains::AnyDomain::Set(domain) => {
                        AssignmentValue::Set(domain.fixed_values()?)
                    }
                    propaga_domains::AnyDomain::Float(domain) => {
                        AssignmentValue::Float(domain.lower_bound())
                    }
                };
                Some((var, value))
            })
            .collect()
    }
}

fn choose_set_branch_element(
    undecided: &[i32],
    var: VariableId,
    ordering: crate::config::ValueOrdering,
) -> i32 {
    debug_assert!(!undecided.is_empty());
    match ordering {
        crate::config::ValueOrdering::Descending | crate::config::ValueOrdering::ReverseSplit => {
            undecided
                .iter()
                .copied()
                .max()
                .expect("undecided non-empty")
        }
        crate::config::ValueOrdering::Random => {
            let mut values = undecided.to_vec();
            shuffle_values_deterministic(&mut values, var);
            values[0]
        }
        crate::config::ValueOrdering::Median | crate::config::ValueOrdering::Middle => {
            let mut values = undecided.to_vec();
            values.sort_unstable();
            values[values.len() / 2]
        }
        _ => undecided
            .iter()
            .copied()
            .min()
            .expect("undecided non-empty"),
    }
}

fn set_membership_branch_order(ordering: crate::config::ValueOrdering) -> [bool; 2] {
    match ordering {
        crate::config::ValueOrdering::Descending | crate::config::ValueOrdering::ReverseSplit => {
            [false, true]
        }
        _ => [true, false],
    }
}

fn variable_index(order: &[VariableId], var: VariableId) -> usize {
    order
        .iter()
        .position(|&candidate| candidate == var)
        .unwrap_or(usize::MAX)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FloatBranchSide {
    /// Tighten the upper bound (`remove_above`).
    Above,
    /// Tighten the lower bound (`remove_below`).
    Below,
}

fn weighted_score(engine: &Engine, var: VariableId, weight: Option<u32>) -> u64 {
    let size = engine.domain(var).size() as u64;
    let weight = weight.unwrap_or(1).max(1) as u64;
    size.saturating_mul(1_000) / weight
}

fn domain_min_key(engine: &Engine, var: VariableId) -> i32 {
    engine
        .int_domain(var)
        .and_then(|domain| domain.min())
        .unwrap_or(i32::MAX)
}

fn domain_max_key(engine: &Engine, var: VariableId) -> i32 {
    engine
        .int_domain(var)
        .and_then(|domain| domain.max())
        .unwrap_or(i32::MIN)
}

fn max_regret_score(engine: &Engine, var: VariableId) -> i32 {
    let Some(domain) = engine.int_domain(var) else {
        return 0;
    };
    let Some(min) = domain.min() else {
        return 0;
    };
    let Some(max) = domain.max() else {
        return 0;
    };
    for value in (min + 1)..=max {
        if domain.contains(value) {
            return value - min;
        }
    }
    0
}

fn order_middle(values: &mut [i32], min: i32, max: i32) {
    let mean = (f64::from(min) + f64::from(max)) / 2.0;
    values.sort_by(|left, right| {
        let left_dist = (f64::from(*left) - mean).abs();
        let right_dist = (f64::from(*right) - mean).abs();
        left_dist
            .partial_cmp(&right_dist)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.cmp(right))
    });
}

fn shuffle_values_deterministic(values: &mut [i32], var: VariableId) {
    if values.len() < 2 {
        return;
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    var.hash(&mut hasher);
    values.len().hash(&mut hasher);
    for &value in values.iter() {
        value.hash(&mut hasher);
    }
    let mut state = hasher.finish() ^ 0x9e37_79b9_7f4a_7c15;
    for i in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        values.swap(i, j);
    }
}

fn order_first_interval_or_split(values: &mut Vec<i32>, min: Option<i32>, max: Option<i32>) {
    if values.is_empty() {
        return;
    }
    let Some(min) = min else {
        return;
    };
    let mut first_end = values[0];
    for window in values.windows(2) {
        if window[1] == window[0] + 1 {
            first_end = window[1];
        } else {
            break;
        }
    }
    let has_gap = values.last().is_some_and(|last| *last > first_end);
    if has_gap {
        values.sort_by_key(|value| if *value <= first_end { 0u8 } else { 1 });
        return;
    }
    if let Some(max) = max {
        let midpoint = min + (max - min) / 2;
        values.sort_by_key(|value| {
            let distance = value.abs_diff(midpoint);
            (distance, *value)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RestartPolicy, SearchPhase, ValueOrdering, VariableOrdering};
    use crate::value::AssignmentValue;
    use propaga_domains::IntervalDomain;
    use propaga_propagators::{AllDifferentPropagator, DisjunctivePropagator, DisjunctiveTask};
    use std::time::Duration;

    #[test]
    fn search_phases_finish_earlier_group_before_later() {
        let mut engine = Engine::new();
        let early = engine.new_variable(IntervalDomain::new(1, 3));
        let late = engine.new_variable(IntervalDomain::new(1, 2));
        let search = DepthFirstSearch::with_config(
            vec![early, late],
            SearchConfig {
                learning: false,
                restart_policy: RestartPolicy::None,
                variable_ordering: VariableOrdering::Mrv,
                ..SearchConfig::default()
            },
        )
        .with_search_phases(vec![
            SearchPhase::new(
                vec![early],
                VariableOrdering::InputOrder,
                ValueOrdering::Ascending,
            ),
            SearchPhase::new(vec![late], VariableOrdering::Mrv, ValueOrdering::Descending),
        ]);
        // Without phases, MRV would prefer `late` (smaller domain). Phases keep `early` first.
        let selected = search.select_variable(&engine);
        assert_eq!(selected, Some(early));
    }

    #[test]
    fn root_propagation_prunes_domains_before_branching() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.fix_variable(start_a, 0).unwrap();
        engine.add_propagator(Box::new(DisjunctivePropagator::new(vec![
            DisjunctiveTask {
                start: start_a,
                duration: 4,
            },
            DisjunctiveTask {
                start: start_b,
                duration: 2,
            },
        ])));

        let mut search = DepthFirstSearch::with_config(
            vec![start_b],
            SearchConfig {
                learning: true,
                restart_policy: RestartPolicy::None,
                ..SearchConfig::default()
            },
        );
        let solution = search.solve(&mut engine).expect("solution exists");
        assert_eq!(solution, vec![(start_b, AssignmentValue::Int(4))]);
        assert_eq!(search.stats().nodes, 1);
    }

    #[test]
    fn collect_each_enumerates_both_set_membership_branches() {
        use propaga_domains::{AnyDomain, SetIntervalDomain};

        let mut engine = Engine::new();
        let set = engine.new_variable(AnyDomain::Set(SetIntervalDomain::universe(1..=1)));
        let mut search = DepthFirstSearch::new(vec![set]);
        let mut solutions = Vec::new();
        search.solve_each(&mut engine, |solution| {
            solutions.push(solution.clone());
            true
        });
        assert_eq!(solutions.len(), 2);
        let mut memberships: Vec<Vec<i32>> = solutions
            .into_iter()
            .map(|solution| match &solution[0].1 {
                AssignmentValue::Set(values) => values.clone(),
                other => panic!("expected set assignment, got {other:?}"),
            })
            .collect();
        memberships.sort();
        assert_eq!(memberships, vec![vec![], vec![1]]);
    }

    #[test]
    fn float_branch_cuts_prefer_registered_hole() {
        use propaga_domains::{AnyDomain, FloatDomain};

        let mut engine = Engine::new();
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let mut search = DepthFirstSearch::new(vec![y]);
        search.register_float_hole(y, 1.0);
        let cuts = search.float_branch_cuts(&engine, y, 0.0, 2.0);
        assert_eq!(cuts[0], (FloatBranchSide::Above, next_float_down(1.0)));
        assert_eq!(cuts[1], (FloatBranchSide::Below, next_float_up(1.0)));
    }

    #[test]
    fn float_hole_split_finds_solution_away_from_point() {
        use propaga_domains::{AnyDomain, FloatDomain};
        use propaga_propagators::{ForbiddenAssignmentPropagator, encode_forbidden_float};

        let mut engine = Engine::new();
        let y = engine.new_variable(AnyDomain::Float(FloatDomain::new(0.0, 2.0)));
        let encoded = encode_forbidden_float(&mut engine, y, 1.0);
        engine.add_propagator(Box::new(ForbiddenAssignmentPropagator::new(
            encoded.forbidden,
        )));
        let mut search = DepthFirstSearch::new(vec![y]);
        search.register_float_hole(y, 1.0);
        let solution = search.solve(&mut engine).expect("solution exists");
        let AssignmentValue::Float(value) = solution[0].1 else {
            panic!("expected float assignment");
        };
        assert!(
            (value - 1.0).abs() > 0.0,
            "hole split should avoid exact blocked point, got {value}"
        );
    }

    #[test]
    fn solves_three_variable_all_different() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 3)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

        let mut search = DepthFirstSearch::new(vars.clone());
        let solution = search.solve(&mut engine).expect("solution exists");

        let values: Vec<i32> = solution
            .into_iter()
            .filter_map(|(_, value)| match value {
                AssignmentValue::Int(value) => Some(value),
                _ => None,
            })
            .collect();
        let mut sorted = values.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 3);
        assert!(values.iter().all(|value| (1..=3).contains(value)));
    }

    #[test]
    fn root_propagation_rejects_obvious_unsat() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 2)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

        let mut search = DepthFirstSearch::new(vars);
        assert!(search.solve(&mut engine).is_none());
        assert_eq!(search.stats().nodes, 0);
    }

    #[test]
    fn learning_records_nogoods_on_conflict() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 2)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

        let mut search = DepthFirstSearch::new(vars);
        assert!(
            search
                .solve_without_initial_propagation(&mut engine)
                .is_none()
        );
        assert!(search.stats().conflicts > 0);
        assert!(search.nogood_count() > 0);
    }

    #[test]
    fn lcv_orders_values_by_domain_frequency() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.fix_variable(b, 2).unwrap();
        engine.fix_variable(c, 2).unwrap();

        let search = DepthFirstSearch::with_config(
            vec![a, b, c],
            SearchConfig {
                learning: false,
                restart_policy: RestartPolicy::None,
                value_ordering: crate::config::ValueOrdering::Lcv,
                ..SearchConfig::default()
            },
        );
        let values = search.ordered_values(&engine, a);
        assert_eq!(values.last(), Some(&2));
    }

    #[test]
    fn phase_saving_prefers_last_assigned_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));

        let mut search = DepthFirstSearch::with_config(
            vec![a],
            SearchConfig {
                learning: false,
                restart_policy: RestartPolicy::None,
                phase_saving: true,
                ..SearchConfig::default()
            },
        );
        search.record_phase(a, 2);
        assert_eq!(search.ordered_values(&engine, a), vec![2, 1, 3]);
    }

    #[test]
    fn posts_nogood_propagator_on_conflict() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..3)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 2)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

        let mut search = DepthFirstSearch::with_config(
            vars,
            SearchConfig {
                learning: true,
                restart_policy: RestartPolicy::None,
                ..SearchConfig::default()
            },
        );
        assert!(
            search
                .solve_without_initial_propagation(&mut engine)
                .is_none()
        );
        assert!(search.nogood_count() > 0);
        assert!(search.stats().nogoods_learned > 0);
    }

    #[test]
    fn respects_time_limit() {
        let mut engine = Engine::new();
        let vars: Vec<_> = (0..20)
            .map(|_| engine.new_variable(IntervalDomain::new(1, 20)))
            .collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars.clone())));

        let mut search = DepthFirstSearch::with_config(
            vars,
            SearchConfig {
                learning: false,
                restart_policy: RestartPolicy::None,
                time_limit: Some(Duration::from_millis(1)),
                ..SearchConfig::default()
            },
        );
        assert!(search.solve(&mut engine).is_none());
        assert!(search.stats().timed_out);
    }

    #[test]
    fn set_branch_element_follows_value_ordering() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(0, 0));
        let undecided = [1, 2, 5];
        assert_eq!(
            choose_set_branch_element(&undecided, var, ValueOrdering::Ascending),
            1
        );
        assert_eq!(
            choose_set_branch_element(&undecided, var, ValueOrdering::Descending),
            5
        );
        assert_eq!(
            choose_set_branch_element(&undecided, var, ValueOrdering::Median),
            2
        );
        assert_eq!(
            set_membership_branch_order(ValueOrdering::Ascending),
            [true, false]
        );
        assert_eq!(
            set_membership_branch_order(ValueOrdering::Descending),
            [false, true]
        );
    }

    #[test]
    fn middle_prefers_value_closest_to_bound_mean() {
        let mut values = vec![1, 2, 4, 5];
        order_middle(&mut values, 1, 5);
        // Mean of bounds is 3.0; 2 and 4 are equidistant — prefer the smaller.
        assert_eq!(values[0], 2);
        assert_eq!(values[1], 4);
    }

    #[test]
    fn max_regret_and_bound_selectors_pick_expected_variable() {
        use propaga_domains::{HybridDomain, IntervalDomain};

        let mut engine = Engine::new();
        let low = engine.new_variable(IntervalDomain::new(1, 3));
        let high = engine.new_variable(IntervalDomain::new(10, 12));
        let gappy = engine.new_variable(
            HybridDomain::new(0, 5)
                .remove(1)
                .remove(2)
                .remove(3)
                .remove(4),
        );

        assert_eq!(max_regret_score(&engine, low), 1);
        assert_eq!(max_regret_score(&engine, gappy), 5);

        let search = DepthFirstSearch::with_config(
            vec![low, high, gappy],
            SearchConfig {
                variable_ordering: VariableOrdering::MaxRegret,
                learning: false,
                restart_policy: RestartPolicy::None,
                ..SearchConfig::default()
            },
        );
        assert_eq!(search.select_variable(&engine), Some(gappy));

        let search = DepthFirstSearch::with_config(
            vec![low, high],
            SearchConfig {
                variable_ordering: VariableOrdering::SmallestMin,
                learning: false,
                restart_policy: RestartPolicy::None,
                ..SearchConfig::default()
            },
        );
        assert_eq!(search.select_variable(&engine), Some(low));

        let search = DepthFirstSearch::with_config(
            vec![low, high],
            SearchConfig {
                variable_ordering: VariableOrdering::LargestMax,
                learning: false,
                restart_policy: RestartPolicy::None,
                ..SearchConfig::default()
            },
        );
        assert_eq!(search.select_variable(&engine), Some(high));
    }
}
