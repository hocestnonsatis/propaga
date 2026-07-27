# Propaga

A propagator-based constraint solver written in Rust.

Propaga combines a typed propagation engine with pluggable domains, composable propagators, and conflict-driven search. Use it as a library via the `Model` API, from FlatZinc files, or through the `propaga` CLI.

**v1.0.0** (FlatZinc milestone) — [crates.io](https://crates.io/crates/propaga-cli) · [GitHub Releases](https://github.com/hocestnonsatis/propaga/releases) · [CHANGELOG](CHANGELOG.md)

## Installation

Requires Rust **1.88+** (Edition 2024).

```bash
cargo install propaga-cli
propaga --help
```

Prebuilt binaries: [GitHub Releases](https://github.com/hocestnonsatis/propaga/releases)

**As a library** ([docs.rs](https://docs.rs/propaga-core)):

```toml
propaga-core = "0.6"
propaga-model = "0.6"
propaga-flatzinc = "0.6"
```

FlatZinc support covers the MiniZinc **FlatZinc 1.6 stdlib workflow** (compile `.mzn` → solve `.fzn`). See [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) for the full matrix, including float interval + IEEE-hole semantics.

## Workspace

| Crate | Role |
|-------|------|
| `propaga-core` | Variables, domains, propagators, explanations, nogoods |
| `propaga-domains` | Interval, bitset, and hybrid domain implementations |
| `propaga-engine` | Propagation engine with trail and event scheduling |
| `propaga-propagators` | Built-in global and primitive propagators |
| `propaga-search` | DFS search, nogood learning, restarts, optimization |
| `propaga-model` | High-level modeling API |
| `propaga-flatzinc` | FlatZinc parser and compiler |
| `propaga-cli` | Command-line interface |

## Capabilities

### Propagation

Equality, disequality, linear constraints, ordering (`<=`, `<`), reified comparisons, all-different (GAC), global cardinality, table, element, cumulative, and disjunctive propagators. Explanation-aware trail with synchronized backtracking.

### Search

MRV, DOM, DOM/W-DEG, activity-based, and input-order variable ordering; ascending, descending, LCV, split, reverse-split, median, random, and interval value ordering; first-UIP nogood learning; optional lazy clause pruning; Luby, constant, geometric, linear, and on-solution restarts; phase saving; parallel portfolio search (`--workers`, including FlatZinc `seq_search` phases); lexicographic multi-objective optimization; branch-and-bound for single-objective optimization.

### FlatZinc

Parses and compiles FlatZinc 1.6 builtins for the MiniZinc stdlib workflow: integer, bool, set, and float variables; linear and global constraints; reified forms; `output` directives; `solve satisfy | minimize | maximize` (including float and set-cardinality objectives); lexicographic and Pareto objectives (int, float, set-cardinality); search annotations (`int_search`, `bool_search`, `float_search`, `set_search`, `seq_search`, `restart_*`); and user `predicate` declarations with nested expansion. Batch solving with `propaga solve --dir`. CLI flags override annotation defaults when explicitly set.

Full constraint matrix: [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md). MiniZinc workflow and stdlib corpus: [benchmarks/minizinc/README.md](benchmarks/minizinc/README.md).

### Scheduling

JSON input format for cumulative, sequential, and disjunctive scheduling problems. Per-task resource demand and multiple scheduling modes.

## CLI

```
propaga sudoku [--puzzle ... | --file ...]
propaga n-queens [--size N] [--visual]
propaga solve --file model.fzn | --dir benchmarks/
propaga schedule --file schedule.json
```

Global options:

| Flag | Default | Description |
|------|---------|-------------|
| `--format` | `plain` | `plain` or `json` |
| `--stats` | off | Print search statistics |
| `--all` | off | Enumerate all solutions |
| `--solutions N` | — | Cap enumeration with `--all` |
| `--time-limit SECS` | — | Wall-clock cutoff (`TIMEOUT` / `status: timeout`) |
| `--no-learning` | off | Disable nogood learning |
| `--restarts` | (from annotation or `luby`) | `none`, `luby`, `luby:N`, `constant:N`, or `geometric:B:N` |
| `--var-ordering` | (from annotation or `mrv`) | `mrv`, `dom`, `dom-wdeg`, `input-order`, `activity` |
| `--value-ordering` | (from annotation or `asc`) | `asc`, `desc`, `lcv`, `split`, `reverse-split`, `median`, `random`, `interval` |
| `--no-phase-saving` | off | Disable phase saving |
| `--workers N` | `1` | Portfolio worker count for `solve` and puzzles |
| `--deterministic` | off | Use only the base search configuration in portfolio mode |

## Quick start

From a clone:

```bash
cargo test
cargo run -p propaga-cli -- sudoku --stats
cargo run -p propaga-cli -- n-queens --size 8 --visual
cargo run -p propaga-cli -- solve --file benchmarks/magic_square.fzn --stats
cargo run -p propaga-cli -- schedule --file benchmarks/schedule_cumulative.json --stats
bash benchmarks/run.sh
```

Examples in `examples/`. Micro-benchmarks: `cargo bench -p propaga-propagators`.

## Roadmap

[ROADMAP.md](ROADMAP.md)

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT OR Apache-2.0, at your option.
