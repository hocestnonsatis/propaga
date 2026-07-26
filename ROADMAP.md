# Propaga Roadmap

Forward-looking ideas beyond v1.0.0. See [README.md](README.md) for what ships today and [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) for FlatZinc coverage.

## Next

- Dominance-cut pruning during Pareto search (OR-encoding over improving objectives)

## Shipped after v1.0.0

- Float / set objectives in lexicographic and Pareto multi-objective search
- Incremental Pareto front maintenance (streamed DFS, online filtering)

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
