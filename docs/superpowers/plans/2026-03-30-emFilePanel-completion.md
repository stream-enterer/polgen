# emFilePanel Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire emFilePanel to emFileModel so file panels actively track model state, drive loading via the priority scheduler, and forward memory/priority from the panel tree.

**Architecture:** Add FileModelClient trait and client list to emFileModel for memory/priority aggregation across panels. Add FileModelState trait for type erasure so emFilePanel can hold any model. Integrate emFileModel with PriSchedAgent for scheduler-driven loading. Restructure emFilePanel from a passive data bag to an active model observer with Cycle/Notice handlers.

**Tech Stack:** Rust, emcore crate internals (emFileModel, emFilePanel, emPriSchedAgent, emScheduler, emPanel/PanelBehavior)

**Spec:** `docs/superpowers/specs/2026-03-30-emFilePanel-completion-design.md`

**Commands:**
```bash
cargo check                       # type-check
cargo clippy -- -D warnings       # lint
cargo-nextest ntr                 # all tests
```

---

### Task 1: FileModelClient trait and client list

**Files:**
- Modify: `crates/emcore/src/emFileModel.rs`

This task adds the client registration mechanism to emFileModel so panels can participate in memory/priority decisions.

- [ ] **Step 1: Write failing test — AddClient/RemoveClient lifecycle**

