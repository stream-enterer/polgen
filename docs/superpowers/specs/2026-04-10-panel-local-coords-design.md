# Panel-Local Coordinate Space for paint_panel_recursive

**Date**: 2026-04-10
**Status**: Approved

## Problem

The Rust `paint_panel_recursive` skips `SetScaling`/`SetTransformation` — widgets
receive pixel-space coordinates (`w≈800`, `h≈600`) instead of panel-local
coordinates (`w=1.0`, `h≈0.75`) as in C++. This diverges from C++ architecture
and causes:

1. DrawOp diff tooling must normalize by a detected scale factor, adding
   imprecision
2. f64 computations happen on different-magnitude values, potentially producing
   different ULP rounding at pixel edges

## Design

### Approach: SetTransformation in paint_panel_recursive

Match C++ `emView::Paint()` by calling `SetTransformation` before each panel's
`Paint` call.

### Changes

#### 1. emPainter.rs — Add SetTransformation

Add `SetTransformation(origin_x, origin_y, scale_x, scale_y)` matching the C++
method signature. Sets origin and scale in one call:

```rust
pub fn SetTransformation(&mut self, ox: f64, oy: f64, sx: f64, sy: f64) {
    self.state.offset_x = ox;
    self.state.offset_y = oy;
    self.state.scale_x = sx;
    self.state.scale_y = sy;
}
```

#### 2. emView.rs — paint_panel_recursive

After `SetClipping` (which must run while `scale=1.0`), call
`SetTransformation`:

```
Current:
  set_offset(base + vx, base + vy)
  SetClipping(clip_x - vx, clip_y - vy, clip_w, clip_h)
  Paint(painter, vw, paint_h)

New:
  set_offset(base + vx, base + vy)
  SetClipping(clip_x - vx, clip_y - vy, clip_w, clip_h)
  SetTransformation(base + vx, base + vy, vw, vw / pixel_tallness)
  Paint(painter, 1.0, tallness)
```

Where `tallness = layout_rect.h / layout_rect.w`.

#### 3. pixel_scale audit

Any site computing pixel_scale from `w * h` (the Paint parameters) must be
updated. Currently `viewed_rect.w * viewed_rect.h / w / h` gives ~1.0 because
both are in pixels. With `w=1.0`, this gives `VW²`. Fix by computing
pixel_scale from `state.viewed_rect` directly (matching C++ which uses
`GetViewedWidth()` / `GetViewedHeight()`).

#### 4. DrawOp tooling

After this change, recorded DrawOps have panel-local coordinates matching C++.
The scale-normalization logic in `diff_draw_ops.py` can be simplified or
removed.

### Safety Analysis

- **SetClipping call order**: `SetClipping` runs while `scale=1.0`, so it
  transforms identically to before (identity transform). Stored clip is in
  pixel space. Later `SetTransformation` only affects paint operations.
- **Widget Paint code**: Widgets use `w/h` proportionally
  (`painter.PaintRect(0, 0, w, h, ...)`). With `scale=VW`:
  `0 * VW + origin = origin`, `1.0 * VW + origin = origin + VW`.
  Identical pixels.
- **Golden tests**: Pixel output should be mathematically identical. Run golden
  suite to confirm.

### What Doesn't Change

- SetClipping semantics (safe due to call order)
- emPainter paint method internals
- Golden test expected outputs
- Widget Paint implementations (w/h used proportionally)

### Risks

- pixel_scale computations are the main breakage risk — must grep and fix all
  sites
- If any widget uses hardcoded pixel values (pre-existing bug), it would produce
  wrong results at new scale
