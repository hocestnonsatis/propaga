use propaga_flatzinc::{compile, parse};
use std::fs;
use std::path::PathBuf;

fn stdlib_models_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/minizinc/stdlib")
}

fn bundled_fzn_path(base: &str) -> PathBuf {
    stdlib_models_dir().join(format!("{base}.fzn"))
}

fn precompiled_fzn_path(base: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/flatzinc-stdlib")
        .join(format!("{base}.fzn"))
}

fn fzn_path_for_model(base: &str) -> Option<PathBuf> {
    let precompiled = precompiled_fzn_path(base);
    if precompiled.exists() {
        return Some(precompiled);
    }
    let bundled = bundled_fzn_path(base);
    if bundled.exists() {
        return Some(bundled);
    }
    None
}

#[test]
fn all_stdlib_mzn_models_compile_when_fzn_present() {
    let dir = stdlib_models_dir();
    if !dir.exists() {
        return;
    }
    let mut failures = Vec::new();
    let mut tested = 0usize;
    for entry in fs::read_dir(&dir).expect("read stdlib models") {
        let entry = entry.expect("dir entry");
        let mzn = entry.path();
        if mzn.extension().and_then(|s| s.to_str()) != Some("mzn") {
            continue;
        }
        let base = mzn.file_stem().unwrap().to_string_lossy();
        let Some(fzn_path) = fzn_path_for_model(&base) else {
            continue;
        };
        tested += 1;
        let source = fs::read_to_string(&fzn_path).expect("read fzn");
        match parse(&source).and_then(compile) {
            Ok(_) => {}
            Err(err) => failures.push(format!("{base} ({}): {err}", fzn_path.display())),
        }
    }
    assert!(
        tested > 0,
        "no stdlib FlatZinc fixtures found under `{}`",
        dir.display()
    );
    assert!(
        failures.is_empty(),
        "stdlib compile failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn stdlib_corpus_lists_expected_models() {
    let dir = stdlib_models_dir();
    let expected = [
        "array_bool_xor",
        "bool_lin_le",
        "count_test",
        "distribute",
        "float_eq_reif",
        "float_log2",
        "float_sin",
        "int_lin_ne_reif",
        "int_min",
        "int_plus",
        "lex_less",
        "nvalue",
        "set_eq_reif",
    ];
    for name in expected {
        let mzn = dir.join(format!("{name}.mzn"));
        assert!(mzn.is_file(), "missing stdlib model `{}`", mzn.display());
        let fzn = bundled_fzn_path(name);
        assert!(fzn.is_file(), "missing bundled fzn `{}`", fzn.display());
    }
}
