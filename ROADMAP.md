# Propaga Roadmap

Forward-looking ideas beyond Sprint 22. See [README.md](README.md) for shipped features.

## Near term

- FlatZinc bool variables and `bool_eq` / `bool2int`
- `propaga solve --dir benchmarks/` batch mode
- `--time-limit SECS` wall-clock cutoff during search
- Criterion micro-benchmarks for all-different GAC and cumulative propagation
- MiniZinc stdlib compatibility matrix in docs

## Medium term

- Activity-based variable ordering (VSIDS-style)
- Parallel portfolio search (rayon worker pool)
- FlatZinc search annotations (`:: int_search`, `:: restart`)
- Multi-objective optimization (lexicographic or Pareto front)

## Long term

- Set and float variable domains
- Lazy clause generation integration
- Published API docs on docs.rs
- WASM / browser demo for small puzzles
