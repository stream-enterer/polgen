# Phase 4: Dynamic Plugin Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the static plugin resolver so all plugins load exclusively via `dlopen`/`dlsym` at runtime, and add RPATH so the binary finds plugin `.so` files without `LD_LIBRARY_PATH`.

**Architecture:** The dynamic loading infrastructure is already fully implemented (`emTryOpenLib`/`emTryResolveSymbol` in emStd2.rs). `TryCreateFilePanel` already has a dynamic loading fallback path. We remove the static resolver shortcut, delete `static_plugins.rs`, remove emmain's compile-time dependencies on plugin crates, add RPATH to the binary, and write an integration test proving all production plugins load dynamically.

**Tech Stack:** Rust, `libloading` (already a dependency), cargo build scripts (`build.rs`).

---

## File Structure

### Files Modified
- `crates/emcore/src/emFpPlugin.rs` — remove `STATIC_RESOLVER` thread-local, `StaticResolverFn` type, `set_static_plugin_resolver()` function, and static-resolver branch in `TryCreateFilePanel`
- `crates/emmain/src/lib.rs` — remove `pub mod static_plugins;` line
- `crates/emmain/Cargo.toml` — remove `emstocks` and `emfileman` dependencies
- `crates/eaglemode/src/main.rs` — remove `set_static_plugin_resolver` call
- `crates/eaglemode/Cargo.toml` — add `emfileman` as dev-dependency (for test builds)

### Files Created
- `crates/eaglemode/build.rs` — set RPATH to `$ORIGIN` for the binary
- `crates/eaglemode/tests/integration/dynamic_plugins.rs` — integration test loading all production plugins dynamically

### Files Deleted
- `crates/emmain/src/static_plugins.rs`

---

## Task 1: Add RPATH via build.rs

**Files:**
- Create: `crates/eaglemode/build.rs`

This ensures `./target/debug/eaglemode` finds plugin `.so` files in its own directory when run outside of `cargo run` (which already has `LD_LIBRARY_PATH` via `.cargo/config.toml`).

- [ ] **Step 1: Create build.rs**

```rust
fn main() {
    // Set RPATH to $ORIGIN so the binary finds plugin .so files
    // in the same directory (target/debug/ or target/release/).
    // This supplements .cargo/config.toml's LD_LIBRARY_PATH which
    // only applies to cargo-invoked commands.
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
}
```

Write this to `crates/eaglemode/build.rs`.

- [ ] **Step 2: Verify RPATH is set**

Run:
```bash
cargo build -p eaglemode 2>&1 | tail -3
readelf -d target/debug/eaglemode | grep -i -E 'rpath|runpath'
```

Expected: Output contains a line like `(RUNPATH)  Library runpath: [$ORIGIN]`.

- [ ] **Step 3: Commit**

```bash
git add crates/eaglemode/build.rs
git commit -m "feat(eaglemode): add RPATH via build.rs for plugin .so discovery

Bakes \$ORIGIN into the binary's RUNPATH so plugins are found in the
same directory when running outside of cargo (which already sets
LD_LIBRARY_PATH via .cargo/config.toml)."
```

---

## Task 2: Add integration test for all production plugins

**Files:**
- Create: `crates/eaglemode/tests/integration/dynamic_plugins.rs`
- Modify: `crates/eaglemode/tests/integration/main.rs`
- Modify: `crates/eaglemode/Cargo.toml` (add `emfileman` dev-dependency)

This test proves every production plugin loads via `dlopen`/`dlsym`. It runs today (static resolver is never set in tests) and will continue to run after we remove the static resolver.

- [ ] **Step 1: Add emfileman dev-dependency to eaglemode**

In `crates/eaglemode/Cargo.toml`, under `[dev-dependencies]`, add:

```toml
emfileman = { path = "../emfileman" }
```

This ensures `cargo test -p eaglemode` builds `libemFileMan.so`. (emstocks is already a dev-dependency.)

- [ ] **Step 2: Write the integration test**

Create `crates/eaglemode/tests/integration/dynamic_plugins.rs`:

