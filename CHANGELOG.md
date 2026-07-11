# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