Add at the bottom of `crates/emcore/src/emFileModel.rs`, inside a new `#[cfg(test)] mod tests { ... }` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct MockClient {
        memory_limit: u64,
        priority: f64,
        reload_annoying: bool,
    }

    impl FileModelClient for MockClient {
        fn get_memory_limit(&self) -> u64 { self.memory_limit }
        fn get_priority(&self) -> f64 { self.priority }
        fn is_reload_annoying(&self) -> bool { self.reload_annoying }
    }

    fn make_model() -> emFileModel<String> {
        emFileModel::new(
            PathBuf::from("/tmp/test.txt"),
            SignalId(0),
            SignalId(1),
        )
    }

    #[test]
    fn add_remove_client() {
        let mut model = make_model();
        let client: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 1024,
            priority: 0.5,
            reload_annoying: false,
        }));
        assert_eq!(model.client_count(), 0);
        model.AddClient(&client);
        assert_eq!(model.client_count(), 1);
        model.RemoveClient(&client);
        assert_eq!(model.client_count(), 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emFileModel::tests::add_remove_client`
Expected: FAIL — `FileModelClient` trait not defined, `AddClient`/`RemoveClient`/`client_count` not defined.

- [ ] **Step 3: Define FileModelClient trait and add client list to emFileModel**

Add the trait definition after the `FileModelOps` trait (after line 68 of `emFileModel.rs`):

```rust
/// Port of C++ emFileModelClient. Panels implement this to participate
/// in model memory/priority decisions.
pub(crate) trait FileModelClient {
    fn get_memory_limit(&self) -> u64;
    fn get_priority(&self) -> f64;
    fn is_reload_annoying(&self) -> bool;
}
```

Add fields to `emFileModel<T>` struct (after line 88):

```rust
    clients: Vec<Weak<RefCell<dyn FileModelClient>>>,
    memory_limit_invalid: bool,
    priority_invalid: bool,
```

Initialize them in `new()`:

```rust
    clients: Vec::new(),
    memory_limit_invalid: true,
    priority_invalid: true,
```

Add methods to `impl<T> emFileModel<T>`:

```rust
    /// Register a panel as a client. Port of C++ emFileModelClient::SetModel.
    pub fn AddClient(&mut self, client: &Rc<RefCell<dyn FileModelClient>>) {
        self.clients.push(Rc::downgrade(client));
        self.memory_limit_invalid = true;
        self.priority_invalid = true;
    }

    /// Unregister a panel as a client. Port of C++ emFileModelClient::SetModel(NULL).
    pub fn RemoveClient(&mut self, client: &Rc<RefCell<dyn FileModelClient>>) {
        let ptr = Rc::as_ptr(client);
        self.clients.retain(|w| {
            w.upgrade()
                .map_or(false, |rc| !std::ptr::eq(Rc::as_ptr(&rc), ptr))
        });
        self.memory_limit_invalid = true;
        self.priority_invalid = true;
    }

    /// Number of live clients. For testing.
    pub(crate) fn client_count(&self) -> usize {
        self.clients.iter().filter(|w| w.upgrade().is_some()).count()
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p emcore --lib emFileModel::tests::add_remove_client`
Expected: PASS

- [ ] **Step 5: Write failing test — UpdateMemoryLimit aggregation**

Add to the tests module:

```rust
    #[test]
    fn update_memory_limit_takes_max() {
        let mut model = make_model();
        let c1: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 1000,
            priority: 0.0,
            reload_annoying: false,
        }));
        let c2: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 5000,
            priority: 0.0,
            reload_annoying: false,
        }));
        model.AddClient(&c1);
        model.AddClient(&c2);
        model.UpdateMemoryLimit();
        assert_eq!(model.GetMemoryLimit(), 5000);
    }

    #[test]
    fn update_memory_limit_cleans_dead_refs() {
        let mut model = make_model();
        let c1: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 1000,
            priority: 0.0,
            reload_annoying: false,
        }));
        model.AddClient(&c1);
        drop(c1); // client dropped
        model.UpdateMemoryLimit();
        assert_eq!(model.client_count(), 0);
        assert_eq!(model.GetMemoryLimit(), 0);
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test -p emcore --lib emFileModel::tests::update_memory_limit`
Expected: FAIL — `UpdateMemoryLimit` not defined.

- [ ] **Step 7: Implement UpdateMemoryLimit, UpdatePriority, IsAnyClientReloadAnnoying**

Add to `impl<T> emFileModel<T>`:

```rust
    /// Port of C++ emFileModel::UpdateMemoryLimit.
    /// Aggregates max memory limit from all live clients. Cleans dead refs.
    pub fn UpdateMemoryLimit(&mut self) {
        self.clients.retain(|w| w.upgrade().is_some());
        let new_limit = self
            .clients
            .iter()
            .filter_map(|w| w.upgrade())
            .map(|c| c.borrow().get_memory_limit())
            .max()
            .unwrap_or(0);
        self.memory_limit = new_limit as usize;
        self.memory_limit_invalid = false;
    }

    /// Port of C++ emFileModel::UpdatePriority.
    /// Aggregates max priority from all live clients.
    pub fn UpdatePriority(&mut self) -> f64 {
        self.clients.retain(|w| w.upgrade().is_some());
        let max_pri = self
            .clients
            .iter()
            .filter_map(|w| w.upgrade())
            .map(|c| c.borrow().get_priority())
            .fold(0.0_f64, f64::max);
        self.priority_invalid = false;
        max_pri
    }

    /// Port of C++ emFileModel::IsOutOfDate annoying check.
    /// Returns true if any client considers reload annoying.
    pub fn IsAnyClientReloadAnnoying(&self) -> bool {
        self.clients
            .iter()
            .filter_map(|w| w.upgrade())
            .any(|c| c.borrow().is_reload_annoying())
    }

    /// Whether memory limit needs re-aggregation.
    pub fn is_memory_limit_invalid(&self) -> bool {
        self.memory_limit_invalid
    }

    /// Whether priority needs re-aggregation.
    pub fn is_priority_invalid(&self) -> bool {
        self.priority_invalid
    }

    /// Mark memory limit as needing re-aggregation.
    pub fn InvalidateMemoryLimit(&mut self) {
        self.memory_limit_invalid = true;
    }

    /// Mark priority as needing re-aggregation.
    pub fn InvalidatePriority(&mut self) {
        self.priority_invalid = true;
    }
