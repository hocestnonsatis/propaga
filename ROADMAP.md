# Propaga Roadmap

Forward-looking ideas beyond v0.4.0. See [README.md](README.md) for what ships today.

## Long term

- Full engine integration for set and float variable domains

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
