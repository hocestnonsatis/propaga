# Propaga FlatZinc compatibility

Propaga targets **full MiniZinc FlatZinc 1.6 builtin support** for the standard library workflow: compile `.mzn` with MiniZinc, solve `.fzn` with Propaga. This matrix summarizes supported forms, decomposition notes, and remaining gaps.

See also [README.md](../../README.md) for solver features and [README.md](README.md) for the MiniZinc compile workflow.

## Summary

| Area | Status |
|------|--------|
| Integer / bool primitives & linear constraints | **Supported** |
| Set variables, parameters, and builtins | **Supported** |
| Float interval arithmetic & linear constraints | **Supported** (sound intervals, not exact reals) |
| Stdlib globals (`count`, `among`, `lex_*`, `nvalue`, …) | **Supported** (decomposition) |
| Single-objective minimize / maximize | **Supported** (int, float, set cardinality) |
| Lexicographic / Pareto multi-objective | **Supported** (int objectives only) |
| `function` / `test` top-level | **Skipped** (like `annotation`) |
| `sort`, `array_float_*`, `float_dom`, `float_in` | **Supported** (decomposition) |

## Variable declarations

| FlatZinc form | Status | Notes |
|---------------|--------|-------|
| `var int: x;` / `var low..high: x;` | Supported | |
| `var int: x = N;` | Supported | Fixed variable |
| `array [L..U] of var int: xs;` | Supported | |
| `var bool: b;` | Supported | Modeled as `0..1` integer |
| `array [L..U] of var bool: bs;` | Supported | Modeled as `0..1` integers |
| `var set of L..U: x;` | Supported | `SetIntervalDomain` with cardinality |
| `var low..high: x;` (float bounds) | Supported | Inclusive interval domain |

## Parameters

| Form | Status | Notes |
|------|--------|-------|
| `int: n = N;` | Supported | |
| `array [L..U] of int: xs = [...];` | Supported | |
| `bool: flag = true;` | Supported | |
| `float: pi = 3.14;` | Supported | |
| `set of L..U: s = { ... };` | Supported | Used in `set_in` and related constraints |

## Integer & bool constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `int_eq`, `int_ne`, `int_le`, `int_lt`, `int_ge`, `int_gt` | Supported | Primitive propagators |
| `int_*_reif` | Supported | Reified propagators |
| `int_lin_eq`, `int_lin_le`, `int_lin_ge`, `int_lin_ne` | Supported | Linear scalar propagators |
| `int_lin_*_reif` | Supported | Reified linear scalar |
| `int_plus`, `int_abs`, `int_times`, `int_div`, `int_mod` | Supported | Table / decomposition |
| `int_min`, `int_max`, `int_pow`, `int_pow_fixed` | Supported | Table decomposition |
| `int2float` | Supported | Channeling propagator |
| `min` / `max` (generic) | Supported | Dispatches to `int_*` or `float_*` by domain |
| `element`, `array_int_element`, `array_var_int_element` | Supported | Element propagator |
| `array_int_maximum`, `array_int_minimum` | Supported | Reified decomposition |
| `bool_eq`, `bool2int` | Supported | Equality on `0..1` |
| `bool_not`, `bool_and`, `bool_or`, `bool_xor` | Supported | Table decomposition |
| `bool_clause`, `bool_clause_reif` | Supported | Clause decomposition |
| `bool_le`, `bool_lt`, `bool_*_reif`, `bool_eq_reif` | Supported | Table / reified |
| `bool_lin_eq`, `bool_lin_le` | Supported | Linear scalar on `0..1` vars |
| `array_bool_and`, `array_bool_xor` | Supported | Decomposition |
| `array_bool_element`, `array_var_bool_element` | Supported | Element |

**Decomposition note:** `int_times`, `int_div`, `int_mod`, and `int_pow` use domain tables capped at **10 000 tuples**. Larger Cartesian products return an unsupported error at compile time.

## Set constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `set_card` | Supported | Cardinality propagator |
| `set_subset`, `set_superset`, `set_eq`, `set_ne` | Supported | Set propagators / decomposition |
| `set_in`, `set_le`, `set_lt`, `set_diff`, `set_symdiff` | Supported | Set decomposition |
| `set_union`, `set_intersect` | Supported | Set union / intersection |
| `set_*_reif` | Supported | Reified set propagators |

## Float constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `float_le`, `float_eq`, `float_lt`, `float_ne` | Supported | Interval propagators |
| `float_times`, `float_plus`, `float_div`, `float_abs` | Supported | Interval arithmetic |
| `float_min`, `float_max` | Supported | Reified interval decomposition |
| `float_sqrt`, `float_sin`, `float_cos`, `float_ln`, `float_log2`, `float_exp` | Supported | Unary interval ops; `float_log2` via `ln` / `ln(2)` |
| `float_ceil`, `float_floor`, `float_round` | Supported | Unary interval ops |
| `float_lin_eq`, `float_lin_le`, `float_lin_ge`, `float_lin_ne` | Supported | `FloatLinear*` propagators |
| `float_lin_*_reif` | Supported | Reified float linear |
| `float_*_reif` | Supported | Reified float comparisons |
| `float_dom`, `float_in` | Supported | Interval union / membership decomposition |
| `array_float_element`, `array_var_float_element`, `array_float_maximum`, `array_float_minimum` | Supported | Reified decomposition |