```rust
//! Verify all production plugins load via dlopen/dlsym.
//!
//! These tests do NOT use the static plugin resolver — they exercise
//! the dynamic loading path in TryCreateFilePanel exclusively.
//!
//! Requires:
//!   - EM_DIR set to repo root (for plugin config discovery)
//!   - LD_LIBRARY_PATH including target/debug/ (for dlopen)
//!   - cargo build (to produce .so files)

use emcore::emContext::emContext;
use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};

/// Helper: create a plugin pointing at a specific library/function.
fn plugin_for(library: &str, function: &str) -> emFpPlugin {
    let mut p = emFpPlugin::new();
    p.library = library.to_string();
    p.function = function.to_string();
    p
}

#[test]
fn dynamic_load_emDirFpPluginFunc() {
    let ctx = emContext::NewRoot();
    let parent = PanelParentArg::new(ctx);
    let plugin = plugin_for("emFileMan", "emDirFpPluginFunc");
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp");
    assert!(result.is_ok(), "emDirFpPluginFunc failed: {result:?}");
}

#[test]
fn dynamic_load_emDirStatFpPluginFunc() {
    let ctx = emContext::NewRoot();
    let parent = PanelParentArg::new(ctx);
    let plugin = plugin_for("emFileMan", "emDirStatFpPluginFunc");
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp");
    assert!(result.is_ok(), "emDirStatFpPluginFunc failed: {result:?}");
}

#[test]
fn dynamic_load_emFileLinkFpPluginFunc() {
    let ctx = emContext::NewRoot();
    let parent = PanelParentArg::new(ctx);
    let plugin = plugin_for("emFileMan", "emFileLinkFpPluginFunc");
    // emFileLink panels expect an .emFileLink file; missing file returns error panel.
    // What matters is that the symbol resolved — not that the panel content is valid.
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/nonexistent.emFileLink");
    assert!(result.is_ok(), "emFileLinkFpPluginFunc failed: {result:?}");
}

#[test]
fn dynamic_load_emStocksFpPluginFunc() {
    let ctx = emContext::NewRoot();
    let parent = PanelParentArg::new(ctx);
    let plugin = plugin_for("emStocks", "emStocksFpPluginFunc");
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/test.emStocks");
    assert!(result.is_ok(), "emStocksFpPluginFunc failed: {result:?}");
}
```

- [ ] **Step 3: Register the module in main.rs**

In `crates/eaglemode/tests/integration/main.rs`, add:

```rust
mod dynamic_plugins;
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo-nextest ntr -E 'test(dynamic_load)'
```

Expected: All 4 tests pass. (They work today because the static resolver is never set in the test harness — `TryCreateFilePanel` falls through to dynamic loading.)

- [ ] **Step 5: Commit**

```bash
git add crates/eaglemode/Cargo.toml crates/eaglemode/tests/integration/dynamic_plugins.rs crates/eaglemode/tests/integration/main.rs
git commit -m "test(integration): add dynamic plugin loading tests for all production plugins

Exercises TryCreateFilePanel -> dlopen -> dlsym for each of the 4
production plugin functions (emDir, emDirStat, emFileLink, emStocks).
These tests prove the dynamic path works independently of the static
plugin resolver."
```

---

## Task 3: Remove static plugin resolver infrastructure

**Files:**
- Modify: `crates/emcore/src/emFpPlugin.rs:13-31,244-273`
- Delete: `crates/emmain/src/static_plugins.rs`
- Modify: `crates/emmain/src/lib.rs:15`
- Modify: `crates/emmain/Cargo.toml:15-16`
- Modify: `crates/eaglemode/src/main.rs:25-28`

All changes in this task are atomic — intermediate states don't compile. Apply all edits, then build and test.

- [ ] **Step 1: Remove STATIC_RESOLVER from emFpPlugin.rs**

In `crates/emcore/src/emFpPlugin.rs`, delete lines 13-31 (the comment block, `StaticResolverFn` type alias, `STATIC_RESOLVER` thread-local, and `set_static_plugin_resolver` function):

```rust
// DELETE everything from line 13 through line 31:
// ── Static plugin resolver hook ─────────────────────────────────────
// ...
// pub fn set_static_plugin_resolver(...) { ... }
```

- [ ] **Step 2: Simplify TryCreateFilePanel to use only dynamic loading**

In `crates/emcore/src/emFpPlugin.rs`, replace the function body of `TryCreateFilePanel` (lines 222-295). The new body removes the static resolver check and de-indents the dynamic path:

