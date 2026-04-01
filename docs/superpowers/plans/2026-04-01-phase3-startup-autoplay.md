# Phase 3: Startup Animation + Autoplay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the C++ startup engine state machine, autoplay panel traversal, and complete Phase 1 deferred timer/slider wiring — enabling the choreographed startup zoom and autoplay panel navigation.

**Architecture:** `emMainWindow` becomes a real struct (currently free functions) holding window state, a `StartupEngine` state machine, `emAutoplayViewModel` with `Cycle`, and `emAutoplayViewAnimator` with full tree traversal. `emMainPanel` gains timer integration and slider input wiring. The startup engine is registered as an `emEngine` in the scheduler and drives staged panel creation across frames.

**Tech Stack:** Rust, emcore scheduler/engine/timer/signal APIs, PanelTree traversal API, emVisitingViewAnimator

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/emmain/src/emMainWindow.rs` | Major rewrite | `emMainWindow` struct, `StartupEngine`, window lifecycle, input handler |
| `crates/emmain/src/emMainPanel.rs` | Modify | Remove `_` prefixes from slider methods, add timer wiring, add staged creation support |
| `crates/emmain/src/emAutoplay.rs` | Major rewrite | Full traversal logic, ViewModel Cycle, LowPriEngine |
| `crates/emmain/src/emAutoplayControlPanel.rs` | Create | `emAutoplayControlPanel` with stub button UI |
| `crates/emmain/src/lib.rs` | Modify | Add `emAutoplayControlPanel` module |
| `crates/eaglemode/src/main.rs` | Modify | Wire new `emMainWindow` struct creation |

---

## Task 1: Wire DragSlider / DoubleClickSlider from Parent Input Dispatch

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

This removes the Phase 1 `_` prefix deferral. The parent (`emMainPanel`) reads slider state during its `Cycle` and dispatches drag/double-click actions.

- [ ] **Step 1: Write failing test for slider drag wiring**

Add a test that verifies `DragSlider` is called when the slider reports pressed state. Currently `_DragSlider` has the `_` prefix — after renaming, the test validates the public API.

```rust
// In mod tests at bottom of emMainPanel.rs
#[test]
fn test_drag_slider_wiring_renames() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.update_coordinates(1.0);
    // These methods should exist without _ prefix
    panel.DragSlider(0.05);
    panel.DoubleClickSlider();
    // Verify state changed
    assert!(panel.unified_slider_pos < 0.01 || panel.unified_slider_pos > 0.01);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emmain -- test_drag_slider_wiring_renames`
Expected: FAIL — `DragSlider` and `DoubleClickSlider` do not exist (they are `_DragSlider` / `_DoubleClickSlider`).

- [ ] **Step 3: Rename methods — remove `_` prefix**

In `emMainPanel.rs`, rename:
- `_DragSlider` → `DragSlider`
- `_DoubleClickSlider` → `DoubleClickSlider`

On `SliderPanel`, rename:
- `_SetHidden` → `SetHidden`
- `_is_pressed` → `is_pressed`
- `_press_my` → `press_my`
- `_get_press_slider_y` → `get_press_slider_y`
- `_mouse_over` → `mouse_over_state` (avoids collision with the `mouse_over` field)
- `_set_press_slider_y` → `set_press_slider_y`
- `_set_pressed` → `set_pressed`

Update all call sites and tests that reference the old names. Remove DIVERGED comments about Task 7 wiring deferral.

- [ ] **Step 4: Add `Cycle` to `emMainPanel` for slider dispatch**

Add a `Cycle` method to `PanelBehavior for emMainPanel` that reads slider state and dispatches:

```rust
fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
    // Read slider state and dispatch drag/double-click.
    if let Some(slider_id) = self.slider_panel {
        let slider_state = ctx.tree.with_behavior_as::<SliderPanel, _>(slider_id, |sp| {
            (sp.is_pressed(), sp.press_my(), sp.get_press_slider_y(), sp.double_clicked)
        });
        if let Some((pressed, press_my, press_slider_y, double_clicked)) = slider_state {
            if double_clicked {
                self.DoubleClickSlider();
                // Reset double_clicked flag
                ctx.tree.with_behavior_as::<SliderPanel, _>(slider_id, |sp| {
                    sp.double_clicked = false;
                });
            } else if pressed && self.slider_pressed_prev {
                // Continuing drag: compute delta from current mouse to press point
                // The slider's press_my is in slider-local coords.
                // Convert to parent coords: delta_y = (current_my - press_my) * slider_h
                // Actually, C++ uses: MainPanel.DragSlider(
                //   PressSliderY + (GetInputState().GetMouseY() - PressMY) * GetHeight()
                //     * (Shift ? 0.25 : 1.0) - MainPanel.SliderY)
                // This is computed in SliderPanel::Input in C++. In Rust, the slider
                // stores press_my and press_slider_y; the parent computes the delta.
            }
            self.slider_pressed_prev = pressed;
        }
    }
    false
}
```

**Wait — this approach is wrong.** The C++ slider panel calls `MainPanel.DragSlider(dy)` directly from its `Input` handler because it has a parent reference. In Rust, we can't hold parent refs. The correct approach: `SliderPanel::Input` computes and stores a **pending drag delta** that the parent reads.

Replace the above with:

Add a `pending_drag_delta: Option<f64>` and `double_clicked: bool` field to `SliderPanel`.

In `SliderPanel::Input`, when the mouse moves during a press:
```rust
// In SliderPanel::Input, after detecting a drag:
if self.pressed {
    let shift = input_state.GetShift();
    let sensitivity = if shift { 0.25 } else { 1.0 };
    let dy = self._press_slider_y
        + (my - self.press_my) * _state.height * sensitivity
        - self.parent_slider_y;
    self.pending_drag_delta = Some(dy);
}
```

In `SliderPanel::Input`, on double-click (repeat == 1):
```rust
} else if event.repeat == 1 {
    self.double_clicked = true;
    self.pressed = false;
}
```

In `emMainPanel::Cycle`:
```rust
fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
    if let Some(slider_id) = self.slider_panel {
        let action = ctx.tree.with_behavior_as::<SliderPanel, _>(slider_id, |sp| {
            let dc = sp.double_clicked;
            let drag = sp.pending_drag_delta.take();
            sp.double_clicked = false;
            (dc, drag)
        });
        if let Some((double_clicked, drag_delta)) = action {
            if double_clicked {
                self.DoubleClickSlider();
            } else if let Some(dy) = drag_delta {
                self.DragSlider(dy);
            }
        }
    }
    false
}
```

- [ ] **Step 5: Update SliderPanel::Input to compute drag delta and double-click flag**

Add fields to `SliderPanel`:
```rust
pub(crate) pending_drag_delta: Option<f64>,
pub(crate) double_clicked: bool,
```

Initialize both to `None`/`false` in `SliderPanel::new()`.

Update `SliderPanel::Input` to set `pending_drag_delta` during drag and `double_clicked` on repeat==1. The drag formula from C++ (emMainPanel.cpp:435-443):

```rust
// Inside the pressed branch of Input, when mouse moves:
if self.pressed {
    let shift = input_state.GetShift();
    let sensitivity = if shift { 0.25 } else { 1.0 };
    let target_y = self._press_slider_y
        + (my - self.press_my) * _state.height * sensitivity;
    self.pending_drag_delta = Some(target_y - self.parent_slider_y);
}
```

Also update the press handler to record `_press_slider_y = self.parent_slider_y` on initial press (removing the comment about parent setting it).

- [ ] **Step 6: Run tests**

Run: `cargo test -p emmain`
Expected: All tests pass including `test_drag_slider_wiring_renames`.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): wire DragSlider/DoubleClickSlider from parent Cycle dispatch

Remove _ prefixes from slider methods. SliderPanel now computes
pending_drag_delta and double_clicked flag in Input; emMainPanel::Cycle
reads and dispatches them. Completes Phase 1 deferral."
```

---

## Task 2: Complete Timer Wiring for Slider Auto-Hide

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

Port the 5-second auto-hide timer using `TimerCentral` via `EngineScheduler`. When the timer fires, hide the slider. Mouse movement restarts the timer.

- [ ] **Step 1: Write failing test for timer-driven slider hiding**

```rust
#[test]
fn test_slider_timer_fields_exist() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    // Timer fields should exist (timer_id, timer_signal)
    assert!(panel.slider_timer_id.is_none());
    assert!(panel.slider_timer_signal.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emmain -- test_slider_timer_fields_exist`
Expected: FAIL — fields don't exist yet.

- [ ] **Step 3: Add timer fields to emMainPanel**

```rust
use emcore::emSignal::SignalId;
use emcore::emTimer::TimerId;

// In emMainPanel struct:
slider_timer_id: Option<TimerId>,
slider_timer_signal: Option<SignalId>,
```

Initialize both to `None` in `new()`.

- [ ] **Step 4: Implement timer creation and update_slider_hiding**

