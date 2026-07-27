# Propaga FlatZinc compatibility

Propaga targets **full MiniZinc FlatZinc 1.6 builtin support** for the standard library workflow: compile `.mzn` with MiniZinc, solve `.fzn` with Propaga. This matrix summarizes supported forms, decomposition notes, and remaining gaps.

See also [README.md](../../README.md) for solver features and [README.md](README.md) for the MiniZinc compile workflow.

## Summary

| Area | Status |
|------|--------|
| Integer / bool primitives & linear constraints | **Supported** |
| Set variables, parameters, and builtins | **Supported** |
| Float interval arithmetic & linear constraints | **Supported** (sound intervals + IEEE holes; not exact reals) |
| Stdlib globals (`count`, `among`, `lex_*`, `nvalue`, …) | **Supported** (decomposition) |
| Single-objective minimize / maximize | **Supported** (int, float, set cardinality) |
| Lexicographic / Pareto multi-objective | **Supported** (int, float, set-cardinality) |
| `function` / `test` top-level | **Skipped** (like `annotation`) |
| `sort`, `array_float_*`, `float_dom`, `float_in` | **Supported** |

## Variable declarations

| FlatZinc form | Status | Notes |
|---------------|--------|-------|
| `var int: x;` / `var low..high: x;` | Supported | |
| `var int: x = N;` | Supported | Fixed variable |
| `array [L..U] of var int: xs;` | Supported | |
| `var bool: b;` | Supported | Modeled as `0..1` integer |
| `array [L..U] of var bool: bs;` | Supported | Modeled as `0..1` integers |
| `var set of L..U: x;` | Supported | `SetIntervalDomain` with cardinality |
| `array [L..U] of var set of A..B: xs;` | Supported | Indexed set variables |
| `var low..high: x;` (float bounds) | Supported | Inclusive interval; may gain interior IEEE holes during search |
| `array [L..U] of var float: xs;` / `array [L..U] of var lo..hi: xs;` (float) | Supported | Indexed float variables |

## Parameters

| Form | Status | Notes |
|------|--------|-------|
| `int: n = N;` | Supported | |
| `array [L..U] of int: xs = [...];` | Supported | |
| `bool: flag = true;` | Supported | |
| `float: pi = 3.14;` | Supported | |
| `array [L..U] of float: xs = [...];` | Supported | Compiled to fixed float auxiliaries |
| `array [L..U] of bool: bs = [...];` | Supported | Compiled to fixed `0..1` integers |
| `set of L..U: s = { ... };` | Supported | Used in `set_in` and related constraints |
| `array [L..U] of set of int: xs = [{…}, …];` | Supported | Compiled to fixed set variables |

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
| `element`, `array_int_element`, `array_var_int_element` | Supported | Element; standard forms are 1-based, `_nonshifted` is 0-based |
| `array_int_maximum`, `array_int_minimum` | Supported | Reified decomposition |
| `bool_eq`, `bool2int` | Supported | Equality on `0..1` |
| `bool_not`, `bool_and`, `bool_or`, `bool_xor` | Supported | Table decomposition |
| `bool_clause`, `bool_clause_reif` | Supported | Clause decomposition |
| `bool_le`, `bool_lt`, `bool_*_reif`, `bool_eq_reif` | Supported | Table / reified |
| `bool_lin_eq`, `bool_lin_le` | Supported | Linear scalar on `0..1` vars |
| `array_bool_and`, `array_bool_xor` | Supported | Decomposition |
| `array_bool_element`, `array_var_bool_element` | Supported | Element; `array_var_bool_element` is 1-based, `_nonshifted` is 0-based |

**Decomposition note:** `int_times`, `int_div`, `int_mod`, and `int_pow` use domain tables capped at **10 000 tuples**. Larger Cartesian products return an unsupported error at compile time.

## Set constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `set_card` | Supported | Constant or variable cardinality (`SetCardEqPropagator`) |
| `set_subset`, `set_superset`, `set_eq`, `set_ne` | Supported | Native `set_eq` / subset propagators; `set_ne` via reified eq |
| `set_in`, `set_le`, `set_lt`, `set_diff`, `set_symdiff` | Supported | Native `set_diff` / `set_symdiff` propagators |
| `set_union`, `set_intersect` | Supported | Set union / intersection; both tighten cardinality bounds |
| `set_*_reif` | Supported | Reified set propagators |
| `array_var_set_element` | Supported | 1-based index; `_nonshifted` is 0-based |
| `array_set_element` | Supported | Constant set arrays; 1-based / `_nonshifted` 0-based |

## Float constraints

| Constraint | Status | Propaga mapping |
|------------|--------|-----------------|
| `float_le`, `float_eq`, `float_lt`, `float_ne` | Supported | Interval propagators; `eq`/`ne` share or exclude holes |
| `float_times`, `float_plus`, `float_div`, `float_abs` | Supported | Interval arithmetic; holes projected when a side is fixed / safe |
| `float_min`, `float_max` | Supported | Reified interval decomposition |
| `float_sqrt`, `float_sin`, `float_cos`, `float_ln`, `float_log2`, `float_exp` | Supported | Unary interval ops; hole-aware when locally invertible / monotonic; fixed `sqrt`/`ln`/`exp` images reverse-project; `float_log2` via `ln` / `ln(2)` |
| `float_ceil`, `float_floor`, `float_round` | Supported | Unary interval ops; constant domains collapse to fixed |
| `float_lin_eq`, `float_lin_le`, `float_lin_ge`, `float_lin_ne` | Supported | `FloatLinear*` (eq projects holes; ne excludes interior forcing points) |
| `float_lin_*_reif` | Supported | Reified float linear |
| `float_*_reif` | Supported | Reified float comparisons |
| `float_dom`, `float_in` | Supported | Interval union / membership decomposition |
| `array_float_element`, `array_var_float_element`, `array_float_maximum`, `array_float_minimum` | Supported | Native float element (hole-aware; 1-based / `_nonshifted` 0-based) + max/min decomposition |

