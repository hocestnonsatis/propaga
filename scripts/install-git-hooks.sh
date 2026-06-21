#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

install -m 755 .githooks/pre-commit .git/hooks/pre-commit
echo "Installed pre-commit hook: cargo fmt --all before each commit"
