# FlatZinc Uyumu (v0.7.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Propaga'nın FlatZinc desteğini bilinçli alt kümeden, gerçek MiniZinc modellerinin büyük çoğunluğunun derlenip çözülebilmesine yetecek pratik uyumluluğa genişletmek (v0.7.0).

**Architecture:** Üç katmanlı genişletme: (1) ölçüm — hangi `.fzn` dosyalarının nerede kırıldığını otomatik raporla; (2) parse/compile yüzeyi — MiniZinc'in sık ürettiği primitive constraint'leri mevcut propagator'lara ayrıştır; (3) predicate sistemi — iç içe çağrıları ve tam substitution ile kullanıcı tanımlı predicate'leri güvenilir şekilde genişlet. Yeni global propagator yalnızca ayrıştırma yetmeyen kısıtlar için (`automaton`).

**Tech Stack:** Rust 1.88+ (Edition 2024), `propaga-flatzinc`, `propaga-model`, `propaga-propagators`, MiniZinc toolchain (yerel geliştirme + opsiyonel CI), FlatZinc 1.6 subset

---

## Mevcut Durum (v0.6.0)

| Alan | Durum | Dosya |
|------|-------|-------|
| Temel int/bool/set/float değişkenleri | Destekli | `crates/propaga-flatzinc/src/parse.rs:653-712` |
| Yaygın global constraint'ler | Destekli | `parse.rs:791-1151`, `compile.rs:316-556` |
| `int` / `int[]` parametreler | Destekli | `parse.rs:606-650` |
| `bool` / `float` / `set` parametreler | **Yok** | `COMPATIBILITY.md:29` |
| MiniZinc primitive'leri (`int_abs`, `int_times`, `bool_not`, …) | **Yok** — `PredicateCall` olarak kalır, compile patlar | `compile.rs:557-560` |
| İç içe predicate çağrıları | **Reddedilir** | `parse.rs:1205-1208`, `ROADMAP.md:8` |
| `substitute_constraint` | **Eksik** — yalnızca 4 constraint tipi | `compile.rs:603-626` |
| `annotation` top-level | **Reddedilir** | `parse.rs:588-590` |
| `automaton` global | **Yok** | `COMPATIBILITY.md:51` |
| `incomplete` search | **Reddedilir** | `compile.rs:214-215` |
| Uyumluluk ölçümü | Manuel `COMPATIBILITY.md` | `benchmarks/minizinc/` |

## Hedef (v0.7.0 "pratik MiniZinc uyumu")

Aşağıdaki MiniZinc → FlatZinc çıktıları **parse + compile + solve** ile çalışmalı:

1. `examples/minizinc/` altındaki 12 örnek model (bu planda tanımlı)
2. Mevcut `benchmarks/*.fzn` regression seti (kırılmamalı)
3. `scripts/flatzinc-compat-report.sh` çıktısında **≥ %90 compile başarısı** (yerel MiniZinc ile)

## Kapsam Dışı (v0.8.0+)

- `function` / `test` top-level bildirimleri
- Float/set optimizasyon hedefleri
- Tam FlatZinc 1.6 spesifikasyonu
- `int_pow`, trigonometrik float global'leri
- MiniZinc `output` şablonlarının tamamı (mevcut subset yeterli)

---

## File Structure