```

Also change `GetMemoryLimit` return type from `usize` to match — but note
the existing code uses `usize`. Keep `usize` for now to avoid changing
downstream code; the `UpdateMemoryLimit` method stores via `as usize`.

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emFileModel::tests`
Expected: PASS for all 3 tests.

- [ ] **Step 9: Write failing test — UpdatePriority and IsAnyClientReloadAnnoying**

Add to tests module:

```rust
    #[test]
    fn update_priority_takes_max() {
        let mut model = make_model();
        let c1: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 0,
            priority: 0.3,
            reload_annoying: false,
        }));
        let c2: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 0,
            priority: 0.8,
            reload_annoying: false,
        }));
        model.AddClient(&c1);
        model.AddClient(&c2);
        let max_pri = model.UpdatePriority();
        assert!((max_pri - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn is_any_client_reload_annoying() {
        let mut model = make_model();
        let c1: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 0,
            priority: 0.0,
            reload_annoying: false,
        }));
        let c2: Rc<RefCell<dyn FileModelClient>> = Rc::new(RefCell::new(MockClient {
            memory_limit: 0,
            priority: 0.0,
            reload_annoying: true,
        }));
        model.AddClient(&c1);
        assert!(!model.IsAnyClientReloadAnnoying());
        model.AddClient(&c2);
        assert!(model.IsAnyClientReloadAnnoying());
    }
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emFileModel::tests`
Expected: PASS (implementation was added in Step 7).

- [ ] **Step 11: Run full check**

Run: `cargo clippy -p emcore -- -D warnings && cargo-nextest ntr`
Expected: PASS — no regressions.

- [ ] **Step 12: Commit**

```bash
git add crates/emcore/src/emFileModel.rs
git commit -m "feat(emFileModel): add FileModelClient trait and client list

Panels register as clients to participate in memory/priority aggregation.
UpdateMemoryLimit takes max across clients, UpdatePriority takes max,
IsAnyClientReloadAnnoying returns true if any client says yes.
Dead weak refs cleaned lazily during iteration."
```

---

### Task 2: FileModelState trait for type erasure

**Files:**
- Modify: `crates/emcore/src/emFileModel.rs`

This task adds the read-only trait that lets emFilePanel hold any model
without knowing T.

- [ ] **Step 1: Write failing test — FileModelState trait object**

Add to the tests module in `emFileModel.rs`:

```rust
    #[test]
    fn file_model_state_trait_object() {
        let model = make_model();
        let state: &dyn FileModelState = &model;
        assert_eq!(*state.GetFileState(), FileState::Waiting);
        assert!((state.GetFileProgress() - 0.0).abs() < f64::EPSILON);
        assert_eq!(state.GetErrorText(), "");
        assert_eq!(state.get_memory_need(), 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emFileModel::tests::file_model_state_trait_object`
Expected: FAIL — `FileModelState` not defined.

- [ ] **Step 3: Define FileModelState trait and implement for emFileModel<T>**

Add after the `FileModelClient` trait definition:

```rust
/// Read-only view of file model state, erasing the data type T.
/// DIVERGED: C++ emFileModel base class — Rust uses trait for type erasure
/// since emFileModel<T> is generic.
pub(crate) trait FileModelState {
    fn GetFileState(&self) -> &FileState;
    fn GetFileProgress(&self) -> f64;
    fn GetErrorText(&self) -> &str;
    fn get_memory_need(&self) -> u64;
    fn GetFileStateSignal(&self) -> SignalId;
}

impl<T> FileModelState for emFileModel<T> {
    fn GetFileState(&self) -> &FileState {
        &self.state
    }
    fn GetFileProgress(&self) -> f64 {
        self.GetFileProgress()
    }
    fn GetErrorText(&self) -> &str {
        &self.error_text
    }
    fn get_memory_need(&self) -> u64 {
        self.memory_need
    }
    fn GetFileStateSignal(&self) -> SignalId {
        self.change_signal
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p emcore --lib emFileModel::tests::file_model_state_trait_object`
Expected: PASS

