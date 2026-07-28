# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Lexicographic and Pareto multi-objective search now accept float and set-cardinality objectives (same typed targets as single-objective optimization).
- Incremental Pareto enumeration: DFS streams solutions via `solve_each` and updates the non-dominated front online (no full feasible-set buffer).
- Typed Pareto dominance cuts (`DominanceCutPropagator`) for int, float, and set-cardinality thresholds; Pareto search posts cuts between solutions for all objective types.
- `ForbiddenAssignmentPropagator` blocks rediscovery of int / float / set assignments during Pareto enumeration.
- `encode_forbidden_float` excludes a continuous float point via reified `≤ next_down ∨ ≥ next_up`, and Pareto blocking adds those reifs as search decisions.
- DFS float branching can split around registered blocked IEEE points; Pareto search feeds blocked float values into those split hints.
- `FloatNePropagator` prunes forbidden float values at interval endpoints; `FloatEqReif(false)` delegates to it; FlatZinc `float_lt` uses strict `¬(b ≤ a)`.
- `FloatLinearNePropagator` for `sum ≠ rhs` with bound-touch and unit-endpoint pruning; `ReifiedFloatLinearEq(false)` no longer posts contradictory `≤` and `≥` cuts.
- `FloatDomain` can record interior excluded IEEE points; `ExtendedPropagationContext::exclude_float_point` wires exclusion into `FloatNe` / assignment blocking, and DFS prefers splitting at domain holes.
- Hole-aware float arithmetic: `FloatDomain::affine` and fixed-operand `plus`/`times`/`divide` preserve holes; `FloatEq` shares holes; plus/times/div propagators project holes when a side is fixed.
- `FloatLinearEqPropagator` for `float_lin_eq` with affine hole sharing when two variables remain free; `float_lin_ne` excludes interior equality-forcing points when other terms are fixed.
- Unary float maps (`abs`, `sqrt`, `ln`, `exp`) preserve or safely project holes; `FloatUnaryPropagator` reverse-projects through locally invertible cases.
- `FloatElementPropagator` for float `array_element`: shares holes when the index is fixed and projects holes absent from every remaining candidate.
- Locally monotonic `sin`/`cos` preserve and reverse-project holes; `ceil`/`floor`/`round` collapse to a fixed point when constant on the domain.
- FlatZinc `float_search` / `set_search` parsing; `indomain_random` (deterministic) and `indomain_reverse_split` value orderings; `most_constrained` / `least_constrained` aliases.
- FlatZinc `indomain_interval` value ordering and multi-phase `seq_search` (per nested group selectors until that group's variables are fixed).
- Stdlib corpus fixtures `seq_search`, `search_selectors`, `float_search_ann`, `set_search_ann`, `bool_search_ann`, `indomain_random_ann`, `reverse_split_ann`, and `median_luby_ann` with compile + SAT regression in `builtin_corpus` (including portfolio SAT for `seq_search`).
- Portfolio search attaches model `search_phases` to every worker so FlatZinc `seq_search` is respected with `--workers > 1`.
- `ExtendedPropagationContext::tighten_set_cardinality` for set-cardinality bound updates during propagation.
- `SetUnionPropagator` tightens set-cardinality bounds (`|A∪B| ≥ max(|A|,|B|,|glb(A)∪glb(B)|,|A|+|glb(R)\lub(A)|)`, `|A∪B| ≤ |A|+|B|`, subset relations).
- `SetIntersectPropagator` tightens set-cardinality bounds (`|A∩B| ≤ min(|A|,|B|, overlap)`, `|A| ≥ |R| + |glb(A)\lub(R)|`, operand outside-overlap lower bounds).
- `SetSubsetPropagator` tightens set-cardinality bounds (`|A| ≤ |B|`, `|B| ≥ |A| + |glb(B)\lub(A)|` when `A ⊆ B`).
- Handwritten FlatZinc fixtures `float_round.fzn` and `set_subset.fzn`; CI smoke solves for `set_union.fzn`, `set_intersect.fzn`, and `float_round.fzn`.
- `FloatUnaryPropagator` reverse-projects fixed integer images of `ceil` / `floor` / `round` onto the input domain.
- Handwritten FlatZinc fixtures `float_floor.fzn` and `float_ceil.fzn` (fixed-image reverse projection); CI smoke for `float_floor.fzn`.
- CLI and README document the full `--var-ordering` / `--value-ordering` alias sets (including `split`, `reverse-split`, `interval`, `random`, `activity`).
- CLI `--var-ordering` accepts FlatZinc aliases `smallest` / `occurrence` / `degree` (same mapping as compile); CI smoke-solves `float_ceil.fzn`.
- Stdlib corpus fixture `bool_search_ann` (`bool_search` + `restart_geometric`) with compile + SAT regression.
- Stdlib corpus fixture `indomain_random_ann` (`indomain_random` value ordering) with compile + SAT regression.
- Stdlib corpus fixture `reverse_split_ann` (`indomain_reverse_split` + `restart_constant`) with compile + SAT regression.
- Stdlib corpus fixture `median_luby_ann` (`activity` + `indomain_median` + `restart_luby`) with compile + SAT regression.
- BnB / lexicographic / Pareto search attach model `search_phases` (FlatZinc `seq_search` respected under optimize paths).
- `SetIntervalDomain::with_cardinality` clamps requested bounds to `[|GLB|, |LUB|]`.
- `SetIntervalDomain::force_in` / `force_out` keep cardinality bounds aligned with GLB/LUB sizes.
- CI smoke solves for `set_subset.fzn` and `set_eq.fzn`.
- Handwritten FlatZinc fixture `seq_search_minimize.fzn`; CI smoke for BnB + `seq_search` and `float_abs.fzn`.
- `FloatUnaryPropagator` reverse-projects fixed images of `exp` / `ln` / `sqrt` onto the input.
- Handwritten FlatZinc fixture `float_exp.fzn`; CI smoke for `float_exp`, `float_times`, `lexicographic_multi`, and `pareto_biobjective`.
- Handwritten FlatZinc fixtures `set_diff.fzn` and `set_symdiff.fzn`; CI smoke solves for both.
- `SetIntervalDomain::wipeout` for failed membership forces; `with_cardinality(…, 0)` collapses to the fixed empty set.
- `Model::set_var_aux` for non-decision set auxiliaries; FlatZinc `set_diff` / `set_symdiff` decompositions use it.
- Handwritten FlatZinc fixtures `float_ln.fzn` and `float_sqrt.fzn`; CI smoke solves for both.
- `Model::int_var_aux` and `Model::float_var_aux` for non-decision decomposition variables.
- Handwritten FlatZinc fixture `float_log2.fzn`; CI smoke for `float_log2` and `set_cardinality`.
- Float / int FlatZinc decompositions (`float_max`/`min`, `float_in`/`dom`, array float extrema, `array_int_maximum`/`minimum`, `int_plus` overflow path, `array_bool_xor`) create auxiliaries with `*_aux`.
- `Model::int_var_fixed` is no longer a search decision variable.
- CI smoke solves for `float_minimize.fzn` and `set_optimize.fzn`.
- FlatZinc compile paths for unit-sum chains / reified linear / set-element indexing, and globals decompositions (lex / count / nvalue / …), create intermediates with `int_var_aux` (sort permutation variables remain decisions).
- Handwritten FlatZinc fixture `float_sin.fzn`; CI smoke for `float_sin`, `set_param`, `float_bounds`, and `float_lin_le`.
- Handwritten FlatZinc fixture `float_cos.fzn`; CI smoke for `float_cos`, `lex_less`, `all_different_only`, and `generic_min`.
- CI smoke solves for core int/bool/reified/BnB/globals fixtures: `int_plus`, `int_times`, `int_abs`, `int_min`, `int_lin_ne`, `bool_clause`, `bool_xor`, `bool_logic`, `reified_eq`, `reified_ne`, `maximize_x`, `minimize_cost`, `regular_chain`, `automaton_chain`, and `nested_predicate`.
- CI smoke solves for remaining handwritten fixtures: reified linear/lt/`bool_reify`, bounded/weighted sums, `count`/`cumulative`/`disjunctive`, ordered/strict chains, `permutation_sum`/`predicate_multi`/`table_puzzle`, and `int_search` order/restart.
- Native `SetDiffPropagator` for FlatZinc `set_diff` (replaces multi-constraint aux decomposition); `Model::set_diff`.
- Native `SetSymDiffPropagator` for FlatZinc `set_symdiff`; `Model::set_symdiff`.
- `SetCardEqPropagator` / `Model::set_card_eq` for FlatZinc `set_card(S, k)` with a variable cardinality; handwritten `set_card_var.fzn` + CI smoke.
- Native `SetEqPropagator` for FlatZinc `set_eq` (membership + cardinality sync; replaces dual-subset encoding).
- FlatZinc `array_var_set_element` uses 1-based indices; `array_var_set_element_nonshifted` stays 0-based; handwritten fixtures + CI smoke.
- FlatZinc `array_var_bool_element` uses 1-based indices via an index shift into `ElementPropagator`; `_nonshifted` stays 0-based; handwritten fixtures + CI smoke.
- FlatZinc `array_int_element` / `array_var_int_element` / `array_bool_element` / `array_float_element` / `array_var_float_element` use 1-based indices (with `_nonshifted` 0-based variants); shared index-shift helper; CI smoke for int/float element.
- FlatZinc `array [L..U] of var set of A..B` declarations; handwritten `array_set_vars.fzn` + CI smoke.
- FlatZinc `array [L..U] of var float` / float-bounded array declarations; handwritten `array_float_vars.fzn` + CI smoke.
- FlatZinc `array [L..U] of set of int` parameter arrays and `array_set_element` (1-based / `_nonshifted`); handwritten `array_set_element.fzn` + CI smoke.
- FlatZinc `array [L..U] of float` / `of bool` parameter arrays; handwritten `array_float_element.fzn` / `array_bool_element.fzn` + CI smoke.
- Native `SetNePropagator` for FlatZinc `set_ne` (replaces reified `set_eq`); handwritten `set_ne.fzn` + CI smoke.
- `SetLtPropagator` for FlatZinc `set_lt` (`A ⊂ B` via subset + `|B| ≥ |A| + 1` cards); handwritten `set_lt.fzn` + CI smoke.
- Native `FloatMinMaxPropagator` for FlatZinc `float_min` / `float_max` (replaces reified OR encoding); handwritten `float_min_max.fzn` + CI smoke.
- Native `IntMinMaxPropagator` for FlatZinc `int_min` / `int_max` (replaces domain tables); `array_float_minimum` / `maximum` fold via binary min/max; fixtures `int_min_max.fzn` / `array_float_min_max.fzn` + CI smoke.
- Native `IntAbsPropagator` for FlatZinc `int_abs` (replaces domain table); `array_int_minimum` / `maximum` fold via binary min/max; fixture `array_int_min_max.fzn` + CI smoke.
- Native `IntTimesPropagator` for FlatZinc `int_times` (replaces domain table / 10k cap); handwritten `int_times_large.fzn` + CI smoke.
- Native `IntDivPropagator` for FlatZinc `int_div` (trunc toward zero, excludes divisor 0); `int_plus` always posts `linear_eq`; handwritten `int_div.fzn` + CI smoke.
- Native `IntModPropagator` for FlatZinc `int_mod`; `bool_not` posts `a + b = 1`; handwritten `int_mod.fzn` + CI smoke.
- FlatZinc `bool_and` / `bool_or` / `bool_xor` via linear and reified encodings (no truth tables); handwritten `bool_and_or_xor.fzn` + CI smoke.
- FlatZinc `int_pow_fixed` via `int_times` multiply chain (no domain table); handwritten `int_pow_fixed.fzn` / `bool_le_lt.fzn` + CI smoke.
- FlatZinc `int_pow` via exponent case-split + `element` (table fallback for huge exponent spans); fix `ElementPropagator` incorrectly intersecting value with every array cell when the index is unfixed; handwritten `int_pow.fzn` + CI smoke.
- `ElementPropagator` prunes indices whose cells are bound/fixed-disjoint from the value (not only when the value is fixed); handwritten `array_element_prune.fzn` + CI smoke.
- `FloatDomain::ceil` / `floor` / `round` drop integer images whose preimages are emptied by holes (endpoint-only cases); wide spans still use a hole-free bound fallback.
- FlatZinc / CLI `indomain_middle` value ordering (domain value closest to the mean of current bounds); stdlib corpus fixture `indomain_middle_ann`.
- FlatZinc / CLI variable selectors: true `smallest` / `largest` / `max_regret`, `anti_first_fail` as largest-domain, `dom_w_deg` (plus `occurrence`/`degree` ≈ W-DEG); corpus fixture `max_regret_ann`.
- `set_search` value selectors choose which undecided element to branch on and whether to try membership in/out first; `float_search` reverse-split tries the upper half first; corpus fixture `set_search_max_ann`.
- FlatZinc `set_le` / `set_lt` (+ reifs) use sorted-list lexicographic order via `SetLexPropagator` (Rust `Model::set_lt` remains proper-subset); fixtures `set_le.fzn` / `set_lt.fzn` + CI smoke.
- `SetLexReifPropagator` posts the negated lex relation when reif is false (`¬(A≤B)≡B<A`, `¬(A<B)≡B≤A`); fixture `set_le_reif.fzn` + CI smoke.
- `float_ceil` / `floor` / `round` reverse-project integer output holes onto singleton preimages; unary reverse projection keeps pre-tighten holes so endpoint sync cannot drop them.
- `FloatMinMaxPropagator` reverse-projects result holes onto an operand when the other side cannot realize that value as the min/max; handwritten `float_min_hole.fzn` + CI smoke.
- `float_abs` reverse-projects result holes `h > 0` to both `±h` on the input (including domains that straddle zero); fixed nonnegative images tighten to the abs preimage hull.
- `float_div` reverse-projects via `a = c·b` and `b = a/c` (when `0 ∉ Dom(c)`); `FloatDomain::divide` maps divisor holes when the dividend is fixed; handwritten `float_div.fzn` + CI smoke.
- `float_sin` / `float_cos` reverse-project a fixed image onto the unique preimage when the input domain is locally monotonic; handwritten `float_sin_fixed.fzn` + CI smoke.
- `int2float` keeps discrete holes aligned both ways (missing ints punch float holes; float holes remove ints); handwritten `int2float.fzn` + CI smoke.
- `ceil` / `floor` / `round` wide spans (>10 000 integers) still shrink hole-emptied image endpoints via bounded end scans (interior remains hole-free).
- `FloatDomain` bound tightening that lands on a hole advances past it (pinning to a hole empties the domain).
- Float `array_element` treats a singleton bound-overlap that lands on a hole as disjoint (index prune); handwritten `array_float_element_prune.fzn` + CI smoke.
- `float_min` / `float_max` forward images drop a hole when one operand forbids it and the other lies entirely above (min) / below (max) that hole; handwritten `float_max_hole.fzn` + CI smoke.
- `float_lin_eq` affine hole sharing ignores unfixed zero-coeff terms so two free nonzero-coeff variables still link; handwritten `float_lin_eq_zero_coeff.fzn` + CI smoke.
- `float_plus` / `float_div` / `float_times` reverse-project using pre-tighten result holes (same pattern as unary); handwritten `float_cos_fixed.fzn` + CI smoke.
- `float_eq` shares pre-tighten holes so a hole that bound sync would drop at a new endpoint is still mirrored onto the other side.
- FlatZinc `float_search` precision is applied during DFS: floats whose domain width is at most the precision are fixed (value from the active ordering: min/max endpoints or midpoint); handwritten `float_search_precision.fzn` + CI smoke.
- Nested `float_search` precision inside `seq_search` is kept per phase (and lifts a global fallback when standalone `float_search` is absent); handwritten `seq_float_search_precision.fzn` + CI smoke.
- When a float domain reaches `float_search` precision, DFS assigns lower/upper/midpoint according to the active value ordering (`indomain_min` / `indomain_max` / split-style).
- Float `indomain_interval` splits at the leftmost interior hole (first contiguous component) instead of the hole nearest the midpoint.
- `float_min` / `float_max` `sync_equal` keeps pre-tighten holes when bound sync would drop them (parity with `float_eq`); handwritten `float_min_sync_hole.fzn` + CI smoke.
- Float domain `size()` for MRV / anti-first-fail uses coarse width buckets (plus holes) instead of a constant `2` for every unfixed float.
- Variable selectors `smallest` / `largest` / `max_regret` use float bounds (scaled keys / width proxy) instead of ignoring non-int domains.
- The same selectors also use set cardinality (`card_min` / `card_max`) and undecided membership count for `max_regret`.
- `float_le` shares holes when ≤ collapses both sides to the same fixed value; `float_ne` treats singleton overlap excluded by a hole as already separated.
- DFS bumps VSIDS activity and WDEG weights on float/set wipeouts (nogood learning remains int-only).
- Activity (VSIDS) tie-breaks prefer the earlier decision variable, matching Dom / input-order stability.
- Float phase saving: when `phase_saving` is on, precision fixes reuse the last successful float assignment if it remains in-domain.
- `float_abs` with a fixed nonnegative image forces the unique remaining preimage when the opposite sign is a hole; handwritten `float_abs_unique.fzn` + CI smoke.
- `ceil`/`floor`/`round` reverse-project unfixed output integer bounds onto the input (not only fixed images / singleton holes).
- `float_eq_reif` infers false when the only overlapping IEEE point is a hole on either side (`FloatNe` parity).
- `int2float` always maps explicit float holes onto the int domain and punches endpoint float holes on wide spans.
- `set_eq_reif` / `set_ne` treat disjoint cardinality bounds as definite inequality.
- `set_subset_reif` forces reif=false when ⊆ is card-impossible (`|A| + |glb(B)\lub(A)| > |B|`).
- `set_in_reif=false` prunes GLB members from the value domain (mirrors `reif=true` LUB pruning).
- `set_lt` forces the sole extra superset element when the subset is fixed and `|B| = |A| + 1` with one LUB candidate left.
- `float_le_reif` infers reif using hole-aware admissible endpoints (`min`/`max` admissible IEEE points).
- Reified float linear (`le`/`ge`/`eq`) entailment uses hole-aware admissible term sums.
- `set_lex` with fixed right forces required left members when omitting them breaks `≤_lex` / `<_lex`.
- DFS float precision fixes avoid midpoint holes by choosing an admissible representative before branching stops.
- `float_lt_reif` is now native, so `reif = 0` prunes as `left >= right` instead of relying on a weaker decomposition.
- `set_eq_reif(reif=0)` now breaks the last equalizer, matching native `set_ne` pruning strength.
- `set_subset_reif` also detects when `|A|_min` exceeds the shared `lub(A) ∩ lub(B)` capacity.
- Float `indomain_interval` now keeps preferring the leftmost component when precision assignment stops branching.

### Fixed

- `array_var_set_element` indexes from 1 (was incorrectly 0-based like `_nonshifted`).
- `array_var_bool_element` indexes from 1 (was incorrectly 0-based like `_nonshifted`).
- `array_int_element` / `array_var_int_element` / `array_bool_element` / `array_float_element` / `array_var_float_element` index from 1 (were incorrectly 0-based).
- `SetEqReifPropagator` only treats sets as definitely unequal on GLB/LUB conflicts (not mere domain asymmetry), assigns the reif literal when equality is decided, and syncs cardinality when reif is true.
- `SetSubsetReifPropagator` assigns the reif on definite subset/violation, syncs cardinality when reif is true, and fails when reif is false but subsethood is inevitable.
- `SetInReifPropagator` assigns the reif from membership facts (value outside LUB ⇒ false, in GLB ⇒ true) instead of failing, and forces set out when reif is false.
- `SetInPropagator` no longer forces GLB members out when they are outside the integer domain (unsound for `value ∈ S`); when `S` is fixed it prunes the value to `S`'s members.
- Lexicographic search restores the engine after each objective's branch-and-bound so interior optima are pinned correctly before optimizing the next priority.
- `FloatLeReifPropagator` with reif=false now tightens the correct float bounds (strict greater-than encoding).
- DFS `solve_each` set branching no longer applies in/out forces eagerly before trail marks (both membership branches are explored).
- `SetIntersectPropagator` / `SetSubsetPropagator` cardinality bounds use `|lub \ glb|` outside estimates (were unsound `|lub \ lub|`); intersect forces result membership only from shared GLB.
- FlatZinc `set_diff` posts `left ⊆ result ∪ right` instead of incorrectly requiring `result ∪ right ⊆ left`.
- Failed `force_set_in` / `force_set_out` no longer replace domains with a valid empty set (which wiped cardinality); DFS skips int nogood learning on set/float conflicts.
- `FloatDomain::round` uses true `round` bounds (with constant collapse) instead of the loose `floor + [0,1]` envelope.

### Changed

- COMPATIBILITY.md documents float IEEE-hole projection per constraint family; README no longer lists shipped float array/membership builtins as gaps.
- COMPATIBILITY.md clarifies `set_diff` decomposition (`L ⊆ R ∪ B`) and non-decision auxiliaries.
- README FlatZinc section notes that decomposition auxiliaries are not search decision variables.

### Known limitations

- `ceil`/`floor`/`round` still drop sparse holes when the map is non-constant (preimages are intervals).
- Float propagation remains interval-based (sound, not exact real arithmetic).

## [1.0.0] - 2026-07-13

### Added

- Full MiniZinc FlatZinc 1.6 builtin support for the standard library workflow (int / bool / set / float).
- Set parameters and extended predicate parameter types (`var set`, `var float`).
- Float and set single-objective optimization (`minimize` / `maximize`).
- Stdlib global decompositions: `count`, `among`, `at_least`, `at_most`, `distribute`, `nvalue`, `sort`, lexicographic and monotonicity globals.
- Integer / float primitives: `int_plus`, `int_lin_ne`, `int_min`/`int_max`/`int_pow`, generic `min`/`max`, full bool builtin set.
- Float core: interval arithmetic, linear constraints, reified forms, unary transcendental ops (`float_sin`, `float_log2`, …).
- Float array builtins: `array_float_element`, `array_var_float_element`, `array_float_maximum`, `array_float_minimum`.
- Float membership: `float_dom`, `float_in`.
- Set builtins beyond union/intersect: equality, reified comparisons, `set_in`, etc.
- MiniZinc stdlib corpus (`benchmarks/minizinc/stdlib/`) with bundled FlatZinc fixtures and CI `minizinc-stdlib` job.
- `scripts/flatzinc-full-compat-report.sh` acceptance gate for models + stdlib.

### Fixed

- `set_in` propagation no longer requires the int domain to contain every set GLB element.
- Float linear `<=` / `>=` propagators now tighten the correct interval bounds.

### Changed

- `function` and `test` top-level declarations are skipped during parse (like `annotation`).
- Unknown `min` / `max` predicate calls dispatch to `int_*` or `float_*` by variable domain.
- Optimization objectives use typed `ObjectiveSpec` (int, float, set cardinality) with JSON/plain CLI output.

### Known limitations

- Lexicographic and Pareto multi-objective search remain int-only.
- Table-based int decompositions (`int_times`, `int_pow`, …) cap at 10 000 tuples.
- Float propagation is interval-based (conservative, not exact real arithmetic).

## [0.7.0] - 2026-07-12

### Added

- FlatZinc primitive constraints: `int_abs`, `int_times`, `int_div`, `int_mod`.
- FlatZinc bool primitives: `bool_not`, `bool_and`, `bool_or`.
- FlatZinc `automaton` global (compiled via `regular` propagator).
- `bool` and `float` FlatZinc parameters.
- Recursive nested predicate expansion with full constraint substitution.
- Top-level `annotation` statements are skipped during parse.
- `decompose` module in `propaga-flatzinc` for primitive constraint lowering.
- Compile corpus test (`crates/propaga-flatzinc/tests/compile_corpus.rs`).
- MiniZinc model corpus under `benchmarks/minizinc/models/`.
- `scripts/flatzinc-compat-report.sh` for local MiniZinc compatibility checks.
- Regression benchmarks: `int_abs.fzn`, `bool_logic.fzn`, `int_times.fzn`, `nested_predicate.fzn`, `automaton_chain.fzn`.

### Changed

- `incomplete` search annotations are tolerated (treated like `complete`).
- Unknown predicates now produce explicit compile errors instead of silent `PredicateCall` leftovers.

### Known limitations

- `int_times` / `int_div` / `int_mod` use table decomposition with a 10_000 tuple cap.
- Float parameters are parsed but not yet usable in float constraint expressions.
- `function` / `test` top-level declarations remain unsupported.

## [0.6.0] - 2026-07-11

### Added

- Set global propagators: `set_union`, `set_intersect`.
- Float global propagator: `float_times` with interval arithmetic.
- Model API: `set_union`, `set_intersect`, `float_times`.
- FlatZinc: `set_union`, `set_intersect`, `float_times` constraints.
- Benchmarks: `set_union.fzn`, `set_intersect.fzn`, `float_times.fzn`.
- `FloatDomain::times` and `FloatDomain::divide` interval helpers.

### Known limitations

- Set/float optimization objectives remain int-only.
- Float globals beyond `float_times` not yet supported.
- Set globals beyond union/intersect/card/subset not yet supported.

## [0.5.0] - 2026-07-10

### Added

- `AnyDomain` engine storage for int, set, and float variables.
- `SetIntervalDomain` with GLB/LUB/cardinality representation.
- Set propagators: `SetCardPropagator`, `SetSubsetPropagator`.
- Float propagators: `FloatLePropagator`, `FloatEqPropagator`.
- Model API: `set_var`, `float_var`, `set_card`, `set_subset`, `float_le`, `float_eq`.
- Typed search branching and `AssignmentValue` in solutions.
- FlatZinc subset: set/float variables and `set_card`, `set_subset`, `float_le`, `float_eq`.
- Benchmarks: `set_cardinality.fzn`, `float_bounds.fzn`.

### Known limitations

- Optimization (minimize/maximize/Pareto) remains int-only.
- Float propagation is interval-only (no global float constraints).
- Set globals beyond `set_card`/`set_subset` not yet supported.

## [0.4.0] - 2026-07-10

### Added

- Pareto-front multi-objective enumeration (`ParetoOptimization`, `Model::pareto_optimize`).
- FlatZinc `solve :: pareto([...]) satisfy` annotation and CLI JSON `pareto_solutions`.
- `regular` global constraint (DFA compiled to table propagator) with FlatZinc compile support.
- LCG clause propagator posting during search when `SearchConfig::clause_learning` is enabled.
- WASM demo build script (`scripts/build-wasm.sh`) and GitHub Pages deploy workflow.
- Benchmarks: `regular_chain.fzn`, `pareto_biobjective.fzn`.

### Changed

- `ClauseStore` learned clauses are posted as `ClausePropagator` instances on the engine.
- WASM demo page includes an N-Queens solver section.

### Known limitations

- Set/float domains are not yet wired into the propagation engine.
- Pareto enumeration collects all solutions before filtering (practical for small fronts).

## [0.3.0] - 2026-07-05

### Added

- `EngineCheckpoint` API with `checkpoint`, `restore_checkpoint`, and `fork_at_checkpoint` for parallel search workers.
- Parallel portfolio search using rayon when `--workers > 1` (FlatZinc `solve`, Sudoku, and portfolio API).
- Multi-constraint FlatZinc predicate bodies (`constraint A /\ constraint B` and semicolon chains).
- FlatZinc lexicographic multi-objective (`solve minimize x, y`); CLI JSON includes `objective_values`.
- Lazy clause pruning in DFS via `SearchConfig::clause_learning`.
- Benchmarks: `predicate_multi.fzn`, `lexicographic_multi.fzn`.

### Changed

- `CompiledInstance.objective` replaced by `objectives: Vec<ObjectiveSpec>`.
- `SolveGoal::Minimize` / `Maximize` now hold expression lists for lexicographic goals.
- Propagators implement `DynClone` for engine fork at checkpoint.

### Known limitations

- Set/float domains are not yet wired into the propagation engine.
- Predicate bodies still inline-expand only; nested predicate calls in bodies are rejected.
- `clause_learning` prunes branches but does not post learned clauses as propagators yet.

## [0.2.0] - 2026-07-02

### Added

- Schedule JSON output for `propaga schedule --format json`.
- Value orderings `indomain_split` and `indomain_median`.
- Activity-based variable ordering (VSIDS-style) and CLI/FlatZinc `activity` selector.
- Restart policies `restart_linear` and `restart_on_solution`.
- FlatZinc `bool_search` annotation support.
- Global propagators: `circuit`, `inverse`, `diffn`.
- Predicate inline expansion for simple single-constraint bodies.
- Cumulative constraints with variable duration/height arrays.
- Portfolio search (`--workers`, `--deterministic`) and lexicographic multi-objective optimization API.
- Set and float domain types in `propaga-domains`.
- `propaga-wasm` crate with Sudoku and N-Queens WASM bindings and browser demo.
- Lazy clause generation spike (`ClauseStore` in `propaga-search`).

### Changed

- FlatZinc unknown constraint names become predicate calls when a matching predicate is declared.

### Known limitations

- Portfolio search runs configurations sequentially on a shared engine (parallel rayon workers require engine snapshots).
- Set/float domains are not yet wired into the propagation engine.
- Predicate bodies support a single primitive constraint only.

## [0.1.0] - 2026-06-21

### Added

- Workspace of eight crates: `propaga-core`, `propaga-domains`, `propaga-engine`, `propaga-propagators`, `propaga-search`, `propaga-model`, `propaga-flatzinc`, `propaga-cli`.
- Propagation engine with explanation-aware trail and event scheduling.
- Domain implementations: interval, bitset, and hybrid.
- Built-in propagators: equality, linear, ordering, reified, all-different (GAC), GCC, table, element, cumulative, disjunctive, nogood.
- Search: MRV/DOM/DOM-W-DEG variable ordering, LCV value ordering, first-UIP nogood learning, Luby restarts, phase saving, branch-and-bound optimization.
- High-level `Model` API for constraint posting and solving.
- FlatZinc subset parser and compiler with satisfy/minimize/maximize.
- CLI (`propaga`): `sudoku`, `n-queens`, `solve` (single file and batch directory), `schedule` (JSON).
- CLI flags: `--stats`, `--format json`, `--all`, `--solutions`, `--time-limit`, `--no-learning`, `--restarts`, `--var-ordering`, `--value-ordering`, `--no-phase-saving`.
- Scheduling JSON format with cumulative, sequential, and disjunctive modes.
- Criterion benchmarks for propagator micro-benchmarks.
- FlatZinc compatibility matrix at `benchmarks/minizinc/COMPATIBILITY.md`.

### Known limitations

- FlatZinc is a subset: no set/float variables in the engine, predicate bodies limited to one primitive constraint.
- Multi-objective optimization supports lexicographic API; FlatZinc multi-objective directives are not yet parsed.
- See [ROADMAP.md](ROADMAP.md) for planned features.

[0.3.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.3.0
[0.2.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.2.0
[0.1.0]: https://github.com/hocestnonsatis/propaga/releases/tag/v0.1.0