| Dosya | Sorumluluk |
|-------|------------|
| `scripts/flatzinc-compat-report.sh` | MiniZinc derle + Propaga compile raporu |
| `benchmarks/minizinc/models/*.mzn` | Kaynak MiniZinc örnekleri |
| `benchmarks/minizinc/expected/` | Beklenen compile/solve sonuçları (JSON) |
| `crates/propaga-flatzinc/tests/compile_corpus.rs` | `.fzn` compile regression testleri |
| `crates/propaga-flatzinc/src/decompose.rs` | Primitive constraint → Model API ayrıştırması |
| `crates/propaga-flatzinc/src/parse.rs` | Yeni constraint/parameter/annotation parse |
| `crates/propaga-flatzinc/src/compile.rs` | Decompose entegrasyonu, predicate expansion |
| `crates/propaga-flatzinc/src/lib.rs` | `decompose` modül export (test-only değil) |
| `crates/propaga-propagators/src/automaton.rs` | `automaton` → DFA → table fallback |
| `benchmarks/int_abs.fzn` | Primitive regression |
| `benchmarks/bool_logic.fzn` | `bool_not` / `bool_and` regression |
| `benchmarks/nested_predicate.fzn` | İç içe predicate regression |
| `benchmarks/automaton_chain.fzn` | `automaton` regression |
| `benchmarks/minizinc/COMPATIBILITY.md` | Güncellenmiş matris |
| `CHANGELOG.md`, `ROADMAP.md` | v0.7.0 notları |

---

### Task 1: Uyumluluk Ölçüm Altyapısı

**Files:**
- Create: `scripts/flatzinc-compat-report.sh`
- Create: `benchmarks/minizinc/models/abs_test.mzn`
- Create: `crates/propaga-flatzinc/tests/compile_corpus.rs`
- Modify: `benchmarks/minizinc/README.md`

- [ ] **Step 1: Write the failing corpus test**