```rust
    /// Create a file panel via this plugin's function.
    /// Port of C++ `emFpPlugin::TryCreateFilePanel`.
    pub fn TryCreateFilePanel(
        &self,
        parent: &PanelParentArg,
        name: &str,
        path: &str,
    ) -> Result<Box<dyn PanelBehavior>, FpPluginError> {
        use crate::emStd2::{emTryResolveSymbol, LibError};

        let mut cached = self.cached.borrow_mut();

        // Invalidate cache if library changed (matches C++ CachedLibName check)
        if cached.lib_name != self.library {
            *cached = CachedFunctions::default();
            cached.lib_name = self.library.clone();
        }

        // Resolve function if not cached or function name changed
        if cached.func.is_none() || cached.func_name != self.function {
            if self.function.is_empty() {
                return Err(FpPluginError::EmptyFunctionName);
            }

            let ptr = unsafe {
                emTryResolveSymbol(&self.library, false, &self.function)
            }
            .map_err(|e| match e {
                LibError::LibraryLoad { library, message } => {
                    FpPluginError::LibraryLoad { library, message }
                }
                LibError::SymbolResolve {
                    library,
                    symbol,
                    message,
                } => FpPluginError::SymbolResolve {
                    library,
                    symbol,
                    message,
                },
            })?;

            cached.func =
                Some(unsafe { std::mem::transmute::<*const (), emFpPluginFunc>(ptr) });
            cached.func_name = self.function.clone();
        }

        let func = cached.func.expect("func was just set");
        drop(cached); // release borrow before calling plugin function

        let mut error_buf = String::new();
        match func(parent, name, path, self, &mut error_buf) {
            Some(panel) => Ok(panel),
            None => Err(FpPluginError::PluginFunctionFailed {
                function: self.function.clone(),
                message: if error_buf.is_empty() {
                    format!(
                        "Plugin function {} in {} failed.",
                        self.function, self.library
                    )
                } else {
                    error_buf
                },
            }),
        }
    }
```

- [ ] **Step 3: Delete static_plugins.rs**

Delete `crates/emmain/src/static_plugins.rs`.

- [ ] **Step 4: Remove module declaration from emmain lib.rs**

In `crates/emmain/src/lib.rs`, delete the line:
```rust
pub mod static_plugins;
```

- [ ] **Step 5: Remove emstocks and emfileman dependencies from emmain**

In `crates/emmain/Cargo.toml`, delete:
```toml
emstocks = { path = "../emstocks" }
emfileman = { path = "../emfileman" }
```

- [ ] **Step 6: Remove set_static_plugin_resolver call from main.rs**

In `crates/eaglemode/src/main.rs`, delete lines 25-28:
```rust
    // 2. Register static plugin resolver
    emcore::emFpPlugin::set_static_plugin_resolver(
        emMain::static_plugins::resolve_static_plugin,
    );
```

- [ ] **Step 7: Build and verify**

Run:
```bash
cargo check 2>&1 | tail -5
```

Expected: Clean compilation with no errors. The `RefCell` import in emFpPlugin.rs may become unused — if so, remove it.

- [ ] **Step 8: Run all tests**

Run:
```bash
cargo-nextest ntr
```

Expected: All tests pass, including the new `dynamic_load_*` tests from Task 2 and the existing `plugin_invocation` and `plugin_e2e` tests.

- [ ] **Step 9: Run clippy**

Run:
```bash
cargo clippy -- -D warnings
```

Expected: Clean. Watch for unused imports (`RefCell` in emFpPlugin.rs if it was only used by `STATIC_RESOLVER`).

- [ ] **Step 10: Commit**

```bash
git add -u
git commit -m "feat(plugins): remove static plugin resolver, use dynamic loading exclusively

Delete static_plugins.rs and the STATIC_RESOLVER thread-local in
emFpPlugin.rs. TryCreateFilePanel now always resolves plugin functions
via dlopen/dlsym. Remove emmain's compile-time dependencies on
emstocks and emfileman — they are now pure runtime plugins."
```

---

## Task 4: Smoke test — cargo run

- [ ] **Step 1: Full workspace build**

Run:
```bash
cargo build 2>&1 | tail -3
```

Expected: Builds all workspace members including plugin `.so` files.

- [ ] **Step 2: Verify .so files exist**

Run:
```bash
ls -1 target/debug/lib{emFileMan,emStocks,test_plugin}.so
```

Expected: All three files listed.

- [ ] **Step 3: Run the application briefly**

Run:
```bash
timeout 3 cargo run 2>&1; echo "exit: $?"
```

Expected: Application starts (may show window briefly), exits with timeout signal. No `LibraryLoad` or `SymbolResolve` errors in output.

---

## Verification

After all tasks are complete:

1. **No static resolver in codebase:**
   ```bash
   rg 'STATIC_RESOLVER|set_static_plugin_resolver|static_plugins' crates/
   ```
   Expected: No matches.

2. **All tests pass:**
   ```bash
   cargo-nextest ntr
   ```

3. **Clippy clean:**
   ```bash
   cargo clippy -- -D warnings
   ```

4. **RPATH set:**
   ```bash
   readelf -d target/debug/eaglemode | grep RUNPATH
   ```
   Expected: `[$ORIGIN]`

5. **Dynamic loading works for all plugins:**
   ```bash
   cargo-nextest ntr -E 'test(dynamic_load)'
   ```
   Expected: 4 tests pass.
