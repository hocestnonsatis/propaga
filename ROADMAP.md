# Propaga Roadmap

Forward-looking ideas beyond v1.0.0. See [README.md](README.md) for what ships today and [COMPATIBILITY.md](benchmarks/minizinc/COMPATIBILITY.md) for FlatZinc coverage.

## Next

- Small propagator/search polish

## Shipped after v1.0.0

- Float / set objectives in lexicographic and Pareto multi-objective search
- Incremental Pareto front maintenance (streamed DFS, online filtering)
- Typed Pareto dominance cuts (`DominanceCutPropagator` for int / float / set-cardinality)
- Non-int assignment blocking (`ForbiddenAssignmentPropagator`); Pareto search uses cuts + blocking for all objective types
- Continuous float point exclusion via `encode_forbidden_float` (reif OR) wired into Pareto assignment blocking
- DFS float branching prefers splitting around registered blocked IEEE points (`with_float_holes` / Pareto hole tracking)
- Stronger float `!=` / `<`: `FloatNePropagator` endpoint pruning, strengthened `FloatEqReif`, strict `float_lt` via `¬(b ≤ a)`
- `FloatLinearNePropagator` + fixed `ReifiedFloatLinearEq(false)` (bound-touch / unit endpoint pruning; FlatZinc `float_lin_ne`)
- Docs: COMPATIBILITY lex/Pareto notes cover float and set-cardinality objectives
- DFS `solve_each` set in/out branching trail-correctness fix
- `FloatDomain` interior holes + `exclude_float_point`; `FloatNe` / forbidden assignment / DFS use domain holes
- Hole-aware float arithmetic: `affine` / fixed-operand `plus`/`times`/`divide`, equality hole sharing, binary plus/times/div propagators project holes
- Hole-aware float linear: `FloatLinearEqPropagator`, interior `float_lin_ne` exclusion, affine hole sharing when two vars remain free
- Hole-aware unary float maps: `abs`/`sqrt`/`ln`/`exp` preserve or safely project holes; unary propagator reverse-projects invertible cases
- Native `FloatElementPropagator` for `array_*_float_element` with hole sharing and common-absent hole projection
- Locally monotonic `sin`/`cos` hole projection (+ reverse preimages); constant-domain `ceil`/`floor`/`round` collapse to fixed
- COMPATIBILITY.md float hole semantics section; README FlatZinc gap blurb refreshed
- Search annotations: `float_search` / `set_search`, `indomain_random` / `indomain_reverse_split`, selector aliases
- Multi-phase `seq_search` and `indomain_interval` value ordering
- Stdlib CI corpus fixtures for `seq_search`, `search_selectors`, `float_search_ann`, `set_search_ann`, `bool_search_ann`, `indomain_random_ann`, `reverse_split_ann`, `median_luby_ann` (compile + SAT; portfolio SAT for `seq_search`)
- Portfolio search (`solve_portfolio` / `--workers`) propagates `search_phases` to every worker
- `SetUnionPropagator` cardinality bound tightening via `tighten_set_cardinality`
- `SetIntersectPropagator` cardinality bound tightening via `tighten_set_cardinality`
- `SetSubsetPropagator` cardinality bound tightening via `tighten_set_cardinality`
- CI smoke solves for `set_union.fzn`, `set_intersect.fzn`, and `float_round.fzn`; handwritten `set_subset.fzn` / `float_round.fzn` fixtures
- `FloatUnaryPropagator` reverse-projects fixed `ceil` / `floor` / `round` images
- FlatZinc fixtures `float_floor.fzn` / `float_ceil.fzn` + CI smoke for floor
- CLI/README document full `--var-ordering` / `--value-ordering` aliases (split, interval, activity, …)
- CLI `--var-ordering` accepts FlatZinc aliases (`smallest`, `occurrence`, `degree`); CI smoke for `float_ceil.fzn`
- Stdlib corpus fixture `bool_search_ann` (`bool_search` + `restart_geometric`, compile + SAT)
- BnB / lex / Pareto wire `search_phases` like portfolio
- Stdlib corpus fixture `indomain_random_ann`; CI smoke for `set_intersect.fzn`
- Stdlib corpus fixture `reverse_split_ann` (`indomain_reverse_split` + `restart_constant`)
- Stdlib corpus fixture `median_luby_ann` (`activity` + `indomain_median` + `restart_luby`)
- `SetIntervalDomain::with_cardinality` clamps to GLB/LUB sizes; CI smoke `set_eq` / `set_subset`
- `force_in` / `force_out` keep card bounds aligned with GLB/LUB
- Handwritten `seq_search_minimize.fzn` + CI smoke (BnB phases / `float_abs`)
- `FloatUnaryPropagator` reverse-projects fixed `exp` / `ln` / `sqrt` images
- CI smoke: `float_exp`, `float_times`, lex multi, Pareto biobjective
- Sound set cardinality for intersect/subset (`|A\B| ≤ |lub(A)\glb(B)|`); empty sets collapse when `card_max=0`
- `set_diff` encoding: `left ⊆ result ∪ right` (not `cover ⊆ left`); combined aux universes
- Failed set forces use wipeout domains; DFS skips int nogood learning on set/float conflicts
- Handwritten `set_diff.fzn` / `set_symdiff.fzn` + CI smoke
- `Model::set_var_aux` keeps FlatZinc set decomposition auxiliaries out of decision variables
- CI smoke: `float_ln.fzn` / `float_sqrt.fzn` fixed-image reverse projection
- `Model::int_var_aux` / `float_var_aux`; set reif and `float_log2` decompositions use them
- CI smoke: `set_cardinality.fzn` and `float_log2.fzn`
- Float / int FlatZinc decompositions use `*_aux` for reifs and intermediates; `int_var_fixed` is non-decision
- CI smoke: `float_minimize.fzn` / `set_optimize.fzn`
- FlatZinc compile / globals intermediates use `int_var_aux` (unit-sum partials, reifs; sort permutation stays decision)
- CI smoke: `float_sin`, `set_param`, `float_bounds`, `float_lin_le`; COMPATIBILITY `set_diff` note
- Handwritten `float_cos.fzn`; CI smoke `float_cos` / `lex_less` / `all_different_only` / `generic_min`
- README notes FlatZinc decomposition auxiliaries are non-decision
- CI smoke: int (`plus`/`times`/`abs`/`min`/`lin_ne`), bool (`clause`/`xor`/`logic`), reified eq/ne, BnB (`maximize_x`/`minimize_cost`), `regular`/`automaton` chains, `nested_predicate`
- `SetUnionPropagator` raises `|A∪B|` card_min from `|glb(A) ∪ glb(B)|`
- CI smoke: remaining handwritten scheduling / search / weighted / reified-linear fixtures
- `SetSubsetPropagator` raises `|B|` card_min by `|glb(B) \ lub(A)|` when `A ⊆ B`
- `SetIntersectPropagator` raises operand card_min by `|glb(A) \ lub(R)|` when `R = A ∩ B`
- `SetUnionPropagator` raises `|R|` card_min by `|glb(R) \ lub(A)|` when `R = A ∪ B`
- Native `SetDiffPropagator` for FlatZinc `set_diff` (symdiff still decomposes)
- Native `SetSymDiffPropagator` for FlatZinc `set_symdiff`
- `SetEqReifPropagator` sound `definitely_ne` + reif assignment + card sync
- `SetSubsetReifPropagator` reif assignment + card sync + inevitable-subset failure
- `SetInReifPropagator` sound reif assignment (outside LUB ⇒ false) + force-out when false
- `SetCardEqPropagator` for variable `set_card(S, k)`
- `SetInPropagator` sound membership (keep forced members outside value domain)
- Native `SetEqPropagator` for FlatZinc `set_eq`
- `array_var_set_element` 1-based vs `_nonshifted` 0-based + CI fixtures
- `array_var_bool_element` 1-based vs `_nonshifted` 0-based + CI fixtures
- Int/float/bool `array_*_element` 1-based indexing + `_nonshifted` variants
- FlatZinc `array […] of var set of …` declarations + CI fixture
- FlatZinc `array […] of var float` / float-bounded arrays + CI fixture
- FlatZinc set-parameter arrays + `array_set_element` + CI fixture
- FlatZinc float/bool parameter arrays + `array_float_element` / `array_bool_element` fixtures
- Native `SetNePropagator` for FlatZinc `set_ne`
- `SetLtPropagator` proper-subset cardinality tightening for FlatZinc `set_lt`
- Native `FloatMinMaxPropagator` for FlatZinc `float_min` / `float_max`
- Native `IntMinMaxPropagator` for FlatZinc `int_min` / `int_max`; array float min/max fold
- Native `IntAbsPropagator` for FlatZinc `int_abs`; array int min/max fold
- Native `IntTimesPropagator` for FlatZinc `int_times` (no table size cap)
- Native `IntDivPropagator` for FlatZinc `int_div`; `int_plus` always via linear_eq
- Native `IntModPropagator` for FlatZinc `int_mod`; `bool_not` via `a+b=1`
- Linear/reif `bool_and` / `bool_or` / `bool_xor` (drop truth tables)