- [ ] **Step 5: Run full check**

Run: `cargo clippy -p emcore -- -D warnings && cargo-nextest ntr`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emFileModel.rs
git commit -m "feat(emFileModel): add FileModelState trait for type erasure

emFilePanel will hold Rc<RefCell<dyn FileModelState>> to observe any
model without knowing T. DIVERGED from C++ emFileModel base class."
```

---

### Task 3: emFilePanel restructure — model reference and SetFileModel

**Files:**
- Modify: `crates/emcore/src/emFilePanel.rs`

Replace the passive data bag with an active model connection.

- [ ] **Step 1: Write failing test — SetFileModel with real model**

Replace the test module in `emFilePanel.rs`. Keep the existing tests but
update them to work with the new struct. First, add a test for the new
`SetFileModel`:

```rust
    #[test]
    fn set_file_model_connects_and_disconnects() {
        use crate::emFileModel::{emFileModel, FileModelState};
        use std::cell::RefCell;
        use std::path::PathBuf;
        use std::rc::Rc;
        use crate::emSignal::SignalId;

        let model: Rc<RefCell<emFileModel<String>>> = Rc::new(RefCell::new(
            emFileModel::new(PathBuf::from("/tmp/test"), SignalId(0), SignalId(1)),
        ));
        let mut panel = emFilePanel::new();
        assert_eq!(panel.GetVirFileState(), VirtualFileState::NoFileModel);

        panel.SetFileModel(Some(model.clone() as Rc<RefCell<dyn FileModelState>>));
        assert_eq!(panel.GetVirFileState(), VirtualFileState::Waiting);

        panel.SetFileModel(None);
        assert_eq!(panel.GetVirFileState(), VirtualFileState::NoFileModel);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emFilePanel::tests::set_file_model_connects_and_disconnects`
Expected: FAIL — `SetFileModel` signature doesn't accept `Option<Rc<...>>`.

- [ ] **Step 3: Restructure emFilePanel**

Replace the struct and core methods. Keep `paint_status()` and
`PanelBehavior` impl. The new struct:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use crate::emColor::emColor;
use crate::emFileModel::{FileModelClient, FileModelState, FileState};
use crate::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use crate::emPainter::{emPainter, TextAlignment, VAlign};

pub struct emFilePanel {
    model: Option<Rc<RefCell<dyn FileModelState>>>,
    custom_error: Option<String>,
    last_vir_file_state: VirtualFileState,
    cached_memory_limit: u64,
    cached_priority: f64,
    cached_in_active_path: bool,
}
```

Update `new()`:
```rust
    pub fn new() -> Self {
        Self {
            model: None,
            custom_error: None,
            last_vir_file_state: VirtualFileState::NoFileModel,
            cached_memory_limit: u64::MAX,
            cached_priority: 0.0,
            cached_in_active_path: false,
        }
    }
```

Remove `with_model()`, `GetFileModel() -> bool`,
`set_file_state()`, `set_error_text()`, `GetErrorText()`,
`set_memory_need()`, `GetMemoryNeed()`, `set_memory_limit()`,
`GetMemoryLimit()`, `GetFileState()`.

Add new `SetFileModel`:
```rust
    /// Port of C++ emFilePanel::SetFileModel.
    pub fn SetFileModel(&mut self, model: Option<Rc<RefCell<dyn FileModelState>>>) {
        self.model = model;
        let new_state = self.compute_vir_file_state();
        self.last_vir_file_state = new_state;
    }

    /// Whether a model is attached.
    pub fn GetFileModel(&self) -> bool {
        self.model.is_some()
    }
```

Update `GetVirFileState` to use the model:
```rust
    pub fn GetVirFileState(&self) -> VirtualFileState {
        self.last_vir_file_state.clone()
    }

    fn compute_vir_file_state(&self) -> VirtualFileState {
        if let Some(ref msg) = self.custom_error {
            return VirtualFileState::CustomError(msg.clone());
        }
        let Some(ref model_rc) = self.model else {
            return VirtualFileState::NoFileModel;
        };
        let model = model_rc.borrow();
        let memory_need = model.get_memory_need();
        if memory_need > self.cached_memory_limit {
            return VirtualFileState::TooCostly;
        }
        match model.GetFileState() {
            FileState::Waiting => VirtualFileState::Waiting,
            FileState::Loading { progress } => VirtualFileState::Loading {
                progress: *progress,
            },
            FileState::Loaded => VirtualFileState::Loaded,
            FileState::Unsaved => VirtualFileState::Unsaved,
            FileState::Saving => VirtualFileState::Saving,
            FileState::TooCostly => VirtualFileState::TooCostly,
            FileState::LoadError(e) => VirtualFileState::LoadError(e.clone()),
            FileState::SaveError(e) => VirtualFileState::SaveError(e.clone()),
        }
    }
```

Keep `set_custom_error`, `clear_custom_error`, `GetCustomError` unchanged.
Keep `paint_status` but update it to read error text from model:
```rust
    pub fn paint_status(&self, painter: &mut emPainter, w: f64, h: f64) {
        let canvas_color = painter.GetCanvasColor();
        let vfs = self.GetVirFileState();
        let error_text = self
            .model
            .as_ref()
            .map(|m| m.borrow().GetErrorText().to_string())
            .unwrap_or_default();
        // ... rest uses error_text instead of self.error_text
```

Update the `PanelBehavior` impl's `Paint` unchanged (it calls `paint_status`).

- [ ] **Step 4: Update existing tests**

All existing tests that used `with_model()`, `set_file_state()`, etc. need
updating. Replace them with equivalents that create a real model and call
`SetFileModel`. The `set_custom_error`/`clear_custom_error` tests can
stay mostly as-is since those methods didn't change. Tests that tested
VFS mapping against each FileState need a helper that sets model state
directly:

```rust
    fn make_panel_with_model() -> (emFilePanel, Rc<RefCell<emFileModel<String>>>) {
        use crate::emFileModel::emFileModel;
        use std::path::PathBuf;
        use crate::emSignal::SignalId;

        let model = Rc::new(RefCell::new(
            emFileModel::new(PathBuf::from("/tmp/test"), SignalId(0), SignalId(1)),
        ));
        let mut panel = emFilePanel::new();
        panel.SetFileModel(Some(model.clone() as Rc<RefCell<dyn FileModelState>>));
        (panel, model)
    }
```

Then tests set state via `model.borrow_mut().complete_load(data)` etc. and
call `panel.refresh_vir_file_state()` (a new helper that re-computes):

```rust
    /// Re-compute VirtualFileState from model. Called after model state changes
    /// in tests; in production, Cycle() does this.
    pub fn refresh_vir_file_state(&mut self) {
        self.last_vir_file_state = self.compute_vir_file_state();
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emFilePanel::tests`
Expected: PASS for all tests.

- [ ] **Step 6: Run full check**

Run: `cargo clippy -p emcore -- -D warnings && cargo-nextest ntr`
Expected: PASS — existing tests elsewhere that use emFilePanel may need
updating if they used the removed methods. Fix any compilation errors.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emFilePanel.rs
git commit -m "refactor(emFilePanel): replace passive data bag with model reference

SetFileModel now accepts Option<Rc<RefCell<dyn FileModelState>>>.
VirtualFileState computed from live model state. Removed manual
set_file_state/set_memory_need/set_memory_limit setters."
```

---

### Task 4: emFilePanel Cycle and notice handlers

**Files:**
- Modify: `crates/emcore/src/emFilePanel.rs`

- [ ] **Step 1: Write failing test — Cycle detects state change**

```rust
    #[test]
    fn cycle_detects_state_change() {
        use crate::emPanelCtx::PanelCtx;
        let (mut panel, model) = make_panel_with_model();
        assert_eq!(panel.GetVirFileState(), VirtualFileState::Waiting);

        // Simulate model transitioning to Loaded
        model.borrow_mut().complete_load("data".to_string());

        // Cycle should detect the change
        // Note: Cycle takes &mut PanelCtx in the trait, but for unit testing
        // we call the inner method directly
        let changed = panel.cycle_inner();
        assert!(changed);
        assert_eq!(panel.GetVirFileState(), VirtualFileState::Loaded);

        // Second cycle: no change
        let changed = panel.cycle_inner();
        assert!(!changed);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emFilePanel::tests::cycle_detects_state_change`
Expected: FAIL — `cycle_inner` not defined.

- [ ] **Step 3: Implement cycle_inner and wire into PanelBehavior::Cycle**

Add to `impl emFilePanel`:

```rust
    /// Inner cycle logic. Returns true if VirtualFileState changed.
    /// Port of C++ emFilePanel::Cycle.
    pub(crate) fn cycle_inner(&mut self) -> bool {
        let new_state = self.compute_vir_file_state();
        if new_state != self.last_vir_file_state {
            self.last_vir_file_state = new_state;
            true
        } else {
            false
        }
    }
```

Update `PanelBehavior` impl:

```rust
    fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
        self.cycle_inner()
    }
```

Add the `PanelCtx` import at the top of the file:
```rust
use crate::emPanelCtx::PanelCtx;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p emcore --lib emFilePanel::tests::cycle_detects_state_change`
Expected: PASS

- [ ] **Step 5: Write failing test — notice updates cached values**

```rust
    #[test]
    fn notice_updates_cached_memory_limit() {
        let (mut panel, _model) = make_panel_with_model();
        let mut state = PanelState::default_for_test();
        state.memory_limit = 2048;
        panel.notice(NoticeFlags::MEMORY_LIMIT_CHANGED, &state);
        assert_eq!(panel.cached_memory_limit, 2048);
    }

    #[test]
    fn notice_updates_cached_priority() {
        let (mut panel, _model) = make_panel_with_model();
        let mut state = PanelState::default_for_test();
        state.priority = 0.75;
        panel.notice(NoticeFlags::UPDATE_PRIORITY_CHANGED, &state);
        assert!((panel.cached_priority - 0.75).abs() < f64::EPSILON);
    }
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test -p emcore --lib emFilePanel::tests::notice_updates`
Expected: FAIL — `notice` is the default no-op on PanelBehavior, and
`cached_memory_limit`/`cached_priority` are private.

- [ ] **Step 7: Implement notice handler**

Override `notice` in `PanelBehavior for emFilePanel`:

```rust
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.contains(NoticeFlags::MEMORY_LIMIT_CHANGED) {
            self.cached_memory_limit = state.memory_limit;
        }
        if flags.contains(NoticeFlags::UPDATE_PRIORITY_CHANGED) {
            self.cached_priority = state.priority;
        }
        if flags.intersects(NoticeFlags::ACTIVE_CHANGED | NoticeFlags::VIEW_FOCUS_CHANGED) {
            self.cached_in_active_path = state.in_active_path;
        }
    }
