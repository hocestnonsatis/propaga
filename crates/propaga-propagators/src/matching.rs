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
                &pair_right,
                matched_component,
                &components,
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
        if dist[left] == INF {
            continue;
        }
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
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
        let components = tarjan_scc(&value_graph, graph.value_count);

        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            for value in collect_values(&ctx, var) {
                let hk = value_in_some_matching(&ctx, &vars, var, value);
                let Some(&value_idx) = graph.value_index.get(&value) else {
                    assert!(!hk);
                    continue;
                };
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
    fn batch_prune_matches_value_support() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(crate::AllDifferentPropagator::new(vec![a, b, c])));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(c).size(), 1);
        assert_eq!(engine.domain(c).min(), Some(3));
    }

    #[test]
    fn regin_scc_agrees_with_value_support_on_four_variables() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 4));
        let b = engine.new_variable(IntervalDomain::new(1, 4));
        let c = engine.new_variable(IntervalDomain::new(1, 4));
        let d = engine.new_variable(IntervalDomain::new(1, 5));
        let vars = vec![a, b, c, d];
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
        let components = tarjan_scc(&value_graph, graph.value_count);

        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            for value in collect_values(&ctx, var) {
                let hk = value_in_some_matching(&ctx, &vars, var, value);
                let Some(&value_idx) = graph.value_index.get(&value) else {
                    assert!(!hk);
                    continue;
                };
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
    fn regin_scc_agrees_with_value_support_on_random_small_domains() {
        let mut engine = Engine::new();
        let domain_specs = [(1, 4), (1, 4), (1, 5), (2, 5), (1, 3)];
        let vars: Vec<_> = domain_specs
            .iter()
            .map(|&(lo, hi)| engine.new_variable(IntervalDomain::new(lo, hi)))
            .collect();
        let ctx = ReadOnlyEngine(&engine);
        let graph = build_bipartite_graph(&ctx, &vars);
        if hopcroft_karp(&graph.adj, vars.len(), graph.value_count) != vars.len() {
            return;
        }
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let value_graph = build_regin_value_graph(&ctx, &vars, &graph, &pair_left);
        let components = tarjan_scc(&value_graph, graph.value_count);

        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            for value in collect_values(&ctx, var) {
                let hk = value_in_some_matching(&ctx, &vars, var, value);
                let Some(&value_idx) = graph.value_index.get(&value) else {
                    assert!(!hk);
                    continue;
                };
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
                    assert_regin_agrees_with_hk(&ReadOnlyEngine(&engine), &vars);
                }
            }

            let mut engine = Engine::new();
            let mut vars: Vec<_> = (0..n.saturating_sub(1))
                .map(|_| engine.new_variable(IntervalDomain::new(1, 4)))
                .collect();
            vars.push(engine.new_variable(IntervalDomain::new(1, if n == 2 { 4 } else { 5 })));
            assert_regin_agrees_with_hk(&ReadOnlyEngine(&engine), &vars);
        }
    }

    fn assert_regin_agrees_with_hk(ctx: &ReadOnlyEngine<'_>, vars: &[VariableId]) {
        let graph = build_bipartite_graph(ctx, vars);
        if hopcroft_karp(&graph.adj, vars.len(), graph.value_count) != vars.len() {
            return;
        }
        let (pair_left, pair_right) =
            hopcroft_karp_matching(&graph.adj, vars.len(), graph.value_count);
        let value_graph = build_regin_value_graph(ctx, vars, &graph, &pair_left);
        let components = tarjan_scc(&value_graph, graph.value_count);

        for (left, &var) in vars.iter().enumerate() {
            let matched = pair_left[left].expect("matched");
            for value in collect_values(ctx, var) {
                let hk = value_in_some_matching(ctx, vars, var, value);
                let Some(&value_idx) = graph.value_index.get(&value) else {
                    assert!(!hk);
                    continue;
                };
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

    struct ReadOnlyEngine<'a>(&'a Engine);

    impl PropagationContext for ReadOnlyEngine<'_> {
        fn domain(&self, var: VariableId) -> &dyn DomainView<Value = i32> {
            self.0.domain(var)
        }

        fn remove_below(&mut self, _: VariableId, _: i32) -> bool {
            false
        }

        fn remove_above(&mut self, _: VariableId, _: i32) -> bool {
            false
        }

        fn remove_value(&mut self, _: VariableId, _: i32) -> bool {
            false
        }

        fn fixed_value(&self, var: VariableId) -> Option<i32> {
            self.0.domain(var).fixed_value()
        }
    }
}
