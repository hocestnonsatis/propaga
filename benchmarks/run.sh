#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo test --workspace"
cargo test --workspace

echo "==> FlatZinc benchmarks"
cargo run -q -p propaga-cli -- solve --dir benchmarks --quiet

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