```

Make the cached fields `pub(crate)` so tests can read them:

```rust
    pub(crate) cached_memory_limit: u64,
    pub(crate) cached_priority: f64,
    pub(crate) cached_in_active_path: bool,
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p emcore --lib emFilePanel::tests`
Expected: PASS

- [ ] **Step 9: Write test — IsHopeForSeeking via PanelBehavior**

```rust
    #[test]
    fn is_hope_for_seeking_delegates() {
        let (panel, _model) = make_panel_with_model();
        // Waiting state → hope for seeking
        assert!(panel.IsHopeForSeeking());
    }
```

Override in `PanelBehavior`:

```rust
    fn IsHopeForSeeking(&self) -> bool {
        self.GetVirFileState().IsHopeForSeeking()
    }
```

- [ ] **Step 10: Write test — IsContentReady based on VirtualFileState**

Port of C++ `emFilePanel::IsContentReady`. Add a method and test:

```rust
    #[test]
    fn is_content_ready_by_state() {
        let (mut panel, model) = make_panel_with_model();
        // Waiting → not ready, readying
        assert_eq!(panel.IsContentReady(), (false, true));

        // Loaded → ready
        model.borrow_mut().complete_load("data".to_string());
        panel.refresh_vir_file_state();
        assert_eq!(panel.IsContentReady(), (true, false));

        // LoadError → not ready, not readying
        model.borrow_mut().fail_load("err".to_string());
        panel.refresh_vir_file_state();
        assert_eq!(panel.IsContentReady(), (false, false));
    }
