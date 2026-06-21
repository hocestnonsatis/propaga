# Releasing Propaga

This document describes how to cut a new release after CI is green on `main`.

## Prerequisites

1. [crates.io](https://crates.io) account with publish access to all `propaga-*` crates.
2. GitHub repository secret `CARGO_REGISTRY_TOKEN` (crates.io API token).
3. All changes merged to `main` and `CHANGELOG.md` updated.

## Automated release (recommended)

Push a version tag; the [release workflow](.github/workflows/release.yml) runs tests, builds binaries, creates a GitHub Release, and publishes crates.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Manual release

### 1. Verify locally

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
bash benchmarks/run.sh
cargo doc --workspace --no-deps
bash scripts/publish-crates.sh   # dry-run only; comment out publish loop first
```

### 2. Publish crates (dependency order)

```bash
export CARGO_REGISTRY_TOKEN=...
bash scripts/publish-crates.sh
```

Or publish individually:

```bash
cargo publish -p propaga-core
# wait for crates.io index (~30s)
cargo publish -p propaga-domains
# ... propaga-engine, propaga-propagators, propaga-search,
#     propaga-model, propaga-flatzinc, propaga-cli
```

### 3. GitHub Release

Create a release for tag `v0.1.0` with notes from `CHANGELOG.md`.
Attach binary artifacts from the release workflow, or build locally:

```bash
cargo build --release -p propaga-cli
```

The binary is `target/release/propaga`.

## Install after release

```bash
cargo install propaga-cli
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/hocestnonsatis/propaga/releases).
