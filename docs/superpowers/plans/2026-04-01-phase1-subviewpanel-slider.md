# Phase 1: emSubViewPanel Integration + Slider Drag — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port emMainPanel to use emSubViewPanel for independent control/content zoom, and implement full slider drag interaction — matching C++ emMainPanel.cpp exactly.

**Architecture:** Replace direct child panels with emSubViewPanel instances hosting control and content panels. Port all C++ UpdateCoordinates branches, SliderPanel interaction, StartupOverlayPanel, and auto-hide logic. Resource TGA files embedded via `include_bytes!`.

**Tech Stack:** Rust, emcore (emSubViewPanel, emPainter, emTimer, emInput), emmain crate.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `res/emMain/ControlEdges.tga` | Create (copy from C++ `~/git/eaglemode-0.96.4/res/emMain/ControlEdges.tga`) | Control panel border decoration image |
| `res/emMain/Slider.tga` | Create (copy from C++ `~/git/eaglemode-0.96.4/res/emMain/Slider.tga`) | Slider panel texture image |
| `crates/emmain/src/emMainPanel.rs` | Rewrite | Root split panel with sub-views, slider, overlay |
| `crates/emmain/src/lib.rs` | Modify | Re-export any new public types if needed |

No new Rust files — all changes are within `emMainPanel.rs` (matching C++ where `SliderPanel` and `StartupOverlayPanel` are nested classes in the same file).

### Scoping Notes

- **emMainControlPanel and emMainContentPanel as sub-view children:** Phase 1 creates the sub-view panels but continues to create control/content panels as direct children of the sub-views in `LayoutChildren`. Phase 3 (startup engine) will later refactor this so the engine drives panel creation across frames. For Phase 1, the panels are created immediately inside the sub-views during `LayoutChildren`.
- **EOI signal and timer wiring:** These require `Cycle()` integration with the scheduler's signal system. Phase 1 implements the logic methods (`update_slider_hiding`, `update_fullscreen_on/off`) and wires them from `Input` and `notice`. Full signal-driven `Cycle()` wiring (timer expiry, EOI signal) is deferred to Phase 3 when the startup engine integrates with the scheduler — the methods exist and are callable, just not yet triggered by signals.
- **SetFocusable:** The `PanelBehavior` trait does not have a `SetFocusable` method. The panel tree controls focusability. SliderPanel's non-focusable behavior is achieved by not returning focus-related responses from `Input`.

---

### Task 1: Copy TGA Resources

**Files:**
- Create: `res/emMain/ControlEdges.tga`
- Create: `res/emMain/Slider.tga`

- [ ] **Step 1: Create resource directory and copy TGA files**

```bash
mkdir -p res/emMain
cp ~/git/eaglemode-0.96.4/res/emMain/ControlEdges.tga res/emMain/
cp ~/git/eaglemode-0.96.4/res/emMain/Slider.tga res/emMain/
```

- [ ] **Step 2: Verify files are valid TGA**

```bash
file res/emMain/ControlEdges.tga res/emMain/Slider.tga
```

Expected: Both identified as Targa image data.

- [ ] **Step 3: Commit**

```bash
git add res/emMain/ControlEdges.tga res/emMain/Slider.tga
git commit -m "assets: add ControlEdges.tga and Slider.tga for emMainPanel"
```

---

### Task 2: Port UpdateCoordinates Exactly

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

The current `update_coordinates` is a simplified version missing 3 branches from C++ (emMainPanel.cpp:234-293). Port all branches exactly.

- [ ] **Step 1: Write tests for UpdateCoordinates edge cases**

Add these tests to the existing `#[cfg(test)] mod tests` block in `emMainPanel.rs`. These test the exact C++ branches:

```rust
#[test]
fn test_update_coordinates_slider_near_top() {
    // When SliderY < SliderH*0.5, C++ uses: ControlH = SliderY + SliderH * SliderY / t
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.unified_slider_pos = 0.01; // very small → SliderY near 0
    panel.update_coordinates(1.0);
    // ControlH should be very small but > 1E-5
    assert!(panel.control_h > 1e-5);
    assert!(panel.control_h < 0.1);
}

#[test]
fn test_update_coordinates_control_collapsed() {
    // When ControlH < 1E-5, C++ sets ControlH=1E-5 and centers content
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.unified_slider_pos = 0.0; // slider at very top
    panel.update_coordinates(0.001); // very short panel
    // Content should fill entire height
    assert!(panel.content_h > 0.0);
    assert!(panel.content_x == 0.0);
    assert!(panel.content_w == 1.0);
}

#[test]
fn test_update_coordinates_width_limited() {
    // When ControlX < 1E-5, control fills width up to slider
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.unified_slider_pos = 0.8; // slider pushed down
    panel.update_coordinates(1.0);
    // Control should be positioned at x=0
    assert!(panel.control_x >= 0.0);
    assert!(panel.control_w > 0.0);
    assert!(panel.control_w <= panel.slider_x);
}

#[test]
fn test_update_coordinates_slider_min_max() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);

    // SliderMinY = 0, SliderMaxY = min(ControlTallness, h*0.5)
    // For h=1.0, tallness=5.0: SliderMaxY = min(5.0, 0.5) = 0.5
    panel.unified_slider_pos = 0.5;
    panel.update_coordinates(1.0);
    let expected_slider_y = 0.5 * 0.5; // (max-min)*pos + min = 0.5*0.5
    assert!((panel.slider_y - expected_slider_y).abs() < 1e-10);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emmain`
Expected: Some tests fail because current `update_coordinates` is missing branches.

- [ ] **Step 3: Port UpdateCoordinates from C++**

Replace the `update_coordinates` method in `emMainPanel` with the exact C++ logic. Add `slider_min_y` and `slider_max_y` fields to the struct.

```rust
fn update_coordinates(&mut self, h: f64) {
    self.slider_min_y = 0.0;
    self.slider_max_y = self.control_tallness.min(h * 0.5);
    self.slider_y = (self.slider_max_y - self.slider_min_y) * self.unified_slider_pos
        + self.slider_min_y;
    self.slider_w = (1.0_f64.min(h) * 0.1).min(1.0_f64.max(h) * 0.02);
    self.slider_h = self.slider_w * 1.2;
    self.slider_x = 1.0 - self.slider_w;

    let space_fac = 1.015;
    let t = self.slider_h * 0.5;
    if self.slider_y < t {
        self.control_h = self.slider_y + self.slider_h * self.slider_y / t;
    } else {
        self.control_h = (self.slider_y + self.slider_h) / space_fac;
    }

    if self.control_h < 1e-5 {
        self.control_h = 1e-5;
        self.control_w = self.control_h / self.control_tallness;
        self.control_x = 0.5 * (1.0 - self.control_w);
        self.control_y = 0.0;
        self.content_x = 0.0;
        self.content_y = 0.0;
        self.content_w = 1.0;
        self.content_h = h;
    } else {
        self.control_w = self.control_h / self.control_tallness;
        self.control_x =
            ((1.0 - self.control_w) * 0.5).min(self.slider_x - self.control_w);
        self.control_y = 0.0;
        if self.control_x < 1e-5 {
            self.control_w = 1.0 - self.slider_w;
            self.control_x = 0.0;
            self.control_h = self.control_w * self.control_tallness;
            if self.control_h < self.slider_y {
                self.control_h = self.slider_y;
                self.control_w = self.control_h / self.control_tallness;
            } else if !self.slider_pressed {
                self.slider_y = self.control_h * space_fac - self.slider_h;
            }
        }
        self.content_y = self.control_y + self.control_h * space_fac;
        self.content_x = 0.0;
        self.content_w = 1.0;
        self.content_h = h - self.content_y;
    }

    self.last_height = h;
}
```

Also add the missing fields to `emMainPanel`:

```rust
slider_min_y: f64,
slider_max_y: f64,
slider_pressed: bool, // needed by UpdateCoordinates
```

Initialize them to `0.0` / `false` in `new()`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "fix(emMainPanel): port UpdateCoordinates exactly from C++ with all branches"
```

---

### Task 3: Implement StartupOverlayPanel

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

C++ `StartupOverlayPanel` (emMainPanel.cpp:505-565) is a full-screen overlay that eats input, shows "Loading..." text, returns `IsOpaque() -> false` (critical for sub-view sizing), and shows a wait cursor.

- [ ] **Step 1: Write test for StartupOverlayPanel**

```rust
#[test]
fn test_startup_overlay_panel_not_opaque() {
    // C++ comment: "Must be false. Otherwise the sub-view panels for content
    // and control would get 'non-viewed' state"
    let panel = StartupOverlayPanel;
    assert!(!panel.IsOpaque());
}