**Soundness note:** Float propagation is **interval-based**. Bounds are conservative; non-convex unary functions (e.g. `sin`) widen to `[-1, 1]` when the input span exceeds one period.

## Global constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `all_different` | Supported | GAC all-different |
| `cumulative` | Supported | Overload + time-table edges |
| `disjunctive` | Supported | Disjunctive propagator |
| `global_cardinality` | Supported | 2-arg and 4-arg forms |
| `table` | Supported | Tuple table propagator |
| `circuit` | Supported | Hamiltonian circuit |
| `inverse` | Supported | Inverse array |
| `diffn` | Supported | Non-overlap rectangles (fixed size) |
| `regular`, `automaton` | Supported | DFA → table |
| `count`, `among`, `at_least`, `at_most` | Supported | Global decomposition |
| `distribute`, `nvalue` | Supported | Global decomposition |
| `lex_less`, `lex_lesseq`, `lex_greater`, `lex_greatereq` | Supported | Lexicographic decomposition |
| `increasing`, `decreasing` | Supported | Pairwise order constraints |
| `sort` | Supported | Permutation + `increasing` decomposition |

## Top-level statements

| Statement | Status | Notes |
|-----------|--------|-------|
| `var` / `array` / parameters | Supported | |
| `constraint` | Supported | |
| `solve` / `output` | Supported | |
| `predicate` | Supported | Multi-constraint bodies; nested calls |
| `annotation` | Skipped | Ignored during parse |
| `function` | Skipped | Ignored during parse (v1.0.0) |
| `test` | Skipped | Ignored during parse (v1.0.0) |

## Solve directives

| Directive | Status | Notes |
|-----------|--------|-------|
| `solve satisfy;` | Supported | |
| `solve minimize x;` / `maximize x;` | Supported | Int, float, or set (cardinality) objective |
| `solve minimize x, y;` | Supported | Lexicographic (int objectives) |
| `solve :: pareto([...]) satisfy` | Supported | Int objectives; CLI JSON `pareto_solutions` |

## Search annotations

| Feature | Status | Notes |
|---------|--------|-------|
| `int_search` / `bool_search` | Supported subset | Variable list, common selectors, `complete` |
| `restart_luby`, `restart_constant`, `restart_geometric`, `restart_none` | Supported | |
| `incomplete` | Tolerated | Treated like `complete` |

FlatZinc search annotations are applied when solving with `propaga solve`. CLI flags override annotation defaults when explicitly provided:

| CLI flag | Overrides annotation |
|----------|----------------------|
| `--var-ordering` | `int_search` variable selection |
| `--value-ordering` | `int_search` value selection |
| `--restarts` | `restart_*` policy |

Supported `int_search` variable selectors: `input_order`, `first_fail`, `smallest`, `largest`, `occurrence`, `degree`, `anti_first_fail`.

Supported `int_search` value selectors: `indomain_min`, `indomain_max`, `indomain_split`, `indomain_median`.

## CLI features (FlatZinc path)

| Flag | Status |
|------|--------|
| `propaga solve --file model.fzn` | Supported |
| `propaga solve --dir benchmarks/` | Supported | Batch `.fzn` directory |
| `--time-limit SECS` | Supported | Wall-clock cutoff |
| `--all`, `--solutions N` | Supported | Satisfy instances |
| `--stats`, `--format json` | Supported | Typed assignments; objective values for float/set |
| `--workers N` | Supported | Portfolio search (satisfy) |

## MiniZinc workflow

```bash
# Compile MiniZinc to FlatZinc (requires MiniZinc toolchain)
minizinc --compile-only -o /tmp/model.fzn model.mzn

# Single instance
cargo run -p propaga-cli -- solve --file /tmp/model.fzn --stats

# Stdlib corpus regression (bundled .fzn or CI-precompiled)
cargo test -p propaga-flatzinc stdlib -- --nocapture

# Full compat report (requires MiniZinc)
bash scripts/flatzinc-full-compat-report.sh
```

When a compiled model fails with `Unsupported constraint`, check the tables above. For stdlib coverage, see `benchmarks/minizinc/stdlib/` and the `minizinc-stdlib` CI job.

## Acceptance gate

`scripts/flatzinc-full-compat-report.sh` compiles every model under `benchmarks/minizinc/{models,stdlib}/` and attempts a solve. Expected: `==> N passed, 0 failed` when MiniZinc is installed.
