# Contributing to Propaga

Thank you for your interest in contributing!

## Development setup

```bash
git clone https://github.com/hocestnonsatis/propaga.git
cd propaga
cargo test --workspace
```

Optional pre-commit hook (runs `cargo fmt`):

```bash
bash scripts/install-git-hooks.sh
```

## Before submitting a PR

1. **Format:** `cargo fmt --all`
2. **Lint:** `cargo clippy --workspace -- -D warnings`
3. **Test:** `cargo test --workspace`
4. **Benchmarks (optional):** `bash benchmarks/run.sh`

CI runs the same checks on every pull request.

## Pull request guidelines

- Keep changes focused; one logical change per PR when possible.
- Add or update tests for behavior changes.
- Update [README.md](README.md) or [benchmarks/minizinc/COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) when user-facing behavior changes.
- Note FlatZinc subset limitations when adding or changing parser/compiler behavior.

## Versioning policy (1.x)

Propaga is currently at **1.x** (latest published: **1.1.0**). Unreleased work may land on `main` before the next tag.

- Minor releases (`1.N.0`) may add APIs and FlatZinc coverage; document breaking changes in [CHANGELOG.md](CHANGELOG.md).
- Patch releases are bug fixes and non-breaking additions.
- FlatZinc support remains an explicit subset; see [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md).
- Releases are user-gated; follow [RELEASE.md](RELEASE.md) only when cutting a tag.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