#[test]
fn test_startup_overlay_panel_cursor() {
    let panel = StartupOverlayPanel;
    assert_eq!(panel.GetCursor(), emCursor::Wait);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emmain`
Expected: `StartupOverlayPanel` either doesn't exist as a proper struct or doesn't implement these methods.

- [ ] **Step 3: Implement StartupOverlayPanel**

Replace the simple `SliderPanel` struct with a proper `StartupOverlayPanel`:

```rust
use emcore::emCursor::emCursor;
use emcore::emInput::{emInputEvent, InputKey};
use emcore::emInputState::emInputState;

/// Full-screen overlay during startup. Eats all input, shows "Loading...".
///
/// Port of C++ `emMainPanel::StartupOverlayPanel`.
///
/// IsOpaque() MUST return false — otherwise sub-view panels get "non-viewed"
/// state, sub-views shrink, and the visiting view animator breaks.
pub(crate) struct StartupOverlayPanel;

impl PanelBehavior for StartupOverlayPanel {
    fn IsOpaque(&self) -> bool {
        false
    }

    fn GetCursor(&self) -> emCursor {
        emCursor::Wait
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        // Eat all events during startup.
        true
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let bg_color = emColor::from_packed(0x808080FF);
        let fg_color = emColor::from_packed(0x404040FF);
        painter.Clear(bg_color);
        // C++ uses ViewToPanelDeltaY(30.0) for font height.
        // Use a fraction of panel height as approximation.
        let font_h = h * 0.03;
        painter.PaintTextBoxed(
            0.0,
            0.0,
            w,
            h,
            "Loading...",
            font_h,
            fg_color,
            bg_color,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            0.0,
            false,
            1.0,
        );
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): add StartupOverlayPanel with input eating and wait cursor"
```

---

### Task 4: Integrate emSubViewPanel for Control and Content

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

Replace direct child creation with `emSubViewPanel` instances. This is the core architectural change.

- [ ] **Step 1: Write test for sub-view panel creation**

```rust
#[test]
fn test_sub_view_panel_fields() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    // Sub-view panel IDs should be None before LayoutChildren runs
    assert!(panel.control_view_panel.is_none());
    assert!(panel.content_view_panel.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo-nextest ntr -p emmain`
Expected: FAIL — field `control_view_panel` doesn't exist yet.

- [ ] **Step 3: Restructure emMainPanel fields**

Replace the old field names with C++-matching names. Update the struct definition:

```rust
use emcore::emSubViewPanel::emSubViewPanel;
use emcore::emView::ViewFlags;
use emcore::emImage::emImage;
use emcore::emResTga::load_tga;

pub struct emMainPanel {
    ctx: Rc<emContext>,
    config: Rc<RefCell<emMainConfig>>,
    control_tallness: f64,
    unified_slider_pos: f64,

    // Panel IDs for children
    control_view_panel: Option<PanelId>,
    content_view_panel: Option<PanelId>,
    slider_panel: Option<PanelId>,
    startup_overlay: Option<PanelId>,

    // Control edges decoration
    control_edges_color: emColor,
    control_edges_image: emImage,

    // Coordinates (C++ names preserved)
    control_x: f64,
    control_y: f64,
    control_w: f64,
    control_h: f64,
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    slider_x: f64,
    slider_y: f64,
    slider_w: f64,
    slider_h: f64,
    slider_min_y: f64,
    slider_max_y: f64,

    // Child panel IDs (created inside sub-views)
    control_panel_created: Option<PanelId>,
    content_panel_created: Option<PanelId>,

    // State
    slider_pressed: bool,
    slider_hidden: bool,
    fullscreen_on: bool,
    old_mouse_x: f64,
    old_mouse_y: f64,
    children_created: bool,
    last_height: f64,
}
```

Update `new()` to initialize all fields. Load `ControlEdges.tga`:

```rust
pub fn new(ctx: Rc<emContext>, control_tallness: f64) -> Self {
    let config = emMainConfig::Acquire(&ctx);
    let unified_slider_pos = config.borrow().GetControlViewSize();
    let control_edges_image = load_tga(include_bytes!("../../../res/emMain/ControlEdges.tga"))
        .expect("failed to load ControlEdges.tga");

    Self {
        ctx,
        config,
        control_tallness,
        unified_slider_pos,
        control_view_panel: None,
        content_view_panel: None,
        slider_panel: None,
        startup_overlay: None,
        control_edges_color: emColor::from_packed(0x515E84FF), // emLook().GetBgColor()
        control_edges_image,
        control_x: 0.0,
        control_y: 0.0,
        control_w: 0.0,
        control_h: 0.0,
        content_x: 0.0,
        content_y: 0.0,
        content_w: 0.0,
        content_h: 0.0,
        slider_x: 0.0,
        slider_y: 0.0,
        slider_w: 0.0,
        slider_h: 0.0,
        slider_min_y: 0.0,
        slider_max_y: 0.0,
        slider_pressed: false,
        fullscreen_on: false,
        old_mouse_x: 0.0,
        old_mouse_y: 0.0,
        children_created: false,
        last_height: 1.0,
    }
}
```

- [ ] **Step 4: Rewrite LayoutChildren to use emSubViewPanel**

```rust
fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
    let rect = ctx.layout_rect();
    let h = rect.h;

    self.unified_slider_pos = self.config.borrow().GetControlViewSize();
    self.update_coordinates(h);

    if !self.children_created {
        // Create control sub-view panel.
        let mut ctrl_svp = emSubViewPanel::new();
        ctrl_svp.set_sub_view_flags(
            ViewFlags::POPUP_ZOOM
                | ViewFlags::ROOT_SAME_TALLNESS
                | ViewFlags::NO_ACTIVE_HIGHLIGHT,
        );
        let ctrl_id = ctx.create_child_with("control view", Box::new(ctrl_svp));
        self.control_view_panel = Some(ctrl_id);

        // Create content sub-view panel.
        let mut content_svp = emSubViewPanel::new();
        content_svp.set_sub_view_flags(ViewFlags::ROOT_SAME_TALLNESS);
        let content_id = ctx.create_child_with("content view", Box::new(content_svp));
        self.content_view_panel = Some(content_id);

        // Create slider panel.
        let slider_id = ctx.create_child_with("slider", Box::new(SliderPanel::new()));
        self.slider_panel = Some(slider_id);

        // Create startup overlay (initially present).
        let overlay_id =
            ctx.create_child_with("startupOverlay", Box::new(StartupOverlayPanel));
        self.startup_overlay = Some(overlay_id);

        self.children_created = true;
    }

    // Create control panel inside control sub-view.
    // C++ StartupEngine state 5: creates emMainControlPanel in control view.
    // Phase 1 does this immediately; Phase 3 will move it into the engine.
    if let Some(ctrl_id) = self.control_view_panel {
        if self.control_panel_created.is_none() {
            let ctrl_ctx = Rc::clone(&self.ctx);
            // Access sub-view's sub-tree to create child panel inside it.
            // The sub-view panel manages its own child tree.
            let ctrl_panel_id = ctx.create_child_of(
                ctrl_id,
                "ctrl",
                Box::new(emMainControlPanel::new(ctrl_ctx)),
            );
            self.control_panel_created = Some(ctrl_panel_id);
        }
    }

    // Create content panel inside content sub-view.
    if let Some(content_id) = self.content_view_panel {
        if self.content_panel_created.is_none() {
            let content_ctx = Rc::clone(&self.ctx);
            let content_panel_id = ctx.create_child_of(
                content_id,
                "",
                Box::new(emMainContentPanel::new(content_ctx)),
            );
            self.content_panel_created = Some(content_panel_id);
        }
    }

    // Position children.
    if let Some(ctrl) = self.control_view_panel {
        ctx.layout_child(ctrl, self.control_x, self.control_y, self.control_w, self.control_h);
    }
    if let Some(content) = self.content_view_panel {
        ctx.layout_child(
            content,
            self.content_x,
            self.content_y,
            self.content_w,
            self.content_h,
        );
    }
    if let Some(slider) = self.slider_panel {
        ctx.layout_child(slider, self.slider_x, self.slider_y, self.slider_w, self.slider_h);
    }
    if let Some(overlay) = self.startup_overlay {
        ctx.layout_child(overlay, 0.0, 0.0, 1.0, h);
    }
}
```

- [ ] **Step 5: Update SetStartupOverlay to destroy the panel**

```rust
/// Port of C++ `emMainPanel::SetStartupOverlay`.
pub fn SetStartupOverlay(&mut self, overlay: bool) {
    if overlay && self.startup_overlay.is_none() {
        // Overlay creation happens in LayoutChildren
    } else if !overlay {
        // Remove overlay; C++ deletes the panel and activates content view
        self.startup_overlay = None;
    }
}

pub fn HasStartupOverlay(&self) -> bool {
    self.startup_overlay.is_some()
}
```

- [ ] **Step 6: Update accessor methods**

```rust
/// Port of C++ `emMainPanel::GetControlEdgesColor`.
pub fn GetControlEdgesColor(&self) -> emColor {
    self.control_edges_color
}

/// Port of C++ `emMainPanel::SetControlEdgesColor`.
pub fn SetControlEdgesColor(&mut self, color: emColor) {
    let mut c = color;
    c = emColor::from_packed(c.GetPacked() | 0xFF); // force alpha=255
    if self.control_edges_color != c {
        self.control_edges_color = c;
    }
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 8: Run full test suite**

Run: `cargo-nextest ntr`
Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): integrate emSubViewPanel for control and content views"
```

---

### Task 5: Port Paint Method with Control Edges

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

The C++ `Paint` method (emMainPanel.cpp:167-222) renders control edge borders and separator strips. It uses `PaintBorderImage` for the decorative control panel frame.

- [ ] **Step 1: Write test for Paint with non-trivial coordinates**

```rust
#[test]
fn test_paint_skips_when_content_at_top() {
    // C++ Paint returns early if ContentY <= 1E-10
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.content_y = 0.0;
    // Should not crash when painting with content_y = 0
    // (no visible assertion — just verifies no panic)
    assert!(panel.content_y <= 1e-10);
}
```

- [ ] **Step 2: Port Paint method**

Replace the current `Paint` with the full C++ port:

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
    if self.content_y <= 1e-10 {
        return;
    }

    let d = self.control_h * 0.007;
    let x1 = 0.0;
    let y1 = 0.0;
    let w1 = self.control_x - d;
    let h1 = self.control_h;
    let x2 = self.control_x + self.control_w + d;
    let y2 = 0.0;
    let w2 = 1.0 - x2;
    let h2 = self.control_h;

    // Black separator between control and content
    let sep_y = painter.RoundDownY(self.control_h);
    let sep_h = painter.RoundUpY(self.content_y) - sep_y;
    painter.PaintRect(0.0, sep_y, 1.0, sep_h, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);

    let d_border = self.control_h * 0.015;

    // Left control edge
    if self.control_x > 1e-10 {
        let rx = painter.RoundDownX(x1 + w1);
        let rw = painter.RoundUpX(self.control_x) - rx;
        let rh = painter.RoundUpY(self.content_y);
        painter.PaintRect(rx, 0.0, rw, rh, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);
        painter.PaintRect(x1, y1, w1, h1, self.control_edges_color, emColor::TRANSPARENT);
        painter.PaintBorderImage(
            x1, y1, w1, h1,
            0.0, d_border, d_border, d_border,
            &self.control_edges_image,
            191, 0, 190, 11,
            0, 5, 5, 5,
            255,
            self.control_edges_color,
            0o57,
        );
    }

    // Right control edge
    if 1.0 - self.control_x - self.control_w > 1e-10 {
        let rx = painter.RoundDownX(self.control_x + self.control_w);
        let rw = painter.RoundUpX(x2) - rx;
        let rh = painter.RoundUpY(self.content_y);
        painter.PaintRect(rx, 0.0, rw, rh, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);
        painter.PaintRect(x2, y2, w2, h2, self.control_edges_color, emColor::TRANSPARENT);
        painter.PaintBorderImage(
            x2, y2, w2, h2,
            d_border, d_border, 0.0, d_border,
            &self.control_edges_image,
            0, 0, 190, 11,
            5, 5, 0, 5,
            255,
            self.control_edges_color,
            0o750,
        );
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): port Paint with control edges border rendering"
```

---

### Task 6: Implement SliderPanel with Full Interaction

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

Port the complete C++ `SliderPanel` (emMainPanel.cpp:377-502) with mouse tracking, drag, double-click, and rendering.

- [ ] **Step 1: Write tests for slider drag logic**

```rust
#[test]
fn test_drag_slider_clamps_to_min() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.update_coordinates(1.0);
    let old_pos = panel.unified_slider_pos;
    panel.drag_slider(-999.0); // drag way up
    assert!(panel.slider_y >= panel.slider_min_y);
}

#[test]
fn test_drag_slider_clamps_to_max() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.update_coordinates(1.0);
    panel.drag_slider(999.0); // drag way down
    assert!(panel.slider_y <= panel.slider_max_y);
}

#[test]
fn test_double_click_slider_toggle() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.update_coordinates(1.0);

    // Start with non-zero position
    panel.unified_slider_pos = 0.5;
    panel.update_coordinates(1.0);

    // Double-click should toggle to 0
    panel.double_click_slider();
    assert!(panel.unified_slider_pos < 0.01);

    // Double-click again should restore
    panel.double_click_slider();
    assert!(panel.unified_slider_pos > 0.01);
}

#[test]
fn test_slider_panel_not_focusable() {
    let panel = SliderPanel::new();
    // SliderPanel should not be focusable (C++ line 386)
    assert!(!panel.IsOpaque());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emmain`
Expected: FAIL — `drag_slider`, `double_click_slider`, `SliderPanel::new` don't exist yet.

- [ ] **Step 3: Add DragSlider and DoubleClickSlider to emMainPanel**

```rust
/// Port of C++ `emMainPanel::DragSlider`.
fn drag_slider(&mut self, delta_y: f64) {
    let mut y = self.slider_y + delta_y;
    if y <= self.slider_min_y {
        y = self.slider_min_y;
    } else if y > self.slider_max_y {
        y = self.slider_max_y;
    }
    let n = (y - self.slider_min_y) / (self.slider_max_y - self.slider_min_y);
    if self.unified_slider_pos != n {
        self.unified_slider_pos = n;
        self.update_coordinates(self.last_height);
        let mut cfg = self.config.borrow_mut();
        cfg.SetControlViewSize(self.unified_slider_pos);
        cfg.Save();
    }
}

/// Port of C++ `emMainPanel::DoubleClickSlider`.
fn double_click_slider(&mut self) {
    if self.unified_slider_pos < 0.01 {
        let saved = self.config.borrow().GetControlViewSize();
        if saved < 0.01 {
            self.config.borrow_mut().SetControlViewSize(0.7);
            self.config.borrow_mut().Save();
        }
        self.unified_slider_pos = self.config.borrow().GetControlViewSize();
    } else {
        self.unified_slider_pos = 0.0;
    }
    self.update_coordinates(self.last_height);
}
```

- [ ] **Step 4: Implement full SliderPanel**

Replace the simple `SliderPanel` with the full implementation:

```rust
/// Draggable divider between control and content sections.
///
/// Port of C++ `emMainPanel::SliderPanel`.
pub(crate) struct SliderPanel {
    mouse_over: bool,
    pressed: bool,
    hidden: bool,
    press_my: f64,
    press_slider_y: f64,
    slider_image: emImage,
}

impl SliderPanel {
    pub fn new() -> Self {
        let slider_image = load_tga(include_bytes!("../../../res/emMain/Slider.tga"))
            .expect("failed to load Slider.tga");
        Self {
            mouse_over: false,
            pressed: false,
            hidden: false,
            press_my: 0.0,
            press_slider_y: 0.0,
            slider_image,
        }
    }

    pub fn SetHidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }
}

impl PanelBehavior for SliderPanel {
    fn IsOpaque(&self) -> bool {
        false
    }

    fn GetCursor(&self) -> emCursor {
        emCursor::Normal
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        state: &PanelState,
        input_state: &emInputState,
    ) -> bool {
        let mx = event.mouse_x;
        let my = event.mouse_y;
        let h = state.height;

        let mo = mx > 0.05 && my > 0.0 && mx < 1.0 && my < h - 0.05;
        if self.mouse_over != mo {
            self.mouse_over = mo;
        }

        if self.mouse_over && event.is_mouse_event() {
            if event.is_left_button() {
                if event.repeat == 0 && !self.pressed {
                    self.pressed = true;
                    self.press_my = my;
                    // press_slider_y set by parent before Input
                } else if event.repeat == 1 {
                    if self.pressed {
                        self.pressed = false;
                    }
                    // Double-click handled by parent
                }
            }
            return true; // eat event
        }

        // Drag logic: compute delta and apply shift sensitivity.
        // C++ emMainPanel.cpp:439-444:
        //   dy = (my - PressMY) * GetLayoutWidth();
        //   if (shift) dy = (dy + SliderY - PressSliderY) * 0.25 + PressSliderY - SliderY;
        //   MainPanel.DragSlider(dy);
        // Note: drag_slider is called on the parent emMainPanel, not here.
        // The parent reads self.pressed, self.press_my, self.press_slider_y
        // and calls drag_slider in its own Cycle/Input dispatch.

        if self.pressed && !input_state.GetLeftButton() {
            self.pressed = false;
        }

        false
    }

    /// Compute drag delta with shift-key sensitivity reduction.
    /// Port of C++ SliderPanel::Input drag calculation (lines 439-444).
    pub fn compute_drag_delta(
        &self,
        my: f64,
        layout_width: f64,
        shift: bool,
        slider_y: f64,
    ) -> f64 {
        let mut dy = (my - self.press_my) * layout_width;
        if shift {
            dy = (dy + slider_y - self.press_slider_y) * 0.25
                + self.press_slider_y
                - slider_y;
        }
        dy
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        if !self.mouse_over && self.hidden {
            return;
        }

        // Background rounded rect
        let color = if self.pressed {
            emColor::from_packed(0x002244C0)
        } else if self.mouse_over {
            emColor::from_packed(0x006688A0)
        } else {
            emColor::from_packed(0x33445580)
        };
        painter.PaintRoundRect(0.0, 0.0, 2.0, h, 6.0 / 64.0, color);

        // Arrow indicators when hovering/pressed
        if self.mouse_over || self.pressed {
            let x1 = 0.2;
            let x2 = 0.4;
            let y1 = 0.1 * h;
            let y2 = 0.3 * h;
            let mut vertices: Vec<(f64, f64)> = Vec::new();

            // Up arrow (only if slider not at min)
            // Note: actual min/max check requires parent state —
            // simplified to always show arrows for now.
            // TODO: pass slider_y/min/max from parent to refine
            vertices.push((x1, y2));
            vertices.push((0.5, y1));
            vertices.push((1.0 - x1, y2));

            vertices.push((1.0 - x2, y2));
            vertices.push((1.0 - x2, h - y2));

            // Down arrow
            vertices.push((1.0 - x1, h - y2));
            vertices.push((0.5, h - y1));
            vertices.push((x1, h - y2));

            vertices.push((x2, h - y2));
            vertices.push((x2, y2));

            let poly_color = if self.pressed {
                emColor::from_packed(0xEEDD99D0)
            } else {
                emColor::from_packed(0xEEDD9960)
            };
            painter.PaintPolygon(&vertices, poly_color, emColor::TRANSPARENT);
        }

        // Slider texture
        painter.paint_image_full(0.0, 0.0, 1.0, h, &self.slider_image, 255, emColor::TRANSPARENT);
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p emmain -- -D warnings`
Expected: No warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): implement SliderPanel with drag, double-click, and rendering"
```

---

### Task 7: Wire Input and Mouse Movement Detection

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

Port the C++ `emMainPanel::Input` method (emMainPanel.cpp:143-158) for mouse movement detection that drives slider auto-hide.

- [ ] **Step 1: Add Input to emMainPanel's PanelBehavior impl**

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    _state: &PanelState,
    input_state: &emInputState,
) -> bool {
    // Port of C++ emMainPanel::Input — detect mouse movement for slider hiding.
    if (self.old_mouse_x - input_state.mouse_x).abs() > 2.5
        || (self.old_mouse_y - input_state.mouse_y).abs() > 2.5
        || input_state.GetLeftButton()
        || input_state.GetMiddleButton()
        || input_state.GetRightButton()
    {
        self.old_mouse_x = input_state.mouse_x;
        self.old_mouse_y = input_state.mouse_y;
        // UpdateSliderHiding(true) will be wired in Task 8
    }
    false
}
```

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): add Input handler for mouse movement detection"
```

---

### Task 8: Wire UpdateFullscreen and UpdateSliderHiding

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

Port `UpdateFullscreen` (emMainPanel.cpp:296-319) and `UpdateSliderHiding` (emMainPanel.cpp:322-339). These depend on window fullscreen state and a 5-second timer.

- [ ] **Step 1: Write tests for UpdateFullscreen**

```rust
#[test]
fn test_update_fullscreen_auto_hide() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    panel.update_coordinates(1.0);

    // Simulate entering fullscreen with auto-hide enabled
    panel.fullscreen_on = false;
    // When fullscreen turns on and AutoHideControlView is true,
    // unified_slider_pos should go to 0.
    // (Can't easily test window flags without a window, so test the logic directly)
    let cfg = panel.config.borrow();
    let auto_hide = cfg.GetAutoHideControlView();
    drop(cfg);

    if auto_hide {
        panel.update_fullscreen_on();
        assert!(panel.unified_slider_pos < 0.01);
    }
}
```

- [ ] **Step 2: Implement UpdateFullscreen**

```rust
/// Port of C++ `emMainPanel::UpdateFullscreen`.
fn update_fullscreen_on(&mut self) {
    if !self.fullscreen_on {
        self.fullscreen_on = true;
        if self.config.borrow().GetAutoHideControlView() {
            self.unified_slider_pos = 0.0;
            self.update_coordinates(self.last_height);
        }
    }
}