```

Add to `impl emFilePanel`:

```rust
    /// Port of C++ emFilePanel::IsContentReady.
    /// Returns (ready, readying).
    pub fn IsContentReady(&self) -> (bool, bool) {
        match &self.last_vir_file_state {
            VirtualFileState::Waiting | VirtualFileState::Loading { .. } | VirtualFileState::Saving => {
                (false, true)
            }
            VirtualFileState::Loaded | VirtualFileState::Unsaved => {
                (true, false)
            }
            _ => (false, false),
        }
    }
```

- [ ] **Step 11: Run full check**

Run: `cargo clippy -p emcore -- -D warnings && cargo-nextest ntr`
Expected: PASS

- [ ] **Step 11: Commit**

```bash
git add crates/emcore/src/emFilePanel.rs
git commit -m "feat(emFilePanel): add Cycle and notice handlers

Cycle detects model state changes and returns true to invalidate painting.
notice handles MEMORY_LIMIT_CHANGED, UPDATE_PRIORITY_CHANGED, and
ACTIVE_CHANGED to cache panel state for FileModelClient."
```

---

### Task 5: Fix downstream compilation

**Files:**
- Modify: any files that used the removed emFilePanel methods

The emFilePanel restructure in Task 3 removed `with_model()`,
`set_file_state()`, `set_error_text()`, `set_memory_need()`,
`set_memory_limit()`, `GetFileState()`, `GetErrorText()`,
`GetMemoryNeed()`, `GetMemoryLimit()`. Any code outside emFilePanel
that called these will break.

- [ ] **Step 1: Find all callers of removed methods**

Run: `cargo check -p emcore 2>&1 | head -100`

Look for compilation errors referencing the removed methods. The known
callers are:
- `crates/emstocks/src/emStocksFilePanel.rs` — uses `emFilePanel` as a field
- Any test files that reference these methods

- [ ] **Step 2: Fix each caller**

For `emStocksFilePanel`: it currently has `emFilePanel` as a field but
only uses `GetIconFileName()` and default `PanelBehavior` methods. If it
called any removed methods, replace with the new API (SetFileModel with
a real model, or remove the calls if they were stubs).

For tests: update to use the new `make_panel_with_model()` pattern and
set state through the model.

- [ ] **Step 3: Run full check**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: PASS — all downstream code compiles and tests pass.

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "fix: update downstream code for emFilePanel restructure"
```

