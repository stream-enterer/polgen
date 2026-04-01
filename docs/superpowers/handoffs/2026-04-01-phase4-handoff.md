Execute `/superpowers:writing-plans` for Phase 4 of @docs/superpowers/specs/2026-04-01-parity-completions-design.md

Phase 4 covers item 3 (Dynamic Plugin Loading). Save the plan to `docs/superpowers/plans/2026-04-01-phase4-dynamic-plugins.md`.

**Context from prior phases:**

- Phase 1 (complete): [paste completion report here]
- Phase 2 (complete): [paste completion report here]
- Phase 3 (complete): [paste completion report here]

**Phase 3 deferrals to carry forward (if any):**
[fill in after Phase 3]

**Key things to verify before writing the plan:**
- Read current state of `crates/emcore/src/emFpPlugin.rs` (has STATIC_RESOLVER thread-local + set_static_plugin_resolver)
- Read `crates/emmain/src/static_plugins.rs` (to be deleted)
- Read `crates/emmain/src/lib.rs` (has `pub mod static_plugins`)
- Read `crates/eaglemode/src/main.rs` (calls set_static_plugin_resolver)
- Read `crates/emcore/src/emStd2.rs` emTryOpenLib / emTryResolveSymbol — understand library name resolution
- Check `crates/emfileman/Cargo.toml` and `crates/emstocks/Cargo.toml` — verify crate-type includes cdylib
- Check what Cargo names the output .so files (`libemfileman.so`? `libemFileMan.so`?)
- Read all `.emFpPlugin` config files in `etc/emCore/FpPlugins/` — check Library field values
- Read `crates/test_plugin/` — understand existing dynamic loading test infrastructure
- Read `crates/eaglemode/tests/behavioral/fp_plugin.rs` and `tests/integration/plugin_e2e.rs`

**Scope from spec:**
1. Delete `static_plugins.rs` and remove `set_static_plugin_resolver` call
2. Remove `STATIC_RESOLVER` thread-local from emFpPlugin.rs
3. Configure RPATH via build.rs so emTryOpenLib finds plugin .so files
4. Update `.emFpPlugin` Library names to match Cargo output (lowercase crate names)
5. End-to-end test: plugins load via dlopen/dlsym, no static resolver

**Lessons from prior phases:**
[fill in accumulated lessons after Phase 3]
