# Panel-Local Coordinate Space Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Switch paint_panel_recursive from pixel-space to panel-local coordinate space, matching C++ emView::Paint() architecture.

**Architecture:** Add SetTransformation to emPainter (matching C++ method). Call it in paint_panel_recursive after SetClipping to set scale=ViewedWidth. Pass w=1.0, h=tallness to widget Paint. Pixel output is mathematically identical since widgets use w/h proportionally and the painter's scale converts back to pixels.

**Tech Stack:** Rust, emcore crate, golden tests for validation

---

### Task 1: Add SetTransformation to DrawOp enum

**Files:**
- Modify: `crates/emcore/src/emPainterDrawList.rs:14-26` (DrawOp enum)
- Modify: `crates/emcore/src/emPainterDrawList.rs:208-252` (variant_name)
- Modify: `crates/emcore/src/emPainterDrawList.rs:59-206` (serialize_op)
- Modify: `crates/emcore/src/emPainterDrawList.rs:376-410` (replay)

- [ ] **Step 1: Add SetTransformation variant to DrawOp enum**

In `crates/emcore/src/emPainterDrawList.rs`, add after the `SetOffset` variant (line 18):

```rust
SetTransformation {
    ox: f64,
    oy: f64,
    sx: f64,
    sy: f64,
},
```

- [ ] **Step 2: Add variant_name match arm**

In the `variant_name` function, add after the `SetOffset` arm:

```rust
DrawOp::SetTransformation { .. } => "SetTransformation",
```

- [ ] **Step 3: Add serialize_op match arm**

In the `serialize_op` function, add after the `SetOffset` arm:

```rust
DrawOp::SetTransformation { ox, oy, sx, sy } => {
    format!(r#"{{"seq":{seq},"op":"SetTransformation","ox":{ox},"oy":{oy},"sx":{sx},"sy":{sy}}}"#)
}
```

- [ ] **Step 4: Add replay match arm**

In the `replay` method, add after the `SetOffset` arm:

```rust
DrawOp::SetTransformation { ox, oy, sx, sy } => {
    painter.SetTransformation(
        ox - tile_offset.0,
        oy - tile_offset.1,
        *sx,
        *sy,
    );
}
```

