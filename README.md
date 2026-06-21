# Propaga

A modern, propagator-based constraint solver written in Rust.

Propaga provides a clean propagation engine with typed variable handles, pluggable domains, and composable propagators. The goal is to make constraint propagation safe, extensible, and approachable — suitable for both learning and real problems.

## Crates

| Crate | Description |
|-------|-------------|
| `propaga-core` | Core types: `VariableId`, `Domain`, `Propagator`, explanations, nogoods |
| `propaga-domains` | `IntervalDomain`, `BitsetDomain`, `HybridDomain` |
| `propaga-engine` | Propagation engine with trail and event scheduling |
| `propaga-propagators` | Built-in propagators (equality, linear, ordering, disjunctive, GAC all-different, GCC, table, element, cumulative, nogood) |
| `propaga-search` | Search with MRV/DOM/W-DEG, LCV, first-UIP nogood learning, branch-and-bound optimization, Luby restarts |
| `propaga-model` | High-level modeling API |
| `propaga-flatzinc` | FlatZinc subset parser and compiler |
| `propaga-cli` | Command-line solver (`propaga sudoku`, `propaga n-queens`, `propaga solve`, `propaga schedule`) |

## Features

### Search (Sprint 1, 8–10, 16, 18)
- **First-UIP conflict analysis**: backward explanation resolution with 1UIP nogood trimming
- **Sound propagator conflicts**: propagator pruning no longer expands nogoods to all prior branches
- **Cumulative overload nogoods**: mandatory overlapping tasks only (not full branch history)
- **Disjunctive overlap nogoods**: fixed overlapping start pair recorded on conflict
- **Nogood propagator**: learned nogoods posted into the engine for early pruning
- **Root propagation**: full fixpoint propagation before search and after each restart
- **Luby restarts**: configurable restart policy (`--restarts luby`, `luby:256`, `none`)
- **Phase saving**: last assigned value is tried first on backtrack/restart (`--no-phase-saving` to disable)
- **SearchConfig**: `Model::set_search_config()` for learning/restart/value-ordering tuning

### Propagation (Sprint 2–3, 17)
- **AllDifferent GAC** via Hopcroft-Karp matching plus Regin SCC batch value pruning (single matching, Tarjan SCC, free-value handling)
- **GlobalCardinality (GCC)**: bounds-consistent cardinality propagator via `Model::gcc`
- **Table**, **Element**, and **Cumulative** (overload + time-table edge finding) propagators
- **Explanation-aware trail** with synchronized backtracking

### FlatZinc (Sprint 2–5, 19–21)
- Subset parser: `var`/`array` declarations, `int`/`int` array params, `all_different`, `int_eq`, `int_ne`, `int_le`, `int_lt`, `int_ge`, `int_gt`, `int_*_reif`, `int_lin_eq`, `int_lin_le`, `int_lin_ge`, `int_lin_*_reif`, `element`, `cumulative`, `disjunctive`, `global_cardinality`, `table`, `output`, `solve satisfy|minimize|maximize`
- CLI: `propaga solve --file benchmarks/magic_square.fzn`

### FlatZinc GCC and table (Sprint 19)
- **`global_cardinality`**: two-arg `(cover, vars)` and four-arg `(vars, cover, lbound, ubound)` forms compiled to `Model::gcc`
- **`table`**: tuple sets `{...}` compiled to `Model::table`
- Benchmarks: `gcc_exact.fzn`, `table_puzzle.fzn`

### FlatZinc output (Sprint 20)
- Parses `output [ show("text", var), ... ];` directives
- Plain output renders formatted solution lines; JSON includes an `outputs` array
- `magic_square.fzn` includes a sample output directive

### Optimization (Sprint 21)
- **`solve minimize` / `solve maximize`** in FlatZinc with branch-and-bound over a single objective variable
- `Model::optimize()` and `OptimizationSearch` in `propaga-search`
- CLI prints `objective (minimize|maximize) = N` and JSON `objective` field
- Benchmarks: `maximize_x.fzn`, `minimize_cost.fzn`

### Search heuristics and limits (Sprint 22)
- **Variable ordering**: `--var-ordering mrv` (default), `dom`, or `dom-wdeg`
- **Solution cap**: `--solutions N` with `--all` stops after `N` solutions (Sudoku, N-Queens, FlatZinc)
- CI runs `cargo fmt --check` and `cargo clippy -D warnings`

