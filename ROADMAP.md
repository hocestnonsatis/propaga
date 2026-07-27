# Propaga Roadmap

Forward-looking ideas beyond v1.0.0. See [README.md](README.md) for what ships today and [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) for FlatZinc coverage.

## Next

- Small polish / docs / corpus outside float ceil/floor/round reverse projection

## Shipped after v1.0.0

- Float / set objectives in lexicographic and Pareto multi-objective search
- Incremental Pareto front maintenance (streamed DFS, online filtering)
- Typed Pareto dominance cuts (`DominanceCutPropagator` for int / float / set-cardinality)
- Non-int assignment blocking (`ForbiddenAssignmentPropagator`); Pareto search uses cuts + blocking for all objective types
- Continuous float point exclusion via `encode_forbidden_float` (reif OR) wired into Pareto assignment blocking
- DFS float branching prefers splitting around registered blocked IEEE points (`with_float_holes` / Pareto hole tracking)
- Stronger float `!=` / `<`: `FloatNePropagator` endpoint pruning, strengthened `FloatEqReif`, strict `float_lt` via `¬(b ≤ a)`
- `FloatLinearNePropagator` + fixed `ReifiedFloatLinearEq(false)` (bound-touch / unit endpoint pruning; FlatZinc `float_lin_ne`)
- Docs: COMPATIBILITY lex/Pareto notes cover float and set-cardinality objectives
- DFS `solve_each` set in/out branching trail-correctness fix
- `FloatDomain` interior holes + `exclude_float_point`; `FloatNe` / forbidden assignment / DFS use domain holes
- Hole-aware float arithmetic: `affine` / fixed-operand `plus`/`times`/`divide`, equality hole sharing, binary plus/times/div propagators project holes
- Hole-aware float linear: `FloatLinearEqPropagator`, interior `float_lin_ne` exclusion, affine hole sharing when two vars remain free
- Hole-aware unary float maps: `abs`/`sqrt`/`ln`/`exp` preserve or safely project holes; unary propagator reverse-projects invertible cases
- Native `FloatElementPropagator` for `array_*_float_element` with hole sharing and common-absent hole projection
- Locally monotonic `sin`/`cos` hole projection (+ reverse preimages); constant-domain `ceil`/`floor`/`round` collapse to fixed
- COMPATIBILITY.md float hole semantics section; README FlatZinc gap blurb refreshed
- Search annotations: `float_search` / `set_search`, `indomain_random` / `indomain_reverse_split`, selector aliases
- Multi-phase `seq_search` and `indomain_interval` value ordering
- Stdlib CI corpus fixtures for `seq_search`, `search_selectors`, `float_search_ann`, `set_search_ann` (compile + SAT; portfolio SAT for `seq_search`)
- Portfolio search (`solve_portfolio` / `--workers`) propagates `search_phases` to every worker
- `SetUnionPropagator` cardinality bound tightening via `tighten_set_cardinality`
- `SetIntersectPropagator` cardinality bound tightening via `tighten_set_cardinality`
- `SetSubsetPropagator` cardinality bound tightening via `tighten_set_cardinality`
- CI smoke solves for `set_union.fzn` and `float_round.fzn`; handwritten `set_subset.fzn` / `float_round.fzn` fixtures
- `FloatUnaryPropagator` reverse-projects fixed `ceil` / `floor` / `round` images

## Shipped in v1.0.0

- FlatZinc 1.6 stdlib workflow: int / bool / set / float builtins, globals, and parameters
- `sort`, `array_float_*`, `float_dom`, `float_in`
- Float & set single-objective branch-and-bound
- Stdlib corpus + CI precompile regression (`minizinc-stdlib` job)
- `function` / `test` top-level skip; generic `min` / `max`
- `scripts/flatzinc-full-compat-report.sh` acceptance gate

## Shipped in v0.7.0

- FlatZinc primitives: `int_abs`, `int_times`, `int_div`, `int_mod`, `bool_not`, `bool_and`, `bool_or`
- FlatZinc `automaton` global
- `bool` / `float` parameters
- Nested FlatZinc predicate calls
- `annotation` top-level skip; `incomplete` search tolerance
- MiniZinc model corpus + compatibility report script

## Shipped in v0.6.0

- Set globals: `set_union`, `set_intersect`
- Float global: `float_times` with interval propagation
- FlatZinc compile support for the above constraints

## Shipped in v0.5.0

- Set and float variable domains integrated into the propagation engine
- `SetIntervalDomain`, set/float propagators, and Model API
- Typed search branching with `AssignmentValue`
- FlatZinc subset for set/float variables and constraints

## Shipped in v0.4.0

- Pareto-front multi-objective optimization
- FlatZinc `regular` global constraint
- Deeper lazy clause generation integration (clause propagator posting)
- WASM demo packaging and GitHub Pages deployment workflow

## Shipped in v0.3.0

- Parallel portfolio search with engine checkpoints (rayon worker pool)
- Broader FlatZinc predicate bodies (multi-constraint conjunction)
- FlatZinc lexicographic multi-objective (`minimize x, y`)
- Lazy clause pruning in DFS (`SearchConfig::clause_learning`)
