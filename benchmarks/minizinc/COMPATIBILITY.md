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
| Other globals (`regular`, `circuit`, `diffn`, …) | Not supported | Parse error: `Unsupported constraint` |

### Partial support

| Feature | Status | Notes |
|---------|--------|-------|
| `cumulative` variable duration/height arrays | Partial | Inline lists and int param arrays only; variable arrays error |
| `solve :: int_search(...)` | Ignored | Annotation skipped; search uses model decision order |
| `solve :: restart(...)` | Ignored | CLI `--restarts` applies instead |
| `predicate` / `function` / `test` | Skipped | May drop model semantics |
| Complex output expressions | Partial | Simple `show("text", var)` supported |

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

## Performance benchmarks

Criterion micro-benchmarks live in `crates/propaga-propagators/benches/propagation.rs`:

```bash
cargo bench -p propaga-propagators
```

CI runs `cargo bench -p propaga-propagators --no-run` to verify bench compilation without executing full runs.
