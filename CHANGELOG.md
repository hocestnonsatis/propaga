# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Lexicographic and Pareto multi-objective search now accept float and set-cardinality objectives (same typed targets as single-objective optimization).
- Incremental Pareto enumeration: DFS streams solutions via `solve_each` and updates the non-dominated front online (no full feasible-set buffer).
- Typed Pareto dominance cuts (`DominanceCutPropagator`) for int, float, and set-cardinality thresholds; Pareto search posts cuts between solutions for all objective types.
- `ForbiddenAssignmentPropagator` blocks rediscovery of int / float / set assignments during Pareto enumeration.
- `encode_forbidden_float` excludes a continuous float point via reified `≤ next_down ∨ ≥ next_up`, and Pareto blocking adds those reifs as search decisions.
- DFS float branching can split around registered blocked IEEE points; Pareto search feeds blocked float values into those split hints.
- `FloatNePropagator` prunes forbidden float values at interval endpoints; `FloatEqReif(false)` delegates to it; FlatZinc `float_lt` uses strict `¬(b ≤ a)`.
- `FloatLinearNePropagator` for `sum ≠ rhs` with bound-touch and unit-endpoint pruning; `ReifiedFloatLinearEq(false)` no longer posts contradictory `≤` and `≥` cuts.
- `ExtendedPropagationContext::tighten_set_cardinality` for set-cardinality bound updates during propagation.

### Fixed

- Lexicographic search restores the engine after each objective's branch-and-bound so interior optima are pinned correctly before optimizing the next priority.
- `FloatLeReifPropagator` with reif=false now tightens the correct float bounds (strict greater-than encoding).

## [1.0.0] - 2026-07-13

### Added

- Full MiniZinc FlatZinc 1.6 builtin support for the standard library workflow (int / bool / set / float).
- Set parameters and extended predicate parameter types (`var set`, `var float`).
- Float and set single-objective optimization (`minimize` / `maximize`).
- Stdlib global decompositions: `count`, `among`, `at_least`, `at_most`, `distribute`, `nvalue`, `sort`, lexicographic and monotonicity globals.
- Integer / float primitives: `int_plus`, `int_lin_ne`, `int_min`/`int_max`/`int_pow`, generic `min`/`max`, full bool builtin set.
- Float core: interval arithmetic, linear constraints, reified forms, unary transcendental ops (`float_sin`, `float_log2`, …).
- Float array builtins: `array_float_element`, `array_var_float_element`, `array_float_maximum`, `array_float_minimum`.
- Float membership: `float_dom`, `float_in`.
- Set builtins beyond union/intersect: equality, reified comparisons, `set_in`, etc.
- MiniZinc stdlib corpus (`benchmarks/minizinc/stdlib/`) with bundled FlatZinc fixtures and CI `minizinc-stdlib` job.
- `scripts/flatzinc-full-compat-report.sh` acceptance gate for models + stdlib.

### Fixed

- `set_in` propagation no longer requires the int domain to contain every set GLB element.
- Float linear `<=` / `>=` propagators now tighten the correct interval bounds.

### Changed

- `function` and `test` top-level declarations are skipped during parse (like `annotation`).
- Unknown `min` / `max` predicate calls dispatch to `int_*` or `float_*` by variable domain.
- Optimization objectives use typed `ObjectiveSpec` (int, float, set cardinality) with JSON/plain CLI output.

### Known limitations

- Lexicographic and Pareto multi-objective search remain int-only.
- Table-based int decompositions (`int_times`, `int_pow`, …) cap at 10 000 tuples.
- Float propagation is interval-based (conservative, not exact real arithmetic).

## [0.7.0] - 2026-07-12

### Added