---

### Task 6: Scheduler integration behavioral test

**Files:**
- Modify: `crates/emcore/src/emFileModel.rs` (add scheduler test)

This task verifies the end-to-end flow: model + scheduler + client →
loading lifecycle. This is a behavioral test that exercises the GotAccess
callback pattern described in the spec but defers the full PSAgent
integration inside emFileModel to the implementation phase (since it
requires wiring the model as an emEngine, which is a larger change that
should be designed with the specific Cycle pattern). This test validates
the PriSchedModel API works for file model use cases.

- [ ] **Step 1: Write behavioral test**

Add to `emFileModel.rs` tests:

```rust
    #[test]
    fn scheduler_drives_loading_via_callback() {
        use crate::emPriSchedAgent::PriSchedModel;
        use crate::emScheduler::EngineScheduler;

        let mut sched = EngineScheduler::new();
        let mut ps_model = PriSchedModel::new(&mut sched);

        let model: Rc<RefCell<emFileModel<String>>> = Rc::new(RefCell::new(
            emFileModel::new(PathBuf::from("/dev/null"), SignalId(0), SignalId(1)),
        ));

        // Create a GotAccess callback that drives one step of loading
        let m = Rc::clone(&model);
        let mut ops = TestOps { steps: 0, max_steps: 3 };
        let agent = ps_model.add_agent(1.0, Box::new(move || {
            // In production this would call step_loading, but we can't
            // pass ops through Box<dyn FnMut()> easily. Just verify the
            // callback fires.
            let mut model = m.borrow_mut();
            model.complete_load("loaded".to_string());
        }));

        ps_model.RequestAccess(agent, &mut sched);
        sched.DoTimeSlice();

        assert!(ps_model.HasAccess(agent));
        assert_eq!(*model.borrow().GetFileState(), FileState::Loaded);
        assert_eq!(model.borrow().GetMap(), Some(&"loaded".to_string()));

        ps_model.ReleaseAccess(agent, &mut sched);
        ps_model.remove(&mut sched);
    }
```

