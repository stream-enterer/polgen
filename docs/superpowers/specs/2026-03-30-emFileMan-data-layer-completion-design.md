# emFileMan Data Layer Completion Design

Date: 2026-03-30

## Objective

Complete every partially-implemented file in `crates/emfileman/` to full C++
API parity so the panel rendering layer has no data/model blockers. Fix two
emcore infrastructure gaps that block panel creation.

## Scope

- 7 emFileMan files to complete (emDirModel, emFileLinkModel, emFileManConfig,
  emFileManTheme, emFileManThemeNames, emFileManViewConfig, emFileManModel)
- 2 emcore infrastructure fixes (PanelParentArg, CreateFilePanel return type)
- 1 new utility (synchronous image file loader for theme border images)
- Zero stubs — all features fully implemented with existing infrastructure

## Out of Scope

- PanelBehavior traits, Paint methods, panel rendering, LayoutChildren,
  UpdateChildren, Input handling, UI widget composition
- Full plugin-based async image loading system (emTga/emBmp/emGif app modules)
- Any new panel files

---

## Section 1: emcore Infrastructure Fixes

### 1a. PanelParentArg Extension

**File:** `crates/emcore/src/emFpPlugin.rs`

Current state: `PanelParentArg` carries only `Rc<emContext>`. DIVERGED comment
documents simplified version.

Change: Add `parent_panel: Option<PanelId>` field.

```rust
pub struct PanelParentArg {
    root_context: Rc<emContext>,
    parent_panel: Option<PanelId>,  // NEW
}

impl PanelParentArg {
    pub fn new(root_context: Rc<emContext>) -> Self {
        Self { root_context, parent_panel: None }
    }

    pub fn with_parent(root_context: Rc<emContext>, parent: PanelId) -> Self {
        Self { root_context, parent_panel: Some(parent) }
    }

    pub fn root_context(&self) -> &Rc<emContext> { &self.root_context }
    pub fn parent_panel(&self) -> Option<PanelId> { self.parent_panel }
}
```

Update DIVERGED comment to document partial integration (carries parent ID but
not full C++ layout constraint forwarding).

### 1b. CreateFilePanel Return Type

**File:** `crates/emcore/src/emFpPlugin.rs`

Current state: `emFpPluginFunc` returns `Option<Rc<RefCell<dyn PanelBehavior>>>`.
`PanelCtx::create_child_with` takes `Box<dyn PanelBehavior>`.

Change: Switch to `Box<dyn PanelBehavior>`.

```rust
pub type emFpPluginFunc = fn(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>>;
```

Update `CreateFilePanel` and `CreateFilePanelWithStat` to return
`Box<dyn PanelBehavior>`. Update all 4 plugin entry points (emStocks + 3
emFileMan). DIVERGED comment explains: panels are tree-owned via Box, not
shared via Rc.

---

## Section 2: Model Wrapping Pattern

Each incomplete file follows the same structural pattern. The existing "data"
struct (e.g., `emDirModelData`) keeps its current code. A new "model" struct
wraps it with the appropriate emcore model infrastructure.

### Pattern A: emFileModel composition (emDirModel)

```rust
pub struct emDirModel {
    file_model: emFileModel<emDirModelData>,
}

impl FileModelOps for emDirModel {
    fn reset_data(&mut self) { self.file_model.data_mut().reset_data() }
    fn try_start_loading(&mut self) -> Result<(), String> { ... }
    fn try_continue_loading(&mut self) -> Result<bool, String> { ... }
    // ... delegates to emDirModelData's existing methods
}
```

### Pattern B: emRecFileModel composition (emFileLinkModel)

```rust
pub struct emFileLinkModel {
    rec_model: emRecFileModel<emFileLinkData>,
}
```

### Pattern C: emConfigModel composition (emFileManConfig, emFileManTheme)

```rust
pub struct emFileManConfig {
    config_model: emConfigModel<emFileManConfigData>,
}
```

### Pattern D: Plain model with generation counter (emFileManViewConfig, emFileManModel, emFileManThemeNames)

```rust
pub struct emFileManViewConfig {
    // ... fields ...
    change_generation: Rc<Cell<u64>>,
}
```

All model types get `Acquire(ctx, ...)` delegating to `ctx.acquire::<Self>(...)`.

---

## Section 3: Per-File Completions

