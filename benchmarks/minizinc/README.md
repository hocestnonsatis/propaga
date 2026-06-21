# MiniZinc → FlatZinc benchmarks

Propaga solves **FlatZinc** (`.fzn`) directly. To run MiniZinc models:

1. Install [MiniZinc](https://www.minizinc.org/) (includes `minizinc` and `mzn2fzn` or compile via IDE).
2. Compile a model to FlatZinc:

```bash
minizinc --compile-only -o benchmarks/minizinc/my_model.fzn benchmarks/minizinc/my_model.mzn
```

3. Solve with Propaga:

```bash
cargo run -p propaga-cli -- solve --file benchmarks/minizinc/my_model.fzn --stats
```

## Hand-maintained FlatZinc in this repo

The root `benchmarks/` folder contains curated `.fzn` files (no MiniZinc toolchain required in CI):

| File | Idea |
|------|------|
| `magic_square.fzn` | 3×3 magic square |
| `permutation_sum.fzn` | All-different + sum |
| `bounded_sum.fzn` | `int_lin_le` unit sum |
| `disjunctive_two.fzn` | Two-task disjunctive |
| `cumulative.fzn` | Two-task cumulative |

When adding MiniZinc sources here, prefer small models that use supported constraints only (`all_different`, `int_*`, `int_lin_eq`, `int_lin_le`, `int_lin_ge`, `int_lin_*_reif`, `element`, `cumulative`, `disjunctive`, `global_cardinality`, `table`, `solve minimize|maximize`).