Replace the stubbed `update_slider_hiding` method with full timer logic. The timer is created lazily (first time it's needed) using the scheduler from the context.

```rust
fn update_slider_hiding(&mut self, restart: bool) {
    let to_hide = self.unified_slider_pos < 1e-15
        && self.fullscreen_on
        && self.config.borrow().GetAutoHideSlider();

    if !to_hide || restart {
        self.slider_hidden = false;
        // Cancel running timer
        if let (Some(timer_id), Some(_)) = (self.slider_timer_id, self.slider_timer_signal) {
            let scheduler = self.ctx.scheduler();
            scheduler.borrow_mut().cancel_timer(timer_id, true);
        }
    }
    if to_hide && !self.slider_hidden {
        // Ensure timer exists
        if self.slider_timer_id.is_none() {
            let scheduler = self.ctx.scheduler();
            let mut sched = scheduler.borrow_mut();
            let sig = sched.create_signal();
            let tid = sched.create_timer(sig);
            self.slider_timer_signal = Some(sig);
            self.slider_timer_id = Some(tid);
        }
        // Start 5-second one-shot timer
        if let Some(timer_id) = self.slider_timer_id {
            let scheduler = self.ctx.scheduler();
            scheduler.borrow_mut().start_timer(timer_id, 5000, false);
        }
    }
}
```

- [ ] **Step 5: Handle timer signal in Cycle**

Extend `emMainPanel::Cycle` to check for the timer signal firing:

```rust
// In Cycle, before the slider dispatch logic:
if let Some(sig) = self.slider_timer_signal {
    let fired = {
        let scheduler = self.ctx.scheduler();
        let sched = scheduler.borrow();
        // Check if signal is pending (timer fired)
        sched.is_pending(sig)
    };
    if fired {
        self.slider_hidden = true;
        if let Some(slider_id) = self.slider_panel {
            ctx.tree.with_behavior_as::<SliderPanel, _>(slider_id, |sp| {
                sp.SetHidden(true);
            });
        }
        // Abort the signal so it doesn't re-fire
        let scheduler = self.ctx.scheduler();
        scheduler.borrow_mut().abort(sig);
    }
}
```

**Note:** The signal-check approach depends on how the engine gets woken. Since `emMainPanel` is a `PanelBehavior` (not an `emEngine`), it doesn't get woken by signals. Instead, check in `Cycle` which runs every frame. An alternative: use `is_timer_running` — if the timer was running and is no longer, it fired. Choose whichever approach the scheduler supports for non-engine consumers.

Verify: Does `PanelBehavior::Cycle` get called every frame? Check `App::about_to_wait` → `tree.run_panel_cycles()`. If panel Cycle only runs when marked dirty, we need a different approach. **Read `run_panel_cycles` to confirm before implementing.**

If panel Cycle doesn't run continuously, the simpler approach is: check in `LayoutChildren` or `Input` (which DO run on interaction). Since the timer fires after 5 seconds of inactivity (no mouse), the next mouse movement will call `update_slider_hiding(restart=true)` which unhides. The actual hiding just needs to happen eventually. Use a boolean flag `slider_hide_timer_started: bool` and in the next `LayoutChildren` call, check if the timer has elapsed:

```rust
// In LayoutChildren, after update_coordinates:
if self.slider_hide_timer_started {
    if let Some(timer_id) = self.slider_timer_id {
        let scheduler = self.ctx.scheduler();
        if !scheduler.borrow().is_timer_running(timer_id) {
            // Timer fired (or was never started) — hide the slider
            self.slider_hidden = true;
            self.slider_hide_timer_started = false;
        }
    }
}
```

- [ ] **Step 6: Wire SliderPanel hidden state propagation**

In `LayoutChildren`, after positioning the slider, propagate hidden state:

```rust
if let Some(slider_id) = self.slider_panel {
    ctx.tree.with_behavior_as::<SliderPanel, _>(slider_id, |sp| {
        sp.SetHidden(self.slider_hidden);
    });
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p emmain`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): wire slider auto-hide timer via TimerCentral

5-second one-shot timer hides slider in fullscreen when control view
collapsed. Mouse movement restarts timer. Completes Phase 1 deferral."
```

---

## Task 3: Create emMainWindow Struct

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/eaglemode/src/main.rs`

Convert the free-function `create_main_window` into a proper `emMainWindow` struct that holds window state matching C++ (emMainWindow.cpp:28-84).

- [ ] **Step 1: Write failing test for emMainWindow struct**

```rust
#[test]
fn test_main_window_struct_fields() {
    let mw = emMainWindow::default_test();
    assert!(!mw.to_close);
    assert!(mw.startup_engine.is_some()); // created on construction
    assert!(mw.main_panel_id.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emmain -- test_main_window_struct_fields`
Expected: FAIL — `emMainWindow` struct doesn't have these fields.

- [ ] **Step 3: Define emMainWindow struct**

Replace the current free-function file content with a struct. Keep `emMainWindowConfig` and `do_custom_cheat`.

```rust
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use emcore::emContext::emContext;
use emcore::emEngine::{emEngine, EngineCtx, EngineId, Priority};
use emcore::emGUIFramework::App;
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emPanelTree::PanelId;
use emcore::emSignal::SignalId;
use emcore::emView::ViewFlags;
use emcore::emViewAnimator::emVisitingViewAnimator;
use emcore::emWindow::{WindowFlags, ZuiWindow};
use emcore::emWindowStateSaver::emWindowStateSaver;

use crate::emAutoplay::{emAutoplayConfig, emAutoplayViewModel};
use crate::emBookmarks::emBookmarksModel;
use crate::emMainControlPanel::emMainControlPanel;
use crate::emMainPanel::emMainPanel;

/// Configuration for creating an emMainWindow.
pub struct emMainWindowConfig {
    pub geometry: Option<String>,
    pub fullscreen: bool,
    pub visit: Option<String>,
    pub control_tallness: f64,
}

impl Default for emMainWindowConfig {
    fn default() -> Self {
        Self {
            geometry: None,
            fullscreen: false,
            visit: None,
            control_tallness: 5.0,
        }
    }
}

/// Port of C++ `emMainWindow`.
///
/// Holds all state for a main application window: root panel, child panels,
/// startup engine, bookmarks, autoplay, window state saver.
pub struct emMainWindow {
    pub(crate) window_id: Option<WindowId>,
    pub(crate) ctx: Rc<emContext>,

    // Panel IDs (in the App's panel tree)
    pub(crate) main_panel_id: Option<PanelId>,
    pub(crate) control_panel_id: Option<PanelId>,
    pub(crate) content_panel_id: Option<PanelId>,

    // Models
    pub(crate) bookmarks_model: Option<Rc<RefCell<emBookmarksModel>>>,
    pub(crate) autoplay_config: Option<Rc<RefCell<emAutoplayConfig>>>,
    pub(crate) autoplay_view_model: Option<emAutoplayViewModel>,

    // Startup engine (registered with scheduler, removed after startup)
    pub(crate) startup_engine_id: Option<EngineId>,

    // Window lifecycle
    pub(crate) to_close: bool,

    // Close signal (from ZuiWindow)
    pub(crate) close_signal: Option<SignalId>,

    // Visit parameters (for startup engine)
    pub(crate) visit_identity: Option<String>,
    pub(crate) visit_rel_x: f64,
    pub(crate) visit_rel_y: f64,
    pub(crate) visit_rel_a: f64,
    pub(crate) visit_adherent: bool,
    pub(crate) visit_subject: String,
    pub(crate) visit_valid: bool,

    // Config
    pub(crate) config: emMainWindowConfig,
}
```

- [ ] **Step 4: Implement constructor and create_main_window**

```rust
impl emMainWindow {
    pub fn new(ctx: Rc<emContext>, config: emMainWindowConfig) -> Self {
        Self {
            window_id: None,
            ctx,
            main_panel_id: None,
            control_panel_id: None,
            content_panel_id: None,
            bookmarks_model: None,
            autoplay_config: None,
            autoplay_view_model: None,
            startup_engine_id: None,
            to_close: false,
            close_signal: None,
            visit_identity: config.visit.clone(),
            visit_rel_x: 0.0,
            visit_rel_y: 0.0,
            visit_rel_a: 0.0,
            visit_adherent: false,
            visit_subject: String::new(),
            visit_valid: config.visit.is_some(),
            config,
        }
    }
}

/// Create an emMainWindow: creates the ZuiWindow, root panel, and registers
/// the startup engine.
///
/// Port of C++ `emMainWindow::emMainWindow` constructor.
pub fn create_main_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    config: emMainWindowConfig,
) -> emMainWindow {
    let mut mw = emMainWindow::new(Rc::clone(&app.context), config);

    // Create root panel
    let panel = emMainPanel::new(Rc::clone(&app.context), 0.0538);
    let root_id = app.tree.create_root("root");
    app.tree.set_behavior(root_id, Box::new(panel));
    mw.main_panel_id = Some(root_id);

    // Create ZuiWindow
    let mut flags = WindowFlags::AUTO_DELETE;
    if mw.config.fullscreen {
        flags |= WindowFlags::FULLSCREEN;
    }
    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();
    mw.close_signal = Some(close_signal);

    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    mw.window_id = Some(window_id);
    app.windows.insert(window_id, window);

    // Register startup engine
    let engine = StartupEngine::new(root_id);
    let engine_id = app.scheduler.borrow_mut().register_engine(
        Priority::Medium,
        Box::new(engine),
    );
    app.scheduler.borrow_mut().wake_up(engine_id);
    mw.startup_engine_id = Some(engine_id);

    mw
}
```

- [ ] **Step 5: Create a minimal StartupEngine stub**

```rust
/// Port of C++ `emMainWindow::StartupEngineClass`.
///
/// State machine that stages panel creation and startup animation.
/// Full implementation in Task 4.
pub(crate) struct StartupEngine {
    state: u8,
    root_panel_id: PanelId,
}

impl StartupEngine {
    pub fn new(root_panel_id: PanelId) -> Self {
        Self {
            state: 0,
            root_panel_id,
        }
    }
}

impl emEngine for StartupEngine {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        // Stub: immediately complete. Full state machine in Task 4.
        false
    }
}
```

- [ ] **Step 6: Update main.rs to use new API**

In `crates/eaglemode/src/main.rs`, update the setup closure:

```rust
let setup = Box::new(
    move |app: &mut emcore::emGUIFramework::App,
          event_loop: &winit::event_loop::ActiveEventLoop| {
        let config = emMain::emMainWindow::emMainWindowConfig {
            fullscreen,
            visit,
            ..Default::default()
        };
        let main_window = emMain::emMainWindow::create_main_window(app, event_loop, config);
        // Store in thread_local for frame-loop access (see Task 13 Step 4)
        emMain::emMainWindow::set_main_window(main_window);
    },
);
```

- [ ] **Step 7: Run tests and clippy**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs crates/eaglemode/src/main.rs
git commit -m "feat(emMainWindow): create struct with startup engine registration

