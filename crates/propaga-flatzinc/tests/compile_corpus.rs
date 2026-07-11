use propaga_flatzinc::{compile, parse};
use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks")
}

#[test]
fn all_handwritten_fzn_instances_compile() {
    let dir = corpus_dir();
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).expect("read benchmarks") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("fzn") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read fzn");
        let label = path.file_name().unwrap().to_string_lossy().to_string();
        match parse(&source).and_then(compile) {
            Ok(_) => {}
            Err(err) => failures.push(format!("{label}: {err}")),
        }
    }
    assert!(
        failures.is_empty(),
        "compile failures:\n{}",
        failures.join("\n")
    );
}
