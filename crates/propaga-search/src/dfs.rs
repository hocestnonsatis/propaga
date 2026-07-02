use crate::config::SearchConfig;
use crate::conflict::{ConflictAnalyzer, NogoodStore};
use crate::stats::{SearchStats, branch_assignments_from_explanation};
use propaga_core::{DomainView, NogoodLiteral, PropagationStatus, VariableId};
use propaga_engine::Engine;
use propaga_propagators::NogoodPropagator;
use std::collections::HashMap;
use std::time::Instant;

/// Assignment mapping variables to their chosen values.
pub type Solution = Vec<(VariableId, i32)>;

/// Depth-first search with MRV, nogood learning, and optional restarts.
pub struct DepthFirstSearch {
    variables: Vec<VariableId>,
    config: SearchConfig,
    nogoods: NogoodStore,
    stats: SearchStats,
    nodes_since_restart: u64,
    restart_index: u32,
    phases: HashMap<VariableId, i32>,
    weights: HashMap<VariableId, u32>,
    activities: HashMap<VariableId, u32>,
    pending_solution_restart: bool,
    deadline: Option<Instant>,
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
            nogoods: NogoodStore::new(),
            stats: SearchStats::default(),
            nodes_since_restart: 0,
            restart_index: 0,
            phases: HashMap::new(),
            weights: HashMap::new(),
            activities: HashMap::new(),
            pending_solution_restart: false,
            deadline: None,
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

    /// Returns up to `limit` solutions found by exhaustive DFS.
    pub fn solve_all_limited(
        &mut self,
        engine: &mut Engine,
        limit: Option<usize>,
    ) -> Vec<Solution> {
        self.begin_search();
        let mut solutions = Vec::new();
        if self.propagate_root(engine) {
            self.collect_all(engine, &mut solutions, limit);
        }
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
        if self.config.learning && self.nogoods.is_violated(&assignment) {
            return None;
        }

        let var = self.select_variable(engine)?;
        let values = self.ordered_values(engine, var);

        for value in values {
            if self.config.learning && self.nogoods.would_violate(&assignment, var, value) {
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

    fn collect_all(
        &mut self,
        engine: &mut Engine,
        solutions: &mut Vec<Solution>,
        limit: Option<usize>,
    ) {
        if self.check_timeout() {
            return;
        }

        if limit.is_some_and(|max| solutions.len() >= max) {
            return;
        }

        if engine.is_solved() {
            solutions.push(self.collect_solution(engine));
            if matches!(
                self.config.restart_policy,
                crate::config::RestartPolicy::OnSolution
            ) {
                self.pending_solution_restart = true;
            }
            return;
        }

        let assignment = branch_assignments_from_explanation(engine.explanation());
        if self.config.learning && self.nogoods.is_violated(&assignment) {
            return;
        }

        let Some(var) = self.select_variable(engine) else {
            return;
        };

        let values = self.ordered_values(engine, var);

        for value in values {
            if self.config.learning && self.nogoods.would_violate(&assignment, var, value) {
                continue;
            }

            self.record_branch();
            let level = engine.trail_mark();
            self.record_phase(var, value);
            match engine.fix_variable(var, value) {
                Ok(PropagationStatus::Failure) => {
                    let jumped = self.handle_failure(engine, level);
                    if jumped {
                        return;
                    }
                }
                Ok(_) => {
                    self.collect_all(engine, solutions, limit);
                    if limit.is_some_and(|max| solutions.len() >= max) {
                        self.stats.record_backtrack();
                        engine.trail_backtrack(level);
                        return;
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
    }

    fn handle_failure(&mut self, engine: &mut Engine, level: usize) -> bool {
        self.stats.record_backtrack();
        self.stats.record_conflict();

        if self.config.learning
            && let Some(conflict) = engine.last_conflict()
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

    fn select_variable(&self, engine: &Engine) -> Option<VariableId> {
        let candidates: Vec<VariableId> = self
            .variables
            .iter()
            .copied()
            .filter(|&var| !engine.domain(var).is_fixed())
            .collect();

        match self.config.variable_ordering {
            crate::config::VariableOrdering::Mrv => candidates
                .into_iter()
                .min_by_key(|&var| engine.domain(var).size()),
            crate::config::VariableOrdering::Dom => {
                candidates.into_iter().min_by(|&left, &right| {
                    let left_size = engine.domain(left).size();
                    let right_size = engine.domain(right).size();
                    left_size.cmp(&right_size).then_with(|| {
                        variable_index(&self.variables, left)
                            .cmp(&variable_index(&self.variables, right))
                    })
                })
            }
            crate::config::VariableOrdering::DomWdeg => {
                candidates.into_iter().min_by(|&left, &right| {
                    let left_score = weighted_score(engine, left, self.weights.get(&left).copied());
                    let right_score =
                        weighted_score(engine, right, self.weights.get(&right).copied());
                    left_score.cmp(&right_score).then_with(|| {
                        variable_index(&self.variables, left)
                            .cmp(&variable_index(&self.variables, right))
                    })
                })
            }
            crate::config::VariableOrdering::InputOrder => self
                .variables
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
                            variable_index(&self.variables, left)
                                .cmp(&variable_index(&self.variables, right))
                        })
                })
            }
        }
    }

    fn ordered_values(&self, engine: &Engine, var: VariableId) -> Vec<i32> {
        let domain = engine.domain(var);
        let mut values = Vec::new();

        if let (Some(min), Some(max)) = (domain.min(), domain.max()) {
            for value in min..=max {
                if domain.contains(value) {
                    values.push(value);
                }
            }
        }

        match self.config.value_ordering {
            crate::config::ValueOrdering::Ascending => {}
            crate::config::ValueOrdering::Descending => values.reverse(),
            crate::config::ValueOrdering::Lcv => {
                values.sort_by_key(|value| {
                    self.variables
                        .iter()
                        .filter(|&&other| other != var && engine.domain(other).contains(*value))
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
            crate::config::ValueOrdering::Median => {
                if !values.is_empty() {
                    let median = values[values.len() / 2];
                    values.retain(|&value| value != median);
                    values.sort_unstable();
                    values.insert(0, median);
                }
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
                engine
                    .domain(var)
                    .is_fixed()
                    .then_some((var, engine.domain(var).min().expect("fixed domain")))
            })
            .collect()
    }
}

fn variable_index(order: &[VariableId], var: VariableId) -> usize {
    order
        .iter()
        .position(|&candidate| candidate == var)
        .unwrap_or(usize::MAX)
}

fn weighted_score(engine: &Engine, var: VariableId, weight: Option<u32>) -> u64 {
    let size = engine.domain(var).size() as u64;
    let weight = weight.unwrap_or(1).max(1) as u64;
    size.saturating_mul(1_000) / weight
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RestartPolicy;
    use propaga_domains::IntervalDomain;
    use propaga_propagators::{AllDifferentPropagator, DisjunctivePropagator, DisjunctiveTask};
    use std::time::Duration;

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
        assert_eq!(solution, vec![(start_b, 4)]);
        assert_eq!(search.stats().nodes, 1);
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

        let values: Vec<i32> = solution.into_iter().map(|(_, value)| value).collect();
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
}
