# Propaga Roadmap

Forward-looking ideas beyond v0.3.0. See [README.md](README.md) for what ships today.

## Medium term

- Pareto-front multi-objective optimization

## Long term

- Full engine integration for set and float variable domains
- Deeper lazy clause generation integration with search (propagator posting)
- WASM demo packaging and hosted deployment

## Shipped in v0.3.0

- Parallel portfolio search with engine checkpoints (rayon worker pool)
- Broader FlatZinc predicate bodies (multi-constraint conjunction)
- FlatZinc lexicographic multi-objective (`minimize x, y`)
- Lazy clause pruning in DFS (`SearchConfig::clause_learning`)