### Scheduling (Sprint 3–8)
- JSON schedule format (`capacity`, `horizon`, `tasks`, optional `mode` or legacy `sequential`/`disjunctive` flags)
- Modes: `cumulative` (default), `sequential`, `disjunctive`
- Cumulative mandatory intervals only when start/end is fixed or singleton (no false root overload)
- Per-task resource `demand` (default 1); FlatZinc `cumulative(..., heights, capacity)` supported
- CLI: `propaga schedule --file benchmarks/schedule_three_tasks.json --stats`
- Benchmarks: `schedule_cumulative.json`, `schedule_cumulative_cap2.json`, `schedule_cumulative_demand.json`, `schedule_disjunctive.json`, `schedule_cumulative_mode.json`
- Cumulative mode supports nogood learning via overload conflict literals

### Linear inequalities (Sprint 7–9, 14)
- FlatZinc `int_lin_le` / `int_lin_ge` / `int_lin_eq` with unit or general integer coefficients (`LinearScalarLe`/`Ge`)
- Reified linear: `int_lin_le_reif`, `int_lin_ge_reif`, `int_lin_eq_reif` (`ReifiedScalarLe`/`Ge`/`Eq`); eq reif=0 prunes values that would force the sum
- Benchmarks: `bounded_sum.fzn`, `bounded_sum_ge.fzn`, `weighted_sum.fzn`, `weighted_sum_ge.fzn`, `reified_lin_le.fzn`, `reified_lin_eq.fzn`, `reified_lin_ne.fzn`
- MiniZinc workflow: `benchmarks/minizinc/README.md`

### Search ordering (Sprint 4)
- **LCV value ordering**: `--value-ordering lcv` tries values that appear in fewer other domains first
- Default remains ascending (`--value-ordering asc`)

### Ordering constraints (Sprint 5–6)
- **`LessEqual` / `LessThan` propagators**: bound-consistent `<=` and `<` via `Model::less_equal` / `Model::less_than`
- **`greater_equal` / `greater_than`**: swapped posting helpers on the model API
- FlatZinc: `int_le`, `int_lt`, `int_ge`, `int_gt`
- **`Disjunctive` propagator**: pairwise single-machine disjunctive scheduling with fixed-start edge finding, theta-tree mandatory precedences, and energy overload detection; FlatZinc `disjunctive`
- **`Reified` propagators**: `int_eq_reif`, `int_ne_reif`, `int_le_reif`, `int_lt_reif`, `int_ge_reif`, `int_gt_reif` over 0/1 reification variables
- Benchmarks: `ordered_chain.fzn`, `strict_chain.fzn`, `disjunctive_two.fzn`, `disjunctive_three.fzn`
- Schedule output reads fixed `end` times from the engine after search (not only start decision vars)

### CLI
- Plain/JSON output, batch Sudoku, stats, `--all`, `--visual`
- `--no-learning` to disable nogood learning
- `--restarts` for restart policy control
- `--value-ordering` (`asc` or `lcv`)
- `--var-ordering` (`mrv`, `dom`, or `dom-wdeg`)
- `--solutions N` to cap enumeration with `--all`
- `--no-phase-saving` to disable phase saving

### Heuristic comparison (Sprint 22)
| Flag combo | Effect |
|------------|--------|
| default | MRV + ascending values + learning + Luby restarts |
| `--no-learning` | DFS without nogoods (often more backtracks) |
| `--value-ordering lcv` | Least constraining value first |
| `--var-ordering dom` | Domain size tie-break ordering |
| `--var-ordering dom-wdeg` | DOM divided by conflict weights |
| `--all --solutions 3` | Stop after three solutions |

## Quick start

```bash
cargo test
cargo run --example sudoku
cargo run -p propaga-cli -- sudoku --stats
cargo run -p propaga-cli -- n-queens --size 8 --visual --stats
cargo run -p propaga-cli -- solve --file benchmarks/magic_square.fzn --stats
cargo run -p propaga-cli -- solve --file benchmarks/maximize_x.fzn --stats
cargo run -p propaga-cli -- solve --file benchmarks/gcc_exact.fzn --stats
cargo run -p propaga-cli -- n-queens --size 12 --var-ordering dom --stats
cargo run -p propaga-cli -- sudoku --all --solutions 3 --stats
cargo run -p propaga-cli -- solve --file benchmarks/cumulative.fzn --stats
cargo run -p propaga-cli -- schedule --file benchmarks/schedule_three_tasks.json --stats
cargo run -p propaga-cli -- schedule --file benchmarks/schedule_cumulative.json --stats
bash benchmarks/run.sh
cargo run -p propaga-cli -- sudoku --format json --stats
cargo run -p propaga-cli -- n-queens --size 8 --restarts luby:256 --stats
cargo run -p propaga-cli -- sudoku --no-learning --stats   # compare learning impact
```

## License

Licensed under either of MIT or Apache-2.0 at your option.