**Soundness note:** Float propagation is **interval-based**. Bounds are conservative; non-convex unary functions (e.g. `sin` over a full period) widen to `[-1, 1]` when the input span exceeds one period.

### Float domain holes

`FloatDomain` stores an inclusive `[min, max]` plus a finite set of excluded IEEE points (**holes**). Holes arise from `float_ne`, assignment blocking (`encode_forbidden_float` / Pareto), and propagators that project exclusions.

| Situation | Hole behavior |
|-----------|---------------|
| `float_eq` | Holes are shared both ways inside the common interval |
| `float_ne` / unit `float_lin_ne` | Forbidden point excluded (endpoint shrink or interior hole) |
| `float_plus` / `float_times` / `float_div` | Holes map through affine / fixed-operand images; reverse when one side is fixed |
| `float_lin_eq` | Affine hole sharing when exactly two variables remain free |
| `float_abs` / `sqrt` / `ln` / `exp` | Preserve or safely project; reverse-project when locally invertible |
| `float_sin` / `float_cos` | Project (and reverse-project) only on locally monotonic intervals |
| `float_ceil` / `float_floor` / `float_round` | Sparse holes dropped when the map is non-constant (preimages are intervals); constant domains collapse to fixed; fixed integer images reverse-project onto the input |
| `array_*_float_element` | Share holes when index is fixed; project holes absent from every remaining candidate |

Holes are **sound over-approximations**: dropping a hole never removes a feasible real, but keeping every hole through non-injective maps is not always possible.

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
| `solve minimize x, y;` | Supported | Lexicographic (int, float, set-cardinality) |
| `solve :: pareto([...]) satisfy` | Supported | Int / float / set-cardinality; CLI JSON `pareto_solutions` |

## Search annotations

| Feature | Status | Notes |
|---------|--------|-------|
| `int_search` / `bool_search` | Supported subset | Variable list, common selectors, `complete` / `incomplete` |
| `float_search` / `set_search` | Supported subset | Same selectors; `float_search` precision is accepted and ignored |
| `seq_search([...])` | Supported | Multi-phase: each nested group uses its own selectors until all its vars are fixed |
| `restart_luby`, `restart_constant`, `restart_geometric`, `restart_none` | Supported | |
| `incomplete` | Tolerated | Treated like `complete` for exploration completeness |

FlatZinc search annotations are applied when solving with `propaga solve`. CLI flags override annotation defaults when explicitly provided:

| CLI flag | Overrides annotation |
|----------|----------------------|
| `--var-ordering` | `*_search` variable selection | Same aliases as FlatZinc (`first_fail`, `smallest`, `activity`, …) |
| `--value-ordering` | `*_search` value selection | Same aliases as FlatZinc (`indomain_min`, `split`, `interval`, …) |
| `--restarts` | `restart_*` policy | |
Supported variable selectors: `input_order`, `first_fail` / `most_constrained`, `smallest`, `largest`, `occurrence`, `degree`, `anti_first_fail` / `least_constrained`, `activity` / `vsids`.

Supported value selectors: `indomain_min`, `indomain_max`, `indomain_split`, `indomain_reverse_split`, `indomain_median`, `indomain_random` (deterministic shuffle), `indomain_interval` (first contiguous component, else split).

## CLI features (FlatZinc path)

| Flag | Status |
|------|--------|
| `propaga solve --file model.fzn` | Supported |
| `propaga solve --dir benchmarks/` | Supported | Batch `.fzn` directory |
| `--time-limit SECS` | Supported | Wall-clock cutoff |
| `--all`, `--solutions N` | Supported | Satisfy instances |
| `--stats`, `--format json` | Supported | Typed assignments; objective values for float/set |
| `--workers N` | Supported | Portfolio search (satisfy); inherits FlatZinc `seq_search` phases |

BnB, lexicographic, and Pareto optimize paths also inherit `seq_search` phases from the compiled model.

## MiniZinc workflow

```bash
# Compile MiniZinc to FlatZinc (requires MiniZinc toolchain)
minizinc -c --solver default --output-fzn-to-file /tmp/model.fzn model.mzn

# Single instance
cargo run -p propaga-cli -- solve --file /tmp/model.fzn --stats

# Stdlib corpus regression (bundled .fzn fixtures)
cargo test -p propaga-flatzinc --test builtin_corpus -- --nocapture

# Full compat report (requires MiniZinc)
bash scripts/flatzinc-full-compat-report.sh
```

When a compiled model fails with `Unsupported constraint`, check the tables above. For stdlib coverage, see `benchmarks/minizinc/stdlib/` and the `minizinc-stdlib` CI job.

## Acceptance gate

`scripts/flatzinc-full-compat-report.sh` compiles every model under `benchmarks/minizinc/{models,stdlib}/` and attempts a solve. Expected: `==> N passed, 0 failed` when MiniZinc is installed.
