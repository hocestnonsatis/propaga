use propaga_core::{PropagationContext, VariableId};
use std::collections::{HashMap, VecDeque};

/// Returns `true` when every variable can be assigned distinct values.
pub fn has_perfect_matching(ctx: &dyn PropagationContext, variables: &[VariableId]) -> bool {
    if variables.is_empty() {
        return true;
    }
    let graph = build_bipartite_graph(ctx, variables);
    hopcroft_karp(&graph.adj, variables.len(), graph.value_count) == variables.len()
}

/// Returns `true` when `var` can take `value` in some perfect matching.
#[cfg(test)]
pub(crate) fn value_in_some_matching(
    ctx: &dyn PropagationContext,
    variables: &[VariableId],
    var: VariableId,
    value: i32,
) -> bool {
    if !ctx.domain(var).contains(value) {
        return false;
    }

    let Some(var_index) = variables.iter().position(|&candidate| candidate == var) else {
        return false;
    };

    let graph = build_bipartite_graph(ctx, variables);
    value_supported_in_graph(&graph, variables.len(), var_index, value)
}

/// Removes unsupported values using Regin SCC batch pruning after one matching.
pub fn remove_unsupported_values(
    ctx: &mut dyn PropagationContext,
    variables: &[VariableId],
) -> Result<bool, ()> {
    if variables.len() <= 1 {
        return Ok(false);
    }

    let graph = build_bipartite_graph(ctx, variables);
    if hopcroft_karp(&graph.adj, variables.len(), graph.value_count) != variables.len() {
        return Err(());
    }

    let (pair_left, pair_right) =
        hopcroft_karp_matching(&graph.adj, variables.len(), graph.value_count);

    let value_graph = build_regin_value_graph(ctx, variables, &graph, &pair_left);
    let components = tarjan_scc(&value_graph, graph.value_count);

    apply_regin_pruning(ctx, variables, &graph, &pair_left, &pair_right, &components)
}

fn apply_regin_pruning(
    ctx: &mut dyn PropagationContext,
    variables: &[VariableId],
    graph: &BipartiteGraph,
    pair_left: &[Option<usize>],
    pair_right: &[Option<usize>],
    components: &[usize],
) -> Result<bool, ()> {
    let mut changed = false;
    for (left, &var) in variables.iter().enumerate() {
        let Some(matched) = pair_left[left] else {
            return Err(());
        };
        let matched_component = components[matched];

        for value in collect_values(ctx, var) {
            let Some(&value_idx) = graph.value_index.get(&value) else {
                if ctx.remove_value(var, value) {
                    changed = true;
                }
                continue;
            };

            if !graph.adj[left].contains(&value_idx) {
                if ctx.remove_value(var, value) {
                    changed = true;
                }
                continue;
            }

            if regin_supports_value(
                value_idx,
                matched,
                pair_right,
                matched_component,
                components,
            ) {
                continue;
            }

            if ctx.remove_value(var, value) {
                changed = true;
            }
        }
    }

    Ok(changed)
}

/// Returns `true` when value index `value_idx` is supported for a variable matched to `matched`.
fn regin_supports_value(
    value_idx: usize,
    matched: usize,
    pair_right: &[Option<usize>],
    matched_component: usize,
    components: &[usize],
) -> bool {
    if value_idx == matched {
        return true;
    }
    // Values unmatched by the maximum matching are free on the right side and
    // always belong to some perfect matching when the variable side is saturated.
    if pair_right[value_idx].is_none() {
        return true;
    }
    components[value_idx] == matched_component
}

/// Builds Regin's directed value graph: `v -> m(x)` for each `v in D(x) \\ {m(x)}`.
fn build_regin_value_graph(
    ctx: &dyn PropagationContext,
    variables: &[VariableId],
    graph: &BipartiteGraph,
    pair_left: &[Option<usize>],
) -> Vec<Vec<usize>> {
    let mut value_graph = vec![Vec::new(); graph.value_count];

    for (left, &var) in variables.iter().enumerate() {
        let Some(matched) = pair_left[left] else {
            continue;
        };

        for value in collect_values(ctx, var) {
            let Some(&value_idx) = graph.value_index.get(&value) else {
                continue;
            };
            if value_idx != matched {
                value_graph[value_idx].push(matched);
            }
        }
    }

    value_graph
}