Note: only origin components get tile_offset subtracted; scale is absolute.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p emcore`
Expected: Fails because `SetTransformation` method doesn't exist on emPainter yet (that's Task 2).

---

### Task 2: Add SetTransformation method to emPainter

**Files:**
- Modify: `crates/emcore/src/emPainter.rs:482-492` (near SetOrigin/SetScaling)

- [ ] **Step 1: Add SetTransformation method**

In `crates/emcore/src/emPainter.rs`, add after the `SetScaling` method (after line 492):

```rust
/// Set the full coordinate transformation (origin + scale) in one call.
/// Matches C++ `emPainter::SetTransformation`.
///
/// The transform from user coordinates to pixel coordinates is:
///   pixel_x = user_x * sx + ox
///   pixel_y = user_y * sy + oy
pub fn SetTransformation(&mut self, ox: f64, oy: f64, sx: f64, sy: f64) {
    self.record_state(DrawOp::SetTransformation { ox, oy, sx, sy });
    self.state.offset_x = ox;
    self.state.offset_y = oy;
    self.state.scale_x = sx;
    self.state.scale_y = sy;
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p emcore`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emPainter.rs crates/emcore/src/emPainterDrawList.rs
git commit -m "feat(emPainter): add SetTransformation method and DrawOp variant

Matches C++ emPainter::SetTransformation — sets origin and scale in one
call. DrawOp recording, serialization, and replay all handle the new
variant."
```

---

### Task 3: Switch paint_panel_recursive to panel-local coordinates

**Files:**
- Modify: `crates/emcore/src/emView.rs:2440-2486` (paint_panel_recursive)

- [ ] **Step 1: Add SetTransformation call and change Paint args**

In `crates/emcore/src/emView.rs`, the current code at lines 2440-2486:

```rust
painter.set_offset(base_offset.0 + vx, base_offset.1 + vy);
painter.SetClipping(clip_x - vx, clip_y - vy, clip_w, clip_h);
```

After `SetClipping` (line 2441), add the SetTransformation call. Also change the Paint call args. The section from line 2440 to 2486 becomes:

```rust
painter.set_offset(base_offset.0 + vx, base_offset.1 + vy);
painter.SetClipping(clip_x - vx, clip_y - vy, clip_w, clip_h);
// Match C++ emView::Paint: set painter transformation so widgets
// operate in panel-local coordinates (width=1.0, height=tallness).
painter.SetTransformation(
    base_offset.0 + vx,
    base_offset.1 + vy,
    vw,
    vw / self.pixel_tallness,
);
```

And change the Paint call (line 2486) from:

```rust
behavior.Paint(painter, vw, paint_h, &state);
```

To:

```rust
let tallness = if layout_rect.w > 0.0 {
    layout_rect.h / layout_rect.w
} else {
    1.0
};
behavior.Paint(painter, 1.0, tallness, &state);
```

Also remove the now-unused `paint_h` computation (lines 2481-2485):

```rust
let paint_h = if layout_rect.w > 0.0 {
    vw * (layout_rect.h / layout_rect.w)
} else {
    vh
};
```

This is replaced by the `tallness` computation above.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p emcore`
Expected: PASS (possibly warnings about unused `vh` — remove if so)

- [ ] **Step 3: Run golden tests**

Run: `cargo test --test golden -- --test-threads=1`
Expected: Same pass/fail results as before (pixel output should be identical).

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p emcore -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "feat(emView): switch paint_panel_recursive to panel-local coordinates

Call SetTransformation(origin, scale) before each panel's Paint, matching
C++ emView::Paint architecture. Widgets now receive w=1.0, h=tallness
instead of pixel-space dimensions. Pixel output is mathematically
identical since the painter's scale converts panel-local coordinates back
to pixels."
```

---

### Task 4: Verify DrawOp output and update diff tooling

**Files:**
- Verify: `scripts/diff_draw_ops.py` (may need scale normalization removed)

- [ ] **Step 1: Dump DrawOps for a known test**

Run: `DUMP_DRAW_OPS=1 cargo test --test golden widget_border_roundrect_thin -- --test-threads=1`

Check that `crates/eaglemode/target/golden-divergence/*.rust_ops.jsonl` now contains `SetTransformation` ops with the expected scale values (matching C++ ViewedWidth).

- [ ] **Step 2: Run the diff tool**

Run: `python3 scripts/diff_draw_ops.py widget_border_roundrect_thin`

Check if the scale-normalization logic in the diff tool produces better or worse results. If the Rust ops now match C++ scale, the normalization may become unnecessary or counterproductive.

- [ ] **Step 3: Update diff tool if needed**

If the diff tool's scale normalization now double-normalizes (since Rust ops are already in panel-local coords), either:
- Add handling for `SetTransformation` ops
- Remove the scale-normalization step for Rust ops

The specific change depends on what `diff_draw_ops.py` currently does — read it and adapt.

- [ ] **Step 4: Commit any tooling changes**

```bash
git add scripts/diff_draw_ops.py
git commit -m "fix(tooling): update DrawOp diff for panel-local coordinates"
```

---

### Task 5: Run full test suite and validate

- [ ] **Step 1: Run full test suite**

Run: `cargo-nextest ntr`
Expected: PASS (same results as before)

- [ ] **Step 2: Run golden tests with divergence log**

Run: `cargo test --test golden -- --test-threads=1`

Check `target/golden-divergence/divergence.jsonl` — the number of failures should be identical to before. If any tests newly fail or pass, investigate.

- [ ] **Step 3: Verify the previous "neutral" claim**

Compare golden test results before and after. If any pixel differences appear, they indicate the change is NOT neutral and require investigation (likely a widget using w/h non-proportionally, which would be a pre-existing C++ divergence).