## Shipped in v1.0.0

- FlatZinc 1.6 stdlib workflow: int / bool / set / float builtins, globals, and parameters
- `sort`, `array_float_*`, `float_dom`, `float_in`
- Float & set single-objective branch-and-bound
- Stdlib corpus + CI precompile regression (`minizinc-stdlib` job)
- `function` / `test` top-level skip; generic `min` / `max`
- `scripts/flatzinc-full-compat-report.sh` acceptance gate

## Shipped in v0.7.0

- FlatZinc primitives: `int_abs`, `int_times`, `int_div`, `int_mod`, `bool_not`, `bool_and`, `bool_or`
- FlatZinc `automaton` global
- `bool` / `float` parameters
- Nested FlatZinc predicate calls
- `annotation` top-level skip; `incomplete` search tolerance
- MiniZinc model corpus + compatibility report script

## Shipped in v0.6.0

- Set globals: `set_union`, `set_intersect`
- Float global: `float_times` with interval propagation
- FlatZinc compile support for the above constraints

## Shipped in v0.5.0

- Set and float variable domains integrated into the propagation engine
- `SetIntervalDomain`, set/float propagators, and Model API
- Typed search branching with `AssignmentValue`
- FlatZinc subset for set/float variables and constraints

## Shipped in v0.4.0

- Pareto-front multi-objective optimization
- FlatZinc `regular` global constraint
- Deeper lazy clause generation integration (clause propagator posting)
- WASM demo packaging and GitHub Pages deployment workflow

## Shipped in v0.3.0

- Parallel portfolio search with engine checkpoints (rayon worker pool)
- Broader FlatZinc predicate bodies (multi-constraint conjunction)
- FlatZinc lexicographic multi-objective (`minimize x, y`)
- Lazy clause pruning in DFS (`SearchConfig::clause_learning`)
