# Propaga

A propagator-based constraint solver written in Rust.

Propaga combines a typed propagation engine with pluggable domains, composable propagators, and conflict-driven search. Use it as a library via the `Model` API, from FlatZinc files, or through the `propaga` CLI.

**v0.1.0** — [crates.io](https://crates.io/crates/propaga-cli) · [GitHub Releases](https://github.com/hocestnonsatis/propaga/releases) · [CHANGELOG](CHANGELOG.md)

## Installation

Requires Rust **1.88+** (Edition 2024).

```bash
cargo install propaga-cli
propaga --help
```

Prebuilt binaries: [GitHub Releases](https://github.com/hocestnonsatis/propaga/releases)

**As a library** ([docs.rs](https://docs.rs/propaga-core)):

```toml
propaga-core = "0.1"
propaga-model = "0.1"
propaga-flatzinc = "0.1"
```

FlatZinc support is an intentional subset — check [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) before compiling MiniZinc models.

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

MRV, DOM, and DOM/W-DEG variable ordering; ascending or LCV value ordering; first-UIP nogood learning; Luby restarts; phase saving; branch-and-bound for single-objective optimization. Configurable via CLI flags or `Model::set_search_config()`.

### FlatZinc

Parses a practical subset of FlatZinc: integer and bool variables, common globals, linear constraints, reified forms, `output` directives, and `solve satisfy | minimize | maximize`. Batch solving with `propaga solve --dir`. Search annotations in `.fzn` files are ignored — use CLI flags instead.

Full constraint matrix: [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md). MiniZinc workflow: [benchmarks/minizinc/README.md](benchmarks/minizinc/README.md).

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
| `--restarts` | `luby` | `none`, `luby`, or `luby:N` |
| `--var-ordering` | `mrv` | `mrv`, `dom`, `dom-wdeg` |
| `--value-ordering` | `asc` | `asc` or `lcv` |
| `--no-phase-saving` | off | Disable phase saving |

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
