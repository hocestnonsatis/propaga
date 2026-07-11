# Propaga Roadmap

Forward-looking ideas beyond v0.6.0. See [README.md](README.md) for what ships today.

## Long term

- Trigonometric and additional float global constraints
- Nested FlatZinc predicate calls in predicate bodies
- Incremental Pareto enumeration

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
