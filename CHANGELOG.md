# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

- FlatZinc is a subset: no set/float variables, no `predicate`/`function` definitions, search annotations ignored.
- Single-objective optimization only.
- See [ROADMAP.md](ROADMAP.md) for planned features.

[0.1.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.1.0
