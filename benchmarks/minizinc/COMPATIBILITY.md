# Propaga FlatZinc compatibility

Propaga implements a **FlatZinc subset** focused on common MiniZinc stdlib constraints. This matrix summarizes what works today and what is partial or unsupported.

See also [README.md](../../README.md) for solver features and [README.md](README.md) for the MiniZinc compile workflow.

## Variable declarations

| FlatZinc form | Status | Notes |
|---------------|--------|-------|
| `var int: x;` / `var low..high: x;` | Supported | |
| `var int: x = N;` | Supported | Fixed variable |
| `array [L..U] of var int: xs;` | Supported | |
| `var bool: b;` | Supported | Modeled as `0..1` integer (Sprint 23) |
| `array [L..U] of var bool: bs;` | Supported | Modeled as `0..1` integers |
| Set / float variables | Not supported | Long-term roadmap |

## Parameters

| Form | Status |
|------|--------|
| `int: n = N;` | Supported |
| `array [L..U] of int: xs = [...];` | Supported |
| Bool / float / set parameters | Not supported |

## Constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `all_different` | Supported | GAC all-different |
| `int_eq`, `int_ne`, `int_le`, `int_lt`, `int_ge`, `int_gt` | Supported | Equality / ordering propagators |
| `int_*_reif` | Supported | Reified propagators |
| `int_lin_eq`, `int_lin_le`, `int_lin_ge` | Supported | Linear scalar propagators |
| `int_lin_*_reif` | Supported | Reified linear scalar |
| `bool_eq` | Supported | Equality on `0..1` vars (Sprint 23) |
| `bool2int` | Supported | Equality link (Sprint 23) |
| `element` | Supported | Element propagator |
| `cumulative` | Supported | Overload + time-table edges; inline or param duration/height arrays |
| `disjunctive` | Supported | Disjunctive propagator |
| `global_cardinality` | Supported | 2-arg and 4-arg forms |
| `table` | Supported | Tuple table propagator |
| `circuit` | Supported | Hamiltonian circuit propagator |
| `inverse` | Supported | Inverse array propagator |
| `diffn` | Supported | Non-overlap rectangles (fixed width/height) |
| Other globals (`regular`, …) | Not supported | Parse error: `Unsupported constraint` |

### Partial support

| Feature | Status | Notes |
|---------|--------|-------|
| `cumulative` variable duration/height arrays | Supported | Variable `array of var int` duration/height arrays |
| `solve :: int_search(...)` | Supported subset | Variable list, `first_fail`/`input_order`/…, `indomain_min`/`indomain_max`, `complete` |
| `solve :: restart_luby(base)` | Supported | Optional second scale argument is parsed and ignored |
| `solve :: restart_constant(scale)` | Supported | Constant node budget between restarts |
| `solve :: restart_geometric(base, scale)` | Supported | Geometric node budget; `base` may be a FlatZinc float literal |
| `solve :: restart_none` | Supported | With or without `()` |
| `predicate` / `function` / `test` | **Rejected** | Parse error: unsupported declaration |
| Unknown top-level statements (`annotation`, etc.) | **Rejected** | Parse error: unsupported top-level statement |

## Solve directives

| Directive | Status |
|-----------|--------|
| `solve satisfy;` | Supported |
| `solve minimize x;` | Supported | Branch-and-bound |
| `solve maximize x;` | Supported | Branch-and-bound |
| Multi-objective | Not supported | Medium-term roadmap |

## CLI features (FlatZinc path)

| Flag | Status |
|------|--------|
| `propaga solve --file model.fzn` | Supported |
| `propaga solve --dir benchmarks/` | Supported | Batch `.fzn` directory (Sprint 23) |
| `--time-limit SECS` | Supported | Wall-clock cutoff (Sprint 23) |
| `--all`, `--solutions N` | Supported | Satisfy instances |
| `--stats`, `--format json` | Supported | Includes `timed_out` when applicable |

## MiniZinc workflow example

```bash
# Compile MiniZinc to FlatZinc (requires MiniZinc toolchain)
minizinc --compile-only -o /tmp/model.fzn model.mzn

# Single instance
cargo run -p propaga-cli -- solve --file /tmp/model.fzn --stats

# Batch curated benchmarks
cargo run -p propaga-cli -- solve --dir benchmarks --quiet

# With time limit (seconds)
cargo run -p propaga-cli -- solve --file benchmarks/magic_square.fzn --time-limit 5 --stats
```

When a compiled model fails with `Unsupported constraint`, check this matrix and simplify the model or extend Propaga.

## Search annotation mapping

FlatZinc search annotations are applied when solving with `propaga solve`. CLI flags override annotation defaults when explicitly provided:

| CLI flag | Overrides annotation |
|----------|----------------------|
| `--var-ordering` | `int_search` variable selection |
| `--value-ordering` | `int_search` value selection |
| `--restarts` | `restart_*` policy |

Supported `int_search` variable selectors: `input_order`, `first_fail`, `smallest`, `largest`, `occurrence`, `degree`, `anti_first_fail`.

Supported `int_search` value selectors: `indomain_min`, `indomain_max`, `indomain_split`, `indomain_median`.

`bool_search` is supported with the same selector vocabulary as `int_search`.

`incomplete` search is not supported. Supported restart policies include `restart_linear` and `restart_on_solution`.

## Performance benchmarks

Criterion micro-benchmarks live in `crates/propaga-propagators/benches/propagation.rs`:

```bash
cargo bench -p propaga-propagators
```

CI runs `cargo bench -p propaga-propagators --no-run` to verify bench compilation without executing full runs.
