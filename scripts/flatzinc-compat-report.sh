#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS="$ROOT/benchmarks/minizinc/models"
OUT="$ROOT/target/flatzinc-compat"
mkdir -p "$OUT"

if ! command -v minizinc >/dev/null 2>&1; then
  echo "minizinc not found; skip compile step"
  exit 0
fi

pass=0
fail=0
for mzn in "$MODELS"/*.mzn; do
  base="$(basename "$mzn" .mzn)"
  fzn="$OUT/$base.fzn"
  minizinc -c --solver default --output-fzn-to-file "$fzn" "$mzn"
  if cargo run -q -p propaga-cli -- solve --file "$fzn" --quiet >/dev/null 2>&1; then
    echo "OK  $base"
    pass=$((pass + 1))
  else
    echo "FAIL $base"
    fail=$((fail + 1))
  fi
done
echo "==> $pass passed, $fail failed"
test "$fail" -eq 0