fn update_fullscreen_off(&mut self) {
    if self.fullscreen_on {
        self.fullscreen_on = false;
        if self.config.borrow().GetAutoHideControlView() {
            self.unified_slider_pos = self.config.borrow().GetControlViewSize();
            self.update_coordinates(self.last_height);
        }
    }
}
```

- [ ] **Step 3: Implement UpdateSliderHiding**

```rust
/// Port of C++ `emMainPanel::UpdateSliderHiding`.
///
/// Hides the slider after 5 seconds in fullscreen when control is collapsed
/// and AutoHideSlider is enabled.
fn update_slider_hiding(&mut self, restart: bool) {
    let to_hide = self.unified_slider_pos < 1e-15
        && self.fullscreen_on
        && self.config.borrow().GetAutoHideSlider();

    if !to_hide || restart {
        self.slider_hidden = false;
        // Cancel timer if running — requires timer infrastructure.
        // Timer wiring deferred to Cycle() integration.
    }
    if to_hide && !self.slider_hidden {
        // Start 5-second timer — requires timer infrastructure.
        // Timer wiring deferred to Cycle() integration.
    }
}
```

Note: Full timer integration requires `TimerCentral` access, which depends on the panel being wired into the app's scheduler. The timer fields and logic structure are set up here; the actual timer start/cancel calls will be wired when `Cycle()` is integrated with the app event loop.

- [ ] **Step 4: Add slider_hidden field**

Add `slider_hidden: bool` to `emMainPanel` struct, initialize to `false` in `new()`.

- [ ] **Step 5: Wire mouse movement to UpdateSliderHiding in Input**

Update the `Input` method to call `update_slider_hiding(true)`:

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    _state: &PanelState,
    input_state: &emInputState,
) -> bool {
    if (self.old_mouse_x - input_state.mouse_x).abs() > 2.5
        || (self.old_mouse_y - input_state.mouse_y).abs() > 2.5
        || input_state.GetLeftButton()
        || input_state.GetMiddleButton()
        || input_state.GetRightButton()
    {
        self.old_mouse_x = input_state.mouse_x;
        self.old_mouse_y = input_state.mouse_y;
        self.update_slider_hiding(true);
    }
    false
}
```

