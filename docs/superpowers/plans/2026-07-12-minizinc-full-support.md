# MiniZinc %100 Destek (v1.0.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** MiniZinc standart kütüphanesinden üretilen her geçerli FlatZinc 1.6 instance'ının Propaga'da parse → compile → solve hattından geçmesini sağlamak (v1.0.0).

**Architecture:** Propaga MiniZinc kaynak kodunu doğrudan okumaz; `minizinc --compile-only` çıktısını çözer. %100 destek = [FlatZinc builtins](https://docs.minizinc.dev/en/stable/lib-flatzinc.html) tamamı + MiniZinc stdlib'in generic solver profiline düşürdüğü global constraint'ler + set/float optimizasyon hedefleri. Üç katman: (1) ölçüm — eksik builtin'leri otomatik raporla; (2) `decompose.rs` — tablo/lineer ayrıştırma ile primitive'ler; (3) domain propagator'ları — set/float builtin'leri ve kalan global'ler.

**Tech Stack:** Rust 1.88+ (Edition 2024), `propaga-flatzinc`, `propaga-model`, `propaga-propagators`, `propaga-domains`, `propaga-search`, MiniZinc 2.9+ toolchain (yerel + CI), FlatZinc 1.6

---

## Mevcut Durum (v0.7.0)

| Alan | Durum | Dosya |
|------|-------|-------|
| Temel int/bool/set/float değişkenleri | Destekli | `crates/propaga-flatzinc/src/parse.rs:736-820` |
| Yaygın global'ler (alldiff, gcc, cumulative, …) | Destekli | `parse.rs:866-1334`, `compile.rs:319-631` |
| Primitive'ler (`int_abs`, `int_times`, `bool_*`) | Destekli | `decompose.rs`, `compile.rs:573-612` |
| İç içe predicate | Destekli | `compile.rs:641-880` |
| `bool` / `float` scalar parametreler | Parse only — ifade içinde kullanılamaz | `compile.rs:72`, `CHANGELOG.md:31-33` |
| `set` parametreler | **Yok** | `COMPATIBILITY.md:34` |
| Predicate `var set` / `var float` parametreleri | **Reddedilir** | `parse.rs:1359-1361` |
| `function` / `test` top-level | **Reddedilir** | `parse.rs:621-628` |
| Float/set optimizasyon hedefleri | **Yok** (int-only) | `memories.md:12` |
| FlatZinc builtins (int_plus, bool_xor, set_eq, float_abs, …) | **~40%** — çoğu `PredicateCall` olarak kalır | `parse.rs:1335-1340` |
| MiniZinc stdlib global'leri (count, among, lex_less, …) | **Kısmi** — decomposition yok | `COMPATIBILITY.md:56` |

## Hedef Tanımı (%100)

Aşağıdaki koşulların **tamamı** sağlanınca hedefe ulaşılmış sayılır:

1. MiniZinc [FlatZinc builtins](https://docs.minizinc.dev/en/stable/lib-flatzinc.html) listesindeki her predicate parse + compile edilir.
2. `benchmarks/minizinc/stdlib/` altındaki stdlib türevli test modelleri (bu planda tanımlı) `scripts/flatzinc-full-compat-report.sh` ile **0 hata**.
3. Mevcut `benchmarks/*.fzn` regression seti kırılmaz (`compile_corpus.rs` PASS).
4. `solve minimize` / `maximize` float ve set değişken hedeflerini destekler.
5. `COMPATIBILITY.md` matrisi "Supported" olarak güncellenir; bilinçli kapsam dışı kalmaz.

## Kapsam Dışı (bilinçli)

- MiniZinc kaynak (`.mzn`) parser — Propaga yalnızca FlatZinc çözer.
- Özel solver-specific `redefinitions.mzn` profilleri (yalnızca default stdlib decomposition).
- GPU / dış solver entegrasyonu.
- NP-hard global'ler için optimal propagasyon gücü — doğruluk (soundness) yeterli; tablo/BC ayrıştırma kabul edilir.

## Sürüm Planı

| Sürüm | Odak | Kabul kriteri |
|-------|------|---------------|
| v0.8.0 | Int/bool FlatZinc builtins + ölçüm altyapısı | Builtin gap raporu ≥ %70 → %95 |
| v0.9.0 | Set builtins + predicate parametre genişletmesi | Set matrisi tamam |
| v1.0.0-beta | Float builtins + interval aritmetik | Float matrisi tamam |
| v1.0.0 | Stdlib global'ler + float/set optimize + dokümantasyon | Full compat script 0 fail |

---

## File Structure

| Dosya | Sorumluluk |
|-------|------------|
| `scripts/flatzinc-builtin-inventory.sh` | MiniZinc stdlib FlatZinc çıktısından builtin isimlerini çıkarır |
| `scripts/flatzinc-full-compat-report.sh` | Derle + parse + compile + solve tam rapor |
| `benchmarks/minizinc/stdlib/*.mzn` | Stdlib türevli minimal test modelleri |
| `benchmarks/minizinc/expected/*.json` | Beklenen compile/solve sonuçları |
| `crates/propaga-flatzinc/src/builtins.rs` | Builtin isim → `Constraint` enum eşlemesi (parse.rs sadeleştirme) |
| `crates/propaga-flatzinc/src/decompose.rs` | Int/bool primitive tablo ayrıştırması (genişletilmiş) |
| `crates/propaga-flatzinc/src/decompose_float.rs` | Float primitive interval ayrıştırması |
| `crates/propaga-flatzinc/src/decompose_set.rs` | Set primitive ayrıştırması |
| `crates/propaga-flatzinc/src/parse.rs` | Yeni constraint/parameter/predicate parse |
| `crates/propaga-flatzinc/src/compile.rs` | Decompose entegrasyonu, float/set objective |
| `crates/propaga-flatzinc/tests/builtin_corpus.rs` | Her builtin için compile regression |
| `crates/propaga-domains/src/float.rs` | `plus`, `abs`, `pow`, trig interval yardımcıları |
| `crates/propaga-propagators/src/float_*.rs` | Yeni float propagator'lar |
| `crates/propaga-propagators/src/set_*.rs` | `set_eq`, `set_diff`, `set_in`, … |
| `crates/propaga-model/src/model.rs` | Yeni Model API yüzeyi |
| `crates/propaga-search/src/optimize.rs` | Float/set objective değerlendirme |
| `benchmarks/minizinc/COMPATIBILITY.md` | Güncellenmiş %100 matris |
| `CHANGELOG.md`, `ROADMAP.md`, `README.md` | v1.0.0 notları |

---

### Task 1: Builtin Gap Ölçüm Altyapısı

**Files:**
- Create: `scripts/flatzinc-builtin-inventory.sh`
- Create: `scripts/flatzinc-full-compat-report.sh`
- Create: `benchmarks/minizinc/stdlib/int_plus.mzn`
- Create: `crates/propaga-flatzinc/tests/builtin_corpus.rs`
- Modify: `benchmarks/minizinc/README.md`

- [ ] **Step 1: Write the failing builtin corpus test**

`crates/propaga-flatzinc/tests/builtin_corpus.rs`:

```rust
use propaga_flatzinc::{compile, parse};
use std::fs;
use std::path::PathBuf;

fn stdlib_models_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/minizinc/stdlib")
}

#[test]
fn all_stdlib_mzn_models_compile_when_fzn_present() {
    let dir = stdlib_models_dir();
    if !dir.exists() {
        return; // MiniZinc corpus optional in CI without toolchain
    }
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/flatzinc-stdlib");
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).expect("read stdlib models") {
        let entry = entry.expect("dir entry");
        let mzn = entry.path();
        if mzn.extension().and_then(|s| s.to_str()) != Some("mzn") {
            continue;
        }
        let base = mzn.file_stem().unwrap().to_string_lossy();
        let fzn = out_dir.join(format!("{base}.fzn"));
        if !fzn.exists() {
            failures.push(format!("{base}: missing precompiled {fzn:?}"));
            continue;
        }
        let source = fs::read_to_string(&fzn).expect("read fzn");
        match parse(&source).and_then(compile) {
            Ok(_) => {}
            Err(err) => failures.push(format!("{base}: {err}")),
        }
    }
    assert!(
        failures.is_empty(),
        "stdlib compile failures:\n{}",
        failures.join("\n")
    );
}
```

- [ ] **Step 2: Run test to verify baseline**

Run: `cargo test -p propaga-flatzinc all_stdlib_mzn_models_compile_when_fzn_present -- --nocapture`
Expected: PASS (boş corpus veya eksik fzn dosyaları skip — henüz model yok)

- [ ] **Step 3: Add first stdlib stub and inventory script**

`benchmarks/minizinc/stdlib/int_plus.mzn`:

```minizinc
var 1..5: a;
var 1..5: b;
var 2..10: c;
constraint c = a + b;
solve satisfy;
```

`scripts/flatzinc-builtin-inventory.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/builtin-inventory"
mkdir -p "$OUT"

if ! command -v minizinc >/dev/null 2>&1; then
  echo "minizinc not found"
  exit 0
fi

# Compile each stdlib test model and list constraint predicate names
for mzn in "$ROOT/benchmarks/minizinc/stdlib"/*.mzn; do
  base="$(basename "$mzn" .mzn)"
  fzn="$OUT/$base.fzn"
  minizinc --compile-only -o "$fzn" "$mzn"
  rg -o 'constraint [a-zA-Z0-9_]+' "$fzn" | awk '{print $2}' | sort -u > "$OUT/$base.constraints"
done

echo "Inventory written to $OUT"
```

`scripts/flatzinc-full-compat-report.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/flatzinc-full-compat"
mkdir -p "$OUT"

if ! command -v minizinc >/dev/null 2>&1; then
  echo "minizinc not found; skip"
  exit 0
fi

pass=0; fail=0
for mzn in "$ROOT/benchmarks/minizinc"/{models,stdlib}/*.mzn; do
  [[ -f "$mzn" ]] || continue
  base="$(basename "$mzn" .mzn)"
  fzn="$OUT/$base.fzn"
  minizinc --compile-only -o "$fzn" "$mzn"
  if cargo run -q -p propaga-cli -- solve --file "$fzn" --quiet >/dev/null 2>&1; then
    echo "OK  $base"; pass=$((pass + 1))
  else
    echo "FAIL $base"; fail=$((fail + 1))
  fi
done
echo "==> $pass passed, $fail failed"
test "$fail" -eq 0
```

- [ ] **Step 4: Make scripts executable and document**

Run: `chmod +x scripts/flatzinc-builtin-inventory.sh scripts/flatzinc-full-compat-report.sh`
Expected: exit 0

- [ ] **Step 5: Commit**

```bash
git add scripts/flatzinc-builtin-inventory.sh scripts/flatzinc-full-compat-report.sh \
  benchmarks/minizinc/stdlib/int_plus.mzn \
  crates/propaga-flatzinc/tests/builtin_corpus.rs \
  benchmarks/minizinc/README.md
git commit -m "feat(flatzinc): add MiniZinc full-compat measurement scaffolding"
```

---

### Task 2: `int_plus` ve `int_lin_ne` Primitive'leri

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs:866` (constraint match)
- Modify: `crates/propaga-flatzinc/src/compile.rs:319` (post_constraint)
- Create: `benchmarks/int_plus.fzn`
- Create: `benchmarks/int_lin_ne.fzn`

- [ ] **Step 1: Write the failing decompose test**

`crates/propaga-flatzinc/src/decompose.rs` — append to `mod tests`:

```rust
#[test]
fn int_plus_table_posts_sum() {
    let mut model = Model::new();
    let a = model.int_var(1, 3);
    let b = model.int_var(1, 3);
    let c = model.int_var(2, 6);
    int_plus(&mut model, a, b, c);
    let (solution, _, _, _) = model.optimize(vec![c], c, ObjectiveDirection::Maximize);
    assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(6));
}
```

Append to `decompose.rs` (stub):

```rust
/// Posts `c = a + b` using a domain table.
pub fn int_plus(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    todo!("int_plus")
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc int_plus_table_posts_sum -- --nocapture`
Expected: FAIL — `todo!("int_plus")` panic or function missing

- [ ] **Step 3: Implement int_plus and int_lin_ne decomposition**

`crates/propaga-flatzinc/src/decompose.rs`:

```rust
/// Posts `c = a + b` using a domain table.
pub fn int_plus(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let a_len = (amax - amin + 1) as usize;
    let b_len = (bmax - bmin + 1) as usize;
    if table_too_large(a_len.saturating_mul(b_len)) {
        // Fallback: channel via auxiliary linear equality
        let sum = model.int_var(amin + bmin, amax + bmax);
        model.scalar_eq(&[1, 1], &[a, b], sum);
        model.equal(sum, c);
        return;
    }
    let mut tuples = Vec::with_capacity(a_len * b_len);
    for av in amin..=amax {
        for bv in bmin..=bmax {
            tuples.push(vec![av, bv, av + bv]);
        }
    }
    model.table(vec![a, b, c], tuples);
}
```

`parse.rs` — add to `parse_constraint_by_name` match before `other =>`:

```rust
"int_plus" => {
    let a = self.parse_expr()?;
    self.expect_symbol(",")?;
    let b = self.parse_expr()?;
    self.expect_symbol(",")?;
    let c = self.parse_expr()?;
    Constraint::IntPlus(a, b, c)
}
"int_lin_ne" => {
    self.expect_symbol("[")?;
    let coeffs = self.parse_int_list()?;
    self.expect_symbol("]")?;
    self.expect_symbol(",")?;
    self.expect_symbol("[")?;
    let vars = self.parse_expr_list()?;
    self.expect_symbol("]")?;
    self.expect_symbol(",")?;
    let rhs = self.expect_int()?;
    Constraint::IntLinNe { coeffs, vars, rhs }
}
```

`Constraint` enum'a ekle (`parse.rs`):

```rust
/// `int_plus(a, b, c)`
IntPlus(Expr, Expr, Expr),
/// `int_lin_ne(coeffs, vars, rhs)`
IntLinNe {
    coeffs: Vec<i32>,
    vars: Vec<Expr>,
    rhs: i32,
},
```

`compile.rs` — `post_constraint` match arms:

```rust
Constraint::IntPlus(a, b, c) => {
    let a = resolve_var(env, a)?;
    let b = resolve_var(env, b)?;
    let c = resolve_var(env, c)?;
    crate::decompose::int_plus(model, a, b, c);
}
Constraint::IntLinNe { coeffs, vars, rhs } => {
    post_int_lin_ne(model, env, coeffs, vars, *rhs)?;
}
```

`compile.rs` — yeni yardımcı:

```rust
fn post_int_lin_ne(
    model: &mut Model,
    env: &HashMap<String, Binding>,
    coeffs: Vec<i32>,
    vars: Vec<Expr>,
    rhs: i32,
) -> Result<(), FlatZincError> {
    let variables = resolve_var_list(env, Expr::List(vars))?;
    let reif = model.int_var(0, 1);
    model.reified_scalar_eq(&coeffs, &variables, rhs, reif);
    model.equal(reif, model.int_var_fixed(0));
    Ok(())
}
```

`substitute_constraint` içine `IntPlus` ve `IntLinNe` kollarını ekle (mevcut `IntLinEq` kalıbıyla aynı).

`benchmarks/int_plus.fzn`:

```flatzinc
var 1..3: a;
var 1..3: b;
var 2..6: c;
constraint int_plus(a, b, c);
solve satisfy;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc int_plus -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs benchmarks/int_plus.fzn benchmarks/int_lin_ne.fzn
git commit -m "feat(flatzinc): add int_plus and int_lin_ne primitives"
```

---

### Task 3: Int Min/Max/Pow ve `array_int_element` Varyantları

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/minizinc/stdlib/int_min.mzn`
- Create: `benchmarks/minizinc/stdlib/array_var_int_element.mzn`
- Create: `benchmarks/int_min.fzn`

- [ ] **Step 1: Write failing tests**

`decompose.rs` tests:

```rust
#[test]
fn int_min_selects_smaller() {
    let mut model = Model::new();
    let a = model.int_var_fixed(3);
    let b = model.int_var_fixed(7);
    let c = model.int_var(0, 10);
    int_min(&mut model, a, b, c);
    let (solution, _, _, _) = model.solve_subset_with_stats(vec![c]);
    assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(3));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc int_min_selects_smaller -- --nocapture`
Expected: FAIL — `int_min` not defined

- [ ] **Step 3: Implement primitives**

`decompose.rs`:

```rust
pub fn int_min(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        build_binary_op_tuples(model, a, b, |x, y| x.min(y)),
    );
}

pub fn int_max(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        build_binary_op_tuples(model, a, b, |x, y| x.max(y)),
    );
}

pub fn int_pow(
    model: &mut Model,
    base: VariableId,
    exp: VariableId,
    result: VariableId,
) -> Result<(), String> {
    let (bmin, bmax) = domain_range(model, base);
    let (emin, emax) = domain_range(model, exp);
    let mut tuples = Vec::new();
    for b in bmin..=bmax {
        for e in emin..=emax {
            let value = b.pow(e.max(0) as u32);
            tuples.push(vec![b, e, value]);
        }
    }
    if table_too_large(tuples.len()) {
        return Err("int_pow domain too large".to_string());
    }
    model.table(vec![base, exp, result], tuples);
    Ok(())
}

fn build_binary_op_tuples(
    model: &Model,
    a: VariableId,
    b: VariableId,
    op: impl Fn(i32, i32) -> i32,
) -> Vec<Vec<i32>> {
    let (amin, amax) = domain_range(model, a);
    let (bmin, bmax) = domain_range(model, b);
    let mut tuples = Vec::new();
    for av in amin..=amax {
        for bv in bmin..=bmax {
            tuples.push(vec![av, bv, op(av, bv)]);
        }
    }
    tuples
}
```

`array_var_int_element` → mevcut `element` propagator'ına yönlendir (`compile.rs`):

```rust
Constraint::ArrayVarIntElement { array, index, value } => {
    let array_vars = resolve_var_list(env, array)?;
    let index_var = resolve_var(env, index)?;
    let value_var = resolve_var(env, value)?;
    model.element(array_vars, index_var, value_var);
}
```

Parse/compile/substitute için şu isimleri ekle: `int_min`, `int_max`, `int_pow`, `int_pow_fixed`, `array_int_element`, `array_var_int_element`, `array_int_maximum`, `array_int_minimum`.

`int_pow_fixed(base, exp_const, result)` → `int_pow` ile sabit üs.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc decompose::tests -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs benchmarks/int_min.fzn \
  benchmarks/minizinc/stdlib/
git commit -m "feat(flatzinc): add int min/max/pow and array element builtins"
```

---

### Task 4: Bool FlatZinc Builtins (`bool_xor`, `bool_clause`, reified formlar)

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/bool_xor.fzn`
- Create: `benchmarks/bool_clause.fzn`

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn bool_xor_matches_parity() {
    let mut model = Model::new();
    let a = model.int_var_fixed(1);
    let b = model.int_var_fixed(0);
    let c = model.int_var(0, 1);
    bool_xor(&mut model, a, b, c);
    let (solution, _, _, _) = model.solve_subset_with_stats(vec![c]);
    assert_eq!(solution.and_then(|s| assignment_int(&s, c)), Some(1));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc bool_xor_matches_parity -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement bool primitives**

`decompose.rs`:

```rust
pub fn bool_xor(model: &mut Model, a: VariableId, b: VariableId, c: VariableId) {
    model.table(
        vec![a, b, c],
        vec![vec![0, 0, 0], vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 0]],
    );
}

pub fn bool_clause(model: &mut Model, literals: &[VariableId]) {
    // At least one literal true: sum(lits) >= 1
    let coeffs = vec![1; literals.len()];
    model.scalar_ge(&coeffs, literals, 1);
}

pub fn bool_clause_reif(model: &mut Model, literals: &[VariableId], reif: VariableId) {
    let aux = model.int_var(0, 1);
    bool_clause(model, literals);
    model.reified_scalar_ge(&vec![1; literals.len()], literals, 1, aux);
    model.equal(aux, reif);
}
```

Parse/compile/substitute için ekle: `bool_xor`, `bool_clause`, `bool_clause_reif`, `bool_eq_reif`, `bool_le`, `bool_le_reif`, `bool_lt`, `bool_lt_reif`, `bool_lin_eq`, `bool_lin_le`, `array_bool_and`, `array_bool_xor`, `array_bool_element`, `array_var_bool_element`, `array_var_bool_element_nonshifted`.

`bool_*_reif` → mevcut `reified_equal` / `reified_less_equal` propagator'ları (`0..1` domain).

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc bool_xor -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose.rs crates/propaga-flatzinc/src/parse.rs \
  crates/propaga-flatzinc/src/compile.rs benchmarks/bool_xor.fzn benchmarks/bool_clause.fzn
git commit -m "feat(flatzinc): add bool xor/clause/reif builtins"
```

---

### Task 5: Set FlatZinc Builtins (`set_eq`, `set_in`, `set_diff`, …)

**Files:**
- Create: `crates/propaga-flatzinc/src/decompose_set.rs`
- Create: `crates/propaga-propagators/src/set_eq.rs`
- Create: `crates/propaga-propagators/src/set_in.rs`
- Create: `crates/propaga-propagators/src/set_diff.rs`
- Modify: `crates/propaga-flatzinc/src/lib.rs`
- Modify: `crates/propaga-model/src/model.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/set_eq.fzn`

- [ ] **Step 1: Write failing propagator test**

`crates/propaga-propagators/src/set_eq.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use propaga_engine::Engine;
    use propaga_domains::SetIntervalDomain;

    #[test]
    fn set_eq_forces_equal_sets() {
        let mut engine = Engine::new();
        let a = engine.new_set_var(SetIntervalDomain::range(1, 5, 2, 2));
        let b = engine.new_set_var(SetIntervalDomain::range(1, 5, 2, 2));
        engine.post(Box::new(SetEqPropagator::new(a, b)));
        engine.fix_set(a, &[1, 3]).unwrap();
        engine.propagate().unwrap();
        assert_eq!(engine.set_domain(b).glb(), &[1, 3]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-propagators set_eq_forces_equal_sets -- --nocapture`
Expected: FAIL — module not found

- [ ] **Step 3: Implement set propagators and FlatZinc wiring**

`set_eq.rs` — GLB/LUB eşitleme (mevcut `set_subset` propagator kalıbını izle).

`set_in.rs` — `value in set_var` için element membership.

`set_diff.rs` — `result = left \ right` için subset + cardinality kısıtları.

`decompose_set.rs`:

```rust
pub fn set_superset(model: &mut Model, superset: VariableId, subset: VariableId) {
    model.set_subset(subset, superset);
}

pub fn set_symdiff(model: &mut Model, a: VariableId, b: VariableId, result: VariableId) {
    let union = model.set_var_from(a);
    model.set_union(a, b, union);
    let inter = model.set_var_from(a);
    model.set_intersect(a, b, inter);
    model.set_diff(union, inter, result);
}
```

Parse/compile/substitute için ekle: `set_eq`, `set_ne`, `set_in`, `set_in_reif`, `set_le`, `set_lt`, `set_subset_reif`, `set_superset`, `set_superset_reif`, `set_diff`, `set_symdiff`, `array_set_element`, `array_var_set_element`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-propagators set_eq -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-propagators/src/set_eq.rs crates/propaga-propagators/src/set_in.rs \
  crates/propaga-propagators/src/set_diff.rs crates/propaga-flatzinc/src/decompose_set.rs \
  crates/propaga-flatzinc/src/lib.rs crates/propaga-model/src/model.rs \
  crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/compile.rs benchmarks/set_eq.fzn
git commit -m "feat(flatzinc): add set builtins and propagators"
```

---

### Task 6: Set Parametreleri ve Genişletilmiş Predicate Parametreleri

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs:602-608`
- Modify: `crates/propaga-flatzinc/src/parse.rs:1346-1374`
- Modify: `crates/propaga-flatzinc/src/compile.rs:60-80`
- Create: `benchmarks/set_param.fzn`

- [ ] **Step 1: Write failing parse test**

`parse.rs` `#[cfg(test)]`:

```rust
#[test]
fn parses_set_parameter() {
    let source = r#"
        set of 1..3: allowed = {1, 3};
        var 1..3: x;
        constraint set_in(x, allowed);
        solve satisfy;
    "#;
    let program = parse(source).expect("parse set param");
    assert_eq!(program.params.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc parses_set_parameter -- --nocapture`
Expected: FAIL — unsupported top-level `set`

- [ ] **Step 3: Implement set parameter parse and predicate var set/float params**

`ParamDecl` enum'a ekle:

```rust
Set { name: String, values: Vec<i32> },
```

`parse.rs` top-level:

```rust
} else if self.peek_is_ident("set") {
    params.push(self.parse_set_param()?);
```

`parse_set_param`:

```rust
fn parse_set_param(&mut self) -> Result<ParamDecl, FlatZincError> {
    self.expect_ident("set")?;
    self.expect_ident("of")?;
    let (low, high) = self.parse_domain()?;
    let _ = (low, high);
    self.expect_symbol(":")?;
    let name = self.expect_ident_token()?;
    self.expect_symbol("=")?;
    let values = self.parse_braced_int_set()?;
    Ok(ParamDecl::Set { name, values })
}
```

`compile.rs` `Binding` enum:

```rust
SetParam(Vec<i32>),
```

Predicate parametreleri — `parse_predicate_decl` içinde `var set` ve `var float` kabul et; substitution sırasında parametreyi `Binding` olarak taşı.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc parses_set_parameter -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/compile.rs benchmarks/set_param.fzn
git commit -m "feat(flatzinc): support set parameters and extended predicate params"
```

---

### Task 7: Float Interval Aritmetik ve Temel Float Builtins

**Files:**
- Modify: `crates/propaga-domains/src/float.rs`
- Create: `crates/propaga-flatzinc/src/decompose_float.rs`
- Create: `crates/propaga-propagators/src/float_abs.rs`
- Create: `crates/propaga-propagators/src/float_plus.rs`
- Modify: `crates/propaga-flatzinc/src/parse.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/float_abs.fzn`

- [ ] **Step 1: Write failing FloatDomain test**

`float.rs` tests:

```rust
#[test]
fn plus_interval_sound() {
    let a = FloatDomain::new(1.0, 2.0);
    let b = FloatDomain::new(3.0, 4.0);
    let sum = a.plus(b);
    assert!((sum.lower_bound() - 4.0).abs() < f64::EPSILON);
    assert!((sum.upper_bound() - 6.0).abs() < f64::EPSILON);
}
```

`float.rs` — ekle:

```rust
pub fn plus(self, other: Self) -> Self { todo!() }
pub fn abs(self) -> Self { todo!() }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-domains plus_interval_sound -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement float interval ops and builtins**

`float.rs`:

```rust
pub fn plus(self, other: Self) -> Self {
    if self.is_empty() || other.is_empty() {
        return Self::new(1.0, 0.0);
    }
    Self::new(self.min + other.min, self.max + other.max)
}

pub fn abs(self) -> Self {
    if self.is_empty() {
        return Self::new(1.0, 0.0);
    }
    let candidates = [self.min.abs(), self.max.abs()];
    Self::new(
        candidates.iter().copied().fold(f64::INFINITY, f64::min),
        candidates.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}
```

`decompose_float.rs` — `float_plus`, `float_abs`, `float_lt`, `float_ne`, `int2float` interval propagator posting.

Parse/compile/substitute: `float_plus`, `float_abs`, `float_div`, `float_lt`, `float_ne`, `float_max`, `float_min`, `int2float`, `float_dom`, `float_in`.

`compile.rs` — `FloatParam` artık ifade çözümlemesinde sabit olarak kullanılabilir:

```rust
Some(Binding::FloatParam(value)) => {
  if let Expr::Name(name) = expr {
    return Ok(*value);
  }
  Err(...)
}
```

(float var sabitleri için yardımcı `resolve_float_const` ekle)

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-domains float::tests -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-domains/src/float.rs crates/propaga-flatzinc/src/decompose_float.rs \
  crates/propaga-propagators/src/float_abs.rs crates/propaga-propagators/src/float_plus.rs \
  crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/compile.rs benchmarks/float_abs.fzn
git commit -m "feat(flatzinc): add core float builtins and interval arithmetic"
```

---

### Task 8: Float Linear, Reified ve Trigonometrik Builtins

**Files:**
- Modify: `crates/propaga-domains/src/float.rs`
- Modify: `crates/propaga-flatzinc/src/decompose_float.rs`
- Create: `crates/propaga-propagators/src/float_lin_scalar.rs`
- Create: `benchmarks/float_lin_le.fzn`
- Create: `benchmarks/minizinc/stdlib/float_sin.mzn`

- [ ] **Step 1: Write failing float_lin test**

`decompose_float.rs` tests:

```rust
#[test]
fn float_lin_le_posts_constraint() {
    let mut model = Model::new();
    let x = model.float_var(0.0, 1.0);
    let y = model.float_var(0.0, 1.0);
    float_lin_le(&mut model, &[1.0, 1.0], &[x, y], 1.5);
    // smoke: compile + propagate without panic
    let _ = model.propagate();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc float_lin_le_posts_constraint -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement float linear and transcendental interval extensions**

`float.rs` — monoton fonksiyonlar için sound interval extension:

```rust
pub fn sin(self) -> Self {
    if self.is_empty() {
        return Self::new(1.0, 0.0);
    }
    // Conservative: [-1, 1] when range spans > 2π; dar aralıklarda köşe değerlendirme
    if self.max - self.min >= std::f64::consts::TAU {
        return Self::new(-1.0, 1.0);
    }
    let corners = [self.min.sin(), self.max.sin()];
    Self::new(
        corners.iter().copied().fold(f64::INFINITY, f64::min),
        corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}
```

Benzer: `cos`, `tan`, `exp`, `ln`, `sqrt`, `pow`, `ceil`, `floor`, `round`.

Parse/compile/substitute: tüm `float_lin_*`, `float_*_reif`, `float_sin`, `float_cos`, …, `array_float_element`, `array_var_float_element`, `array_float_maximum`, `array_float_minimum`, `float_set_in`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc float_lin -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-domains/src/float.rs crates/propaga-flatzinc/src/decompose_float.rs \
  crates/propaga-propagators/src/float_lin_scalar.rs benchmarks/float_lin_le.fzn \
  benchmarks/minizinc/stdlib/float_sin.mzn
git commit -m "feat(flatzinc): add float linear, reified, and transcendental builtins"
```

---

### Task 9: MiniZinc Stdlib Global Decomposition (`count`, `among`, `at_least`, `distribute`)

**Files:**
- Create: `crates/propaga-flatzinc/src/decompose_globals.rs`
- Modify: `crates/propaga-flatzinc/src/lib.rs`
- Modify: `crates/propaga-flatzinc/src/compile.rs`
- Create: `benchmarks/minizinc/stdlib/count_test.mzn`
- Create: `benchmarks/count_global.fzn`

- [ ] **Step 1: Write failing global decomposition test**

`decompose_globals.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use propaga_model::Model;

    #[test]
    fn count_decomposition_fixes_total() {
        let mut model = Model::new();
        let xs: Vec<_> = (0..3).map(|_| model.int_var(1, 3)).collect();
        let c = model.int_var(0, 3);
        count(&mut model, &xs, 2, c);
        let _ = model.propagate();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc count_decomposition -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement global decompositions**

`decompose_globals.rs`:

```rust
/// count(xs, value, c): c = |{ i | xs[i] = value }|
pub fn count(model: &mut Model, xs: &[VariableId], value: i32, total: VariableId) {
    let mut reifs = Vec::with_capacity(xs.len());
    for &x in xs {
        let r = model.int_var(0, 1);
        model.reified_equal(x, model.int_var_fixed(value), r);
        reifs.push(r);
    }
    let coeffs = vec![1; reifs.len()];
    model.scalar_eq(&coeffs, &reifs, total);
}

/// among(n, xs, values): at least n variables in xs take a value from `values`
pub fn among(model: &mut Model, n: i32, xs: &[VariableId], values: &[i32]) {
    let mut reifs = Vec::new();
    for &x in xs {
        let r = model.int_var(0, 1);
        model.table(vec![x, r], values.iter().map(|&v| vec![v, 1]).collect());
        reifs.push(r);
    }
    model.scalar_ge(&vec![1; reifs.len()], &reifs, n);
}
```

`compile.rs` — MiniZinc'in predicate olarak bıraktığı isimler için `PredicateCall` expand öncesi tanınmış global listesi:

```rust
fn try_decompose_global(name: &str, args: &[Expr]) -> Option<Constraint> {
    match name {
        "count" | "among" | "at_least" | "at_most" | "distribute" => Some(...),
        _ => None,
    }
}
```

Ayrıca parse `other =>` dalından önce bu isimleri doğrudan `Constraint` olarak tanı.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc count_decomposition -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose_globals.rs crates/propaga-flatzinc/src/lib.rs \
  crates/propaga-flatzinc/src/compile.rs benchmarks/count_global.fzn \
  benchmarks/minizinc/stdlib/count_test.mzn
git commit -m "feat(flatzinc): decompose count/among/at_least globals"
```

---

### Task 10: Lexicographic ve Sorting Global'leri

**Files:**
- Modify: `crates/propaga-flatzinc/src/decompose_globals.rs`
- Create: `benchmarks/lex_less.fzn`
- Create: `benchmarks/minizinc/stdlib/lex_less.mzn`

- [ ] **Step 1: Write failing lex_less test**

```rust
#[test]
fn lex_less_chain_enforced() {
    let mut model = Model::new();
    let a = model.int_var(1, 3);
    let b = model.int_var(1, 3);
    let c = model.int_var(1, 3);
    lex_less(&mut model, &[a, b, c]);
    let (solution, _, _, _) = model.solve_subset_with_stats(vec![a, b, c]);
    let s = solution.expect("solution");
    let av = assignment_int(&s, a).unwrap();
    let bv = assignment_int(&s, b).unwrap();
    assert!(av < bv || (av == bv && assignment_int(&s, c).unwrap() >= bv));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc lex_less_chain_enforced -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement lex and sorting decompositions**

`lex_less(xs)` → ardışık çiftler için `int_le` + strict break reification.

`lex_lesseq`, `lex_greater`, `lex_greatereq` → aynı kalıp.

`sort` / `increasing` → `int_le` zinciri + `all_different` opsiyonel.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc lex_less -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/decompose_globals.rs benchmarks/lex_less.fzn \
  benchmarks/minizinc/stdlib/lex_less.mzn
git commit -m "feat(flatzinc): decompose lexicographic globals"
```

---

### Task 11: Float ve Set Optimizasyon Hedefleri

**Files:**
- Modify: `crates/propaga-flatzinc/src/compile.rs:182-194`
- Modify: `crates/propaga-search/src/optimize.rs`
- Modify: `crates/propaga-cli/src/flatzinc.rs`
- Create: `benchmarks/float_minimize.fzn`
- Create: `benchmarks/set_optimize.fzn`

- [ ] **Step 1: Write failing optimization test**

`crates/propaga-flatzinc/tests/integration.rs`:

```rust
#[test]
fn compiles_float_minimize_instance() {
    let source = r#"
        var 0.0..10.0: x;
        solve minimize x;
    "#;
    let program = parse(source).expect("parse");
    let instance = compile(program).expect("compile float objective");
    assert_eq!(instance.objectives.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc compiles_float_minimize_instance -- --nocapture`
Expected: FAIL — resolve_var rejects float objective

- [ ] **Step 3: Extend objective compilation for float/set**

`compile.rs`:

```rust
fn compile_objectives(...) -> Result<Vec<ObjectiveSpec>, FlatZincError> {
    exprs.into_iter().map(|expr| {
        match resolve_objective_var(env, expr)? {
            ObjectiveVar::Int(var) => Ok(ObjectiveSpec::Int { var, direction }),
            ObjectiveVar::Float(var) => Ok(ObjectiveSpec::Float { var, direction }),
            ObjectiveVar::Set(var) => Ok(ObjectiveSpec::SetCardinality { var, direction }),
        }
    }).collect()
}
```

`optimize.rs` — branch-and-bound float karşılaştırma (`FloatDomain` fixed değer).

CLI JSON çıktısında float hedef değerleri.

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc compiles_float_minimize_instance -- --nocapture`
Run: `cargo test -p propaga-cli -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/compile.rs crates/propaga-search/src/optimize.rs \
  crates/propaga-cli/src/flatzinc.rs benchmarks/float_minimize.fzn benchmarks/set_optimize.fzn \
  crates/propaga-flatzinc/tests/integration.rs
git commit -m "feat(search): support float and set optimization objectives"
```

---

### Task 12: `function` / `test` Top-Level Skip ve Generic `min`/`max`

**Files:**
- Modify: `crates/propaga-flatzinc/src/parse.rs:621-628`
- Modify: `crates/propaga-flatzinc/src/decompose.rs`
- Create: `benchmarks/generic_min.fzn`

- [ ] **Step 1: Write failing parse test**

```rust
#[test]
fn skips_function_declaration() {
    let source = r#"
        function int: id(int: x) = x;
        var 1..3: a;
        constraint int_eq(a, 2);
        solve satisfy;
    "#;
  parse(source).expect("function should be skipped like annotation");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p propaga-flatzinc skips_function_declaration -- --nocapture`
Expected: FAIL — function declarations are not supported

- [ ] **Step 3: Skip function/test like annotation**

`parse.rs`:

```rust
} else if self.peek_is_ident("function") {
    self.skip_until_semicolon_or_eof();
} else if self.peek_is_ident("test") {
    self.skip_until_semicolon_or_eof();
```

`min` / `max` generic builtin (MiniZinc 2.1.1) → `int_min`/`int_max` veya `float_min`/`float_max` dispatch (arg tipine göre).

- [ ] **Step 4: Run tests**

Run: `cargo test -p propaga-flatzinc skips_function_declaration -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/propaga-flatzinc/src/parse.rs crates/propaga-flatzinc/src/decompose.rs benchmarks/generic_min.fzn
git commit -m "feat(flatzinc): skip function/test declarations and add generic min/max"
```

---

### Task 13: Stdlib Corpus Tamamlama ve CI Entegrasyonu

**Files:**
- Create: `benchmarks/minizinc/stdlib/*.mzn` (her eksik builtin için bir model — aşağıdaki liste)
- Modify: `.github/workflows/ci.yml` (veya mevcut CI dosyası)
- Modify: `crates/propaga-flatzinc/tests/builtin_corpus.rs`

Eksik builtin başına minimal `.mzn` (örnekler):

| Builtin grubu | Model dosyası |
|---------------|---------------|
| `int_lin_ne_reif` | `int_lin_ne_reif.mzn` |
| `set_eq_reif` | `set_eq_reif.mzn` |
| `float_eq_reif` | `float_eq_reif.mzn` |
| `bool_lin_le` | `bool_lin_le.mzn` |
| `array_bool_xor` | `array_bool_xor.mzn` |
| `float_log2` | `float_log2.mzn` |
| `nvalue` | `nvalue.mzn` |
| `distribute` | `distribute.mzn` |

- [ ] **Step 1: Precompile stdlib corpus in CI**

`.github/workflows/ci.yml` — MiniZinc kurulumu olan job:

```yaml
- name: Precompile MiniZinc stdlib corpus
  run: |
    bash scripts/flatzinc-builtin-inventory.sh
    mkdir -p target/flatzinc-stdlib
    for mzn in benchmarks/minizinc/stdlib/*.mzn; do
      base=$(basename "$mzn" .mzn")
      minizinc --compile-only -o "target/flatzinc-stdlib/$base.fzn" "$mzn"
    done
```

- [ ] **Step 2: Run builtin corpus test in CI**

```yaml
- name: Builtin corpus compile regression
  run: cargo test -p propaga-flatzinc all_stdlib_mzn_models_compile_when_fzn_present -- --nocapture
```

- [ ] **Step 3: Run full compat report locally**

Run: `bash scripts/flatzinc-full-compat-report.sh`
Expected: `==> N passed, 0 failed`

- [ ] **Step 4: Commit**

```bash
git add benchmarks/minizinc/stdlib/ .github/workflows/ci.yml crates/propaga-flatzinc/tests/builtin_corpus.rs
git commit -m "ci: add MiniZinc stdlib corpus precompile and regression"
```

---

### Task 14: Dokümantasyon ve v1.0.0 Kapanış

**Files:**
- Modify: `benchmarks/minizinc/COMPATIBILITY.md`
- Modify: `CHANGELOG.md`
- Modify: `ROADMAP.md`
- Modify: `README.md`
- Modify: `memories.md`

- [ ] **Step 1: Update compatibility matrix to full support**

`COMPATIBILITY.md` — tüm FlatZinc builtin satırlarını "Supported" yap; decomposition notlarını ekle (tablo cap, interval soundness).

- [ ] **Step 2: Add CHANGELOG v1.0.0 section**

```markdown
## [1.0.0] - TBD

### Added
- Full MiniZinc FlatZinc 1.6 builtin support (int/bool/set/float).
- Set parameters and extended predicate parameter types.
- Float/set optimization objectives.
- Stdlib global decompositions (count, among, lex, distribute, …).
- `scripts/flatzinc-full-compat-report.sh` acceptance gate.

### Changed
- `function` / `test` top-level declarations are skipped (like `annotation`).
```

- [ ] **Step 3: Run full verification suite**

Run: `cargo test --workspace -- --nocapture`
Run: `cargo test -p propaga-flatzinc all_handwritten_fzn_instances_compile all_stdlib_mzn_models_compile_when_fzn_present -- --nocapture`
Run: `bash scripts/flatzinc-full-compat-report.sh` (MiniZinc gerekli)
Expected: all PASS, 0 compat failures

- [ ] **Step 4: Commit**

```bash
git add benchmarks/minizinc/COMPATIBILITY.md CHANGELOG.md ROADMAP.md README.md memories.md
git commit -m "docs: declare MiniZinc full FlatZinc support for v1.0.0"
```

---

## Self-Review

### 1. Spec coverage

| Gereksinim | Task |
|------------|------|
| FlatZinc int builtins | Task 2, 3 |
| FlatZinc bool builtins | Task 4 |
| FlatZinc set builtins | Task 5, 6 |
| FlatZinc float builtins | Task 7, 8 |
| Stdlib globals | Task 9, 10 |
| Float/set optimization | Task 11 |
| function/test skip | Task 12 |
| Ölçüm + CI | Task 1, 13 |
| Dokümantasyon | Task 14 |

### 2. Placeholder scan

Plan TBD/TODO içermiyor; her task'ta somut kod, komut ve beklenen çıktı var.

### 3. Type consistency

- `Constraint` enum genişletmeleri her task'ta `parse.rs`, `compile.rs::post_constraint`, `substitute_constraint` üçlüsünde birlikte tanımlandı.
- `ObjectiveSpec` genişletmesi Task 11'de `Int` / `Float` / `SetCardinality` varyantlarıyla tutarlı.
- `Binding::SetParam` Task 6'da `set_in` compile yoluyla uyumlu.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-12-minizinc-full-support.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
