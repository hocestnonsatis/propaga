#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo test --workspace"
cargo test --workspace

echo "==> FlatZinc benchmarks"
cargo run -q -p propaga-cli -- solve --file benchmarks/magic_square.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/cumulative.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/cumulative_demand.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/all_different_only.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/permutation_sum.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/ordered_chain.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/strict_chain.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/disjunctive_two.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/disjunctive_edge.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/disjunctive_three.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_eq.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_ne.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_lt.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_lin_le.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_lin_eq.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/reified_lin_ne.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/bounded_sum.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/weighted_sum.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/weighted_sum_ge.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/bounded_sum_ge.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/gcc_exact.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/table_puzzle.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/maximize_x.fzn --quiet
cargo run -q -p propaga-cli -- solve --file benchmarks/minimize_cost.fzn --quiet

echo "==> Scheduling benchmark"
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_three_tasks.json --quiet
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_cumulative.json --quiet
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_cumulative_mode.json --quiet
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_cumulative_cap2.json --quiet
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_cumulative_demand.json --quiet
cargo run -q -p propaga-cli -- schedule --file benchmarks/schedule_disjunctive.json --quiet

echo "==> Puzzle smoke tests"
cargo run -q -p propaga-cli -- sudoku --quiet
cargo run -q -p propaga-cli -- n-queens --size 8 --quiet

echo "All benchmarks passed."