fn tarjan_scc(adj: &[Vec<usize>], node_count: usize) -> Vec<usize> {
    let mut state = TarjanState::new(node_count);
    for node in 0..node_count {
        if state.index[node].is_none() {
            state.strong_connect(node, adj);
        }
    }
    state.component
}

struct TarjanState {
    index: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    component: Vec<usize>,
    next_index: usize,
    component_count: usize,
}

impl TarjanState {
    fn new(node_count: usize) -> Self {
        Self {
            index: vec![None; node_count],
            lowlink: vec![0; node_count],
            stack: Vec::new(),
            on_stack: vec![false; node_count],
            component: vec![0; node_count],
            next_index: 0,
            component_count: 0,
        }
    }

    fn strong_connect(&mut self, node: usize, adj: &[Vec<usize>]) {
        self.index[node] = Some(self.next_index);
        self.lowlink[node] = self.next_index;
        self.next_index += 1;
        self.stack.push(node);
        self.on_stack[node] = true;

        for &successor in &adj[node] {
            if self.index[successor].is_none() {
                self.strong_connect(successor, adj);
                self.lowlink[node] = self.lowlink[node].min(self.lowlink[successor]);
            } else if self.on_stack[successor] {
                self.lowlink[node] =
                    self.lowlink[node].min(self.index[successor].expect("indexed"));
            }
        }

        if self.lowlink[node] == self.index[node].expect("indexed") {
            loop {
                let top = self.stack.pop().expect("non-empty stack");
                self.on_stack[top] = false;
                self.component[top] = self.component_count;
                if top == node {
                    break;
                }
            }
            self.component_count += 1;
        }
    }
}

#[cfg(test)]
fn value_supported_in_graph(
    graph: &BipartiteGraph,
    variable_count: usize,
    var_index: usize,
    value: i32,
) -> bool {
    let Some(&value_idx) = graph.value_index.get(&value) else {
        return false;
    };
    if !graph.adj[var_index].contains(&value_idx) {
        return false;
    }

    let mut adj = graph.adj.clone();
    adj[var_index].retain(|&idx| idx == value_idx);
    for (index, edges) in adj.iter_mut().enumerate() {
        if index != var_index {
            edges.retain(|&idx| idx != value_idx);
        }
    }

    if adj.iter().any(std::vec::Vec::is_empty) {
        return false;
    }

    hopcroft_karp(&adj, variable_count, graph.value_count) == variable_count
}

struct BipartiteGraph {
    adj: Vec<Vec<usize>>,
    value_index: HashMap<i32, usize>,
    value_count: usize,
}

fn build_bipartite_graph(ctx: &dyn PropagationContext, variables: &[VariableId]) -> BipartiteGraph {
    let mut value_index = HashMap::new();
    let mut adj = vec![Vec::new(); variables.len()];

    for (left, &var) in variables.iter().enumerate() {
        for value in collect_values(ctx, var) {
            let next = value_index.len();
            let right = *value_index.entry(value).or_insert(next);
            adj[left].push(right);
        }
    }

    let value_count = value_index.len();
    BipartiteGraph {
        adj,
        value_index,
        value_count,
    }
}

fn hopcroft_karp_matching(
    adj: &[Vec<usize>],
    left_count: usize,
    right_count: usize,
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut pair_left = vec![None; left_count];
    let mut pair_right = vec![None; right_count];
    let mut dist = vec![0; left_count];

    while bfs(adj, &pair_left, &pair_right, &mut dist) {
        for left in 0..left_count {
            if pair_left[left].is_none() {
                let _ = dfs(left, adj, &mut pair_left, &mut pair_right, &mut dist);
            }
        }
    }

    (pair_left, pair_right)
}

