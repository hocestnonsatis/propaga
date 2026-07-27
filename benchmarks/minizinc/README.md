# MiniZinc → FlatZinc benchmarks

Propaga solves **FlatZinc** (`.fzn`) directly. To run MiniZinc models:

1. Install [MiniZinc](https://www.minizinc.org/) (includes `minizinc` and `mzn2fzn` or compile via IDE).
2. Compile a model to FlatZinc:

```bash
minizinc -c --solver default --output-fzn-to-file benchmarks/minizinc/my_model.fzn benchmarks/minizinc/my_model.mzn
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
| `set_union.fzn` / `set_subset.fzn` | Set ops + cardinality |
| `set_intersect.fzn` | Set intersection + cardinality bounds |
| `float_round.fzn` | Constant-domain `float_round` collapse |
| `float_floor.fzn` / `float_ceil.fzn` | Fixed-image reverse projection |

When adding MiniZinc sources here, prefer small models that use supported constraints only. See [COMPATIBILITY.md](COMPATIBILITY.md) for the full list.

## Compatibility report

```bash
bash scripts/flatzinc-compat-report.sh
bash scripts/flatzinc-full-compat-report.sh   # models + stdlib corpus
bash scripts/flatzinc-builtin-inventory.sh    # list FlatZinc constraint names per stdlib model
```

Requires MiniZinc installed locally. CI's `minizinc-stdlib` job regresses the
bundled `.fzn` fixtures (no MiniZinc toolchain required). Fresh MiniZinc
output is solver-specific and is checked locally via the scripts above.

## Stdlib test corpus

MiniZinc models under `benchmarks/minizinc/stdlib/` exercise individual FlatZinc
builtins and search annotations. Each model has a **hand-written** `.fzn` fixture
used for offline/CI compile regression
(`cargo test -p propaga-flatzinc --test builtin_corpus`). Search fixtures include
`seq_search`, `search_selectors`, `float_search_ann`, `set_search_ann`,
`bool_search_ann`, and `indomain_random_ann` (compile + SAT; `seq_search` also under portfolio).

Optional local refresh into `target/flatzinc-stdlib/` (note: MiniZinc 2.9+ may
emit solver-specific FlatZinc that Propaga does not yet accept — prefer the
bundled fixtures for Propaga regression):

```bash
mkdir -p target/flatzinc-stdlib
for mzn in benchmarks/minizinc/stdlib/*.mzn; do
  base=$(basename "$mzn" .mzn)
  minizinc -c --solver default --output-fzn-to-file "target/flatzinc-stdlib/$base.fzn" "$mzn"
done
cargo test -p propaga-flatzinc --test builtin_corpus -- --nocapture
```