- FlatZinc primitive constraints: `int_abs`, `int_times`, `int_div`, `int_mod`.
- FlatZinc bool primitives: `bool_not`, `bool_and`, `bool_or`.
- FlatZinc `automaton` global (compiled via `regular` propagator).
- `bool` and `float` FlatZinc parameters.
- Recursive nested predicate expansion with full constraint substitution.
- Top-level `annotation` statements are skipped during parse.
- `decompose` module in `propaga-flatzinc` for primitive constraint lowering.
- Compile corpus test (`crates/propaga-flatzinc/tests/compile_corpus.rs`).
- MiniZinc model corpus under `benchmarks/minizinc/models/`.
- `scripts/flatzinc-compat-report.sh` for local MiniZinc compatibility checks.
- Regression benchmarks: `int_abs.fzn`, `bool_logic.fzn`, `int_times.fzn`, `nested_predicate.fzn`, `automaton_chain.fzn`.

### Changed

- `incomplete` search annotations are tolerated (treated like `complete`).
- Unknown predicates now produce explicit compile errors instead of silent `PredicateCall` leftovers.

### Known limitations

- `int_times` / `int_div` / `int_mod` use table decomposition with a 10_000 tuple cap.
- Float parameters are parsed but not yet usable in float constraint expressions.
- `function` / `test` top-level declarations remain unsupported.

## [0.6.0] - 2026-07-11

### Added

- Set global propagators: `set_union`, `set_intersect`.
- Float global propagator: `float_times` with interval arithmetic.
- Model API: `set_union`, `set_intersect`, `float_times`.
- FlatZinc: `set_union`, `set_intersect`, `float_times` constraints.
- Benchmarks: `set_union.fzn`, `set_intersect.fzn`, `float_times.fzn`.
- `FloatDomain::times` and `FloatDomain::divide` interval helpers.

### Known limitations

- Set/float optimization objectives remain int-only.
- Float globals beyond `float_times` not yet supported.
- Set globals beyond union/intersect/card/subset not yet supported.

## [0.5.0] - 2026-07-10

### Added

- `AnyDomain` engine storage for int, set, and float variables.
- `SetIntervalDomain` with GLB/LUB/cardinality representation.
- Set propagators: `SetCardPropagator`, `SetSubsetPropagator`.
- Float propagators: `FloatLePropagator`, `FloatEqPropagator`.
- Model API: `set_var`, `float_var`, `set_card`, `set_subset`, `float_le`, `float_eq`.
- Typed search branching and `AssignmentValue` in solutions.
- FlatZinc subset: set/float variables and `set_card`, `set_subset`, `float_le`, `float_eq`.
- Benchmarks: `set_cardinality.fzn`, `float_bounds.fzn`.

### Known limitations

- Optimization (minimize/maximize/Pareto) remains int-only.
- Float propagation is interval-only (no global float constraints).
- Set globals beyond `set_card`/`set_subset` not yet supported.

## [0.4.0] - 2026-07-10

### Added

- Pareto-front multi-objective enumeration (`ParetoOptimization`, `Model::pareto_optimize`).
- FlatZinc `solve :: pareto([...]) satisfy` annotation and CLI JSON `pareto_solutions`.
- `regular` global constraint (DFA compiled to table propagator) with FlatZinc compile support.
- LCG clause propagator posting during search when `SearchConfig::clause_learning` is enabled.
- WASM demo build script (`scripts/build-wasm.sh`) and GitHub Pages deploy workflow.
- Benchmarks: `regular_chain.fzn`, `pareto_biobjective.fzn`.

### Changed

- `ClauseStore` learned clauses are posted as `ClausePropagator` instances on the engine.
- WASM demo page includes an N-Queens solver section.

### Known limitations

- Set/float domains are not yet wired into the propagation engine.
- Pareto enumeration collects all solutions before filtering (practical for small fronts).

## [0.3.0] - 2026-07-05

### Added

