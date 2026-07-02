# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.2.0
[0.1.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.1.0