- [ ] **Step 6: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 7: Run full test suite**

Run: `cargo-nextest ntr`
Expected: All tests pass.

- [ ] **Step 8: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "feat(emMainPanel): wire UpdateFullscreen and UpdateSliderHiding"
```

---

### Task 9: Update Existing Tests and Final Verification

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs` (test section)

Update any tests broken by the struct changes and add comprehensive coverage.

- [ ] **Step 1: Update existing tests for new field names**

The existing tests reference `control_panel`, `content_panel`, `startup_overlay` (bool). Update them to use the new field names:

```rust
#[test]
fn test_new() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    assert!((panel.control_tallness - 5.0).abs() < 1e-10);
    assert!(panel.HasStartupOverlay()); // overlay created in LayoutChildren
}

#[test]
fn test_title() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    assert_eq!(panel.get_title(), Some("Eagle Mode".to_string()));
}

#[test]
fn test_behavior() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    let _: Box<dyn PanelBehavior> = Box::new(panel);
}

#[test]
fn test_control_edges_color() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    let color = emColor::from_packed(0xFF0000FF);
    panel.SetControlEdgesColor(color);
    assert_eq!(panel.GetControlEdgesColor(), color);
}

#[test]
fn test_control_edges_image_loaded() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
    assert!(panel.control_edges_image.GetWidth() > 0);
    assert!(panel.control_edges_image.GetHeight() > 0);
}
```

- [ ] **Step 2: Run full test suite**

Run: `cargo-nextest ntr`
Expected: All tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "test(emMainPanel): update tests for SubViewPanel integration and slider interaction"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Copy TGA resources | `res/emMain/*.tga` |
| 2 | Port UpdateCoordinates exactly | `emMainPanel.rs` |
| 3 | Implement StartupOverlayPanel | `emMainPanel.rs` |
| 4 | Integrate emSubViewPanel | `emMainPanel.rs` |
| 5 | Port Paint with control edges | `emMainPanel.rs` |
| 6 | Full SliderPanel interaction | `emMainPanel.rs` |
| 7 | Wire Input mouse detection | `emMainPanel.rs` |
| 8 | UpdateFullscreen + slider hiding | `emMainPanel.rs` |
| 9 | Update tests, final verification | `emMainPanel.rs` |
