# Plugin Manager Design

Date: 2026-03-30

## Objective

Port the full Eagle Mode plugin system to Rust: dynamic library loading
(`emTryOpenLib`/`emTryResolveSymbol`), plugin function invocation
(`emFpPluginFunc`/`emFpPluginModelFunc`), and the workspace restructuring
required to produce shared libraries. Convert emStocks from a static
in-crate module to a dynamically loaded plugin `.so`, eliminating the
static registration stub.

## Rationale

emStocks was ported as the first outside-emCore app module but used static
registration because the plugin manager didn't exist. This accumulates
stubs: `emStocksFpPlugin.rs` is a placeholder, `emStocksFilePanel.rs` has
no plugin integration, and no `.emFpPlugin` config file exists in the Rust
tree. Every future app module would accumulate the same stubs. Building the
plugin manager now means all subsequent app ports (emBmp, emTga, emFileMan,
etc.) wire up correctly from day one.

## Scope

- Workspace restructuring: single crate -> Cargo workspace with `dylib`
  emCore and `cdylib` plugin crates
- Full `emStd2` dynamic library API: `emTryOpenLib`, `emTryResolveSymbol`,
  `emTryResolveSymbolFromLib`, `emCloseLib`, library table with refcounting
- Plugin function signatures: `emFpPluginFunc` and `emFpPluginModelFunc`
- Plugin invocation: `emFpPlugin::TryCreateFilePanel` and
  `emFpPlugin::TryAcquireModel` with cached function pointers
- `emFpPluginList::CreateFilePanel` — end-to-end file-to-panel creation
- `.emFpPlugin` config files checked into `etc/emCore/FpPlugins/`
- emStocks converted to a separate `cdylib` crate exporting `extern "C"`
  entry point
- Full `emInstallInfo` port (already complete — verify library path
  resolution works end-to-end)

---

## Section 1: Principles & Constraints

### C++ parity (governing principle)

The plugin system matches C++ architecture: shared `dylib` for emCore,
`cdylib` plugins loaded via `dlopen`/`dlsym`, C function signatures at the
boundary, Rust types shared across the boundary because both sides link the
same `libemcore` dylib. No static registration fallback.

### ABI contract

Host binary and all plugin `.so` files must be built with the same rustc
version and the same `libemcore` dylib. This matches the C++ constraint
where all `.so` files ship from the same build. Rust's `dylib` crate type
uses the unstable Rust ABI, which is stable within a single compiler
version.

### File and Name Correspondence (inherited)

`emFpPlugin.h` maps to `emFpPlugin.rs`. Dynamic library functions from
`emStd2.h` map to `emStd2.rs`. `emStocksFpPlugin.cpp` maps to
`emStocksFpPlugin.rs`. Config files use the same `.emFpPlugin` format and
filenames as C++.

### emPainter firewall (inherited)

No changes to emPainter*.rs files.

### Directory structure mirrors C++ (inherited)

Source moves to `crates/emcore/src/` and `crates/emstocks/src/` to become
separate Cargo crates. The internal file layout (one `.rs` file per C++
header) is unchanged. The workspace `Cargo.toml` sits at the repo root.

---

## Section 2: Workspace Restructuring

### Current structure

```
eaglemode-rs/
  Cargo.toml          (single crate: lib + [[bench]])
  src/
    lib.rs            (pub mod emCore; pub mod emStocks;)
    emCore/
      mod.rs
      ...103 .rs files...
    emStocks/
      mod.rs
      ...11 .rs files...
  tests/
    ...
```

### Target structure

```
eaglemode-rs/
  Cargo.toml          (workspace root)
  crates/
    emcore/
      Cargo.toml      (crate-type = ["dylib"], name = "emcore")
      src/             (moved from src/emCore/)
        lib.rs         (was mod.rs)
        ...103 .rs files...
    emstocks/
      Cargo.toml      (crate-type = ["cdylib"], name = "emStocks")
      src/             (moved from src/emStocks/)
        lib.rs         (was mod.rs)
        ...11 .rs files...
    eaglemode/
      Cargo.toml      (host binary)
      src/
        main.rs
  etc/
    emCore/
      FpPlugins/
        emStocks.emFpPlugin
        version
  tests/
    ...                (workspace-level integration tests)
```