- `EngineCheckpoint` API with `checkpoint`, `restore_checkpoint`, and `fork_at_checkpoint` for parallel search workers.
- Parallel portfolio search using rayon when `--workers > 1` (FlatZinc `solve`, Sudoku, and portfolio API).
- Multi-constraint FlatZinc predicate bodies (`constraint A /\ constraint B` and semicolon chains).
- FlatZinc lexicographic multi-objective (`solve minimize x, y`); CLI JSON includes `objective_values`.
- Lazy clause pruning in DFS via `SearchConfig::clause_learning`.
- Benchmarks: `predicate_multi.fzn`, `lexicographic_multi.fzn`.

### Changed

- `CompiledInstance.objective` replaced by `objectives: Vec<ObjectiveSpec>`.
- `SolveGoal::Minimize` / `Maximize` now hold expression lists for lexicographic goals.
- Propagators implement `DynClone` for engine fork at checkpoint.

### Known limitations

- Set/float domains are not yet wired into the propagation engine.
- Predicate bodies still inline-expand only; nested predicate calls in bodies are rejected.
- `clause_learning` prunes branches but does not post learned clauses as propagators yet.

## [0.2.0] - 2026-07-02

### Added

- Schedule JSON output for `propaga schedule --format json`.
- Value orderings `indomain_split` and `indomain_median`.
- Activity-based variable ordering (VSIDS-style) and CLI/FlatZinc `activity` selector.
- Restart policies `restart_linear` and `restart_on_solution`.
- FlatZinc `bool_search` annotation support.
- Global propagators: `circuit`, `inverse`, `diffn`.
- Predicate inline expansion for simple single-constraint bodies.
- Cumulative constraints with variable duration/height arrays.
- Portfolio search (`--workers`, `--deterministic`) and lexicographic multi-objective optimization API.
- Set and float domain types in `propaga-domains`.
- `propaga-wasm` crate with Sudoku and N-Queens WASM bindings and browser demo.
- Lazy clause generation spike (`ClauseStore` in `propaga-search`).

### Changed

- FlatZinc unknown constraint names become predicate calls when a matching predicate is declared.

### Known limitations

- Portfolio search runs configurations sequentially on a shared engine (parallel rayon workers require engine snapshots).
- Set/float domains are not yet wired into the propagation engine.
- Predicate bodies support a single primitive constraint only.

## [0.1.0] - 2026-06-21

### Added

- Workspace of eight crates: `propaga-core`, `propaga-domains`, `propaga-engine`, `propaga-propagators`, `propaga-search`, `propaga-model`, `propaga-flatzinc`, `propaga-cli`.
- Propagation engine with explanation-aware trail and event scheduling.
- Domain implementations: interval, bitset, and hybrid.
- Built-in propagators: equality, linear, ordering, reified, all-different (GAC), GCC, table, element, cumulative, disjunctive, nogood.
- Search: MRV/DOM/DOM-W-DEG variable ordering, LCV value ordering, first-UIP nogood learning, Luby restarts, phase saving, branch-and-bound optimization.
- High-level `Model` API for constraint posting and solving.
- FlatZinc subset parser and compiler with satisfy/minimize/maximize.
- CLI (`propaga`): `sudoku`, `n-queens`, `solve` (single file and batch directory), `schedule` (JSON).
- CLI flags: `--stats`, `--format json`, `--all`, `--solutions`, `--time-limit`, `--no-learning`, `--restarts`, `--var-ordering`, `--value-ordering`, `--no-phase-saving`.
- Scheduling JSON format with cumulative, sequential, and disjunctive modes.
- Criterion benchmarks for propagator micro-benchmarks.
- FlatZinc compatibility matrix at `benchmarks/minizinc/COMPATIBILITY.md`.

### Known limitations

- FlatZinc is a subset: no set/float variables in the engine, predicate bodies limited to one primitive constraint.
- Multi-objective optimization supports lexicographic API; FlatZinc multi-objective directives are not yet parsed.
- See [ROADMAP.md](ROADMAP.md) for planned features.

[0.3.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.3.0
[0.2.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.2.0
[0.1.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.1.0
