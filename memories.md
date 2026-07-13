## Proje Durumu
- Branch: `feat/minizinc-full-support` — MiniZinc FlatZinc 1.6 stdlib planı **tamamlandı** (Task 1–14).
- Plan: `docs/superpowers/plans/2026-07-12-minizinc-full-support.md` (v1.0.0 hedefi).
- v1.0.0 dokümantasyonu güncellendi; crate `version` henüz `0.6.0` (release ayrı adım).
- Commit/push yalnızca açık talep ile.

## v1.0.0 Özet (feat/minizinc-full-support)
- Task 1–10: builtins, set/float, globals, lex
- Task 11: float/set optimization objectives
- Task 12: `function`/`test` skip, generic `min`/`max`
- Task 13: stdlib corpus (13 model) + CI `minizinc-stdlib` job
- Task 14: COMPATIBILITY.md, CHANGELOG, ROADMAP, README

## Bilinen Kalan Boşluklar (v1.0.0)
- Lexicographic/Pareto yalnızca int objective
- Float propagation interval-tabanlı (sound, exact değil)

## Son Düzeltmeler (2026-07-13)
- `SetInPropagator`: `set_in` GLB döngüsü düzeltildi (`set_param.fzn` SAT)
- `FloatLinearLe/GePropagator`: `tighten_float_above/below` yönleri düzeltildi (`float_lin_le.fzn` SAT)
- Eklendi: `sort`, `array_float_*`, `float_dom`, `float_in`

## Tercihler
- Planlar `docs/superpowers/plans/` altında.