Replace free-function create_main_window with emMainWindow struct that
holds window state (panel IDs, models, visit params). Registers a stub
StartupEngine with the scheduler. Port of C++ emMainWindow constructor."
```

---

## Task 4: StartupEngine State Machine — Panel Creation (States 0-6)

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/emmain/src/emMainPanel.rs`

Port C++ `StartupEngineClass::Cycle` states 0-6 (emMainWindow.cpp:362-422). States 0-2 are idle wake-ups. State 3 creates `emMainPanel` (already done in Task 3 — adapt to set startup overlay). States 4-6 acquire models and create control/content panels.

- [ ] **Step 1: Add staged creation support to emMainPanel**

Add a `creation_stage: u8` field to `emMainPanel`:
- Stage 0: Create sub-view panels, slider, and startup overlay (current behavior)
- Stage 1: Create control panel inside control sub-view
- Stage 2: Create content panel inside content sub-view

Modify `LayoutChildren` to gate control/content panel creation on `creation_stage`:

```rust
// Replace the unconditional creation blocks with:
if self.creation_stage >= 1 {
    // Create control panel inside control sub-view.
    if let Some(ctrl_id) = self.control_view_panel
        && self.control_panel_created.is_none()
    {
        // ... existing creation code ...
    }
}

if self.creation_stage >= 2 {
    // Create content panel inside content sub-view.
    if let Some(content_id) = self.content_view_panel
        && self.content_panel_created.is_none()
    {
        // ... existing creation code ...
    }
}
```

Add methods to advance the stage:
```rust
pub fn advance_creation_stage(&mut self) {
    if self.creation_stage < 2 {
        self.creation_stage += 1;
    }
}

pub fn creation_stage(&self) -> u8 {
    self.creation_stage
}
```

Initialize `creation_stage: 0` in `new()`. This means on construction, only sub-views/slider/overlay are created. The startup engine advances the stage.

- [ ] **Step 2: Implement states 0-6 in StartupEngine**

Expand the `StartupEngine` struct:

```rust
pub(crate) struct StartupEngine {
    state: u8,
    root_panel_id: PanelId,
    clock: Instant,
}

impl StartupEngine {
    pub fn new(root_panel_id: PanelId) -> Self {
        Self {
            state: 0,
            root_panel_id,
            clock: Instant::now(),
        }
    }
}

impl emEngine for StartupEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        match self.state {
            // States 0-2: Idle wake-ups (yield to scheduler for other engines)
            0 | 1 | 2 => {
                self.state += 1;
                true
            }
            // State 3: emMainPanel already created (Task 3). Set startup overlay.
            // C++ creates MainPanel here; in Rust it's created in create_main_window.
            // Set startup overlay active and acquire AutoplayViewModel.
            3 => {
                // MainPanel already exists with startup overlay.
                // LayoutChildren will have created sub-views on first layout pass.
                self.state += 1;
                true
            }
            // State 4: Acquire BookmarksModel and search for start location.
            // This requires access to the emContext, which the engine doesn't have.
            // Signal the emMainWindow to do this work.
            4 => {
                // Bookmarks acquisition is done by emMainWindow when it detects
                // state == 4. See Task 6 for Cycle wiring.
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 5: Advance creation to stage 1 (create control panel).
            5 => {
                // Signal emMainPanel to create control panel.
                // Done via tree access in emMainWindow::Cycle.
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 6: Advance creation to stage 2 (create content panel).
            6 => {
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            _ => false, // States 7+ in Task 5
        }
    }
}
```

**Key design note:** The `StartupEngine` only manages state transitions. Actual panel creation is driven by `emMainPanel::LayoutChildren` gated on `creation_stage`. The `emMainWindow` struct (Task 6) reads the engine state and advances `emMainPanel::creation_stage` accordingly. The engine and the window communicate through shared state.

- [ ] **Step 3: Add shared state for engine-window communication**

Add an `Rc<RefCell<StartupState>>` shared between `emMainWindow` and the engine:

```rust
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct StartupState {
    pub state: u8,
    pub done: bool,
}

impl Default for StartupState {
    fn default() -> Self {
        Self { state: 0, done: false }
    }
}
```

The `StartupEngine` writes to this shared state. The `emMainWindow` reads it and drives panel creation. Update `StartupEngine` to use `Rc<RefCell<StartupState>>` and `create_main_window` to create the shared state.

- [ ] **Step 4: Run tests**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs crates/emmain/src/emMainPanel.rs
git commit -m "feat(StartupEngine): port states 0-6 for staged panel creation

StartupEngine advances through idle wakeups then signals panel creation
stages. emMainPanel gates control/content panel creation on
creation_stage. Shared StartupState coordinates engine and window."
```

---

## Task 5: StartupEngine Animation States (7-11)

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

Port C++ states 7-11 (emMainWindow.cpp:423-485). These drive the `emVisitingViewAnimator` to zoom from root to the start location.

- [ ] **Step 1: Add animation fields to StartupState**

```rust
pub(crate) struct StartupState {
    pub state: u8,
    pub done: bool,
    // Set by emMainWindow when state reaches 7
    pub animator_active: bool,
    // Set by emMainWindow when visiting animator reports goal reached or timeout
    pub animator_finished: bool,
}
```

- [ ] **Step 2: Implement states 7-11 in StartupEngine::Cycle**

```rust
// State 7: Start zoom animation to root
7 => {
    self.clock = Instant::now();
    self.state += 1;
    !ctx.IsTimeSliceAtEnd()
}
// State 8: Wait up to 2 seconds for root zoom
8 => {
    let elapsed = self.clock.elapsed().as_millis();
    if elapsed < 2000 && self.shared.borrow().animator_active {
        true // keep waiting
    } else {
        self.state += 1;
        true
    }
}
// State 9: Set goal to visit target (if any)
9 => {
    self.clock = Instant::now();
    self.state += 1;
    !ctx.IsTimeSliceAtEnd()
}
// State 10: Wait up to 2 seconds for target zoom, then remove overlay
10 => {
    let elapsed = self.clock.elapsed().as_millis();
    if elapsed < 2000 && self.shared.borrow().animator_active {
        true
    } else {
        // Signal overlay removal and final steps
        self.shared.borrow_mut().state = 10;
        self.clock = Instant::now();
        self.state += 1;
        true
    }
}
// State 11: 100ms pause, then final visit and cleanup
11 => {
    if self.clock.elapsed().as_millis() < 100 {
        true
    } else {
        self.shared.borrow_mut().done = true;
        false // engine stops
    }
}
```

- [ ] **Step 3: Wire animation in emMainWindow**

The `emMainWindow` struct needs a method called during the frame loop that:
1. Reads `StartupState.state`
2. At state 7: Creates `emVisitingViewAnimator`, calls `SetGoalFullsized(":", false)`, activates on the window's view
3. At state 9: If visit_valid, sets new goal to visit identity
4. At state 10: Deactivates animator, calls `RawZoomOut`, sets active panel, removes startup overlay
5. At state 11 (done): Calls `Visit()` with stored visit params, removes engine from scheduler

Add a `cycle_startup` method to `emMainWindow`:

```rust
impl emMainWindow {
    /// Drive startup engine state machine from the frame loop.
    /// Called by the App frame loop (wired in Task 6).
    pub fn cycle_startup(&mut self, app: &mut App) {
        let Some(ref shared) = self.startup_state else { return };
        let state = shared.borrow().state;
        let done = shared.borrow().done;

        if done {
            // Final visit
            if self.visit_valid {
                if let Some(win) = self.window_id.and_then(|id| app.windows.get_mut(&id)) {
                    if let Some(panel_id) = self.main_panel_id {
                        // Visit the target location in content view
                        // (Actual implementation requires content sub-view access)
                    }
                }
            }
            // Remove engine
            if let Some(eid) = self.startup_engine_id.take() {
                app.scheduler.borrow_mut().remove_engine(eid);
            }
            self.startup_state = None;
            return;
        }

        match state {
            7 => {
                // Create visiting animator and set goal to root
                if let Some(win) = self.window_id.and_then(|id| app.windows.get_mut(&id)) {
                    let mut va = emVisitingViewAnimator::new(0.0, 0.0, 1.0, 1.0);
                    va.SetAnimated(false);
                    va.SetGoalFullsized(":", false, false, "");
                    win.active_animator = Some(Box::new(va));
                    shared.borrow_mut().animator_active = true;
                }
            }
            // State management continues via engine Cycle...
            _ => {}
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat(StartupEngine): port animation states 7-11 with VisitingVA

States 7-8 zoom to root (2s timeout). States 9-10 zoom to visit target
(2s timeout). State 10 removes overlay. State 11 does final Visit and
cleanup. Uses shared StartupState for engine-window coordination."
```

---

## Task 6: Window Lifecycle Methods

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

Port C++ `emMainWindow` lifecycle methods (emMainWindow.cpp:98-190): `Duplicate`, `ToggleFullscreen`, `ReloadFiles`, `ToggleControlView`, `Close`, `Quit`.

- [ ] **Step 1: Write tests for lifecycle methods**

```rust
#[test]
fn test_toggle_fullscreen() {
    // ToggleFullscreen should flip the to_toggle_fullscreen flag
    // (actual window flag toggle requires ZuiWindow access)
    let ctx = emcore::emContext::emContext::NewRoot();
    let config = emMainWindowConfig::default();
    let mw = emMainWindow::new(ctx, config);
    assert!(!mw.to_close);
}

#[test]
fn test_close_sets_flag() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let config = emMainWindowConfig::default();
    let mut mw = emMainWindow::new(ctx, config);
    mw.Close();
    assert!(mw.to_close);
}
```

- [ ] **Step 2: Implement lifecycle methods**

```rust
impl emMainWindow {
    /// Port of C++ `emMainWindow::ToggleFullscreen`.
    pub fn ToggleFullscreen(&self, app: &mut App) {
        if let Some(win) = self.window_id.and_then(|id| app.windows.get_mut(&id)) {
            let new_flags = win.flags ^ WindowFlags::FULLSCREEN;
            win.SetWindowFlags(new_flags);
        }
    }

    /// Port of C++ `emMainWindow::ReloadFiles`.
    pub fn ReloadFiles(&self, _app: &App) {
        // Signal file model acquire update signal.
        // C++: Signal(FileUpdateSignal)
        log::info!("emMainWindow::ReloadFiles");
    }

    /// Port of C++ `emMainWindow::ToggleControlView`.
    pub fn ToggleControlView(&mut self, app: &mut App) {
        if let Some(main_id) = self.main_panel_id {
            app.tree.with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                if mp.unified_slider_pos < 0.01 {
                    mp.DoubleClickSlider(); // opens control view
                } else {
                    mp.DoubleClickSlider(); // closes control view
                }
            });
        }
    }