### 3a. emDirModel.rs (~40% → 100%)

**Has:** `emDirModelData` with 3-phase loading state machine, all entry
accessors, progress calc, deduplication.

**Add:**

- `emDirModel` struct composing `emFileModel<emDirModelData>`
- `FileModelOps` impl delegating to `emDirModelData`'s existing
  `try_start_loading_from`, `try_continue_loading`, `quit_loading`,
  `reset_data`, `calc_memory_need`, `calc_file_progress`
- `Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>>` — name is
  directory path, delegates to `ctx.acquire()`
- Public delegating methods: `GetEntryCount()`, `GetEntry(idx)`,
  `GetEntryIndex(name)`, `IsOutOfDate()` — forward to inner data
- `FileModelState` implementation comes for free from `emFileModel<T>`

**C++ methods accounted for:**
- `Acquire` ✓
- `GetEntryCount` ✓ (existing)
- `GetEntry` ✓ (existing)
- `GetEntryIndex` ✓ (existing)
- `ResetData` ✓ (existing, wired to FileModelOps)
- `TryStartLoading` ✓ (existing, wired to FileModelOps)
- `TryContinueLoading` ✓ (existing, wired to FileModelOps)
- `QuitLoading` ✓ (existing, wired to FileModelOps)
- `CalcMemoryNeed` ✓ (existing, wired to FileModelOps)
- `CalcFileProgress` ✓ (existing, wired to FileModelOps)
- `IsOutOfDate` ✓ (existing)
- `TryStartSaving` / `TryContinueSaving` / `QuitSaving` — no-op (dirs don't save)
- `TryFetchDate` — metadata check via `std::fs::metadata`

### 3b. emFileLinkModel.rs (~50% → 100%)

**Has:** `emFileLinkData` with BasePathType enum, GetFullPath, Record trait.

**Add:**

- `emFileLinkModel` struct composing `emRecFileModel<emFileLinkData>`
- `Acquire(ctx: &Rc<emContext>, name: &str, common: bool) -> Rc<RefCell<Self>>`
  — name is `.emFileLink` file path
- `GetFormatName() -> &str` returns `"emFileLink"`
- `GetFullPath() -> String` delegates to `emFileLinkData::GetFullPath()` with
  the model's file path as context
- Public field accessors delegating to inner `emRecFileModel.data()`:
  `GetBasePathType()`, `GetBasePathProject()`, `GetPath()`, `GetHaveDirEntry()`

**C++ methods accounted for:**
- `Acquire` ✓
- `GetFormatName` ✓
- `GetFullPath` ✓ (existing logic, new delegation)
- Record fields (BasePathType, BasePathProject, Path, HaveDirEntry) ✓ (existing)

### 3c. emFileManConfig.rs (~95% → 100%)

**Has:** `emFileManConfigData` with all 6 fields, Record trait, defaults.

**Add:**

- `emFileManConfig` struct composing `emConfigModel<emFileManConfigData>`
- `Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>>` — singleton, empty name.
  Config path resolved via `emGetConfigDirOverloadable("emFileMan", None)`
- `GetFormatName() -> &str` returns `"emFileManConfig"`
- Getter/setter pairs delegating to `config_model`:
  - `GetSortCriterion()` / `SetSortCriterion()`
  - `GetNameSortingStyle()` / `SetNameSortingStyle()`
  - `GetSortDirectoriesFirst()` / `SetSortDirectoriesFirst()`
  - `GetShowHiddenFiles()` / `SetShowHiddenFiles()`
  - `GetThemeName()` / `SetThemeName()`
  - `GetAutosave()` / `SetAutosave()`
- `GetChangeSignal() -> SignalId` from inner `emConfigModel` (note: this is
  a scheduler SignalId, not a u64 generation counter — emConfigModel provides
  signal infrastructure natively)

### 3d. emFileManTheme.rs (~85% → 100%)

**Has:** `emFileManThemeData` with ~100 fields, Record trait, 4 image path
strings.

**Add:**

- `ImageFileRec` struct:
  ```rust
  pub struct ImageFileRec {
      path: String,
      cached_image: RefCell<Option<emImage>>,
  }

  impl ImageFileRec {
      pub fn GetImage(&self) -> Ref<emImage> { ... }
  }
  ```
  On first `GetImage()` call: resolves path relative to theme directory, reads
  file bytes via `std::fs::read()`, decodes via `load_tga()`. Caches result.
  Returns borrowed reference via `Ref::map`. If path is empty or load fails,
  returns a 1x1 transparent fallback image.

- `emFileManTheme` struct composing `emConfigModel<emFileManThemeData>` plus
  4 `ImageFileRec` instances
- `Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>>` — name is
  theme name, path resolved via `GetThemesDirPath()` + name + `.emFileManTheme`
- `GetFormatName() -> &str` returns `"emFileManTheme"`
- After config load, initialize 4 ImageFileRec from the string path fields
- All ~100 field getters delegate to `config_model.GetRec()`

**Image loading approach:** Synchronous TGA loader. Theme border images are
small TGA files loaded once at theme init. DIVERGED from C++ ImageFileRec
which uses async emImageFileModel loading — synchronous load is sufficient
for these small files and avoids pulling in the full image plugin system.

### 3e. emFileManThemeNames.rs (~90% → 100%)

**Has:** `emFileManThemeNames` catalog struct with all getters, grouping logic,
`HeightToAspectRatioString`.

**Add:**

- `Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>>` — singleton
- Filesystem discovery constructor: scans `GetThemesDirPath()` for
  `*.emFileManTheme` files, reads each file's `DisplayName`, `DisplayIcon`,
  `Height` fields (partial Record parse — only needs 3 fields), builds catalog
  via existing `from_themes()` constructor
- `Cycle()` implementation: checks directory mtime, rescans on change, bumps
  generation counter
- Generation counter: `change_generation: Rc<Cell<u64>>` for change detection

**C++ methods accounted for:**
- `Acquire` ✓
- `GetThemeStyleCount` ✓ (existing)
- `GetThemeAspectRatioCount` ✓ (existing)
- `GetThemeName` ✓ (existing)
- `GetDefaultThemeName` ✓ (existing)
- `GetThemeStyleDisplayName` ✓ (existing)
- `GetThemeStyleDisplayIcon` ✓ (existing)
- `GetThemeAspectRatio` ✓ (existing)
- `IsExistingThemeName` ✓ (existing)
- `GetThemeStyleIndex` ✓ (existing)
- `GetThemeAspectRatioIndex` ✓ (existing)

### 3f. emFileManViewConfig.rs (~30% → 100%)

**Has:** `CompareDirEntries` free function with all 6 sort criteria,
`SortConfig`, `NameSortingStyle` enum.

**Add:**

- `emFileManViewConfig` model struct:
  ```rust
  pub struct emFileManViewConfig {
      config: Rc<RefCell<emFileManConfig>>,
      theme: Rc<RefCell<emFileManTheme>>,
      theme_names: Rc<RefCell<emFileManThemeNames>>,
      sort_criterion: SortCriterion,
      name_sorting_style: NameSortingStyle,
      sort_directories_first: bool,
      show_hidden_files: bool,
      theme_name: String,
      autosave: bool,
      change_generation: Rc<Cell<u64>>,
      revisit_engine: RevisitEngine,
  }
  ```

- `Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>>` — acquires inner
  Config/Theme/ThemeNames, copies current config values to local fields

- Setter methods — each bumps `change_generation`, writes to inner
  `emFileManConfig` if autosave enabled:
  - `SetSortCriterion(sc)`, `SetNameSortingStyle(nss)`,
    `SetSortDirectoriesFirst(b)`, `SetShowHiddenFiles(b)`,
    `SetAutosave(b)`
  - `SetThemeName(name)` — also re-acquires theme, triggers RevisitEngine

- Getter methods:
  - `GetSortCriterion()`, `GetNameSortingStyle()`,
    `GetSortDirectoriesFirst()`, `GetShowHiddenFiles()`,
    `GetThemeName()`, `GetAutosave()`
  - `GetTheme() -> Ref<emFileManTheme>` — borrows from Rc<RefCell>
  - `GetChangeSignal() -> u64` — returns `change_generation.get()` (note:
    u64 generation counter, not SignalId — emFileManViewConfig is a plain
    model without scheduler signal infrastructure; panels poll by comparing
    cached vs current generation)

- `IsUnsaved() -> bool` — compares local fields vs config model fields
- `SaveAsDefault()` — writes all local fields to config model, triggers save

- `CompareDirEntries(&self, e1, e2) -> i32` — method form that builds
  SortConfig from current fields, delegates to existing free function

- `RevisitEngine` struct implementing `emEngine` trait:
  ```rust
  struct RevisitEngine {
      active: bool,
      saved_visit: Option<emVisitingViewAnimator::VisitState>,  // from emViewAnimator.rs
      timer_id: Option<TimerId>,  // from emTimer.rs TimerCentral
  }
  ```
  On theme change: saves current visit state via emVisitingViewAnimator
  (crates/emcore/src/emViewAnimator.rs:651+). On next Cycle: triggers
  animator to restore saved position. Timer via emTimer::TimerCentral
  (crates/emcore/src/emTimer.rs) for delayed activation.

- `Cycle() -> bool` — checks if inner config changed externally, syncs local
  fields, runs RevisitEngine cycle

**C++ methods accounted for:**
- `Acquire` ✓
- `GetChangeSignal` ✓
- All 6 getter/setter pairs ✓
- `GetTheme` ✓
- `CompareDirEntries` ✓ (existing logic, new method wrapper)
- `IsUnsaved` ✓
- `SaveAsDefault` ✓
- `Cycle` ✓
- RevisitEngine (private, internal) ✓

### 3g. emFileManModel.rs (~50% → 100%)

**Has:** `SelectionManager` (all select/deselect/swap/clear), `CommandNode`,
`parse_command_properties`, `SearchDefaultCommandFor`, `CheckDefaultCommand`,
IPC message handling.

**Add:**

- `emFileManModel` model struct:
  ```rust
  pub struct emFileManModel {
      selection: SelectionManager,
      command_root: Option<CommandNode>,
      shift_tgt_sel_path: String,
      ipc_server: emMiniIpcServer,
      command_run_id: u64,
      selection_generation: Rc<Cell<u64>>,
      commands_generation: Rc<Cell<u64>>,
  }
  ```

- `Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>>` — singleton on root
  context. On construction: starts IPC server, loads command tree.

- Selection methods delegate to existing `SelectionManager`, bumping
  `selection_generation` on mutation:
  - All existing methods (SelectAsSource, DeselectAsSource, etc.)
  - `GetShiftTgtSelPath() -> &str` / `SetShiftTgtSelPath(path)`
  - `GetSelectionSignal() -> u64` returns `selection_generation.get()`

- `SelectionToClipboard(view_config: &emFileManViewConfig, source: bool, names_only: bool) -> String`
  — formats selected paths as newline-separated text. If `names_only`, strips
  directory prefix.

- `CreateSortedSrcSelDirEntries(view_config: &emFileManViewConfig) -> Vec<emDirEntry>`
  / `CreateSortedTgtSelDirEntries(...)` — loads emDirEntry for each selected
  path, sorts via `view_config.CompareDirEntries()`

- Command tree methods:
  - `GetCommandRoot() -> Option<&CommandNode>` — returns tree root
  - `GetCommand(cmd_path: &str) -> Option<&CommandNode>` — DFS lookup by path
  - `SearchDefaultCommandFor(file_path: &str) -> Option<&CommandNode>` — existing
    free function, now also a method
  - `SearchHotkeyCommand(hotkey: &emInputHotkey) -> Option<&CommandNode>` — DFS
    matching hotkey field
  - `GetCommandsSignal() -> u64` returns `commands_generation.get()`

- `Icon` and `Look` fields on `CommandNode`:
  - `icon: Option<emImage>` — loaded from Icon property path via synchronous
    TGA loader (same as ImageFileRec)
  - `look: emLook` — built from BgColor/FgColor/ButtonBgColor/ButtonFgColor
    properties

- `HotkeyInput(hotkey: &emInputHotkey) -> bool` — searches for matching
  command, returns true if found (RunCommand called separately by panel layer
  which has view access)

- `RunCommand(cmd: &CommandNode, extra_env: &HashMap<String, String>)` — builds
  args (`[interpreter] cmd_path src_count tgt_count src_paths... tgt_paths...`),
  sets env vars (`EM_FM_SERVER_NAME`, `EM_COMMAND_RUN_ID`), calls
  `emProcess::TryStartUnmanaged`

- `UpdateCommands()` — scans command directory tree, computes CRC per
  directory, compares against stored `dir_crc` on each node, rebuilds changed
  subtrees. Bumps `commands_generation`.

- `UpdateSelection()` — delegates to existing `SelectionManager::UpdateSelection()`,
  bumps `selection_generation` if anything changed

- `GetMiniIpcServerName() -> &str` — returns IPC server name

- `Cycle() -> bool` — calls `UpdateSelection()` + `UpdateCommands()`. IPC
  server polling happens via its own emEngine integration (already built into
  emMiniIpcServer).

- IPC wiring: `emMiniIpcServer` callback calls existing
  `SelectionManager::handle_ipc_message()`, validates command_run_id

**C++ methods accounted for:**
- `Acquire` ✓
- `GetSelectionSignal` ✓
- All selection methods ✓ (existing, delegation + generation bump)
- `GetShiftTgtSelPath` / `SetShiftTgtSelPath` ✓
- `SwapSelection` ✓ (existing)
- `UpdateSelection` ✓ (existing, + generation bump)
- `SelectionToClipboard` ✓
- `CreateSortedSrcSelDirEntries` / `CreateSortedTgtSelDirEntries` ✓
- `GetMiniIpcServerName` ✓
- `GetCommandsSignal` ✓
- `GetCommandRoot` ✓
- `GetCommand` ✓
- `SearchDefaultCommandFor` ✓ (existing)
- `SearchHotkeyCommand` ✓
- `RunCommand` ✓
- `HotkeyInput` ✓
- `Cycle` ✓

---

## Section 4: Synchronous Image File Loader

**File:** `crates/emcore/src/emImageFile.rs` (extend existing file)

Add a standalone function:

```rust
/// Load an image from a file path synchronously.
/// Supports TGA format. Returns None on any error.
/// DIVERGED: C++ uses async emImageFileModel plugin system.
/// This synchronous loader serves small theme images only.
pub fn load_image_from_file(path: &Path) -> Option<emImage>
```

Implementation: `std::fs::read(path)` → `load_tga(&bytes)`. Falls back to
None on read error or decode error. Theme border images are all TGA files
in Eagle Mode.

This function is used by:
- `ImageFileRec::GetImage()` in emFileManTheme
- Icon loading in emFileManModel CommandNode

---

## Section 5: Testing Strategy

### New tests per file

| File | New Tests |
|------|-----------|
| emDirModel | Acquire returns same instance for same path; FileModelOps wiring (reset/load cycle); FileModelState trait access |
| emFileLinkModel | Acquire; GetFormatName; GetFullPath delegation through model wrapper |
| emFileManConfig | Acquire singleton; getter/setter round-trip through emConfigModel; GetChangeSignal |
| emFileManTheme | Acquire by theme name; ImageFileRec lazy load (mock file or test TGA); field access through model |
| emFileManThemeNames | Acquire singleton; filesystem discovery against C++ theme directory; generation counter on rescan |
| emFileManViewConfig | Acquire; all setters bump generation; autosave writes to config; CompareDirEntries method matches free function; IsUnsaved/SaveAsDefault |
| emFileManModel | Acquire singleton; selection ops bump generation; GetCommandRoot after UpdateCommands; RunCommand env var setup; IPC server name; SearchHotkeyCommand |
| emcore infra | PanelParentArg with_parent constructor; CreateFilePanel returns Box |

### Existing tests preserved

All 89 existing tests continue to pass. The data structs they test
(emDirModelData, SelectionManager, CommandNode, etc.) are unchanged — new
model wrappers compose them, they don't replace them.

---

## Section 6: Dependency Order

```
Phase 1 — emcore infrastructure (no emFileMan deps):
  1a. PanelParentArg extension
  1b. CreateFilePanel return type change
  1c. load_image_from_file utility

Phase 2 — Layer 0 models (no inter-emFileMan deps):
  2a. emFileManConfig (emConfigModel wrapper)
  2b. emFileManTheme (emConfigModel + ImageFileRec)
  2c. emFileManThemeNames (filesystem discovery + Acquire)
  2d. emFileLinkModel (emRecFileModel wrapper)

Phase 3 — Layer 1 models (depend on Layer 0):
  3a. emDirModel (emFileModel wrapper)
  3b. emFileManViewConfig (depends on Config + Theme + ThemeNames)
  3c. emFileManModel (depends on emDirEntry, uses IPC server)

Gate per phase: cargo clippy -- -D warnings && cargo-nextest ntr
```