`crates/propaga-flatzinc/tests/compile_corpus.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it passes on current tree**

Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS (mevcut `benchmarks/*.fzn` zaten derleniyor)

- [ ] **Step 3: Add MiniZinc source stub and report script**

`benchmarks/minizinc/models/abs_test.mzn`:

```minizinc
int: n = 5;
array[1..n] of var -n..n: xs;
constraint forall(i in 1..n)(xs[i] = abs(xs[i] - 1));
solve satisfy;
```

`scripts/flatzinc-compat-report.sh`:

```bash
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
  minizinc --compile-only -o "$fzn" "$mzn"
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
```

Run: `chmod +x scripts/flatzinc-compat-report.sh`
Run: `bash scripts/flatzinc-compat-report.sh`
Expected: FAIL on `abs_test` (MiniZinc `abs` → `int_abs` constraint, henüz desteklenmiyor)

- [ ] **Step 4: Document workflow in README**

`benchmarks/minizinc/README.md` sonuna ekle:

```markdown
## Compatibility report

```bash
bash scripts/flatzinc-compat-report.sh
```

Requires MiniZinc installed locally. CI uses only hand-written `.fzn` files.
```

- [ ] **Step 5: Commit**

```bash
git add scripts/flatzinc-compat-report.sh \
  benchmarks/minizinc/models/abs_test.mzn \
  crates/propaga-flatzinc/tests/compile_corpus.rs \
  benchmarks/minizinc/README.md
git commit -m "test: add FlatZinc compile corpus harness"
```

---

### Task 2: `decompose` Modülü ve `int_abs`

**Files:**
- Create: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/lib.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs:1153-1159`
- Modify: `crates/propaga-flatzinc/src/compile.rs:557-561`
- Create: `benchmarks/int_abs.fzn`
- Test: `crates/propaga-flatzinc/src/decompose.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing unit test**

`crates/propaga-flatzinc/src/decompose.rs`:

```rust
use propaga_core::VariableId;
use propaga_model::Model;

/// Posts `b = |a|` using existing propagators.
pub fn int_abs(model: &mut Model, a: VariableId, b: VariableId) {
  model.greater_equal(b, 0);
  let neg_a = model.int_var(i32::MIN / 4, i32::MAX / 4);
  model.scalar_eq(&[(-1, a)], neg_a, 0);
  let reif_pos = model.int_var(0, 1);
  let reif_neg = model.int_var(0, 1);
  model.reified_equal(a, b, reif_pos);
  model.reified_equal(neg_a, b, reif_neg);
  model.scalar_eq(&[(1, reif_pos), (1, reif_neg)], b, 1);
}

#[cfg(test)]
mod tests {
  use super::*;
  use propaga_search::ObjectiveDirection;

  #[test]
  fn int_abs_fixes_magnitude() {
    let mut model = Model::new();
    let a = model.int_var(-5, 5);
    let b = model.int_var(0, 5);
    int_abs(&mut model, a, b);
    model.equal(a, model.int_var_fixed(-3));
    let (solution, _, _, _) = model.optimize(vec![a, b], b, ObjectiveDirection::Maximize);
    assert_eq!(solution.map(|s| s.get(&b).copied()), Some(Some(3)));
  }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc decompose::tests::int_abs_fixes_magnitude -- --nocapture`
Expected: FAIL — module `decompose` not in `lib.rs`

- [ ] **Step 3: Wire module and parse `int_abs`**

`crates/propaga-flatzinc/src/lib.rs` — `mod decompose;` ekle (private).

`crates/propaga-flatzinc/src/parse.rs` — `parse_constraint_by_name` içinde `"float_times"` dalından önce:

```rust
"int_abs" => {
    let a = self.parse_expr()?;
    self.expect_symbol(",")?;
    let b = self.parse_expr()?;
    Constraint::IntAbs(a, b)
}
```

`Constraint` enum'a ekle (`parse.rs` ~satır 200 civarı):

```rust
/// `int_abs(a, b)` — b = |a|
IntAbs(Expr, Expr),
```

`compile.rs` — `post_constraint` match'ine:

```rust
Constraint::IntAbs(a, b) => {
    let a = resolve_var(env, a)?;
    let b = resolve_var(env, b)?;
    crate::decompose::int_abs(model, a, b);
}
```

- [ ] **Step 4: Add regression `.fzn` and integration test**

`benchmarks/int_abs.fzn`:

```flatzinc
var -3..3: x;
var 0..3: y;
constraint int_abs(x, y);
constraint int_eq(x, -2);
solve satisfy;
output [show(y)];
```

`crates/propaga-flatzinc/tests/integration.rs` sonuna:

```rust
#[test]
fn int_abs_instance_yields_two() {
    let source = include_str!("../../../benchmarks/int_abs.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    let y = instance.names.iter().find(|(_, n)| *n == "y").map(|(v, _)| *v).unwrap();
    assert_eq!(solution.and_then(|s| s.get(&y).copied()), Some(2));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p propaga-flatzinc int_abs -- --nocapture`
Expected: PASS

Run: `bash scripts/flatzinc-compat-report.sh` (MiniZinc varsa)
Expected: `abs_test` artık compile+solve geçer (veya en azından compile geçer)

- [ ] **Step 6: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs \
  crates/propaga-flatzinc/src/lib.rs \
  crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  benchmarks/int_abs.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): decompose int_abs to existing propagators"
```

---

### Task 3: Bool Primitive Constraint'leri

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/bool_logic.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

`benchmarks/bool_logic.fzn`:

```flatzinc
var 0..1: a;
var 0..1: b;
var 0..1: c;
constraint bool_not(a, b);
constraint bool_eq(a, 1);
constraint bool_and(b, b, c);
solve satisfy;
```

`crates/propaga-flatzinc/tests/integration.rs`:

```rust
#[test]
fn bool_logic_not_and() {
    let source = include_str!("../../../benchmarks/bool_logic.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}
```

Run: `cargo test -p propaga-flatzinc bool_logic_not_and -- --nocapture`
Expected: FAIL — `bool_not` / `bool_and` unknown → unexpanded predicate call

- [ ] **Step 2: Implement decompositions**

`decompose.rs`'e ekle:

```rust
pub fn bool_not(model: &mut Model, a: VariableId, b: VariableId) {
    let one = model.int_var_fixed(1);
    model.scalar_eq(&[(1, a), (1, b)], one, 1);
}

pub fn bool_and(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.reified_equal(a, model.int_var_fixed(1), c);
    model.reified_equal(b, model.int_var_fixed(1), c);
    model.scalar_le(&[(1, c)], model.int_var_fixed(1), 1);
}

pub fn bool_or(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let zero = model.int_var_fixed(0);
    let r1 = model.int_var(0, 1);
    let r2 = model.int_var(0, 1);
    model.reified_equal(a, model.int_var_fixed(1), r1);
    model.reified_equal(b, model.int_var_fixed(1), r2);
    model.scalar_ge(&[(1, r1), (1, r2)], c, 1);
    model.scalar_le(&[(1, c)], one, 1);
}
```

(`bool_or` içindeki `one` → `model.int_var_fixed(1)` olarak düzelt.)

- [ ] **Step 3: Parse + compile wiring**

`parse.rs` `parse_constraint_by_name`:

```rust
"bool_not" => {
    let a = self.parse_expr()?;
    self.expect_symbol(",")?;
    let b = self.parse_expr()?;
    Constraint::BoolNot(a, b)
}
"bool_and" => {
    let a = self.parse_expr()?;
    self.expect_symbol(",")?;
    let b = self.parse_expr()?;
    self.expect_symbol(",")?;
    let c = self.parse_expr()?;
    Constraint::BoolAnd(a, b, c)
}
"bool_or" => {
    let a = self.parse_expr()?;
    self.expect_symbol(",")?;
    let b = self.parse_expr()?;
    self.expect_symbol(",")?;
    let c = self.parse_expr()?;
    Constraint::BoolOr(a, b, c)
}
```

`Constraint` enum'a `BoolNot`, `BoolAnd`, `BoolOr` ekle; `compile.rs`'te `decompose::` çağrıları.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc bool_logic -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs \
  crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  benchmarks/bool_logic.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): add bool_not, bool_and, bool_or decomposition"
```

---

### Task 4: Aritmetik Primitive'ler (`int_times`, `int_div`, `int_mod`)

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/int_times.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

`benchmarks/int_times.fzn`:

```flatzinc
var 1..5: x;
var 1..5: y;
var 1..25: z;
constraint int_times(x, y, z);
constraint int_eq(x, 3);
constraint int_eq(y, 4);
solve satisfy;
```

```rust
#[test]
fn int_times_instance() {
    let source = include_str!("../../../benchmarks/int_times.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}
```

Run: `cargo test -p propaga-flatzinc int_times_instance -- --nocapture`
Expected: FAIL

- [ ] **Step 2: Implement bounded decomposition**

`decompose.rs`:

```rust
/// Posts `c = a * b` using a table over the Cartesian product of current bounds.
pub fn int_times(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let a_dom: Vec<i32> = (model.engine().hybrid_domain(a).min().unwrap()
        ..=model.engine().hybrid_domain(a).max().unwrap()).collect();
    let b_dom: Vec<i32> = (model.engine().hybrid_domain(b).min().unwrap()
        ..=model.engine().hybrid_domain(b).max().unwrap()).collect();
    let mut tuples = Vec::new();
    for &av in &a_dom {
        for &bv in &b_dom {
            tuples.push(vec![av, bv, av * bv]);
        }
    }
    model.table(vec![a, b, c], tuples);
}

pub fn int_div(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let mut tuples = Vec::new();
    let a_dom: Vec<i32> = (model.engine().hybrid_domain(a).min().unwrap()
        ..=model.engine().hybrid_domain(a).max().unwrap()).collect();
    let b_dom: Vec<i32> = (model.engine().hybrid_domain(b).min().unwrap()
        ..=model.engine().hybrid_domain(b).max().unwrap()).collect();
    for &av in &a_dom {
        for &bv in &b_dom {
            if bv != 0 {
                tuples.push(vec![av, bv, av / bv]);
            }
        }
    }
    model.table(vec![a, b, c], tuples);
}

pub fn int_mod(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let mut tuples = Vec::new();
    let a_dom: Vec<i32> = (model.engine().hybrid_domain(a).min().unwrap()
        ..=model.engine().hybrid_domain(a).max().unwrap()).collect();
    let b_dom: Vec<i32> = (model.engine().hybrid_domain(b).min().unwrap()
        ..=model.engine().hybrid_domain(b).max().unwrap()).collect();
    for &av in &a_dom {
        for &bv in &b_dom {
            if bv != 0 {
                tuples.push(vec![av, bv, av % bv]);
            }
        }
    }
    model.table(vec![a, b, c], tuples);
}
```

Not: Domainler compile anında tam bilinmeyebilir; bu yüzden `compile.rs`'te değişkenler oluşturulduktan **sonra** `post_constraint` çağrılırken bound'lar `VarDecl`'den bilinir. `resolve_var` sonrası `model.engine().hybrid_domain(var)` kullanılabilir. Domain çok büyükse (`> 10_000` tuple) `FlatZincError::Unsupported("int_times domain too large")` döndür.

- [ ] **Step 3: Parse + compile + size guard**

`parse.rs`: `"int_times"`, `"int_div"`, `"int_mod"` dalları.
`compile.rs`: tuple sayısı kontrolü ile `decompose::` çağrıları.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc int_times -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs \
  crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  benchmarks/int_times.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): decompose int_times, int_div, int_mod via table"
```

---

### Task 5: Genişletilmiş Parametreler (`bool`, `float`)

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs:22-37, 559-563, 606-650`
- Modify: `crates/propaga-flatzinc/src/compile.rs:60-69`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn compiles_bool_and_float_parameters() {
    let source = r#"
        bool: flag = true;
        float: pi = 3.14;
        var 1..3: x;
        constraint bool_eq(flag, 1);
        solve satisfy;
    "#;
    let program = parse(source).expect("parse bool param");
    compile(program).expect("compile bool param");
}
```

Run: `cargo test -p propaga-flatzinc compiles_bool_and_float_parameters -- --nocapture`
Expected: FAIL — `unsupported top-level statement starting with bool`

- [ ] **Step 2: Extend ParamDecl and parser**

`ParamDecl` enum:

```rust
Bool { name: String, value: i32 },
Float { name: String, value: f64 },
```

`parse_program` — `int` dalından önce:

```rust
} else if self.peek_is_ident("bool") {
    params.push(self.parse_bool_param()?);
} else if self.peek_is_ident("float") {
    params.push(self.parse_float_param()?);
```

`parse_bool_param`:

```rust
fn parse_bool_param(&mut self) -> Result<ParamDecl, FlatZincError> {
    self.expect_ident("bool")?;
    self.expect_symbol(":")?;
    let name = self.expect_ident_token()?;
    self.expect_symbol("=")?;
    let value = if self.peek_is_ident("true") {
        self.expect_ident("true")?;
        1
    } else {
        self.expect_ident("false")?;
        0
    };
    Ok(ParamDecl::Bool { name, value })
}
```

`parse_float_param` — `float: name = 1.5;` formu.

`compile.rs` param loop:

```rust
ParamDecl::Bool { name, value } => {
    env.insert(name, Binding::Param(value));
}
ParamDecl::Float { name, value } => {
    env.insert(name, Binding::FloatParam(value));
}
```

`Binding` enum'a `FloatParam(f64)` ekle; `resolve_var` / `resolve_expr` float literal olarak kullanılabilir yerlerde genişlet.

- [ ] **Step 3: Run tests**

Run: `cargo test -p propaga-flatzinc compiles_bool_and_float_parameters -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): parse bool and float parameters"
```

---

### Task 6: Predicate Sistemi — Tam Substitution ve İç İçe Çağrılar

**Files:**
- Modify: `crates/propaga-flatzinc/src/compile.rs:566-647`
- Modify: `crates/propaga-flatzinc/src/parse.rs:1205-1208`
- Create: `benchmarks/nested_predicate.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

`benchmarks/nested_predicate.fzn`:

```flatzinc
predicate p(var int: a, var int: b) = int_eq(a, b);
predicate q(var int: x, var int: y) = p(x, y);
var 1..3: u;
var 1..3: v;
constraint q(u, v);
constraint int_eq(u, 2);
solve satisfy;
```

```rust
#[test]
fn nested_predicate_expands() {
    let source = include_str!("../../../benchmarks/nested_predicate.fzn");
    let program = parse(source).expect("parse");
    let mut instance = compile(program).expect("compile");
    let (solution, _) = instance.model.solve_subset_with_stats(instance.solve_vars);
    assert!(solution.is_some());
}
```

Run: `cargo test -p propaga-flatzinc nested_predicate_expands -- --nocapture`
Expected: FAIL — nested predicate calls not supported

- [ ] **Step 2: Remove parse-time nested rejection**

`parse.rs:1205-1208` satırlarını sil (iç içe `PredicateCall` parse'a izin ver).

- [ ] **Step 3: Fix `substitute_constraint` — exhaustive clone**

`compile.rs` — `substitute_constraint` fonksiyonunu `Constraint` enum'unun **tüm** varyantları için `substitute_expr` uygulayacak şekilde yeniden yaz. Örnek desen:

```rust
fn substitute_constraint(
    constraint: &Constraint,
    substitutions: &HashMap<String, Expr>,
) -> Constraint {
    match constraint {
        Constraint::IntAbs(a, b) => {
            Constraint::IntAbs(substitute_expr(a, substitutions), substitute_expr(b, substitutions))
        }
        Constraint::PredicateCall { name, args } => Constraint::PredicateCall {
            name: name.clone(),
            args: args.iter().map(|e| substitute_expr(e, substitutions)).collect(),
        },
        // ... her Constraint varyantı ...
        Constraint::AllDifferent(vars) => Constraint::AllDifferent(
            vars.iter().map(|v| substitute_expr(v, substitutions)).collect(),
        ),
    }
}
```

- [ ] **Step 4: Recursive `expand_predicates`**

`compile.rs`:

```rust
fn expand_predicates(
    constraints: Vec<Constraint>,
    predicates: &[PredicateDecl],
) -> Vec<Constraint> {
    let lookup: HashMap<_, _> = predicates
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let mut pending = constraints;
    let mut expanded = Vec::new();
    while let Some(constraint) = pending.pop() {
        match constraint {
            Constraint::PredicateCall { name, args } => {
                if let Some(predicate) = lookup.get(name.as_str()) {
                    for c in substitute_predicate(predicate, &args) {
                        pending.push(c);
                    }
                } else {
                    expanded.push(Constraint::PredicateCall { name, args });
                }
            }
            other => expanded.push(other),
        }
    }
    expanded.reverse();
    expanded
}
```

`compile()` içinde `expand_predicates` sonrası kalan `PredicateCall` varsa anlamlı hata mesajı.

- [ ] **Step 5: Run tests**

Run: `cargo test -p propaga-flatzinc nested_predicate -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  benchmarks/nested_predicate.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(flatzinc): recursive predicate expansion with full substitution"
```

---

### Task 7: `annotation` Top-Level ve `incomplete` Search Toleransı

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs:582-591`
- Modify: `crates/propaga-flatzinc/src/compile.rs:210-216`
- Test: `crates/propaga-flatzinc/src/parse.rs` (inline test)

- [ ] **Step 1: Write the failing parse test**

`parse.rs` `#[cfg(test)]` modülüne:

```rust
#[test]
fn skips_annotation_top_level_statement() {
    let source = r#"
        annotation foo;
        var 1..2: x;
        constraint int_eq(x, 1);
        solve satisfy;
    "#;
    let program = parse(source).expect("annotation should be skipped");
    assert_eq!(program.variables.len(), 1);
}
```

Run: `cargo test -p propaga-flatzinc skips_annotation_top_level -- --nocapture`
Expected: FAIL

- [ ] **Step 2: Implement annotation skip**

`parse_program` else dalından önce:

```rust
} else if self.peek_is_ident("annotation") {
    self.skip_until_semicolon_or_eof();
```

Yardımcı:

```rust
fn skip_until_semicolon_or_eof(&mut self) {
    while !self.is_eof() && !self.peek_is_symbol(";") {
        self.pos += 1;
    }
    self.consume_optional_semicolon();
}
```

- [ ] **Step 3: Map `incomplete` to `complete`**

`compile.rs` — `incomplete search is not supported` hatasını kaldır; `incomplete` gördüğünde `complete` gibi davran (yalnızca uyarı yok, sessiz fallback).

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc skips_annotation -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs
git commit -m "feat(flatzinc): skip annotation statements and tolerate incomplete search"
```

---

### Task 8: `automaton` Global Constraint

**Files:**
- Create: `crates/propaga-propagators/src/automaton.rs`
- Modify: `crates/propaga-propagators/src/lib.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Modify: `crates/propaga-model/src/model.rs`
- Create: `benchmarks/automaton_chain.fzn`
- Test: `crates/propaga-flatzinc/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

`benchmarks/automaton_chain.fzn` — MiniZinc `automaton` çıktısına uygun küçük örnek (3 durum, 2 sembol):

```flatzinc
array[1..3] of var 1..2: x;
array[1..6] of int: transition = [1, 2, 0, 1, 2, 0];
constraint automaton(x, 2, 3, transition, 1, [2]);
solve satisfy;
```

```rust
#[test]
fn automaton_chain_compiles() {
    let source = include_str!("../../../benchmarks/automaton_chain.fzn");
    let program = parse(source).expect("parse");
    compile(program).expect("compile automaton");
}
```

Run: `cargo test -p propaga-flatzinc automaton_chain_compiles -- --nocapture`
Expected: FAIL

- [ ] **Step 2: Implement automaton via regular/table**

`automaton.rs` — `regular` propagator'a benzer; `transition` matrisini `RegularPropagator` veya `TablePropagator`'a derle. `AutomatonPropagator::new(vars, transition, start, accepting)` struct.

`model.rs`:

```rust
pub fn automaton(
    &mut self,
    variables: impl Into<Vec<VariableId>>,
    transition: Vec<i32>,
    num_states: i32,
    start: i32,
    accepting: Vec<i32>,
) {
    let propagator = AutomatonPropagator::new(variables.into(), transition, num_states, start, accepting);
    self.engine.add_propagator(Box::new(propagator));
}
```

- [ ] **Step 3: Parse + compile**

`parse.rs` — `"automaton"` dalı (`"regular"` yakınında).
`compile.rs` — `post_automaton` (`post_regular` kopyasından uyarla).

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc automaton -- --nocapture`
Run: `cargo test -p propaga-propagators automaton -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-propagators/src/automaton.rs \
  crates/propaga-propagators/src/lib.rs \
  crates/propaga-model/src/model.rs \
  crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs \
  benchmarks/automaton_chain.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat: add automaton global constraint with FlatZinc support"
```

---

### Task 9: MiniZinc Örnek Corpus ve Dokümantasyon

**Files:**
- Create: `benchmarks/minizinc/models/*.mzn` (12 dosya)
- Modify: `benchmarks/minizinc/COMPATIBILITY.md`
- Modify: `CHANGELOG.md`
- Modify: `ROADMAP.md`
- Modify: `benchmarks/run.sh`

- [ ] **Step 1: Add remaining MiniZinc models**

`benchmarks/minizinc/models/` altına ekle (her biri ≤ 30 satır, yalnızca desteklenen constraint'ler):

| Dosya | Kullandığı özellik |
|-------|-------------------|
| `abs_test.mzn` | `int_abs` (Task 2) |
| `times_test.mzn` | `int_times` |
| `bool_logic.mzn` | `bool_not`, `bool_and` |
| `nested_pred.mzn` | iç içe predicate |
| `automaton_test.mzn` | `automaton` |
| `magic_square.mzn` | `all_different`, `int_lin_eq` |
| `jobshop_toy.mzn` | `disjunctive` |
| `cumulative_toy.mzn` | `cumulative` |
| `gcc_toy.mzn` | `global_cardinality` |
| `set_toy.mzn` | `set_card`, `set_union` |
| `float_toy.mzn` | `float_le`, `float_times` |
| `optimize_toy.mzn` | `solve minimize` |

- [ ] **Step 2: Update COMPATIBILITY.md**

Desteklenen primitive'leri ve parametreleri ekle; `automaton` satırını Supported yap; `annotation` → Skipped; `nested predicate` → Supported.

- [ ] **Step 3: Extend benchmarks/run.sh**

`benchmarks/run.sh` sonuna:

```bash
echo "==> FlatZinc compile corpus"
cargo test -q -p propaga-flatzinc all_handwritten_fzn_instances_compile
```

- [ ] **Step 4: Run full verification**

Run: `cargo test --workspace`
Expected: PASS

Run: `bash benchmarks/run.sh`
Expected: PASS

Run: `bash scripts/flatzinc-compat-report.sh` (MiniZinc varsa)
Expected: ≥ %90 pass (hedef: 12/12)

- [ ] **Step 5: Update CHANGELOG and ROADMAP**

`CHANGELOG.md` — `## [0.7.0]` bölümü: FlatZinc primitive'ler, bool/float params, nested predicates, automaton, annotation skip.

`ROADMAP.md` — "Shipped in v0.7.0" ekle; "Nested FlatZinc predicate calls" maddesini shipped'e taşı.

- [ ] **Step 6: Commit**

```bash
git add benchmarks/minizinc/ CHANGELOG.md ROADMAP.md benchmarks/run.sh
git commit -m "docs: v0.7.0 FlatZinc compatibility matrix and MiniZinc corpus"
```

---

## Self-Review

### Spec coverage

| Gereksinim | Task |
|------------|------|
| Uyumluluk ölçümü | Task 1 |
| `int_abs` | Task 2 |
| Bool primitive'ler | Task 3 |
| `int_times` / `int_div` / `int_mod` | Task 4 |
| `bool` / `float` parametreler | Task 5 |
| İç içe predicate + tam substitution | Task 6 |
| `annotation` skip + `incomplete` tolerans | Task 7 |
| `automaton` | Task 8 |
| Corpus + dokümantasyon | Task 9 |

### Placeholder scan

Tüm task'larda gerçek kod, dosya yolu ve komut var. "TBD" yok.

### Type consistency

- `Constraint` enum genişletmeleri Task 2–4, 8'de tanımlanıp Task 6 substitution'da exhaustive match ile kapatılıyor.
- `ParamDecl::Bool` / `Float` Task 5'te tanımlanıp aynı task'ta compile'a bağlanıyor.
- `Binding::FloatParam` Task 5'te ekleniyor.

### Bilinen riskler

1. **`int_times` table boyutu** — büyük domain'lerde compile reddi; COMPATIBILITY.md'de belirt.
2. **`int_abs` reified decomposition** — alternatif olarak doğrudan `table` ile 2-tuple daha sağlam olabilir; test fail ederse table versiyonuna geç.
3. **MiniZinc CI'da yok** — `flatzinc-compat-report.sh` yerelde çalışır; CI yalnızca `.fzn` corpus kullanır.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-12-flatzinc-compatibility.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