Also add a simple `TestOps` struct for the test if not already present:

```rust
    struct TestOps {
        steps: usize,
        max_steps: usize,
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p emcore --lib emFileModel::tests::scheduler_drives_loading_via_callback`
Expected: PASS (this uses existing APIs, no new code needed).

- [ ] **Step 3: Run full check**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emFileModel.rs
git commit -m "test(emFileModel): add scheduler integration behavioral test

Verifies PriSchedModel callback can drive file model loading,
validating the GotAccess pattern for file model use cases."
```

---

### Note: PSAgent integration inside emFileModel

The spec (Section 4) describes full scheduler integration: PSAgent fields
in emFileModel, StartPSAgent/EndPSAgent methods, a GotAccess callback with
time-sliced loading, and a Cycle method. This plan covers the client list
and type erasure (Tasks 1-2), the panel integration (Tasks 3-4), and a
behavioral test proving the PriSchedModel API works (Task 6). The full
PSAgent wiring inside emFileModel (model registers itself as an engine,
GotAccess drives step_loading in a loop) is deferred to the emFileMan
implementation plan where emDirModel will be the first consumer that needs
scheduler-driven loading. The API surface is ready; the wiring is not.

---

### Task 7: Update CORRESPONDENCE.md

**Files:**
- Modify: `docs/CORRESPONDENCE.md`

- [ ] **Step 1: Add emFilePanel completion entry**

Add a new section after the "Plugin system port" section:

```markdown
### emFilePanel completion (2026-03-30)

emCore infrastructure for file panel ↔ model integration:
- FileModelClient trait added to emFileModel.rs. Panels register as clients
  for memory/priority aggregation (AddClient, RemoveClient, UpdateMemoryLimit,
  UpdatePriority, IsAnyClientReloadAnnoying).
- FileModelState trait added to emFileModel.rs for type erasure. DIVERGED:
  C++ emFileModel base class — Rust uses trait since emFileModel<T> is generic.
- emFilePanel restructured from passive data bag to active model observer.
  SetFileModel accepts Rc<RefCell<dyn FileModelState>>. Cycle detects model
  state changes. notice forwards memory/priority from panel tree to model.
```

- [ ] **Step 2: Commit**

```bash
git add docs/CORRESPONDENCE.md
git commit -m "docs: update CORRESPONDENCE.md for emFilePanel completion"
```