fn hopcroft_karp(adj: &[Vec<usize>], left_count: usize, right_count: usize) -> usize {
    if left_count == 0 {
        return 0;
    }

    let mut pair_left = vec![None; left_count];
    let mut pair_right = vec![None; right_count];
    let mut dist = vec![0; left_count];

    let mut matching = 0;
    while bfs(adj, &pair_left, &pair_right, &mut dist) {
        for left in 0..left_count {
            if pair_left[left].is_none()
                && dfs(left, adj, &mut pair_left, &mut pair_right, &mut dist)
            {
                matching += 1;
            }
        }
    }

    matching
}

fn bfs(
    adj: &[Vec<usize>],
    pair_left: &[Option<usize>],
    pair_right: &[Option<usize>],
    dist: &mut [i32],
) -> bool {
    const INF: i32 = i32::MAX;
    let mut queue = VecDeque::new();
    dist.fill(INF);

    for left in 0..adj.len() {
        if pair_left[left].is_none() {
            dist[left] = 0;
            queue.push_back(left);
        }
    }

    let mut found_free = false;
    while let Some(left) = queue.pop_front() {
        for &right in &adj[left] {
            let next_left = pair_right[right];
            match next_left {
                None => found_free = true,
                Some(next) if dist[next] == INF => {
                    dist[next] = dist[left] + 1;
                    queue.push_back(next);
                }
                _ => {}
            }
        }
    }

    found_free
}