### Crate naming

- `emcore` crate produces `libemcore.so` (dylib). The crate name is
  lowercase per Cargo convention; the module inside uses `#[allow(non_snake_case)]`
  as today.
- `emStocks` crate produces `libemStocks.so` (cdylib). Library name
  matches C++ convention — the plugin config's `Library = "emStocks"`
  resolves to `libemStocks.so`.
- `eaglemode` crate is the host binary.

### Import path changes

All `use crate::emCore::` becomes `use emcore::`. All `use crate::emStocks::`
becomes `use emstocks::` (only within emStocks' own crate) or a dylib
import. This is mechanical — every `.rs` file in the repo gets its imports
updated.

### Dependency graph

```
emcore (dylib)
  <- emStocks (cdylib, depends on emcore)
  <- eaglemode (bin, depends on emcore)
```

The host binary does NOT depend on emStocks at compile time. It discovers
and loads emStocks at runtime via `dlopen`.

### Test organization

- Unit tests (`#[cfg(test)]` modules) stay inside each crate.
- Behavioral and integration tests move to `crates/emcore/tests/` and
  `crates/emstocks/tests/` respectively, or stay at workspace root under
  `tests/` for cross-crate integration tests.
- Golden tests remain at workspace root (they test rendering output, not
  crate internals).

---

## Section 3: Dynamic Library API (`emStd2.rs`)

Port the C++ dynamic library management functions from `emStd2.h`/`emStd2.cpp`.

### Types

```rust
/// Opaque handle to a loaded dynamic library.
/// Port of C++ `emLibHandle` (typedef void*).
pub struct emLibHandle {
    entry_index: usize,  // index into LIBRARY_TABLE
}
```

### Library table

Port of C++ `emLibTableEntry` and `emLibTable`:

```rust
struct LibTableEntry {
    filename: String,
    ref_count: u64,      // 0 = infinite (never unloaded)
    handle: libloading::Library,
}

/// Global library table. Single-threaded (no mutex needed — C++ uses
/// emThreadMiniMutex but Rust eaglemode is single-threaded).
thread_local! {
    static LIBRARY_TABLE: RefCell<Vec<LibTableEntry>> = RefCell::new(Vec::new());
}
```

DIVERGED from C++: C++ uses a mutex-protected global array with binary
search by filename. Rust uses `RefCell<Vec<...>>` (single-threaded) with
linear search. Binary search optimization can be added if the table grows
large enough to matter (C++ Eagle Mode has ~34 plugins).

### Functions

```rust
/// Open a dynamic library. Port of C++ `emTryOpenLib`.
///
/// If `is_filename` is false, `lib_name` is a pure name converted to a
/// platform filename: "emStocks" -> "libemStocks.so" (Linux),
/// "libemStocks.dylib" (macOS), "emStocks.dll" (Windows).
///
/// Libraries are cached: opening the same library twice returns the same
/// handle with an incremented refcount.
pub fn emTryOpenLib(lib_name: &str, is_filename: bool) -> Result<emLibHandle, FpPluginError>

/// Resolve a symbol from an open library. Port of C++ `emTryResolveSymbolFromLib`.
///
/// Returns a raw function pointer. Caller must transmute to the correct
/// function signature.
///
/// # Safety
/// The returned pointer is only valid while the library remains open.
pub unsafe fn emTryResolveSymbolFromLib(
    handle: &emLibHandle,
    symbol: &str,
) -> Result<*const (), FpPluginError>

/// Close a dynamic library. Port of C++ `emCloseLib`.
///
/// Decrements refcount. When refcount reaches zero, the library is
/// unloaded and the table entry is removed. If refcount was already
/// zero (infinite lifetime), this is a no-op.
pub fn emCloseLib(handle: emLibHandle)

/// Open a library, resolve a symbol, and set the library to infinite
/// lifetime. Port of C++ `emTryResolveSymbol`.
///
/// The library is never closed after this call (refcount set to 0).
/// This matches C++ behavior where plugin libraries persist for the
/// process lifetime.
///
/// # Safety
/// Same as `emTryResolveSymbolFromLib`.
pub unsafe fn emTryResolveSymbol(
    lib_name: &str,
    is_filename: bool,
    symbol: &str,
) -> Result<*const (), FpPluginError>
```

### Library search path

C++ uses `dlopen("libFoo.so", RTLD_NOW|RTLD_GLOBAL)` which relies on
`LD_LIBRARY_PATH` or the system library search path. The Rust port does
the same: construct the filename (`lib{name}.so`), pass to
`libloading::Library::new()`. During development, `LD_LIBRARY_PATH` must
include the Cargo output directory (`target/debug/` or `target/release/`).

For installed deployments, `emGetInstallPath(InstallDirType::Lib, ...)` provides
the library directory. The plugin system prepends this to the search path
via `std::env::set_var("LD_LIBRARY_PATH", ...)` at startup if not already
set, matching C++ behavior where the installation's `lib/` directory is
in the library path.

---

## Section 4: Plugin Function Signatures

### Panel creation function

Port of C++ `emFpPluginFunc`:

```rust
/// Type of the plugin function for creating a file panel.
/// Port of C++ `emFpPluginFunc`.
///
/// # Arguments
/// - `parent` — parent panel argument (for constructing child panels)
/// - `name` — name of the panel
/// - `path` — filesystem path of the file to show
/// - `plugin` — the plugin record (for reading properties)
/// - `error_buf` — mutable string for returning error messages
///
/// # Returns
/// The created panel as a trait object, or None on failure (with
/// error_buf populated).
pub type emFpPluginFunc = fn(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>>;
```

### Model acquisition function

Port of C++ `emFpPluginModelFunc`:

```rust
/// Type of the plugin model function for acquiring file models.
/// Port of C++ `emFpPluginModelFunc`.
///
/// # Arguments
/// - `context` — the context for the model
/// - `class_name` — class name or base class name of the model
/// - `name` — name of the model (usually the file path)
/// - `common` — true for common/shared model, false for private
/// - `plugin` — the plugin record
/// - `error_buf` — mutable string for returning error messages
///
/// # Returns
/// The acquired model, or None on failure.
pub type emFpPluginModelFunc = fn(
    context: &Rc<emContext>,
    class_name: &str,
    name: &str,
    common: bool,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn Any>>>;
```

DIVERGED from C++: C++ returns `emPanel*` and `emRef<emModel>*`. Rust
returns trait objects (`dyn PanelBehavior`, `dyn Any`) wrapped in
`Rc<RefCell<...>>`. This works because both sides link the same
`libemcore.so` dylib, sharing type definitions and vtables.

### Calling convention and symbol lookup

C++ uses `extern "C"` for two reasons: (1) prevent C++ name mangling so
`dlsym` can find the symbol by name, and (2) use the C calling convention.

Rust uses `#[no_mangle]` for reason (1) — it prevents Rust name mangling
so `dlsym` can find the symbol. For reason (2), since both the host and
plugin link the same `libemcore.so` dylib and are built with the same
rustc, we use the **Rust calling convention** (plain `fn`, no `extern "C"`).
This allows passing Rust types (`Rc`, `&str`, trait objects) directly
without ABI translation. The exported plugin functions use:

```rust
#[no_mangle]
pub fn emStocksFpPluginFunc(...) -> ... { ... }
```

Not `extern "C" fn`. The `#[no_mangle]` ensures `dlsym("emStocksFpPluginFunc")`
works. The Rust ABI ensures complex types pass correctly. This matches
C++'s approach: C++ also passes C++ types (`emString&`, `emPanel*`) through
`extern "C"` functions — the `extern "C"` only affects name mangling and
calling convention, not the types themselves. In Rust we choose the Rust
calling convention to get correct handling of Rust-specific types.

### Safety model

Since host and plugin share the same Rust ABI (same compiler, same dylib),
passing `Rc<RefCell<...>>`, `&str`, `&String`, and trait objects across the
boundary is safe. The `#[no_mangle]` annotation prevents name mangling.
The shared `libemcore.so` dylib ensures type definitions, vtables, and
allocator are identical on both sides.

If a plugin is built with a different rustc version, loading it will likely
crash. This is an acceptable constraint — C++ has the same limitation. The
plugin manager could add a version check (embed `rustc --version` hash in
plugin metadata) as a safety net, matching C++ `.emFpPlugin` version files.

---

## Section 5: Plugin Invocation (`emFpPlugin.rs` completion)

### Cached function pointers

The existing `emFpPlugin` struct has a `cached_library: RefCell<Option<CachedLibrary>>`
field. This is expanded to match C++:

```rust
struct CachedFunctions {
    lib_name: String,       // Library name when cache was populated
    func_name: String,      // Function name when cache was populated
    func: Option<emFpPluginFunc>,
    model_func_name: String,
    model_func: Option<emFpPluginModelFunc>,
}
```

Cache invalidation: if `self.library` != `cached.lib_name`, clear all
cached pointers (matching C++ `CachedLibName` check).

### TryCreateFilePanel

Port of C++ `emFpPlugin::TryCreateFilePanel`:

```rust
impl emFpPlugin {
    /// Create a file panel via this plugin's function.
    /// Port of C++ `emFpPlugin::TryCreateFilePanel`.
    pub fn TryCreateFilePanel(
        &self,
        parent: &PanelParentArg,
        name: &str,
        path: &str,
    ) -> Result<Rc<RefCell<dyn PanelBehavior>>, FpPluginError> {
        // 1. Check cache validity (library name match)
        // 2. Resolve function if not cached
        // 3. Call function
        // 4. Return panel or error
    }
}
```

### TryAcquireModel

Port of C++ `emFpPlugin::TryAcquireModel` and `TryAcquireModelImpl`:

```rust
impl emFpPlugin {
    /// Acquire a model via this plugin's model function.
    /// Port of C++ `emFpPlugin::TryAcquireModelImpl`.
    pub fn TryAcquireModel(
        &self,
        context: &Rc<emContext>,
        class_name: &str,
        name: &str,
        common: bool,
    ) -> Result<Rc<RefCell<dyn Any>>, FpPluginError> {
        // 1. Check cache validity
        // 2. Resolve model function if not cached
        // 3. Call model function
        // 4. Return model or error
    }
}
```

### CreateFilePanel on emFpPluginList

Port of C++ `emFpPluginList::CreateFilePanel`:

```rust
impl emFpPluginList {
    /// Create a panel for a file using the best matching plugin.
    /// Port of C++ `emFpPluginList::CreateFilePanel`.
    ///
    /// On failure, returns an error panel displaying the error message
    /// (matching C++ behavior of returning `emErrorPanel`).
    pub fn CreateFilePanel(
        &self,
        parent: &PanelParentArg,
        name: &str,
        path: &str,
        alternative: usize,
    ) -> Rc<RefCell<dyn PanelBehavior>> {
        // 1. Stat the file
        // 2. SearchPlugin for matching plugin
        // 3. Call plugin.TryCreateFilePanel
        // 4. On error, create emErrorPanel
    }

    /// Overload with pre-computed stat information.
    /// Port of C++ `emFpPluginList::CreateFilePanel` (stat overload).
    pub fn CreateFilePanelWithStat(
        &self,
        parent: &PanelParentArg,
        name: &str,
        absolute_path: &str,
        stat_err: Option<std::io::Error>,
        stat_mode: FileStatMode,
        alternative: usize,
    ) -> Rc<RefCell<dyn PanelBehavior>> { ... }
}
```

### TryAcquireModel on emFpPluginList

Port of C++ `emFpPluginList::TryAcquireModel`:

```rust
impl emFpPluginList {
    /// Acquire a model via the best matching plugin.
    /// Port of C++ `emFpPluginList::TryAcquireModel`.
    pub fn TryAcquireModel(
        &self,
        context: &Rc<emContext>,
        class_name: &str,
        name: &str,
        name_is_file_path: bool,
        common: bool,
        alternative: usize,
        stat_mode: FileStatMode,
    ) -> Result<Rc<RefCell<dyn Any>>, FpPluginError> { ... }
}
```

---

## Section 6: emStocks as Dynamic Plugin

### Crate structure

`crates/emstocks/Cargo.toml`:
```toml
[package]
name = "emstocks"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
# Produces libemStocks.so (library name set via output name)

[dependencies]
emcore = { path = "../emcore" }
```

Note: Cargo's `cdylib` output name is `lib{package_name}.so`. Since the
package name is `emstocks`, the output is `libemstocks.so`. The plugin
config's `Library = "emStocks"` needs to resolve to this. Two options:
1. Set `[lib] name = "emStocks"` to produce `libemStocks.so`
2. Use lowercase `Library = "emstocks"` in the config file

Option 1 matches C++ naming. Use `[lib] name = "emStocks"`.

### Exported function

`crates/emstocks/src/lib.rs` (or `emStocksFpPlugin.rs`):

```rust
/// Plugin entry point for .emStocks files.
/// Port of C++ `emStocksFpPluginFunc` in emStocksFpPlugin.cpp.
#[no_mangle]
pub fn emStocksFpPluginFunc(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emStocksFpPlugin: No properties allowed.".to_string();
        return None;
    }
    let file_model = emStocksFileModel::Acquire(
        parent.root_context(),
        path,
    );
    Some(emStocksFilePanel::new(parent, name, file_model))
}
```

### Config file

`etc/emCore/FpPlugins/emStocks.emFpPlugin`:
```
#%rec:emFpPlugin%#

FileTypes = { ".emStocks" }
FileFormatName = "emStocks"
Priority = 1.0
Library = "emStocks"
Function = "emStocksFpPluginFunc"
```

Identical to C++ config file. The Record parser already handles this format.

---

## Section 7: `emInstallInfo` Verification

`emInstallInfo.rs` is already fully ported (216 lines). It provides:
- `emGetInstallPath(InstallDirType, prj, sub_path)` — all 11 directory types
- `emGetConfigDirOverloadable(prj, sub_dir)` — version-gated host/user config

For the plugin system:
- `emGetInstallPath(InstallDirType::Lib, "emCore", None)` provides the library
  search directory for installed deployments
- `emGetConfigDirOverloadable("emCore", Some("FpPlugins"))` provides the
  plugin config directory (already used by `emFpPluginList::load_plugins`)

No changes needed. Verify end-to-end with integration tests that set
`EM_DIR` to the repo's `etc/` parent directory.

---

## Section 8: Config File Distribution

### Directory structure

```
etc/
  emCore/
    FpPlugins/
      emStocks.emFpPlugin
      version
```

The `version` file contains a version string matching
`~/.eaglemode/emCore/FpPlugins/version` for the user-override mechanism.
Content: `"0.96.4"` (matching C++ version for compatibility during
development).

### Development workflow

Set `EM_DIR` to the repo root so that
`emGetInstallPath(HostConfig, "emCore", None)` resolves to
`<repo>/etc/emCore/`. This makes plugin config discovery work without
installation.

Set `LD_LIBRARY_PATH` to include `target/debug/` (or `target/release/`)
so that `dlopen("libemStocks.so")` finds the built plugin. A wrapper
script or `.cargo/config.toml` runner can automate this.

### Future plugin configs

As more app modules are ported (emBmp, emTga, etc.), their `.emFpPlugin`
config files are added to `etc/emCore/FpPlugins/`. Each gets its own
crate under `crates/`.

---

## Section 9: Testing Strategy

### Layer coverage

| Component | Unit | Behavioral | Integration |
|---|---|---|---|
| emLibHandle / library table | x | | |
| emTryOpenLib | x | x (cache, refcount) | |
| emTryResolveSymbol | x | x (cache invalidation) | |
| emFpPlugin::TryCreateFilePanel | | x (mock plugin .so) | |
| emFpPlugin::TryAcquireModel | | x (mock plugin .so) | |
| emFpPluginList::CreateFilePanel | | | x (end-to-end with emStocks .so) |
| emStocksFpPluginFunc | x | | x (loaded via dlopen) |
| Workspace build | | | x (cargo build --workspace) |

### Test plugin `.so`

A minimal test plugin crate (`crates/test_plugin/`) that exports a trivial
`emFpPluginFunc` returning a dummy panel. Used by behavioral tests to
validate the full dlopen -> resolve -> call -> return path without depending
on emStocks. This isolates plugin system tests from emStocks correctness.

### Integration test: emStocks end-to-end

1. `cargo build --workspace` produces `libemcore.so`, `libemStocks.so`,
   and the host binary
2. Set `EM_DIR` and `LD_LIBRARY_PATH`
3. Host acquires `emFpPluginList`
4. `CreateFilePanel` for a `.emStocks` file
5. Verify panel is an `emStocksFilePanel`

### Existing tests

The 378-line behavioral test suite in `tests/behavioral/fp_plugin.rs` tests
plugin matching, search, and serialization. These tests remain and are
extended with invocation tests.

---

## Section 10: Phase Structure

### Phase 1 — Workspace Restructuring

**Goal:** Convert the single crate into a Cargo workspace with `emcore`
as a `dylib` crate, without changing any functionality.

Work items:
1. Create workspace `Cargo.toml` at repo root
2. Create `crates/emcore/Cargo.toml` with `crate-type = ["dylib"]`
3. Move `src/emCore/` contents to `crates/emcore/src/`
4. Convert `src/emCore/mod.rs` to `crates/emcore/src/lib.rs`
5. Create `crates/eaglemode/Cargo.toml` (host binary or library)
6. Update all imports from `crate::emCore::` to `emcore::`
7. Move emStocks to `crates/emstocks/` with `crate-type = ["cdylib"]`
8. Update emStocks imports
9. Move tests to appropriate crate test directories
10. Verify `cargo build --workspace`, `cargo clippy`, `cargo-nextest ntr`

Gate: All existing tests pass. `cargo build --workspace` produces
`libemcore.so` and `libemStocks.so`. No functional changes.

### Phase 2 — Dynamic Library API

**Goal:** Port `emTryOpenLib`, `emTryResolveSymbol`, `emCloseLib` and the
library table to `emStd2.rs`.

Work items:
1. Define `emLibHandle` type
2. Implement library table with `RefCell<Vec<LibTableEntry>>`
3. Port `emTryOpenLib` with platform filename construction
4. Port `emTryResolveSymbolFromLib` using `libloading::Library::get`
5. Port `emCloseLib` with refcount decrement
6. Port `emTryResolveSymbol` (open + resolve + set infinite lifetime)
7. Unit tests for all functions
8. Behavioral tests: cache hits, refcount lifecycle, error paths

Gate: Can load a `.so`, resolve a symbol, and get a valid function pointer.
Library table caches correctly. Refcount lifecycle works.

### Phase 3 — Plugin Function Signatures & Invocation

**Goal:** Define plugin function types, implement `TryCreateFilePanel` and
`TryAcquireModel` on `emFpPlugin`, implement `CreateFilePanel` on
`emFpPluginList`.

Work items:
1. Define `emFpPluginFunc` and `emFpPluginModelFunc` type aliases
2. Implement `CachedFunctions` struct with cache invalidation
3. Implement `emFpPlugin::TryCreateFilePanel`
4. Implement `emFpPlugin::TryAcquireModel`
5. Implement `emFpPluginList::CreateFilePanel` (both overloads)
6. Implement `emFpPluginList::TryAcquireModel`
7. Create test plugin crate (`crates/test_plugin/`)
8. Behavioral tests with test plugin `.so`
9. Error path tests (missing library, missing symbol, plugin returns None)

Gate: Test plugin `.so` loads, function resolves, panel is created and
returned across the dylib boundary. Error paths produce correct
`FpPluginError` variants.

### Phase 4 — emStocks Plugin Conversion

**Goal:** Convert emStocks from static stub to dynamic plugin.

Work items:
1. Rewrite `emStocksFpPlugin.rs` with `#[no_mangle] extern "C"` entry point
2. Wire `emStocksFilePanel` creation into the plugin function
3. Create `etc/emCore/FpPlugins/emStocks.emFpPlugin` config file
4. Create `etc/emCore/FpPlugins/version` file
5. Integration test: load `.emStocks` file via plugin system
6. Remove all static registration stubs and DIVERGED annotations
7. Update `docs/CORRESPONDENCE.md` with plugin system status

Gate: `emStocksFpPluginFunc` is loaded from `libemStocks.so` via `dlopen`,
creates an `emStocksFilePanel`, and the full path from config file to
panel creation works end-to-end.

### Phase 5 — Polish & Documentation

**Goal:** Development workflow tooling, CI integration, documentation.

Work items:
1. `.cargo/config.toml` runner configuration for `LD_LIBRARY_PATH` and
   `EM_DIR`
2. Verify all golden tests still pass in workspace layout
3. Verify benchmarks still work
4. Update CLAUDE.md if any commands changed
5. Final test audit

Gate: `cargo-nextest ntr` passes all tests including plugin loading.
Development workflow documented. No stubs remain in emStocks.