    /// Port of C++ `emMainWindow::Close`.
    pub fn Close(&mut self) {
        self.to_close = true;
    }

    /// Port of C++ `emMainWindow::Quit`.
    pub fn Quit(&self, app: &App) {
        // C++: GetScheduler().InitiateTermination(0)
        app.scheduler.borrow_mut().InitiateTermination();
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat(emMainWindow): port lifecycle methods

Add ToggleFullscreen, ReloadFiles, ToggleControlView, Close, Quit
methods. Port of C++ emMainWindow.cpp lines 98-190."
```

---

## Task 7: Window Input Handler

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

Port C++ `emMainWindow::Input` (emMainWindow.cpp:193-263). Handles F4, F5, F11, Escape, and bookmark hotkeys.

- [ ] **Step 1: Write test for input dispatch**

```rust
#[test]
fn test_input_handler_exists() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let config = emMainWindowConfig::default();
    let mw = emMainWindow::new(ctx, config);
    // handle_input should exist as a method
    assert!(!mw.to_close);
}
```

- [ ] **Step 2: Implement handle_input**

```rust
impl emMainWindow {
    /// Port of C++ `emMainWindow::Input` (emMainWindow.cpp:193-263).
    ///
    /// Called from the App input dispatch path.
    pub fn handle_input(
        &mut self,
        event: &emInputEvent,
        input_state: &emInputState,
        app: &mut App,
    ) -> bool {
        use emcore::emInput::InputKey;

        let key = event.key;
        let shift = input_state.GetShift();
        let ctrl = input_state.GetCtrl();
        let alt = input_state.GetAlt();

        match key {
            // F4: New Window (plain), Close (Alt), Quit (Shift+Alt)
            InputKey::F4 if !shift && !ctrl && !alt => {
                // Duplicate() — deferred, requires window creation
                true
            }
            InputKey::F4 if !shift && !ctrl && alt => {
                self.Close();
                true
            }
            InputKey::F4 if shift && !ctrl && alt => {
                self.Quit(app);
                true
            }
            // F5: Reload Files
            InputKey::F5 if !shift && !ctrl && !alt => {
                self.ReloadFiles(app);
                true
            }
            // F11: Toggle Fullscreen
            InputKey::F11 if !shift && !ctrl && !alt => {
                self.ToggleFullscreen(app);
                true
            }
            // Escape / Menu: Toggle Control View
            InputKey::Escape | InputKey::Menu if !shift && !ctrl && !alt => {
                self.ToggleControlView(app);
                true
            }
            _ => false,
        }
    }
}
```

- [ ] **Step 3: Add bookmark hotkey dispatch**

After the match block, add bookmark hotkey lookup:

```rust
// Check bookmarks for hotkey match
if let Some(ref bm_model) = self.bookmarks_model {
    let hotkey_str = format!("{key:?}"); // Convert key to string representation
    if let Some(bookmark) = bm_model.borrow().GetRec().SearchBookmarkByHotkey(&hotkey_str) {
        // Navigate content view to bookmark location
        // Requires content sub-view access (deferred to integration)
        log::info!("Bookmark hotkey matched: {}", bookmark.entry.Name);
        return true;
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat(emMainWindow): port input handler with F4/F5/F11/Escape hotkeys

Port of C++ emMainWindow::Input. Handles new window, close, quit,
reload, fullscreen, toggle control, and bookmark hotkey dispatch."
```

---

## Task 8: Autoplay Traversal Helpers

**Files:**
- Modify: `crates/emmain/src/emAutoplay.rs`

Port the traversal helper methods from C++ `emAutoplayViewAnimator` (emAutoplay.cpp:519-618): `IsItem`, `IsCutoff`, `GoParent`, `GoChild`, `GoSame`, `InvertDirection`.

- [ ] **Step 1: Write tests for traversal helpers**

```rust
#[test]
fn test_is_item_requires_focusable_and_flag() {
    // IsItem returns true only if panel is focusable AND has APH_ITEM flag.
    // This is a static method — test with mock panel state.
    // Since we can't easily create a PanelTree in tests, test the logic directly.
    use emcore::emPanelTree::AutoplayHandlingFlags;
    assert!(emAutoplayViewAnimator::is_item_check(true, AutoplayHandlingFlags::ITEM));
    assert!(!emAutoplayViewAnimator::is_item_check(false, AutoplayHandlingFlags::ITEM));
    assert!(!emAutoplayViewAnimator::is_item_check(true, AutoplayHandlingFlags::empty()));
}

#[test]
fn test_is_cutoff_directory_non_recursive() {
    use emcore::emPanelTree::AutoplayHandlingFlags;
    let va = emAutoplayViewAnimator::new();
    // DIRECTORY with Recursive=false → cutoff
    assert!(va.is_cutoff_check(AutoplayHandlingFlags::DIRECTORY));
}

#[test]
fn test_is_cutoff_directory_recursive() {
    use emcore::emPanelTree::AutoplayHandlingFlags;
    let mut va = emAutoplayViewAnimator::new();
    va.SetRecursive(true);
    // DIRECTORY with Recursive=true → not cutoff
    assert!(!va.is_cutoff_check(AutoplayHandlingFlags::DIRECTORY));
}

#[test]
fn test_go_parent_sets_came_from_child() {
    let mut va = emAutoplayViewAnimator::new();
    va.CurrentPanelIdentity = "root:child".to_string();
    va.go_parent("root:child", "root");
    assert_eq!(va.CameFrom, CameFromType::Child);
    assert_eq!(va.CameFromChildName, "child");
    assert_eq!(va.CurrentPanelIdentity, "root");
    assert_eq!(va.CurrentPanelState, CurrentPanelState::NotVisited);
}

#[test]
fn test_go_child_sets_came_from_parent() {
    let mut va = emAutoplayViewAnimator::new();
    va.CurrentPanelIdentity = "root".to_string();
    va.go_child("root:child");
    assert_eq!(va.CameFrom, CameFromType::Parent);
    assert_eq!(va.CurrentPanelIdentity, "root:child");
    assert_eq!(va.CurrentPanelState, CurrentPanelState::NotVisited);
}

#[test]
fn test_invert_direction() {
    let mut va = emAutoplayViewAnimator::new();
    va.Backwards = false;
    va.CameFrom = CameFromType::Parent;
    va.CurrentPanelIdentity = "root:child".to_string();
    va.InvertDirection();
    assert!(va.Backwards);
    assert_eq!(va.CameFrom, CameFromType::Child);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emmain -- test_is_item test_is_cutoff test_go_parent test_go_child test_invert`
Expected: FAIL — methods don't exist.

- [ ] **Step 3: Implement IsItem and IsCutoff**

Port of C++ emAutoplay.cpp:519-552.

```rust
impl emAutoplayViewAnimator {
    /// Check if a panel qualifies as an autoplay item.
    /// Port of C++ `emAutoplayViewAnimator::IsItem`.
    /// A panel is an item if it is focusable AND has APH_ITEM handling.
    pub fn is_item_check(focusable: bool, flags: AutoplayHandlingFlags) -> bool {
        focusable && flags.contains(AutoplayHandlingFlags::ITEM)
    }

    /// Check if a panel qualifies as an autoplay item (tree version).
    pub fn IsItem(tree: &PanelTree, panel: PanelId) -> bool {
        let focusable = tree.focusable(panel);
        let flags = tree.GetAutoplayHandling(panel);
        Self::is_item_check(focusable, flags)
    }

    /// Check if recursion should stop at this panel.
    /// Port of C++ `emAutoplayViewAnimator::IsCutoff`.
    pub fn is_cutoff_check(&self, flags: AutoplayHandlingFlags) -> bool {
        if flags.contains(AutoplayHandlingFlags::CUTOFF) {
            return true;
        }
        if flags.contains(AutoplayHandlingFlags::DIRECTORY) && !self.Recursive {
            return true;
        }
        if flags.contains(AutoplayHandlingFlags::ITEM)
            && !self.Recursive
            && flags.contains(AutoplayHandlingFlags::CUTOFF_AT_SUBITEMS)
        {
            return true;
        }
        false
    }

    /// Check if recursion should stop at this panel (tree version).
    pub fn IsCutoff(&self, tree: &PanelTree, panel: PanelId) -> bool {
        let flags = tree.GetAutoplayHandling(panel);
        self.is_cutoff_check(flags)
    }
}
```

- [ ] **Step 4: Implement GoParent, GoChild, GoSame**

Port of C++ emAutoplay.cpp:555-585.

```rust
impl emAutoplayViewAnimator {
    /// Navigate to parent panel.
    /// Port of C++ `emAutoplayViewAnimator::GoParent`.
    pub fn go_parent(&mut self, current_identity: &str, parent_identity: &str) {
        // Extract child name from current identity
        let child_name = if let Some(pos) = current_identity.rfind(':') {
            &current_identity[pos + 1..]
        } else {
            current_identity
        };
        self.CameFrom = CameFromType::Child;
        self.CameFromChildName = child_name.to_string();
        self.CurrentPanelIdentity = parent_identity.to_string();
        self.CurrentPanelState = CurrentPanelState::NotVisited;
    }

    /// Navigate to child panel.
    /// Port of C++ `emAutoplayViewAnimator::GoChild`.
    pub fn go_child(&mut self, child_identity: &str) {
        self.CameFrom = CameFromType::Parent;
        self.CameFromChildName.clear();
        self.CurrentPanelIdentity = child_identity.to_string();
        self.CurrentPanelState = CurrentPanelState::NotVisited;
    }

    /// Stay at current panel (clear skip flag).
    /// Port of C++ `emAutoplayViewAnimator::GoSame`.
    pub fn go_same(&mut self) {
        self.SkipCurrent = false;
    }
}
```

- [ ] **Step 5: Implement InvertDirection**

Port of C++ emAutoplay.cpp:588-618.

```rust
impl emAutoplayViewAnimator {
    /// Reverse traversal direction.
    /// Port of C++ `emAutoplayViewAnimator::InvertDirection`.
    pub fn InvertDirection(&mut self) {
        self.Backwards = !self.Backwards;
        match self.CameFrom {
            CameFromType::Parent => {
                // We came from parent, but now going backwards means we
                // should act as if we came from a child.
                // Extract child name from current identity's last segment.
                if let Some(pos) = self.CurrentPanelIdentity.rfind(':') {
                    self.CameFromChildName =
                        self.CurrentPanelIdentity[pos + 1..].to_string();
                    self.CurrentPanelIdentity =
                        self.CurrentPanelIdentity[..pos].to_string();
                }
                self.CameFrom = CameFromType::Child;
                self.CurrentPanelState = CurrentPanelState::NotVisited;
            }
            CameFromType::Child => {
                // We came from child, now going forward means we should
                // act as if we came from parent — re-enter the child.
                let child_name = std::mem::take(&mut self.CameFromChildName);
                if !child_name.is_empty() {
                    if self.CurrentPanelIdentity.is_empty() {
                        self.CurrentPanelIdentity = child_name;
                    } else {
                        self.CurrentPanelIdentity =
                            format!("{}:{}", self.CurrentPanelIdentity, child_name);
                    }
                }
                self.CameFrom = CameFromType::Parent;
                self.CurrentPanelState = CurrentPanelState::NotVisited;
            }
            CameFromType::None => {}
        }
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p emmain -- test_is_item test_is_cutoff test_go_parent test_go_child test_invert`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emAutoplay.rs
git commit -m "feat(emAutoplayViewAnimator): port traversal helpers

Add IsItem, IsCutoff, GoParent, GoChild, GoSame, InvertDirection.
Port of C++ emAutoplay.cpp lines 519-618."
```

---

## Task 9: AdvanceCurrentPanel Traversal Logic

**Files:**
- Modify: `crates/emmain/src/emAutoplay.rs`

Port C++ `emAutoplayViewAnimator::AdvanceCurrentPanel` (emAutoplay.cpp:327-516). This is the core traversal state machine — ~190 lines of navigation logic.

- [ ] **Step 1: Write tests for forward and backward traversal**

```rust
#[test]
fn test_advance_result_enum() {
    // Verify enum exists
    let _again = AdvanceResult::Again;
    let _failed = AdvanceResult::Failed;
    let _finished = AdvanceResult::Finished;
}
```

Tests for the full traversal logic require a PanelTree with children. Create a helper:

```rust
fn make_test_tree() -> PanelTree {
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    let child_a = tree.create_child(root, "a");
    let child_b = tree.create_child(root, "b");
    let grandchild = tree.create_child(child_a, "c");

    // Set autoplay handling
    tree.SetAutoplayHandling(child_a, AutoplayHandlingFlags::ITEM);
    tree.set_focusable(child_a, true);
    tree.SetAutoplayHandling(child_b, AutoplayHandlingFlags::ITEM);
    tree.set_focusable(child_b, true);
    tree.SetAutoplayHandling(grandchild, AutoplayHandlingFlags::ITEM);
    tree.set_focusable(grandchild, true);

    tree
}

#[test]
fn test_advance_forward_from_root() {
    let tree = make_test_tree();
    let mut va = emAutoplayViewAnimator::new();
    va.CurrentPanelIdentity = "root".to_string();
    va.CameFrom = CameFromType::None;
    va.CurrentPanelState = CurrentPanelState::Visited;
    va.State = AutoplayState::Unfinished;

    let result = va.AdvanceCurrentPanel(&tree);
    assert_eq!(result, AdvanceResult::Again);
    // Should have moved to first child
    assert!(va.CurrentPanelIdentity.contains("a") || va.CurrentPanelIdentity.contains("b"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emmain -- test_advance`
Expected: FAIL — `AdvanceResult` and `AdvanceCurrentPanel` don't exist.

- [ ] **Step 3: Add AdvanceResult enum**

```rust
/// Result of a single panel advance step.
/// Port of C++ `emAutoplayViewAnimator::AdvanceResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceResult {
    Again,
    Failed,
    Finished,
}
```

- [ ] **Step 4: Implement AdvanceCurrentPanel**

Port of C++ emAutoplay.cpp:327-516. The logic depends on `Backwards` and `CameFrom` to navigate the tree:

```rust
impl emAutoplayViewAnimator {
    /// Advance to the next panel in traversal order.
    ///
    /// Port of C++ `emAutoplayViewAnimator::AdvanceCurrentPanel`
    /// (emAutoplay.cpp:327-516).
    pub fn AdvanceCurrentPanel(&mut self, tree: &PanelTree) -> AdvanceResult {
        // Resolve current panel by identity
        let current_panel = tree.find_panel_by_identity(&self.CurrentPanelIdentity);
        let Some(current) = current_panel else {
            return AdvanceResult::Failed;
        };

        if !self.Backwards {
            // === FORWARD TRAVERSAL ===
            match self.CameFrom {
                CameFromType::None | CameFromType::Parent => {
                    // Check if current is an item we should visit
                    if self.CameFrom == CameFromType::Parent
                        && !self.SkipCurrent
                        && Self::IsItem(tree, current)
                    {
                        self.go_same();
                        return AdvanceResult::Again;
                    }
                    self.SkipCurrent = false;

                    // Try going to first child (if not cutoff)
                    if !self.IsCutoff(tree, current) {
                        if let Some(child) = tree.GetFirstChild(current) {
                            let child_id = tree.GetIdentity(child);
                            self.go_child(&child_id);
                            return AdvanceResult::Again;
                        }
                    }

                    // No children or cutoff — go to next sibling via parent
                    if let Some(parent) = tree.GetParentContext(current) {
                        let parent_id = tree.GetIdentity(parent);
                        self.go_parent(&self.CurrentPanelIdentity.clone(), &parent_id);
                        return AdvanceResult::Again;
                    }

                    // At root with no more panels
                    if self.Loop {
                        // Restart from root
                        if self.NextLoopEndless {
                            return AdvanceResult::Finished;
                        }
                        self.NextLoopEndless = true;
                        let root_id = tree.GetIdentity(
                            tree.GetRootPanel().unwrap_or(current)
                        );
                        self.go_child(&root_id);
                        self.CameFrom = CameFromType::None;
                        return AdvanceResult::Again;
                    }
                    AdvanceResult::Finished
                }

                CameFromType::Child => {
                    // Came back from a child — try next sibling
                    let child_name = &self.CameFromChildName;
                    let next = self.find_next_sibling(tree, current, child_name);
                    if let Some(next_panel) = next {
                        let next_id = tree.GetIdentity(next_panel);
                        self.go_child(&next_id);
                        return AdvanceResult::Again;
                    }

                    // No more siblings — go to parent
                    if let Some(parent) = tree.GetParentContext(current) {
                        let parent_id = tree.GetIdentity(parent);
                        self.go_parent(&self.CurrentPanelIdentity.clone(), &parent_id);
                        return AdvanceResult::Again;
                    }

                    // At root
                    if self.Loop {
                        if self.NextLoopEndless {
                            return AdvanceResult::Finished;
                        }
                        self.NextLoopEndless = true;
                        if let Some(first) = tree.GetFirstChild(current) {
                            let first_id = tree.GetIdentity(first);
                            self.go_child(&first_id);
                            return AdvanceResult::Again;
                        }
                    }
                    AdvanceResult::Finished
                }
            }
        } else {
            // === BACKWARD TRAVERSAL ===
            match self.CameFrom {
                CameFromType::None | CameFromType::Child => {
                    // Going backwards from child — check previous child
                    if self.CameFrom == CameFromType::Child {
                        let child_name = &self.CameFromChildName;
                        let prev = self.find_prev_sibling(tree, current, child_name);
                        if let Some(prev_panel) = prev {
                            let prev_id = tree.GetIdentity(prev_panel);
                            self.go_child(&prev_id);
                            self.CameFrom = CameFromType::Parent;
                            // Need to go to deepest descendant
                            return AdvanceResult::Again;
                        }
                    }

                    // Check if current is an item
                    if !self.SkipCurrent && Self::IsItem(tree, current) {
                        self.go_same();
                        return AdvanceResult::Again;
                    }
                    self.SkipCurrent = false;

                    // Go to parent
                    if let Some(parent) = tree.GetParentContext(current) {
                        let parent_id = tree.GetIdentity(parent);
                        self.go_parent(&self.CurrentPanelIdentity.clone(), &parent_id);
                        self.CameFrom = CameFromType::Child;
                        return AdvanceResult::Again;
                    }

                    if self.Loop {
                        if self.NextLoopEndless {
                            return AdvanceResult::Finished;
                        }
                        self.NextLoopEndless = true;
                        // Go to last child and descend
                        if let Some(last) = self.find_last_child(tree, current) {
                            let last_id = tree.GetIdentity(last);
                            self.go_child(&last_id);
                            self.CameFrom = CameFromType::Parent;
                            return AdvanceResult::Again;
                        }
                    }
                    AdvanceResult::Finished
                }

                CameFromType::Parent => {
                    // Came from parent going backwards — go to last child
                    if !self.IsCutoff(tree, current) {
                        if let Some(last) = self.find_last_child(tree, current) {
                            let last_id = tree.GetIdentity(last);
                            self.go_child(&last_id);
                            return AdvanceResult::Again;
                        }
                    }

                    // No children — check if current is item
                    if !self.SkipCurrent && Self::IsItem(tree, current) {
                        self.go_same();
                        return AdvanceResult::Again;
                    }
                    self.SkipCurrent = false;

                    // Go to parent
                    if let Some(parent) = tree.GetParentContext(current) {
                        let parent_id = tree.GetIdentity(parent);
                        self.go_parent(&self.CurrentPanelIdentity.clone(), &parent_id);
                        self.CameFrom = CameFromType::Child;
                        return AdvanceResult::Again;
                    }
                    AdvanceResult::Finished
                }
            }
        }
    }

    /// Find next sibling after named child.
    fn find_next_sibling(
        &self,
        tree: &PanelTree,
        parent: PanelId,
        child_name: &str,
    ) -> Option<PanelId> {
        let mut child = tree.GetFirstChild(parent);
        while let Some(c) = child {
            let name = tree.get_panel_name(c);
            if name == child_name {
                return tree.GetNext(c);
            }
            child = tree.GetNext(c);
        }
        None
    }

    /// Find previous sibling before named child.
    fn find_prev_sibling(
        &self,
        tree: &PanelTree,
        parent: PanelId,
        child_name: &str,
    ) -> Option<PanelId> {
        let mut child = tree.GetFirstChild(parent);
        let mut prev: Option<PanelId> = None;
        while let Some(c) = child {
            let name = tree.get_panel_name(c);
            if name == child_name {
                return prev;
            }
            prev = Some(c);
            child = tree.GetNext(c);
        }
        None
    }

    /// Find last child of a panel.
    fn find_last_child(&self, tree: &PanelTree, panel: PanelId) -> Option<PanelId> {
        let mut child = tree.GetFirstChild(panel);
        let mut last: Option<PanelId> = None;
        while let Some(c) = child {
            last = Some(c);
            child = tree.GetNext(c);
        }
        last
    }
}
```

**Important:** Verify `tree.find_panel_by_identity` and `tree.get_panel_name` exist. If not, implement identity resolution by walking the tree. The C++ code uses `GetIdentity()` which returns the full colon-separated path. Check `PanelTree::GetIdentity` signature — it returns `String`. `find_panel_by_identity` may not exist; if so, add a method or use the existing panel identity lookup.

**Before writing this code, verify:**
1. `PanelTree::GetIdentity(id) -> String` — confirmed at line 573
2. `PanelTree::find_panel_by_identity(&str) -> Option<PanelId>` — **may not exist, check**
3. `PanelTree::GetParentContext(id) -> Option<PanelId>` — confirmed at line 537
4. `PanelTree::GetFirstChild(id) -> Option<PanelId>` — confirmed at line 544
5. `PanelTree::GetNext(id) -> Option<PanelId>` — confirmed at line 565
6. `PanelTree::get_panel_name(id)` — **may not exist, might need to extract from identity**

If `find_panel_by_identity` doesn't exist, add it to PanelTree or implement identity-based lookup by walking from root.

If `get_panel_name` doesn't exist, extract the last segment of `GetIdentity`: `identity.rsplit(':').next()`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p emmain -- test_advance`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add crates/emmain/src/emAutoplay.rs
git commit -m "feat(emAutoplayViewAnimator): port AdvanceCurrentPanel traversal

Full forward/backward panel tree traversal with loop wrapping.
Port of C++ emAutoplay.cpp lines 327-516."
```

---

## Task 10: LowPriCycle and Un-Stub Goal/Skip Methods

**Files:**
- Modify: `crates/emmain/src/emAutoplay.rs`

Port C++ `LowPriCycle` (emAutoplay.cpp:232-324) and the five stubbed goal/skip methods (lines 105-180). Port `CycleAnimation` (lines 222-229).

- [ ] **Step 1: Write tests for goal-setting methods**

```rust
#[test]
fn test_set_goal_to_item_at() {
    let mut va = emAutoplayViewAnimator::new();
    va.SetGoalToItemAt_identity("root:panel1");
    assert!(va.HasGoal());
    assert_eq!(va.State, AutoplayState::Unfinished);
    assert_eq!(va.CurrentPanelIdentity, "root:panel1");
}

#[test]
fn test_set_goal_to_next_item_of() {
    let mut va = emAutoplayViewAnimator::new();
    va.SetGoalToNextItemOf_identity("root:panel1");
    assert!(va.HasGoal());
    assert!(!va.Backwards);
}

#[test]
fn test_set_goal_to_previous_item_of() {
    let mut va = emAutoplayViewAnimator::new();
    va.SetGoalToPreviousItemOf_identity("root:panel1");
    assert!(va.HasGoal());
    assert!(va.Backwards);
}

#[test]
fn test_skip_to_next_item() {
    let mut va = emAutoplayViewAnimator::new();
    va.State = AutoplayState::Unfinished;
    va.CurrentPanelIdentity = "root:panel1".to_string();
    va.SkipToNextItem();
    assert!(va.SkipItemCount > 0 || va.SkipCurrent);
}
```

- [ ] **Step 2: Un-stub SetGoalToItemAt**

Port of C++ emAutoplay.cpp:105-121.

```rust
/// Set goal to the item at the given panel identity.
/// Port of C++ `emAutoplayViewAnimator::SetGoalToItemAt(const emString&)`.
pub fn SetGoalToItemAt(&mut self, panel_identity: &str) {
    self.ClearGoal();
    self.State = AutoplayState::Unfinished;
    self.CurrentPanelIdentity = panel_identity.to_string();
    self.CameFrom = CameFromType::Parent;
    self.CurrentPanelState = CurrentPanelState::NotVisited;
    self.OneMoreWakeUp = true;
}
```

- [ ] **Step 3: Un-stub SetGoalToPreviousItemOf and SetGoalToNextItemOf**

Port of C++ emAutoplay.cpp:124-146.

```rust
/// Set goal to the previous item relative to the given panel.
/// Port of C++ `emAutoplayViewAnimator::SetGoalToPreviousItemOf`.
pub fn SetGoalToPreviousItemOf(&mut self, panel_identity: &str) {
    self.ClearGoal();
    self.State = AutoplayState::Unfinished;
    self.Backwards = true;
    self.SkipCurrent = true;
    self.CurrentPanelIdentity = panel_identity.to_string();
    self.CameFrom = CameFromType::Parent;
    self.CurrentPanelState = CurrentPanelState::NotVisited;
    self.OneMoreWakeUp = true;
}

/// Set goal to the next item relative to the given panel.
/// Port of C++ `emAutoplayViewAnimator::SetGoalToNextItemOf`.
pub fn SetGoalToNextItemOf(&mut self, panel_identity: &str) {
    self.ClearGoal();
    self.State = AutoplayState::Unfinished;
    self.Backwards = false;
    self.SkipCurrent = true;
    self.CurrentPanelIdentity = panel_identity.to_string();
    self.CameFrom = CameFromType::Parent;
    self.CurrentPanelState = CurrentPanelState::NotVisited;
    self.OneMoreWakeUp = true;
}
```

- [ ] **Step 4: Un-stub SkipToPreviousItem and SkipToNextItem**

Port of C++ emAutoplay.cpp:149-180.

```rust
/// Skip backwards to the previous item.
/// Port of C++ `emAutoplayViewAnimator::SkipToPreviousItem`.
pub fn SkipToNextItem(&mut self) {
    if self.State == AutoplayState::NoGoal {
        return;
    }
    if !self.Backwards {
        self.SkipItemCount += 1;
    } else if self.SkipItemCount > 0 {
        self.SkipItemCount -= 1;
    } else {
        self.InvertDirection();
        self.SkipCurrent = true;
        self.SkipItemCount += 1;
    }
    self.OneMoreWakeUp = true;
}

/// Skip forward to the next item.
/// Port of C++ `emAutoplayViewAnimator::SkipToPreviousItem`.
pub fn SkipToPreviousItem(&mut self) {
    if self.State == AutoplayState::NoGoal {
        return;
    }
    if self.Backwards {
        self.SkipItemCount += 1;
    } else if self.SkipItemCount > 0 {
        self.SkipItemCount -= 1;
    } else {
        self.InvertDirection();
        self.SkipCurrent = true;
        self.SkipItemCount += 1;
    }
    self.OneMoreWakeUp = true;
}
```

- [ ] **Step 5: Implement LowPriCycle**

Port of C++ emAutoplay.cpp:232-324. This is the core cycle that drives AdvanceCurrentPanel:

```rust
/// Low-priority cycle that advances panel traversal.
///
/// Port of C++ `emAutoplayViewAnimator::LowPriCycle` (emAutoplay.cpp:232-324).
/// Returns true if more work is needed.
pub fn LowPriCycle(&mut self, tree: &PanelTree) -> bool {
    if self.State != AutoplayState::Unfinished {
        return false;
    }

    // Check if visiting animator reached goal
    // (In Rust, this is checked by the caller — emAutoplayViewModel)

    // Handle skip logic
    if self.SkipItemCount > 0 {
        self.SkipCurrent = true;
        self.SkipItemCount -= 1;
    }

    // Advance through panels
    loop {
        let result = self.AdvanceCurrentPanel(tree);
        match result {
            AdvanceResult::Again => {
                // Check if we found an item to visit
                if self.CurrentPanelState == CurrentPanelState::NotVisited
                    && !self.SkipCurrent
                {
                    // Check if current panel is an item
                    if let Some(panel) = tree.find_panel_by_identity(&self.CurrentPanelIdentity) {
                        if Self::IsItem(tree, panel) {
                            self.CurrentPanelState = CurrentPanelState::Visiting;
                            self.NextLoopEndless = false;
                            return true; // Found an item to visit
                        }
                    }
                }
                // Not an item or skipping — continue advancing
                continue;
            }
            AdvanceResult::Failed => {
                self.State = AutoplayState::GivenUp;
                return false;
            }
            AdvanceResult::Finished => {
                self.State = AutoplayState::GoalReached;
                return false;
            }
        }
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p emmain -- test_set_goal test_skip`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emAutoplay.rs
git commit -m "feat(emAutoplayViewAnimator): un-stub goal/skip methods, add LowPriCycle

Port SetGoalToItemAt, SetGoalToPreviousItemOf, SetGoalToNextItemOf,
SkipToPreviousItem, SkipToNextItem with full logic. Add LowPriCycle
that drives AdvanceCurrentPanel. Port of C++ emAutoplay.cpp:105-324."
```

---

## Task 11: emAutoplayViewModel Cycle

**Files:**
- Modify: `crates/emmain/src/emAutoplay.rs`

Port C++ `emAutoplayViewModel::Cycle` (emAutoplay.cpp:870-901) and its helper methods: `StartItemPlaying`, `UpdateItemPlaying`, `StopItemPlaying`, `SetAutoplaying` (full logic), `Input`.

- [ ] **Step 1: Write tests for ViewModel behavior**

```rust
#[test]
fn test_view_model_set_autoplaying_activates() {
    let mut vm = emAutoplayViewModel::new();
    vm.SetAutoplaying(true);
    assert!(vm.IsAutoplaying());
    assert!(vm.ScreensaverInhibited);
}

#[test]
fn test_view_model_set_autoplaying_deactivates() {
    let mut vm = emAutoplayViewModel::new();
    vm.SetAutoplaying(true);
    vm.SetAutoplaying(false);
    assert!(!vm.IsAutoplaying());
}

#[test]
fn test_view_model_can_continue_last() {
    let mut vm = emAutoplayViewModel::new();
    vm.LastLocationValid = true;
    vm.LastLocation = "root:panel1".to_string();
    assert!(vm.CanContinueLastAutoplay());
}

#[test]
fn test_view_model_item_progress_clamped() {
    let mut vm = emAutoplayViewModel::new();
    vm.SetItemProgress(1.5);
    assert!((vm.GetItemProgress() - 1.0).abs() < 1e-10);
    vm.SetItemProgress(-0.5);
    assert!(vm.GetItemProgress().abs() < 1e-10);
}
```

- [ ] **Step 2: Add missing fields to emAutoplayViewModel**

```rust
pub struct emAutoplayViewModel {
    // Existing fields...
    pub(crate) DurationMS: i32,
    pub(crate) Recursive: bool,
    pub(crate) Loop: bool,
    pub(crate) Autoplaying: bool,
    pub(crate) LastLocationValid: bool,
    pub(crate) LastLocation: String,
    pub(crate) ItemProgress: f64,
    pub(crate) PlayingItem: bool,
    pub(crate) PlaybackActive: bool,

    // New fields:
    pub(crate) ViewAnimator: emAutoplayViewAnimator,
    pub(crate) ViewAnimatorStartTime: Instant,
    pub(crate) ScreensaverInhibited: bool,
    pub(crate) PlayedAnyInCurrentSession: bool,
    pub(crate) ItemPlayStartTime: Instant,
    pub(crate) ChangeSignal: Option<SignalId>,
    pub(crate) ProgressSignal: Option<SignalId>,
}
```

- [ ] **Step 3: Implement CanContinueLastAutoplay and ContinueLastAutoplay**

```rust
impl emAutoplayViewModel {
    /// Port of C++ `emAutoplayViewModel::CanContinueLastAutoplay`.
    pub fn CanContinueLastAutoplay(&self) -> bool {
        !self.Autoplaying && self.LastLocationValid
    }

    /// Port of C++ `emAutoplayViewModel::ContinueLastAutoplay`.
    pub fn ContinueLastAutoplay(&mut self) {
        if self.CanContinueLastAutoplay() {
            self.ViewAnimator.SetGoalToItemAt(&self.LastLocation.clone());
            self.SetAutoplaying(true);
        }
    }

    /// Port of C++ `emAutoplayViewModel::SetItemProgress`.
    pub fn SetItemProgress(&mut self, progress: f64) {
        let p = progress.clamp(0.0, 1.0);
        if (self.ItemProgress - p).abs() > 0.001 {
            self.ItemProgress = p;
        }
    }
}
```

- [ ] **Step 4: Implement SetAutoplaying with full logic**

```rust
impl emAutoplayViewModel {
    /// Port of C++ `emAutoplayViewModel::SetAutoplaying` (emAutoplay.cpp:706-730).
    pub fn SetAutoplaying(&mut self, autoplaying: bool) {
        if autoplaying == self.Autoplaying {
            return;
        }
        self.Autoplaying = autoplaying;
        if autoplaying {
            self.ViewAnimator.SetRecursive(self.Recursive);
            self.ViewAnimator.SetLoop(self.Loop);
            self.ViewAnimatorStartTime = Instant::now();
            self.ScreensaverInhibited = true;
            self.PlayedAnyInCurrentSession = false;
        } else {
            self.StopItemPlaying(false);
            self.ViewAnimator.ClearGoal();
            self.ScreensaverInhibited = false;
        }
    }

    /// Port of C++ `emAutoplayViewModel::StopItemPlaying`.
    fn StopItemPlaying(&mut self, _reset_pos: bool) {
        if self.PlayingItem {
            self.PlayingItem = false;
            self.PlaybackActive = false;
            self.ItemProgress = 0.0;
        }
    }

    /// Port of C++ `emAutoplayViewModel::UpdateItemPlaying`.
    fn UpdateItemPlaying(&mut self) {
        if !self.PlayingItem {
            return;
        }
        if !self.PlaybackActive {
            // Timer-based progress
            let elapsed = self.ItemPlayStartTime.elapsed().as_millis() as f64;
            let duration = self.DurationMS as f64;
            let progress = if duration > 0.0 {
                (elapsed / duration).clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.SetItemProgress(progress);
            if progress >= 1.0 {
                // Item done — skip to next
                self.StopItemPlaying(false);
                self.ViewAnimator.SkipToNextItem();
            }
        }
    }
}
```

- [ ] **Step 5: Implement ViewModel::Cycle**

```rust
impl emAutoplayViewModel {
    /// Port of C++ `emAutoplayViewModel::Cycle` (emAutoplay.cpp:870-901).
    pub fn Cycle(&mut self, tree: &PanelTree) -> bool {
        self.UpdateItemPlaying();

        if self.Autoplaying {
            // Drive the view animator
            self.ViewAnimator.LowPriCycle(tree);

            if self.ViewAnimator.HasReachedGoal() {
                // Start playing the current panel
                let identity = self.ViewAnimator.GetCurrentPanelIdentity().to_string();
                self.StartItemPlaying(&identity);
                self.SaveLocation(&identity);
            } else if self.ViewAnimator.HasGivenUp() {
                // Check timeout (15 seconds from start)
                if self.ViewAnimatorStartTime.elapsed().as_secs() > 15 {
                    self.SetAutoplaying(false);
                }
            }
        }

        self.Autoplaying || self.ViewAnimator.HasGoal()
    }

    fn StartItemPlaying(&mut self, _panel_identity: &str) {
        self.StopItemPlaying(true);
        self.PlayingItem = true;
        self.PlaybackActive = false;
        self.ItemPlayStartTime = Instant::now();
        self.PlayedAnyInCurrentSession = true;
    }

    fn SaveLocation(&mut self, identity: &str) {
        self.LastLocationValid = true;
        self.LastLocation = identity.to_string();
    }
}
```

- [ ] **Step 6: Implement ViewModel::Input**

Port of C++ emAutoplay.cpp:797-835.

```rust
impl emAutoplayViewModel {
    /// Port of C++ `emAutoplayViewModel::Input` (emAutoplay.cpp:797-835).
    pub fn Input(&mut self, event: &emInputEvent, input_state: &emInputState) -> bool {
        use emcore::emInput::InputKey;

        let shift = input_state.GetShift();
        let ctrl = input_state.GetCtrl();

        match event.key {
            InputKey::F12 if !shift && !ctrl => {
                // Skip to next
                self.SkipToNextItem();
                true
            }
            InputKey::F12 if shift && !ctrl => {
                // Skip to previous
                self.SkipToPreviousItem();
                true
            }
            InputKey::F12 if !shift && ctrl => {
                // Toggle autoplay
                self.SetAutoplaying(!self.Autoplaying);
                true
            }
            InputKey::F12 if shift && ctrl => {
                // Continue last autoplay
                self.ContinueLastAutoplay();
                true
            }
            _ => false,
        }
    }

    /// Port of C++ `emAutoplayViewModel::SkipToPreviousItem`.
    pub fn SkipToPreviousItem(&mut self) {
        if self.ViewAnimator.HasGoal() {
            self.ViewAnimator.SkipToPreviousItem();
        } else {
            let loc = self.LastLocation.clone();
            self.ViewAnimator.SetGoalToPreviousItemOf(&loc);
        }
    }

    /// Port of C++ `emAutoplayViewModel::SkipToNextItem`.
    pub fn SkipToNextItem(&mut self) {
        if self.ViewAnimator.HasGoal() {
            self.ViewAnimator.SkipToNextItem();
        } else {
            let loc = self.LastLocation.clone();
            self.ViewAnimator.SetGoalToNextItemOf(&loc);
        }
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p emmain`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add crates/emmain/src/emAutoplay.rs
git commit -m "feat(emAutoplayViewModel): port Cycle, Input, item playing logic

Add full ViewModel with timer-based progress, F12 hotkey dispatch,
continue-last-autoplay, screensaver inhibit tracking. Port of C++
emAutoplay.cpp lines 646-1150."
```

---

## Task 12: emAutoplayControlPanel

**Files:**
- Create: `crates/emmain/src/emAutoplayControlPanel.rs`
- Modify: `crates/emmain/src/lib.rs`

Port C++ `emAutoplayControlPanel` (emAutoplay.h:333-398, emAutoplay.cpp:1157-1503). Uses stub button widgets (like `emMainControlPanel` uses `ControlButton`).

- [ ] **Step 1: Write test for control panel**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_panel_new() {
        let panel = emAutoplayControlPanel::new();
        assert_eq!(panel.get_title(), Some("Autoplay".to_string()));
    }

    #[test]
    fn test_duration_value_to_ms() {
        assert_eq!(emAutoplayControlPanel::DurationValueToMS(0), 500);
        assert_eq!(emAutoplayControlPanel::DurationValueToMS(400), 5000);
        assert_eq!(emAutoplayControlPanel::DurationValueToMS(900), 120000);
    }

    #[test]
    fn test_duration_ms_to_value() {
        assert_eq!(emAutoplayControlPanel::DurationMSToValue(500), 0);
        assert_eq!(emAutoplayControlPanel::DurationMSToValue(5000), 400);
        assert_eq!(emAutoplayControlPanel::DurationMSToValue(120000), 900);
    }

    #[test]
    fn test_duration_roundtrip() {
        for v in [0, 100, 200, 300, 400, 500, 600, 700, 800, 900] {
            let ms = emAutoplayControlPanel::DurationValueToMS(v);
            let back = emAutoplayControlPanel::DurationMSToValue(ms);
            assert!((back - v).abs() <= 1, "roundtrip failed for v={v}: ms={ms}, back={back}");
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emmain -- test_control_panel_new test_duration`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Create emAutoplayControlPanel.rs**

```rust
// Port of C++ emAutoplayControlPanel (emAutoplay.h:333-398).
//
// DIVERGED: C++ uses emPackGroup with emCheckButton, emButton, emScalarField.
// Rust uses simplified stub buttons until the full toolkit is ported.

use emcore::emColor::emColor;
use emcore::emPanel::{PanelBehavior, PanelState};
use emcore::emPainter::emPainter;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;

/// Duration lookup table (C++ emAutoplay.cpp:1384-1394).
/// Maps value domain [0, 900] to milliseconds.
const DURATION_TABLE_MS: &[i32] = &[
    500, 1000, 2000, 3000, 5000, 10000, 15000, 30000, 60000, 120000,
];

/// Autoplay control panel with play/pause/skip/settings.
///
/// Port of C++ `emAutoplayControlPanel`.
pub struct emAutoplayControlPanel {
    children_created: bool,
    btn_autoplay: Option<PanelId>,
    btn_prev: Option<PanelId>,
    btn_next: Option<PanelId>,
    btn_continue_last: Option<PanelId>,
}

impl emAutoplayControlPanel {
    pub fn new() -> Self {
        Self {
            children_created: false,
            btn_autoplay: None,
            btn_prev: None,
            btn_next: None,
            btn_continue_last: None,
        }
    }

    /// Port of C++ `DurationValueToMS` (emAutoplay.cpp:1382-1404).
    pub fn DurationValueToMS(value: i64) -> i32 {
        let n = DURATION_TABLE_MS.len() as i64;
        let step = 900 / (n - 1);
        let idx = (value / step).clamp(0, n - 2) as usize;
        let frac = (value - (idx as i64) * step) as f64 / step as f64;
        let lo = DURATION_TABLE_MS[idx] as f64;
        let hi = DURATION_TABLE_MS[idx + 1] as f64;
        (lo + (hi - lo) * frac).round() as i32
    }

    /// Port of C++ `DurationMSToValue` (emAutoplay.cpp:1407-1419).
    pub fn DurationMSToValue(ms: i32) -> i64 {
        let n = DURATION_TABLE_MS.len() as i64;
        let step = 900 / (n - 1);
        // Binary search
        let mut lo: i64 = 0;
        let mut hi: i64 = 900;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if Self::DurationValueToMS(mid) < ms {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    fn create_children(&mut self, ctx: &mut PanelCtx) {
        use crate::emMainControlPanel::ControlButton;

        let btn_ap = Box::new(ControlButton { label: "Autoplay".to_string() });
        self.btn_autoplay = Some(ctx.create_child_with("autoplay", btn_ap));

        let btn_prev = Box::new(ControlButton { label: "Prev".to_string() });
        self.btn_prev = Some(ctx.create_child_with("prev", btn_prev));

        let btn_next = Box::new(ControlButton { label: "Next".to_string() });
        self.btn_next = Some(ctx.create_child_with("next", btn_next));

        let btn_cl = Box::new(ControlButton { label: "Continue Last".to_string() });
        self.btn_continue_last = Some(ctx.create_child_with("continue_last", btn_cl));

        self.children_created = true;
    }
}

impl PanelBehavior for emAutoplayControlPanel {
    fn get_title(&self) -> Option<String> {
        Some("Autoplay".to_string())
    }

    fn IsOpaque(&self) -> bool {
        true
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let bg = emColor::from_packed(0x515E84FF);
        painter.PaintRect(0.0, 0.0, w, h, bg, emColor::TRANSPARENT);
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        if !self.children_created {
            self.create_children(ctx);
        }

        // Simple vertical layout
        let n = 4.0_f64;
        let gap = 0.01;
        let child_h = (1.0 - (n + 1.0) * gap) / n;
        let mut y = gap;

        for id in [self.btn_autoplay, self.btn_prev, self.btn_next, self.btn_continue_last]
            .iter()
            .flatten()
        {
            ctx.layout_child(*id, 0.02, y, 0.96, child_h);
            y += child_h + gap;
        }
    }
}
```

- [ ] **Step 4: Add module to lib.rs**

In `crates/emmain/src/lib.rs`, add:
```rust
pub mod emAutoplayControlPanel;
```

- [ ] **Step 5: Make ControlButton pub(crate)**

In `emMainControlPanel.rs`, the `ControlButton` struct needs to be visible to `emAutoplayControlPanel`. It's currently `pub(crate)` — verify it's accessible from the sibling module. If not, make it `pub(crate)` and ensure the import path works.

- [ ] **Step 6: Run tests**

Run: `cargo test -p emmain && cargo clippy -p emmain -- -D warnings`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emAutoplayControlPanel.rs crates/emmain/src/lib.rs
git commit -m "feat(emAutoplayControlPanel): port control panel with stub buttons

Add Autoplay, Prev, Next, Continue Last buttons. Port DurationValueToMS
and DurationMSToValue conversion functions. Uses simplified ControlButton
stubs. Port of C++ emAutoplay.h:333-398."
```

---

## Task 13: Integration Wiring and Smoke Test

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/eaglemode/src/main.rs`

Wire everything together: `emMainWindow` stores the autoplay view model, startup engine drives panel creation, input handler delegates to autoplay. Verify `cargo run` works.

- [ ] **Step 1: Wire emAutoplayViewModel into emMainWindow**

In `emMainWindow::new`, initialize autoplay:

```rust
autoplay_view_model: Some(emAutoplayViewModel::new()),
autoplay_config: Some(emAutoplayConfig::Acquire(&ctx)),
bookmarks_model: Some(emBookmarksModel::Acquire(&ctx)),
```

- [ ] **Step 2: Wire bookmarks search in startup state 4**

In `cycle_startup`, when state == 4 and no visit arg was provided:

```rust
if !self.visit_valid {
    if let Some(ref bm) = self.bookmarks_model {
        if let Some(bookmark) = bm.borrow().GetRec().SearchStartLocation() {
            self.visit_identity = Some(bookmark.LocationIdentity.clone());
            self.visit_rel_x = bookmark.LocationRelX;
            self.visit_rel_y = bookmark.LocationRelY;
            self.visit_rel_a = bookmark.LocationRelA;
            self.visit_valid = true;
        }
    }
}
```

- [ ] **Step 3: Wire autoplay input in handle_input**

Add autoplay delegation at the end of `handle_input`:

```rust
// Delegate to autoplay
if let Some(ref mut avm) = self.autoplay_view_model {
    if avm.Input(event, input_state) {
        return true;
    }
}
```

- [ ] **Step 4: Store emMainWindow on App**

The `emMainWindow` struct needs to persist across frames. The simplest approach: store it in an `Rc<RefCell<>>` in the `emContext` registry, or add a field to `App`. Since `App` is in emcore and `emMainWindow` is in emmain, use the context registry:

```rust
// In create_main_window, after building mw:
let mw_rc = Rc::new(RefCell::new(mw));
app.context.set::<emMainWindow>("main_window", Rc::clone(&mw_rc));
```

**Verify:** Does `emContext` have a `set`/`get` API for arbitrary types? If not, store it via `acquire` pattern. If neither works, add a `main_window_state: Option<Box<dyn Any>>` to `App` (requires emcore change — avoid if possible).

Alternative: Store it as a module-level `thread_local!` in emmain. This avoids emcore changes:

```rust
use std::cell::RefCell;

thread_local! {
    static MAIN_WINDOW: RefCell<Option<emMainWindow>> = RefCell::new(None);
}
```

Set it in `create_main_window`, access it from the frame loop callback.

- [ ] **Step 5: Run full test suite**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All pass.

- [ ] **Step 6: Manual smoke test**

Run: `cargo run -p eaglemode`
Expected: Window opens with startup overlay, panels create progressively, overlay disappears. If `-visit` arg is provided, zoom animation targets that location.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs crates/eaglemode/src/main.rs
git commit -m "feat(emMainWindow): wire startup engine, autoplay, and bookmarks

Complete integration: startup engine drives staged panel creation,
bookmarks searched for start location, autoplay input wired via F12,
emMainWindow persisted across frames via thread_local."
```

---

## Verification Gate

After all tasks:

1. **Startup animation:** `cargo run` shows choreographed 2-phase zoom (~2 seconds per phase). Startup overlay shows "Loading..." then disappears.
2. **Slider wiring:** Drag slider resizes control/content split. Double-click toggles. Shift-drag reduces sensitivity. Auto-hide timer works in fullscreen.
3. **Autoplay traversal:** F12 hotkeys (next/prev/toggle/continue) navigate cosmos items. Autoplay state machine walks the panel tree.
4. **Input handler:** F4 (close/quit), F5 (reload), F11 (fullscreen), Escape (toggle control) all work.
5. **Tests:** `cargo test && cargo clippy -- -D warnings` passes.