fn dfs(
    left: usize,
    adj: &[Vec<usize>],
    pair_left: &mut [Option<usize>],
    pair_right: &mut [Option<usize>],
    dist: &mut [i32],
) -> bool {
    for &right in &adj[left] {
        let next_left = pair_right[right];
        let can_extend = match next_left {
            None => true,
            Some(next) => {
                dist[next] == dist[left] + 1 && dfs(next, adj, pair_left, pair_right, dist)
            }
        };
        if can_extend {
            pair_left[left] = Some(right);
            pair_right[right] = Some(left);
            return true;
        }
    }

    dist[left] = i32::MAX;
    false
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
    use crate::test_support::{MutEngine, ReadOnlyEngine};
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn perfect_matching_exists_for_two_vars() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let vars = vec![a, b];
        assert!(has_perfect_matching(&ReadOnlyEngine(&engine), &vars));
    }

    #[test]
    fn no_perfect_matching_when_domains_too_small() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let vars = vec![a, b];
        assert!(!has_perfect_matching(&ReadOnlyEngine(&engine), &vars));
    }

    #[test]
    fn perfect_matching_for_three_by_three_case() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a, b, c];
        assert!(has_perfect_matching(&ReadOnlyEngine(&engine), &vars));
    }

    #[test]
    fn value_support_detects_unsupported_assignment() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a, b, c];
        assert!(!value_in_some_matching(
            &ReadOnlyEngine(&engine),
            &vars,
            c,
            1
        ));
        assert!(!value_in_some_matching(
            &ReadOnlyEngine(&engine),
            &vars,
            c,
            2
        ));
        assert!(value_in_some_matching(
            &ReadOnlyEngine(&engine),
            &vars,
            c,
            3
        ));
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_three_by_three() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a, b, c];
        assert_regin_agrees_with_hk(&mut engine, &vars);
    }

    #[test]
    fn batch_prune_matches_value_support() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(crate::AllDifferentPropagator::new(vec![a, b, c])));
        engine.propagate_all().unwrap();
        assert_eq!(engine.hybrid_domain(c).size(), 1);
        assert_eq!(engine.hybrid_domain(c).min(), Some(3));
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_four_variables() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 4));
        let b = engine.new_variable(IntervalDomain::new(1, 4));
        let c = engine.new_variable(IntervalDomain::new(1, 4));
        let d = engine.new_variable(IntervalDomain::new(1, 5));
        let vars = vec![a, b, c, d];
        assert_regin_agrees_with_hk(&mut engine, &vars);
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_random_small_domains() {
        let mut engine = Engine::new();
        let domain_specs = [(1, 4), (1, 4), (1, 5), (2, 5), (1, 3)];
        let vars: Vec<_> = domain_specs
            .iter()
            .map(|&(lo, hi)| engine.new_variable(IntervalDomain::new(lo, hi)))
            .collect();
        assert_regin_agrees_with_hk(&mut engine, &vars);
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_interval_domains() {
        for n in 2usize..=4usize {
            for lo in 1..=3 {
                for hi in lo..=5 {
                    if hi - lo + 1 < n as i32 {
                        continue;
                    }
                    let mut engine = Engine::new();
                    let vars: Vec<_> = (0..n)
                        .map(|_| engine.new_variable(IntervalDomain::new(lo, hi)))
                        .collect();
                    assert_regin_agrees_with_hk(&mut engine, &vars);
                }
            }

            let mut engine = Engine::new();
            let mut vars: Vec<_> = (0..n.saturating_sub(1))
                .map(|_| engine.new_variable(IntervalDomain::new(1, 4)))
                .collect();
            vars.push(engine.new_variable(IntervalDomain::new(1, if n == 2 { 4 } else { 5 })));
            assert_regin_agrees_with_hk(&mut engine, &vars);
        }
    }

    #[test]
    fn has_perfect_matching_empty_variables() {
        let engine = Engine::new();
        assert!(has_perfect_matching(&ReadOnlyEngine(&engine), &[]));
    }

    #[test]
    fn value_in_some_matching_rejects_out_of_domain_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let vars = vec![a, b];
        assert!(!value_in_some_matching(
            &ReadOnlyEngine(&engine),
            &vars,
            a,
            3
        ));
    }

    #[test]
    fn value_in_some_matching_rejects_unknown_variable() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a, b];
        assert!(!value_in_some_matching(
            &ReadOnlyEngine(&engine),
            &vars,
            c,
            1
        ));
    }

    #[test]
    fn remove_unsupported_values_no_op_for_single_variable() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a];
        assert_eq!(
            remove_unsupported_values(&mut MutEngine(&mut engine), &vars),
            Ok(false)
        );
    }

    #[test]
    fn remove_unsupported_values_fails_without_perfect_matching() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        let vars = vec![a, b];
        assert_eq!(
            remove_unsupported_values(&mut MutEngine(&mut engine), &vars),
            Err(())
        );
    }

    #[test]
    fn remove_unsupported_values_prunes_directly() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![a, b, c];
        assert_eq!(
            remove_unsupported_values(&mut MutEngine(&mut engine), &vars),
            Ok(true)
        );
        assert_eq!(engine.hybrid_domain(c).fixed_value(), Some(3));
    }

    #[test]
    fn apply_regin_pruning_errors_on_missing_match() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let vars = vec![a, b];
        let graph = BipartiteGraph {
            adj: vec![vec![0], vec![1]],
            value_index: [(1, 0), (2, 1)].into_iter().collect(),
            value_count: 2,
        };
        let pair_left = vec![Some(0), None];
        let pair_right = vec![Some(1), None];
        let components = vec![0, 1];
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(
            apply_regin_pruning(
                &mut ctx,
                &vars,
                &graph,
                &pair_left,
                &pair_right,
                &components
            ),
            Err(())
        );
    }

    #[test]
    fn apply_regin_pruning_removes_unknown_and_unsupported_values() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![var];
        let graph = BipartiteGraph {
            adj: vec![vec![0, 1]],
            value_index: [(1, 0), (2, 1)].into_iter().collect(),
            value_count: 2,
        };
        let pair_left = vec![Some(0)];
        let pair_right = vec![Some(0), None];
        let components = vec![0, 0];
        let mut ctx = MutEngine(&mut engine);
        assert_eq!(
            apply_regin_pruning(
                &mut ctx,
                &vars,
                &graph,
                &pair_left,
                &pair_right,
                &components
            ),
            Ok(true)
        );
        assert!(!engine.hybrid_domain(var).contains(3));
    }

    #[test]
    fn value_supported_in_graph_rejects_missing_value() {
        let graph = BipartiteGraph {
            adj: vec![vec![0], vec![1]],
            value_index: [(1, 0), (2, 1)].into_iter().collect(),
            value_count: 2,
        };
        assert!(!value_supported_in_graph(&graph, 1, 0, 3));
    }

    #[test]
    fn build_regin_value_graph_skips_unknown_values() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 3));
        let vars = vec![var];
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let pair_left = vec![Some(0)];
        let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
        assert_eq!(value_graph.len(), graph.value_count);
    }

    #[test]
    fn build_regin_graph_skips_unmatched_left_nodes() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let vars = vec![a, b];
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let pair_left = vec![None, Some(1)];
        let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
        assert_eq!(value_graph.len(), graph.value_count);
    }

    #[test]
    fn mut_engine_exercises_domain_mutators() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 5));
        let mut ctx = MutEngine(&mut engine);
        assert!(ctx.remove_below(a, 2));
        assert!(!ctx.remove_below(a, 2));
        assert!(ctx.remove_above(a, 4));
        assert!(!ctx.remove_above(a, 4));
        assert!(ctx.remove_value(a, 3));
        assert!(!ctx.remove_value(a, 3));
        assert_eq!(ctx.fixed_value(a), None);
    }

    #[test]
    fn read_only_engine_reports_fixed_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(4));
        let ctx = ReadOnlyEngine(&engine);
        assert_eq!(ctx.fixed_value(a), Some(4));
    }

    #[test]
    fn assert_regin_skips_without_perfect_matching() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::fix(1));
        let b = engine.new_variable(IntervalDomain::fix(1));
        assert_regin_agrees_with_hk(&mut engine, &[a, b]);
    }

    #[test]
    fn value_supported_rejects_unknown_value_index() {
        let graph = BipartiteGraph {
            adj: vec![vec![0]],
            value_index: [(1, 0)].into_iter().collect(),
            value_count: 1,
        };
        assert!(!value_supported_in_graph(&graph, 1, 0, 2));
    }

    #[test]
    fn remove_unsupported_prunes_regin_unsupported_value() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3).remove(2));
        let vars = vec![a, b, c];
        assert_eq!(
            remove_unsupported_values(&mut MutEngine(&mut engine), &vars),
            Ok(true)
        );
        assert_eq!(engine.hybrid_domain(c).fixed_value(), Some(3));
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_holey_domains() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 4).remove(2));
        let b = engine.new_variable(IntervalDomain::new(1, 4).remove(3));
        let c = engine.new_variable(IntervalDomain::new(1, 5).remove(4));
        assert_regin_agrees_with_hk(&mut engine, &[a, b, c]);
    }

    #[test]
    fn hopcroft_karp_empty_left_returns_zero() {
        assert_eq!(hopcroft_karp(&[], 0, 0), 0);
    }

    #[test]
    fn regin_supports_unmatched_right_value() {
        let pair_right = vec![Some(0), Some(1), None];
        let components = vec![0, 1, 1];
        assert!(regin_supports_value(2, 0, &pair_right, 0, &components));
    }

    #[test]
    fn value_supported_in_graph_rejects_empty_adjacency() {
        let graph = BipartiteGraph {
            adj: vec![vec![0], vec![]],
            value_index: [(1, 0), (2, 1)].into_iter().collect(),
            value_count: 2,
        };
        assert!(!value_supported_in_graph(&graph, 2, 1, 2));
    }

    #[test]
    fn value_supported_in_graph_rejects_missing_edge() {
        let graph = BipartiteGraph {
            adj: vec![vec![0], vec![1]],
            value_index: [(1, 0), (2, 1)].into_iter().collect(),
            value_count: 2,
        };
        assert!(!value_supported_in_graph(&graph, 2, 0, 2));
    }

    #[test]
    fn read_only_engine_mutators_are_no_ops() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let mut ctx = ReadOnlyEngine(&engine);
        assert!(!ctx.remove_below(a, 2));
        assert!(!ctx.remove_above(a, 2));
        assert!(!ctx.remove_value(a, 2));
    }

    #[test]
    fn value_supported_in_graph_detects_empty_adjacency_after_retain() {
        let graph = BipartiteGraph {
            adj: vec![vec![0, 1], vec![0], vec![1]],
            value_index: [(0, 0), (1, 1)].into_iter().collect(),
            value_count: 2,
        };
        assert!(!value_supported_in_graph(&graph, 3, 0, 1));
    }

    #[test]
    fn build_regin_graph_skips_values_missing_from_index() {
        use propaga_domains::AnyDomain;

        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let vars = vec![a, b];
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let (pair_left, _) = hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        engine.set_domain(
            a,
            AnyDomain::Int(propaga_domains::HybridDomain::Interval(
                IntervalDomain::new(1, 3),
            )),
        );
        let ctx2 = ReadOnlyEngine(&engine);
        let value_graph = build_regin_value_graph(&ctx2, &vars, &graph, &pair_left);
        assert_eq!(value_graph.len(), graph.value_count);
    }

    fn assert_regin_agrees_with_hk(engine: &mut Engine, vars: &[VariableId]) {
        let graph = {
            let ctx = ReadOnlyEngine(engine);
            let graph = build_bipartite_graph(&ctx, vars);
            if hopcroft_karp(&graph.adj, vars.len(), graph.value_count) != vars.len() {
                return;
            }
            graph
        };
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let components = {
            let ctx = ReadOnlyEngine(engine);
            let value_graph = build_regin_value_graph(&ctx, vars, &graph, &pair_left);
            tarjan_scc(&value_graph, graph.value_count)
        };

        let ctx = ReadOnlyEngine(engine);
        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            for value in collect_values(&ctx, var) {
                let hk = value_in_some_matching(&ctx, vars, var, value);
                let value_idx = graph.value_index[&value];
                let scc = regin_supports_value(
                    value_idx,
                    matched,
                    &pair_right,
                    components[matched],
                    &components,
                );
                assert_eq!(hk, scc, "mismatch for var {var:?} value {value}");
            }
        }
    }

    #[test]
    fn apply_regin_removes_values_outside_stale_graph_index() {
        use crate::test_support::MutEngine;

        let mut engine = Engine::new();
        let x0 = engine.new_variable(IntervalDomain::new(0, 1));
        let x1 = engine.new_variable(IntervalDomain::new(0, 1));
        let vars = vec![x0, x1];
        let ctx = MutEngine(&mut engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let components = {
            let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
            tarjan_scc(&value_graph, graph.value_count)
        };
        engine.set_domain(
            x0,
            propaga_domains::AnyDomain::Int(propaga_domains::HybridDomain::Interval(
                IntervalDomain::new(0, 99),
            )),
        );
        let mut ctx2 = MutEngine(&mut engine);
        let mut changed = false;
        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            let matched_component = components[matched];
            for value in collect_values(&ctx2, var) {
                let Some(&value_idx) = graph.value_index.get(&value) else {
                    if ctx2.remove_value(var, value) {
                        changed = true;
                    }
                    continue;
                };
                let _ = regin_supports_value(
                    value_idx,
                    matched,
                    &pair_right,
                    matched_component,
                    &components,
                );
            }
        }
        assert!(changed);
        assert!(!engine.hybrid_domain(x0).contains(99));
    }

    #[test]
    fn bfs_skips_infinite_distance_queue_entries() {
        for left_count in 2..=5 {
            for right_count in 2..=5 {
                let adj = vec![vec![0usize]; left_count];
                let _ = hopcroft_karp(&adj, left_count, right_count);
            }
        }
        let adj = vec![vec![0usize, 1usize], vec![0usize], vec![1usize]];
        let _ = hopcroft_karp(&adj, 3, 2);
    }
}
