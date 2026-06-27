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
# or batch all .fzn in a folder:
cargo run -p propaga-cli -- solve --dir benchmarks --quiet
```

## Compatibility matrix

See [COMPATIBILITY.md](COMPATIBILITY.md) for supported FlatZinc constraints, partial features, and CLI flags.

## Hand-maintained FlatZinc in this repo

The root `benchmarks/` folder contains curated `.fzn` files (no MiniZinc toolchain required in CI):

| File | Idea |
|------|------|
| `magic_square.fzn` | 3×3 magic square |
| `permutation_sum.fzn` | All-different + sum |
| `bounded_sum.fzn` | `int_lin_le` unit sum |
| `disjunctive_two.fzn` | Two-task disjunctive |
| `cumulative.fzn` | Two-task cumulative |
| `int_search_order.fzn` | `int_search` variable order |
| `int_search_restart.fzn` | `restart_luby` + minimize |

When adding MiniZinc sources here, prefer small models that use supported constraints only. See [COMPATIBILITY.md](COMPATIBILITY.md) for the full list.
