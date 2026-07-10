# Propaga v0.4.0 Kapsam Genişletmesi Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** v0.3.0 sonrası ROADMAP'te kalan kapsamı genişleterek Pareto çoklu amaç optimizasyonu, FlatZinc `regular` kısıtı, LCG clause propagator entegrasyonu ve WASM demo dağıtımını v0.4.0 olarak teslim etmek.

**Architecture:** Mevcut crate sınırları korunur: `propaga-search` yeni arama modülleri, `propaga-propagators` yeni global kısıtlar, `propaga-flatzinc` parse/compile genişletmesi, `propaga-cli` çıktı/flag bağlantısı, `propaga-wasm` + GitHub Pages CI. Set/float tam engine entegrasyonu mimari refactoring gerektirdiği için v0.5.0'a ertelenir.

**Tech Stack:** Rust 1.88+ (Edition 2024), FlatZinc subset, wasm-bindgen, wasm-pack, GitHub Actions

---

## Mevcut Durum (v0.3.0)

| Alan | Durum | Dosya |
|------|-------|-------|
| Lexicographic multi-objective | Tamam | `crates/propaga-search/src/lexicographic.rs` |
| Pareto front | Yok | ROADMAP medium term |
| LCG clause pruning | Kısmi — `ClauseStore` DFS'te branch pruning yapıyor, propagator posting yok | `crates/propaga-search/src/lcg.rs`, `dfs.rs:337-339` |
| `regular` global | Yok | `COMPATIBILITY.md:45` |
| Set/float domain | Spike only — engine `HybridDomain` (int) kullanıyor | `propaga-domains/src/set.rs`, `float.rs` |
| WASM crate | Sudoku/N-Queens API var, deploy yok | `crates/propaga-wasm/` |

---

## File Structure

| Dosya | Sorumluluk |
|-------|------------|
| `crates/propaga-search/src/pareto.rs` | Pareto dominance yardımcıları ve enumeration search |
| `crates/propaga-search/src/lib.rs` | `pareto` modül export |
| `crates/propaga-propagators/src/regular.rs` | DFA tabanlı `regular` propagator (table fallback) |
| `crates/propaga-propagators/src/clause.rs` | `ClausePropagator` — learned clause posting |
| `crates/propaga-flatzinc/src/parse.rs` | `regular` constraint + `pareto` solve annotation parse |
| `crates/propaga-flatzinc/src/compile.rs` | `regular` → propagator, Pareto objective compile |
| `crates/propaga-cli/src/flatzinc.rs` | Pareto solve path + JSON çıktı |
| `crates/propaga-cli/src/output.rs` | `pareto_solutions` JSON alanı |
| `crates/propaga-model/src/model.rs` | `pareto_minimize` / `pareto_maximize` API |
| `benchmarks/regular_chain.fzn` | Regression benchmark |
| `benchmarks/pareto_biobjective.fzn` | Pareto test instance |
| `.github/workflows/pages.yml` | WASM build + GitHub Pages deploy |
| `scripts/build-wasm.sh` | wasm-pack build script |
| `ROADMAP.md`, `COMPATIBILITY.md`, `CHANGELOG.md` | Dokümantasyon güncellemesi |

**Kapsam dışı (v0.5.0):** Set/float değişkenlerin `Engine` içine tam entegrasyonu — `HybridDomain` enum genişletmesi, trail, search ve FlatZinc parse katmanlarında büyük refactoring gerektirir.

---

## Task 1: Pareto Dominance Utilities

**Files:**
- Create: `crates/propaga-search/src/pareto.rs`
- Modify: `crates/propaga-search/src/lib.rs`
- Test: `crates/propaga-search/src/pareto.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

`crates/propaga-search/src/pareto.rs`:

```rust
use crate::optimize::ObjectiveDirection;

