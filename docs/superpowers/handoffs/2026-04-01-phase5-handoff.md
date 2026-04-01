Execute `/superpowers:writing-plans` for Phase 5 of @docs/superpowers/specs/2026-04-01-parity-completions-design.md

Phase 5 covers item 8 (Golden Tests for emMain). Save the plan to `docs/superpowers/plans/2026-04-01-phase5-golden-tests.md`.

**Context from prior phases:**

- Phase 1 (complete): [paste completion report here]
- Phase 2 (complete): [paste completion report here]
- Phase 3 (complete): [paste completion report here]
- Phase 4 (complete): [paste completion report here]

**Phase 4 deferrals to carry forward (if any):**
[fill in after Phase 4]

**Key things to verify before writing the plan:**
- Read `crates/eaglemode/tests/golden/common.rs` — understand comparison functions (pixel ch_tol+max_fail_pct, rect f64 eps, behavioral exact)
- Read `crates/eaglemode/tests/golden/main.rs` — how test modules are registered
- Read `tests/golden/gen/gen_golden.cpp` and `tests/golden/gen/Makefile` — understand C++ reference data generator
- Read a few existing golden tests (e.g., `tests/golden/painter.rs`, `tests/golden/widget.rs`) to understand the pattern
- Check `tests/golden/data/` — what reference data already exists
- Verify ALL rendering changes from Phases 1-3 are finalized (this is the gate condition)
- Check if Phase 4 (dynamic loading) affects any test infrastructure

**Scope from spec:**
1. Extend C++ generator for emMain panels (starfield, eagle logo, main panel layout, cosmos items, bookmarks, control panel)
2. New Rust golden test modules: starfield.rs, eagle_logo.rs, main_panel.rs, cosmos_items.rs, control_panel.rs
3. Register new modules in tests/golden/main.rs
4. Uses existing comparison functions from common.rs

**Important constraint:** Golden tests must be written AFTER all rendering is final. If any Phase 1-4 visual changes are pending, they must be resolved first. The gate is: divergence log clean at tolerance 0.

**Lessons from prior phases:**
[fill in accumulated lessons after Phase 4]
