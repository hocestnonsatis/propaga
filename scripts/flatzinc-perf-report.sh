#!/usr/bin/env bash
# Timed FlatZinc performance corpus report.
# Usage: scripts/flatzinc-perf-report.sh [--release] [--budget-ms N]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MANIFEST="${PROPAGA_PERF_MANIFEST:-benchmarks/perf/manifest.txt}"
PROFILE_FLAG=()
BUDGET_MS="${PROPAGA_PERF_BUDGET_MS:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      PROFILE_FLAG=(--release)
      shift
      ;;
    --budget-ms)
      BUDGET_MS="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [--release] [--budget-ms N]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ ! -f "$MANIFEST" ]]; then
  echo "missing manifest: $MANIFEST" >&2
  exit 1
fi

echo "==> Building propaga-cli ${PROFILE_FLAG[*]:-debug}"
cargo build -q -p propaga-cli "${PROFILE_FLAG[@]}"

BIN="$ROOT/target/debug/propaga"
if [[ ${#PROFILE_FLAG[@]} -gt 0 ]]; then
  BIN="$ROOT/target/release/propaga"
fi

failed=0
echo "file,elapsed_ms,status"
while IFS= read -r rel || [[ -n "$rel" ]]; do
  [[ -z "$rel" || "$rel" =~ ^# ]] && continue
  path="$ROOT/$rel"
  if [[ ! -f "$path" ]]; then
    echo "$rel,," >&2
    echo "missing file: $rel" >&2
    failed=1
    continue
  fi

  start_ns=$(date +%s%N)
  if "$BIN" solve --file "$path" --quiet; then
    status=sat
  else
    status=fail
    failed=1
  fi
  end_ns=$(date +%s%N)
  elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
  echo "$rel,$elapsed_ms,$status"

  if [[ -n "$BUDGET_MS" && "$elapsed_ms" -gt "$BUDGET_MS" ]]; then
    echo "budget exceeded for $rel: ${elapsed_ms}ms > ${BUDGET_MS}ms" >&2
    failed=1
  fi
done < "$MANIFEST"

if [[ "$failed" -ne 0 ]]; then
  echo "perf report: FAILED" >&2
  exit 1
fi
echo "perf report: OK"