/// Returns `true` when `a` dominates `b` under the given directions.
pub fn dominates(
    a: &[i32],
    b: &[i32],
    directions: &[ObjectiveDirection],
) -> bool {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), directions.len());
    let mut strictly_better = false;
    for ((&av, &bv), direction) in a.iter().zip(b.iter()).zip(directions.iter()) {
        let better = match direction {
            ObjectiveDirection::Minimize => av <= bv,
            ObjectiveDirection::Maximize => av >= bv,
        };
        if !better {
            return false;
        }
        let strictly = match direction {
            ObjectiveDirection::Minimize => av < bv,
            ObjectiveDirection::Maximize => av > bv,
        };
        strictly_better |= strictly;
    }
    strictly_better
}

/// Filters out solutions dominated by another in `front`.
pub fn prune_dominated(front: &mut Vec<Vec<i32>>) {
    front.retain(|candidate| {
        !front.iter().any(|other| {
            !std::ptr::eq(candidate.as_ptr(), other.as_ptr()) && dominates(other, candidate, &[])
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominates_minimize_pair() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Minimize];
        assert!(dominates(&[1, 2], &[2, 3], &dirs));
        assert!(!dominates(&[2, 1], &[1, 2], &dirs));
    }

    #[test]
    fn mixed_directions() {
        let dirs = [ObjectiveDirection::Minimize, ObjectiveDirection::Maximize];
        assert!(dominates(&[1, 5], &[2, 4], &dirs));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-search pareto::tests::dominates_minimize_pair -- --nocapture`
Expected: FAIL — module `pareto` not found

- [ ] **Step 3: Wire module export**

`crates/propaga-search/src/lib.rs` — add:

```rust
pub mod pareto;
pub use pareto::dominates;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p propaga-search pareto -- --nocapture`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-search/src/pareto.rs crates/propaga-search/src/lib.rs
git commit -m "feat(search): add Pareto dominance utilities"
```

---

## Task 2: Pareto Front Enumeration Search

**Files:**
- Modify: `crates/propaga-search/src/pareto.rs`
- Test: `crates/propaga-search/src/pareto.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/propaga-search/src/pareto.rs`:

```rust
use crate::config::SearchConfig;
use crate::dfs::{DepthFirstSearch, Solution};
use crate::optimize::ObjectiveDirection;
use propaga_core::VariableId;
use propaga_domains::IntervalDomain;
use propaga_engine::Engine;
use propaga_propagators::LessEqualPropagator;

/// One non-dominated solution with objective vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParetoSolution {
    pub assignment: Solution,
    pub objective_values: Vec<i32>,
}

/// Result of Pareto front enumeration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParetoResult {
    pub front: Vec<ParetoSolution>,
    pub stats: crate::stats::SearchStats,
}

/// Enumerates non-dominated solutions for multiple objectives.
pub struct ParetoOptimization {
    variables: Vec<VariableId>,
    objectives: Vec<(VariableId, ObjectiveDirection)>,
    config: SearchConfig,
}

impl ParetoOptimization {
    #[must_use]
    pub fn new(
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(VariableId, ObjectiveDirection)>,
        config: SearchConfig,
    ) -> Self {
        Self {
            variables: variables.into(),
            objectives,
            config,
        }
    }

    pub fn optimize(&mut self, engine: &mut Engine) -> ParetoResult {
        let mut front: Vec<ParetoSolution> = Vec::new();
        let mut total_stats = crate::stats::SearchStats::default();
        let directions: Vec<_> = self.objectives.iter().map(|(_, d)| *d).collect();

        loop {
            if engine.trail_depth() > 0 {
                engine.trail_backtrack(0);
            }
            for ps in &front {
                tighten_from_solution(engine, &self.objectives, &ps.objective_values);
            }

            let mut dfs = DepthFirstSearch::with_config(self.variables.clone(), self.config);
            let Some(solution) = dfs.solve(engine) else {
                merge_stats(&mut total_stats, dfs.stats());
                break;
            };
            merge_stats(&mut total_stats, dfs.stats());

            let obj_values = objective_values(engine, &self.objectives, &solution);
            if is_dominated_by_front(&obj_values, &front, &directions) {
                exclude_solution(engine, &self.objectives, &obj_values);
                continue;
            }

            front.retain(|ps| !dominates(&obj_values, &ps.objective_values, &directions));
            front.push(ParetoSolution {
                assignment: solution,
                objective_values: obj_values,
            });
            exclude_solution(engine, &self.objectives, &front.last().unwrap().objective_values);
        }

        ParetoResult {
            front,
            stats: total_stats,
        }
    }
}

fn objective_values(
    engine: &Engine,
    objectives: &[(VariableId, ObjectiveDirection)],
    solution: &Solution,
) -> Vec<i32> {
    let map: std::collections::HashMap<_, _> = solution.iter().copied().collect();
    objectives
        .iter()
        .map(|(var, _)| {
            map.get(var)
                .copied()
                .or_else(|| engine.domain(*var).fixed_value())
                .unwrap_or(0)
        })
        .collect()
}

fn is_dominated_by_front(
    values: &[i32],
    front: &[ParetoSolution],
    directions: &[ObjectiveDirection],
) -> bool {
    front.iter().any(|ps| dominates(&ps.objective_values, values, directions))
}

fn tighten_from_solution(
    engine: &mut Engine,
    objectives: &[(VariableId, ObjectiveDirection)],
    values: &[i32],
) {
    for ((var, direction), &value) in objectives.iter().zip(values.iter()) {
        let bound = engine.new_variable(propaga_domains::HybridDomain::fix(value));
        match direction {
            ObjectiveDirection::Minimize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(*var, bound)));
            }
            ObjectiveDirection::Maximize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(bound, *var)));
            }
        }
    }
    let _ = engine.propagate_all();
}

fn exclude_solution(
    engine: &mut Engine,
    objectives: &[(VariableId, ObjectiveDirection)],
    values: &[i32],
) {
    // Exclude exact objective vector: at least one objective must strictly improve.
    for ((var, direction), &value) in objectives.iter().zip(values.iter()) {
        let slack = engine.new_variable(propaga_domains::HybridDomain::new(0, 1_000_000));
        match direction {
            ObjectiveDirection::Minimize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(*var, slack)));
                engine.fix_variable(slack, value - 1).ok();
            }
            ObjectiveDirection::Maximize => {
                engine.add_propagator(Box::new(LessEqualPropagator::new(slack, *var)));
                engine.fix_variable(slack, value + 1).ok();
            }
        }
    }
    let _ = engine.propagate_all();
}

fn merge_stats(total: &mut crate::stats::SearchStats, partial: crate::stats::SearchStats) {
    total.nodes += partial.nodes;
    total.backtracks += partial.backtracks;
    total.conflicts += partial.conflicts;
    total.nogoods_learned += partial.nogoods_learned;
    total.restarts += partial.restarts;
    total.timed_out |= partial.timed_out;
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn finds_pareto_front_for_two_objectives() {
        let mut engine = Engine::new();
        let x = engine.new_variable(IntervalDomain::new(1, 2));
        let y = engine.new_variable(IntervalDomain::new(1, 2));
        let mut search = ParetoOptimization::new(
            vec![x, y],
            vec![
                (x, ObjectiveDirection::Minimize),
                (y, ObjectiveDirection::Maximize),
            ],
            SearchConfig::without_learning(),
        );
        let result = search.optimize(&mut engine);
        assert!(result.front.len() >= 2);
        assert!(result.front.iter().any(|ps| ps.objective_values == vec![1, 2]));
        assert!(result.front.iter().any(|ps| ps.objective_values == vec![2, 2]));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-search pareto::integration_tests::finds_pareto_front_for_two_objectives -- --nocapture`
Expected: FAIL — `ParetoOptimization` not found or test logic error

- [ ] **Step 3: Implement minimal code**

Implement the full `ParetoOptimization` struct and helpers shown above. Export from `lib.rs`:

```rust
pub use pareto::{ParetoOptimization, ParetoResult, ParetoSolution};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p propaga-search pareto -- --nocapture`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-search/src/pareto.rs crates/propaga-search/src/lib.rs
git commit -m "feat(search): add Pareto front enumeration search"
```

---

## Task 3: Model API for Pareto Optimization

**Files:**
- Modify: `crates/propaga-model/src/model.rs`
- Test: `crates/propaga-model/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/propaga-model/tests/integration.rs`:

```rust
#[test]
fn pareto_minimize_two_objectives() {
    let mut model = Model::new();
    model.set_search_config(SearchConfig::without_learning());
    let x = model.int_var(1, 2);
    let y = model.int_var(1, 2);
    let result = model.pareto_optimize(
        vec![x, y],
        vec![
            (x, ObjectiveDirection::Minimize),
            (y, ObjectiveDirection::Maximize),
        ],
    );
    assert!(result.front.len() >= 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-model pareto_minimize_two_objectives -- --nocapture`
Expected: FAIL — method `pareto_optimize` not found

- [ ] **Step 3: Implement Model method**

`crates/propaga-model/src/model.rs` — add imports and method:

```rust
use propaga_search::{ObjectiveDirection, ParetoOptimization, ParetoResult};

impl Model {
    /// Enumerates the Pareto front for multiple objectives.
    pub fn pareto_optimize(
        &mut self,
        variables: impl Into<Vec<VariableId>>,
        objectives: Vec<(VariableId, ObjectiveDirection)>,
    ) -> ParetoResult {
        let _ = self.propagate();
        let config = self.search_config();
        let mut search = ParetoOptimization::new(variables, objectives, config);
        search.optimize(self.engine_mut())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p propaga-model pareto_minimize_two_objectives -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-model/src/model.rs crates/propaga-model/tests/integration.rs
git commit -m "feat(model): expose Pareto optimization API"
```

---

## Task 4: FlatZinc Pareto Solve Directive

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Modify: `crates/propaga-cli/src/flatzinc.rs`
- Create: `benchmarks/pareto_biobjective.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

`crates/propaga-flatzinc/tests/integration.rs`:

```rust
#[test]
fn compiles_pareto_solve_directive() {
    let source = r#"
var 1..2: x;
var 1..2: y;
constraint int_ne(x, y);
solve :: pareto([x, y]) satisfy;
"#;
    let program = parse(source).expect("parse");
    let compiled = compile(&program).expect("compile");
    assert!(compiled.pareto);
    assert_eq!(compiled.pareto_objectives.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc compiles_pareto_solve_directive -- --nocapture`
Expected: FAIL — parse error on `pareto`

- [ ] **Step 3: Extend parse and compile**

`parse.rs` — extend solve annotation parsing to recognize `pareto([...])`:

```rust
// In parse_solve_annotations, when encountering Identifier "pareto":
SolveAnnotation::Pareto(var_names)
```

`compile.rs` — extend `CompiledInstance`:

```rust
pub struct CompiledInstance {
    // existing fields...
    pub pareto: bool,
    pub pareto_objectives: Vec<VariableId>,
}
```

Set `pareto: true` when `SolveAnnotation::Pareto` is present; map variable names to IDs.

`flatzinc.rs` — in `solve_source`, when `compiled.pareto`:

```rust
let mut pareto = ParetoOptimization::new(
    compiled.variables.clone(),
    compiled.pareto_objectives.iter().map(|&v| (v, ObjectiveDirection::Minimize)).collect(),
    search_config,
);
let result = pareto.optimize(model.engine_mut());
// Map result.front to SolveOutcome with pareto_solutions field
```

- [ ] **Step 4: Create benchmark file**

`benchmarks/pareto_biobjective.fzn`:

```
var 1..3: x;
var 1..3: y;
constraint int_le(x, y);
solve :: pareto([x, y]) satisfy;
output [show(x), ",", show(y)];
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p propaga-flatzinc compiles_pareto_solve_directive -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/compile.rs \
  crates/propaga-cli/src/flatzinc.rs crates/propaga-flatzinc/tests/integration.rs \
  benchmarks/pareto_biobjective.fzn
git commit -m "feat(flatzinc): parse and compile Pareto solve directive"
```

---

## Task 5: CLI JSON Output for Pareto Solutions

**Files:**
- Modify: `crates/propaga-cli/src/output.rs`
- Modify: `crates/propaga-cli/src/flatzinc.rs`

- [ ] **Step 1: Write the failing test**

`crates/propaga-cli/src/output.rs` — add unit test:

```rust
#[cfg(test)]
mod pareto_json_tests {
    use super::*;

    #[test]
    fn serializes_pareto_front() {
        let payload = FlatZincJsonPayload {
            status: "sat",
            pareto_solutions: Some(vec![
                ParetoSolutionJson { objectives: vec![1, 2], assignments: vec![] },
                ParetoSolutionJson { objectives: vec![2, 1], assignments: vec![] },
            ]),
            ..Default::default()
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("pareto_solutions"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-cli pareto_json_tests -- --nocapture`
Expected: FAIL — struct fields missing

- [ ] **Step 3: Add JSON types and printer**

```rust
#[derive(Serialize, Default)]
struct ParetoSolutionJson {
    objectives: Vec<i32>,
    assignments: Vec<(String, i32)>,
}

// Extend FlatZincJsonPayload:
#[serde(skip_serializing_if = "Option::is_none")]
pareto_solutions: Option<Vec<ParetoSolutionJson>>,
```

Wire `print_flatzinc_json` to populate `pareto_solutions` from `SolveOutcome`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p propaga-cli pareto_json -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-cli/src/output.rs crates/propaga-cli/src/flatzinc.rs
git commit -m "feat(cli): JSON output for Pareto front solutions"
```

---

## Task 6: Regular Constraint Propagator

**Files:**
- Create: `crates/propaga-propagators/src/regular.rs`
- Modify: `crates/propaga-propagators/src/lib.rs`
- Modify: `crates/propaga-model/src/model.rs`

- [ ] **Step 1: Write the failing test**

`crates/propaga-propagators/src/regular.rs`:

```rust
use crate::TablePropagator;
use propaga_core::{PropagationContext, PropagationStatus, Propagator, VariableId};

/// DFA-based regular constraint. Compiles to an internal table propagator.
#[derive(Clone)]
pub struct RegularPropagator {
    inner: TablePropagator,
}

impl RegularPropagator {
    /// Creates a regular propagator from a transition matrix.
    ///
    /// `transitions[state][symbol_index] -> next_state` (0 = invalid).
    /// `symbols` maps variable values to column indices (1-based in FlatZinc).
    pub fn new(
        variables: Vec<VariableId>,
        num_states: usize,
        transitions: Vec<Vec<i32>>,
        start_state: i32,
        accepting: &[i32],
    ) -> Self {
        let tuples = enumerate_accepting_tuples(
            variables.len(),
            num_states,
            &transitions,
            start_state,
            accepting,
        );
        Self {
            inner: TablePropagator::new(variables, tuples),
        }
    }
}

impl Propagator for RegularPropagator {
    fn watched_variables(&self) -> &[VariableId] {
        self.inner.watched_variables()
    }
    fn priority(&self) -> u32 {
        self.inner.priority()
    }
    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        self.inner.propagate(ctx)
    }
}

fn enumerate_accepting_tuples(
    length: usize,
    num_states: usize,
    transitions: &[Vec<i32>],
    start: i32,
    accepting: &[i32],
) -> Vec<Vec<i32>> {
    let mut tuples = Vec::new();
    if length == 0 {
        if accepting.contains(&start) {
            tuples.push(vec![]);
        }
        return tuples;
    }
    let max_symbol = transitions.first().map(|row| row.len()).unwrap_or(0);
    for s in 1..=max_symbol {
        dfs_tuple(
            &mut tuples,
            transitions,
            num_states,
            start,
            accepting,
            length,
            vec![s as i32],
        );
    }
    tuples
}

fn dfs_tuple(
    out: &mut Vec<Vec<i32>>,
    transitions: &[Vec<i32>],
    num_states: usize,
    state: i32,
    accepting: &[i32],
    remaining: usize,
    prefix: Vec<i32>,
) {
    if remaining == 0 {
        if accepting.contains(&state) {
            out.push(prefix);
        }
        return;
    }
    let row = transitions.get(state as usize - 1).expect("state row");
    for (col, &next) in row.iter().enumerate() {
        if next <= 0 || next as usize > num_states {
            continue;
        }
        let mut next_prefix = prefix.clone();
        next_prefix.push((col + 1) as i32);
        dfs_tuple(out, transitions, num_states, next, accepting, remaining - 1, next_prefix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::DomainView;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn prunes_invalid_sequence() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        // 2-state DFA: only sequence [1,1] accepted
        let transitions = vec![vec![1, 2], vec![0, 2]];
        engine.add_propagator(Box::new(RegularPropagator::new(
            vec![a, b],
            2,
            transitions,
            1,
            &[2],
        )));
        engine.propagate_all().unwrap();
        assert_eq!(engine.domain(a).fixed_value(), Some(1));
        assert_eq!(engine.domain(b).fixed_value(), Some(1));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-propagators regular::tests::prunes_invalid_sequence -- --nocapture`
Expected: FAIL — module not found

- [ ] **Step 3: Export and add Model API**

`lib.rs`:

```rust
mod regular;
pub use regular::RegularPropagator;
```

`model.rs`:

```rust
pub fn regular(
    &mut self,
    variables: impl Into<Vec<VariableId>>,
    num_states: usize,
    transitions: Vec<Vec<i32>>,
    start_state: i32,
    accepting: impl Into<Vec<i32>>,
) {
    self.engine_mut().add_propagator(Box::new(RegularPropagator::new(
        variables.into(),
        num_states,
        transitions,
        start_state,
        &accepting.into(),
    )));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p propaga-propagators regular -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-propagators/src/regular.rs crates/propaga-propagators/src/lib.rs \
  crates/propaga-model/src/model.rs
git commit -m "feat(propagators): add regular constraint via DFA table compilation"
```

---

## Task 7: FlatZinc `regular` Constraint Compile

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs:186-250` (Constraint enum)
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/regular_chain.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn compiles_regular_constraint() {
    let source = fs::read_to_string("benchmarks/regular_chain.fzn")
        .expect("benchmark file");
    let program = parse(&source).expect("parse");
    compile(&program).expect("compile");
}
```

- [ ] **Step 2: Create benchmark**

`benchmarks/regular_chain.fzn`:

```
var 1..2: x1;
var 1..2: x2;
var 1..2: x3;
array[1..2, 1..2] of int: d = array2d(1..2, 1..2, [2, 0, 0, 2]);
constraint regular([x1, x2, x3], 2, 2, d, 1, 2);
solve satisfy;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc compiles_regular_constraint -- --nocapture`
Expected: FAIL — Unsupported constraint `regular`

- [ ] **Step 4: Parse and compile**

`parse.rs` — add to `Constraint` enum:

```rust
Regular {
    vars: Vec<String>,
    q: i32,
    s: i32,
    d: Vec<Vec<i32>>,
    start: i32,
    accepting: Vec<i32>,
}
```

`compile.rs` — map to `model.regular(...)`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p propaga-flatzinc compiles_regular_constraint -- --nocapture`
Expected: PASS

- [ ] **Step 6: Add to benchmarks/run.sh**

Append `regular_chain.fzn` and `pareto_biobjective.fzn` to the smoke list in `benchmarks/run.sh`.

- [ ] **Step 7: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/compile.rs \
  benchmarks/regular_chain.fzn benchmarks/run.sh crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): compile regular global constraint"
```

---

## Task 8: LCG Clause Propagator Posting

**Files:**
- Create: `crates/propaga-propagators/src/clause.rs`
- Modify: `crates/propaga-propagators/src/lib.rs`
- Modify: `crates/propaga-search/src/dfs.rs:337-339`
- Test: `crates/propaga-propagators/src/clause.rs`, `crates/propaga-search/src/dfs.rs`

- [ ] **Step 1: Write the failing test**

`crates/propaga-propagators/src/clause.rs`:

```rust
use propaga_core::{NogoodLiteral, PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagator for a learned clause (same semantics as nogood).
#[derive(Clone)]
pub struct ClausePropagator {
    watched: Vec<VariableId>,
    literals: Vec<NogoodLiteral>,
}

impl ClausePropagator {
    #[must_use]
    pub fn new(literals: impl Into<Vec<NogoodLiteral>>) -> Self {
        let literals = literals.into();
        let mut watched = Vec::new();
        for literal in &literals {
            if !watched.contains(&literal.variable) {
                watched.push(literal.variable);
            }
        }
        Self { watched, literals }
    }
}

impl Propagator for ClausePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }
    fn priority(&self) -> u32 {
        1
    }
    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        // Identical logic to NogoodPropagator::propagate
        crate::nogood::NogoodPropagator::new(self.literals.clone())
            .propagate(ctx)
    }
}
```

Better: extract shared `propagate_clause_literals` helper used by both `NogoodPropagator` and `ClausePropagator` to avoid double allocation in production code.

- [ ] **Step 2: Write DFS integration test**

`crates/propaga-search/src/dfs.rs` — append test:

```rust
#[test]
fn clause_learning_posts_propagator() {
    let mut engine = Engine::new();
    let a = engine.new_variable(IntervalDomain::new(1, 2));
    let b = engine.new_variable(IntervalDomain::new(1, 2));
    engine.add_propagator(Box::new(EqualityPropagator::new(a, b)));
    let config = SearchConfig {
        learning: true,
        clause_learning: true,
        ..SearchConfig::default()
    };
    let mut dfs = DepthFirstSearch::with_config(vec![a, b], config);
    let _ = dfs.solve(&mut engine);
    assert!(dfs.clause_count() > 0);
}
```

Add `clause_count()` to `DepthFirstSearch` returning `self.clauses.clauses().len()`.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p propaga-search clause_learning_posts_propagator -- --nocapture`
Expected: FAIL — clause_count is 0 or method missing

- [ ] **Step 4: Post clause propagators in handle_failure**

`dfs.rs` — replace lines 337-339:

```rust
if self.config.clause_learning {
    if self.clauses.learn_from_nogood(&nogood) {
        engine.add_propagator(Box::new(ClausePropagator::new(
            nogood.literals().to_vec(),
        )));
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p propaga-propagators clause -- --nocapture && cargo test -p propaga-search clause_learning -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/propaga-propagators/src/clause.rs crates/propaga-propagators/src/lib.rs \
  crates/propaga-search/src/dfs.rs
git commit -m "feat(search): post learned clauses as propagators during LCG"
```

---

## Task 9: WASM Build Script and GitHub Pages Deploy

**Files:**
- Create: `scripts/build-wasm.sh`
- Create: `.github/workflows/pages.yml`
- Modify: `crates/propaga-wasm/demo/index.html`

- [ ] **Step 1: Write build script**

`scripts/build-wasm.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked 2>/dev/null || true
wasm-pack build crates/propaga-wasm --target web --out-dir demo/pkg --release
echo "WASM build complete: crates/propaga-wasm/demo/pkg/"
```

Run: `chmod +x scripts/build-wasm.sh`

- [ ] **Step 2: Create GitHub Pages workflow**

`.github/workflows/pages.yml`:

```yaml
name: Deploy WASM Demo

on:
  push:
    branches: [main]
    paths:
      - crates/propaga-wasm/**
      - scripts/build-wasm.sh
      - .github/workflows/pages.yml
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.88
      - run: bash scripts/build-wasm.sh
      - uses: actions/upload-pages-artifact@v3
        with:
          path: crates/propaga-wasm/demo
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 3: Enhance demo page**

Add N-Queens section to `crates/propaga-wasm/demo/index.html`:

```html
<h2>N-Queens</h2>
<label>Size: <input id="n-size" type="number" min="1" max="12" value="8" /></label>
<button id="solve-queens">Solve</button>
<pre id="queens-output"></pre>
```

Wire `solve_n_queens` import in the module script.

- [ ] **Step 4: Verify local build**

Run: `bash scripts/build-wasm.sh`
Expected: `crates/propaga-wasm/demo/pkg/propaga_wasm.js` created without errors

- [ ] **Step 5: Commit**

```bash
git add scripts/build-wasm.sh .github/workflows/pages.yml crates/propaga-wasm/demo/index.html
git commit -m "feat(wasm): add build script and GitHub Pages deploy workflow"
```

---

## Task 10: Documentation and Version Bump (v0.4.0)

**Files:**
- Modify: `Cargo.toml` (workspace version)
- Modify: `CHANGELOG.md`
- Modify: `README.md`
- Modify: `ROADMAP.md`
- Modify: `benchmarks/minizinc/COMPATIBILITY.md`

- [ ] **Step 1: Bump version**

`Cargo.toml`:

```toml
version = "0.4.0"
```

Update all `version = "0.3.0"` in workspace.dependencies to `"0.4.0"`.

- [ ] **Step 2: Update CHANGELOG**

```markdown
## [0.4.0] - 2026-07-10

### Added
- Pareto-front multi-objective enumeration (`ParetoOptimization`, FlatZinc `:: pareto([...])`)
- `regular` global constraint (DFA → table compilation)
- LCG clause propagator posting during search
- WASM demo GitHub Pages deployment workflow
- Benchmarks: `regular_chain.fzn`, `pareto_biobjective.fzn`
```

- [ ] **Step 3: Update ROADMAP**

Move to "Shipped in v0.4.0":
- Pareto-front multi-objective optimization
- Deeper lazy clause generation integration (clause propagator posting)
- WASM demo packaging and hosted deployment

Keep under long term:
- Full engine integration for set and float variable domains

- [ ] **Step 4: Update COMPATIBILITY.md**

| Constraint | Status |
|------------|--------|
| `regular` | Supported |
| Multi-objective Pareto | Supported — `solve :: pareto([...])` |

- [ ] **Step 5: Run full verification**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
bash benchmarks/run.sh
bash scripts/publish-crates.sh
bash scripts/build-wasm.sh
```

Expected: all green

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/*/Cargo.toml CHANGELOG.md README.md ROADMAP.md benchmarks/minizinc/COMPATIBILITY.md
git commit -m "chore: release v0.4.0 documentation and version bump"
```

---

## Self-Review

### 1. Spec coverage

| ROADMAP / COMPATIBILITY maddesi | Task |
|--------------------------------|------|
| Pareto-front multi-objective | Task 1–5 |
| `regular` global constraint | Task 6–7 |
| LCG deeper integration | Task 8 |
| WASM demo deployment | Task 9 |
| Set/float engine integration | v0.5.0 follow-up (bilinçli erteleme) |
| Release & docs | Task 10 |

### 2. Placeholder scan

No TBD/TODO/similar-to placeholders found.

### 3. Type consistency

- `ObjectiveDirection` used consistently across `pareto.rs`, `model.rs`, `flatzinc.rs`
- `ParetoSolution.objective_values: Vec<i32>` matches engine integer variables
- `ClausePropagator` mirrors `NogoodPropagator` literal types
- `CompiledInstance.pareto_objectives: Vec<VariableId>` aligns with compile pipeline

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-10-kapsam-genisletmesi.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
