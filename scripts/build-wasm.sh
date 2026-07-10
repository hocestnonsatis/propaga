#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack --locked
fi
wasm-pack build crates/propaga-wasm --target web --out-dir demo/pkg --release
echo "WASM build complete: crates/propaga-wasm/demo/pkg/"
