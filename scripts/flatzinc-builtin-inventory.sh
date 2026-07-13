#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/builtin-inventory"
mkdir -p "$OUT"

if ! command -v minizinc >/dev/null 2>&1; then
  echo "minizinc not found"
  exit 0
fi

for mzn in "$ROOT/benchmarks/minizinc/stdlib"/*.mzn; do
  base="$(basename "$mzn" .mzn)"
  fzn="$ROOT/target/flatzinc-stdlib/$base.fzn"
  mkdir -p "$ROOT/target/flatzinc-stdlib"
  minizinc --compile-only -o "$fzn" "$mzn"
  rg -o 'constraint [a-zA-Z0-9_]+' "$fzn" | awk '{print $2}' | sort -u > "$OUT/$base.constraints"
done

echo "Inventory written to $OUT"
