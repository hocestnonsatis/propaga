#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
if ! command -v wasm-pack >/dev/null 2>&1; then
  # Prefer the official binary installer: `cargo install wasm-pack` follows
  # crates.io deps (e.g. cargo-platform) that can require a newer rustc than
  # Propaga's MSRV (1.88).
  if command -v curl >/dev/null 2>&1; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
  else
    cargo install wasm-pack
  fi
fi
wasm-pack build crates/propaga-wasm --target web --out-dir demo/pkg --release
echo "WASM build complete: crates/propaga-wasm/demo/pkg/"
