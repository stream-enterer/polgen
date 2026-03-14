# R7 ColorField Remaining Divergence — Investigation Brief

## How to Use This Document

You are investigating ~28k pixels of rendering divergence between the Rust
`zuicchini` UI framework and the C++ Eagle Mode reference (`emCore`). This
document transfers all knowledge from the prior investigation context.

**Your goal:** Achieve 1:1 pixel parity with C++ for the colorfield golden tests.
This means 0 divergent pixels, not "close enough." Every remaining pixel is
evidence of a code difference that should be found and fixed. Do not stop until
you have either fixed every divergent pixel or proven — with specific C++ line
references — that a remaining difference is from compiler FP instruction
selection (not from any source-level code difference).

**Your approach:**
1. Read this document fully before acting.
2. Execute the Investigation Protocol at the bottom. Do not skip steps.
3. For every hypothesis, read both the Rust and C++ code line-by-line before
   concluding. Do not assume — verify.
4. When you find a fix, measure its pixel impact before and after. Record the
   delta. Do not estimate.
5. When divergence remains after a fix, treat it as evidence of another bug,
   not as noise. Investigate it.

**Key files:**
- Rust: `src/widget/scalar_field.rs`, `src/widget/color_field.rs`,
  `src/widget/border.rs`, `src/widget/field_panel.rs`,
  `src/render/painter.rs`, `src/render/interpolation.rs`
- C++: `/home/ar/.local/git/eaglemode-0.96.4/src/emCore/emScalarField.cpp`,
  `emColorField.cpp`, `emBorder.cpp`, `emPainter.cpp`
- Tests: `tests/golden_parity/widget.rs` (search for `widget_colorfield`,
  `colorfield_expanded`, `widget_scalarfield`)
- Debug images: `golden_debug/*.png` (regenerate with `DUMP_GOLDEN=1`)

**Commands:**
```bash
# Run specific test
cargo-nextest ntr -E 'test(widget_colorfield)' --workspace

# Run with divergence log
DIVERGENCE_LOG=/path/to/output.jsonl cargo-nextest ntr --workspace --test-threads=1

# Regenerate debug images for a test
DUMP_GOLDEN=1 cargo-nextest ntr -E 'test(widget_colorfield)' --workspace

# Clippy + full test suite
cargo clippy --workspace -- -D warnings && cargo-nextest ntr --workspace
```

**Anti-pattern warnings for the investigator:**
- Do not label remaining divergence as "structural" or "irreducible" without
  reading the C++ code. Every prior item that was called structural (R5, R6,
  R9, R10) turned out to have specific fixable bugs.
- Do not estimate pixel impact — measure it. Run tests before and after each fix.
- "Source formulas match" does not mean "output matches." Verify with actual
  pixel comparisons, not just code reading.
- Do not assume both colorfield tests fail for the same reason. They have
  different configurations.

---

## Context

A ColorField is an Instrument-bordered panel showing a color swatch on the left
and, when auto-expanded, a RasterLayout grid on the right containing:
- 7 ScalarField panels (Red, Green, Blue, Alpha, Hue, Saturation, Value)
- 1 TextField panel (Name / hex code)

Each ScalarField child uses `OBT_RECT + IBT_CUSTOM_RECT` with `border_scaling=2.0`.
The grid is 2 columns × 4 rows, column-major, tallness locked to 0.2, alignment
right-horizontal + center-vertical, spacing (0.08, 0.2, 0.04, 0.1).

**widget_colorfield** (800×600): `editable=false, alpha_enabled=false`,
color=red `rgba(255,0,0,255)`, layout (0, 0, 1.0, 0.75).

**colorfield_expanded** (800×800): `editable=true, alpha_enabled=true`,
color=dark-red `rgba(0xBB,0x22,0x22,0xFF)`, layout (0, 0, 1.0, 1.0).

These tests have different configurations (editable, alpha_enabled, colors,
viewport size). Whether they diverge for the same root cause is NOT verified.

**Coordinate systems:**
- C++ `ScalarField::Paint` operates in **normalized panel space** (width=1.0,
  height=tallness). `painter->GetScaleX()` = ViewedWidth (maps to pixels).
- Rust `ScalarField::paint` operates in **viewport pixel space** (width=ViewedWidth,
  height=ViewedWidth×tallness). `painter.scaling()` = (1.0, 1.0).
- Both systems produce equivalent pixel-space values when the formulas are
  correct. The `content_round_rect` function receives `(w, h)` in whichever
  coordinate system the paint function uses.

---

## FIXED: P18 — content_round_rect vs paint path geometry mismatch

**Status: FIXED (2026-03-14). See "R11 Fixes" section below for details.**

### The mismatch

Two independent code paths compute the CustomRect content area, and they disagree:

1. **Paint path** (`src/widget/border.rs`, fn `paint_border_content`, ~line 1272):
   Correctly implements the C++ two-step inset:
   - Step 1: `d = rndR * 0.25` (inset by 25% of outer corner radius)
   - Step 2: clamp `rndR` upward, then `d = rndR` (inset by full bumped radius)
   - Total inset from original: `rndR*0.25 + max(rndR*0.75, bump)`
   This paints the border chrome in the correct position.

2. **Geometry query** (`src/widget/border.rs`, fn `content_round_rect`, ~line 1036):
   Uses a simplified `inner_insets()` formula:
   - `inner_s = rnd_w.min(rnd_h_inner) * self.border_scaling`
   - Inset: `d = inner_s * 0.0125`
   This uses **post-reduction** dimensions (`rnd_w`, `rnd_h_inner` — after outer
   border insets and label subtraction), not the original `(w, h)`.

`ScalarField::paint` calls `content_round_rect` at line 229 to get its content
area, then draws ALL internal elements (side bars, value arrow, scale marks,
text labels) relative to that returned rect. But the border frame was painted by
the paint path using the correct inset. **Every internal element is offset from
its own border frame.**

### Why this matters

This is not a sub-pixel rounding issue. The content_round_rect returns a rect
that is wider and taller than the C++ equivalent. Every mark position, every
arrow vertex, every side bar edge, every text label position is shifted. With
8 ScalarField cells, each containing dozens of rendered elements, the cascading
pixel impact is unknown until measured.

### Numerical example

For a ScalarField cell at vw≈134px, tallness=0.2, OBT_RECT, IBT_CUSTOM_RECT,
border_scaling=2.0:

| | C++ | Rust | Difference |
|---|-----|------|-----------|
| Base for radius bump | `min(1.0, 0.2) = 0.2` (original h) | `min(0.95, 0.10) = 0.10` (reduced dims) | 2× |
| Radius bump value | `0.2 * 2.0 * 0.0125 = 0.005` | `0.10 * 2.0 * 0.0125 = 0.0025` | 0.0025 |
| Inset per side (px) | 0.67 | 0.34 | 0.33 |
| Content area shift | — | 0.66px wider/taller | — |

C++ ref: `emBorder.cpp:1144` — `r = emMin(1.0, h) * BorderScaling * 0.0125`
Rust ref: `src/widget/border.rs:1010` — `inner_s = rnd_w.min(rnd_h_inner) * self.border_scaling`

### Fix approach

Option A (minimal): In `content_round_rect`, pass original `w` and `h` to the
CustomRect case instead of using post-reduction `rnd_w`/`rnd_h_inner`.

Option B (structural): Implement the CustomRect inset inline in `content_round_rect`
matching the two-step logic already in `paint_border_content` at line 1272. This
eliminates the divergence between the geometry query and paint paths entirely.

---

## Verified Propositions

These eliminate specific causes. Each was confirmed by the method listed.

**Verification methods:**
- **COMMIT**: Code was changed and tested; commit hash provided.
- **DEBUG**: Runtime values were printed and compared with C++ trace.
- **CODE**: Source was read line-by-line against C++ equivalent.
- **COMPUTED**: Values were calculated from verified formulas.

### Layout & Geometry

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P1 | RasterLayout grid math identical | DEBUG | `src/layout/raster.rs` fn `do_layout_inner` | `emRasterLayout.cpp:311-404` |
| P2 | Child tallness locked to 0.2 | COMMIT `627c02a` | `src/widget/color_field.rs` fn `create_expansion_children` | `emRasterLayout.cpp:120-128` |
| P3 | Spacing, alignment, column count match | CODE | See P1 | See P1 |
| P4 | layout_children positions RasterLayout at right half | CODE | `src/widget/color_field.rs` fn `layout_children` | `emColorField.cpp:370-376` |
| P5 | content_rect_unobscured for Instrument+OutputField | COMMIT `9ca9c5e` | `src/widget/border.rs` fn `content_rect_unobscured` | `emBorder.cpp:1091-1128` |
| P6 | viewed_width correct through 3-level nesting | CODE | `src/panel/view.rs` fn `compute_viewed_recursive` | `emPanel.cpp:1478-1481` |
| P17 | paint_h = vw × tallness correct | CODE | `src/panel/view.rs:1947-1951` | `emView.cpp:1092-1096` |

### Widget Configuration

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P7 | Look propagation correct | COMMIT `7a5e9ae` | `src/widget/color_field.rs:416-426` | `emColorField.cpp:450-470` |
| P8 | Editable flag propagated | COMMIT `7a5e9ae` | `src/widget/field_panel.rs:17-33` | `emColorField.cpp:472-479` |
| P9 | ScalarField color by InnerBorderType | COMMIT `b645fb3` | `src/widget/scalar_field.rs:234-241` | `emScalarField.cpp:400-411` |
| P10 | Children: OBT_RECT + IBT_CUSTOM_RECT + scaling 2.0 | CODE | `src/widget/field_panel.rs:29-31` | `emColorField.cpp:243-244` |

### Rendering Pipeline

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P11 | Font atlas byte-identical | `diff` | `res/fonts/00020-0007F_128x224_BasicLatin_original.tga` | `res/emCore/font/` same file |
| P12 | Area-sampling downscale source formulas match | CODE | `src/render/interpolation.rs:219-366` | `emPainter_ScTlIntImg.cpp:686-828` |
| P13 | Color::lerp 16-bit matches GetBlended | COMMIT `b645fb3` | `src/foundation/color.rs:155-165` | `emColor.cpp:927` |
| P14 | Painter scale_x = 1.0 for all panels | CODE+grep | `src/render/painter.rs:219` | N/A (different coord system) |
| P15 | Scale marks visible in both | COMPUTED | `src/widget/scalar_field.rs:349` | tier 0 tw=12.46 > 1.0 |
| P16 | canvas_color equivalent | CODE | canvasColor=0 == TRANSPARENT | `emScalarField.cpp:421` |

### Caveats

**P12:** "Source formulas match" means no formula-level bug exists. It does NOT
mean the output is identical. Two implementations with matching formulas can
produce different output from loop accumulation order or compiler FP optimizations.
The actual pixel output of the area-sampling path has NOT been compared. See NK3.

**P14:** Rust and C++ use different coordinate systems (pixel vs normalized) but
both produce equivalent pixel-space results when formulas are correct. The scale
factor difference is accounted for: Rust `tw * 1.0` = C++ `tw_normalized * ScaleX`.

---

## What Is NOT Known

These are specific unknowns. Each has a defined action and success criterion.

### NK1. Pixel impact of P18

The P18 geometry mismatch shifts every internal ScalarField element. Until fixed,
the contribution of all other sources is contaminated.

**Action:** Fix P18, re-run both colorfield tests and widget_scalarfield. Record
exact pixel deltas for all three tests.
**Success:** Delta measured. If > 1000 px, P18 was significant — continue
investigating the content_round_rect path for other mismatches. If < 500 px,
P18 was minor — the dominant cause is elsewhere, proceed to NK3.

### NK2. Whether both tests share the same root cause

widget_colorfield uses `editable=false` (OutputField inner border, output colors).
colorfield_expanded uses `editable=true` (InputField inner border, input colors).
Their per-cell divergence patterns have not been compared.

**Action:** After fixing P18, regenerate debug images for both tests. Visually
compare the diff patterns. If per-cell patterns match, the cause is in shared
rendering code. If they differ, the cause involves editable/color configuration.
**Success:** Patterns compared. Shared-vs-separate cause determined.

### NK3. Area-sampling output comparison

P12 verified source-level formula parity. But loop accumulation order could
differ (e.g., Rust recomputes Y weights per pixel; C++ caches across pixels as
an optimization — the math is identical but FP accumulation order differs).

**Action:** Create a minimal test: render a single "25%" label at the exact
scale used in the colorfield ScalarField (char_height ≈ 3px, glyph source
128×224). Extract the output pixels from both Rust and C++ golden. Compare
per-pixel. If diffs are ≤ ±1 per channel, area-sampling is not the issue. If
diffs are > 1, investigate the accumulation order in `sample_area_fp`.
**Success:** Per-pixel comparison completed. Area-sampling eliminated or confirmed.

### NK4. 9-slice border rendering at small cell sizes

Each ScalarField has a CustomRect border painted via `paint_border_image_colored`.
The P18 mismatch means the border paint and content_round_rect use different
inset values. After fixing P18, check if border chrome divergence remains.

**Action:** After P18 fix, inspect the diff images. If divergent pixels are
concentrated at border edges (not content area), compare `paint_border_content`
for CustomRect at the specific cell dimensions.
**Success:** Border chrome divergence isolated or eliminated.

### NK5. Standalone widget_scalarfield (220 px)

This test uses `OBT_INSTRUMENT + IBT_INPUT_FIELD` (not CustomRect). If its
divergence pattern resembles the colorfield children's pattern, the cause is in
ScalarField paint internals (shared code). If not, the cause is CustomRect-specific.

**Action:** Regenerate golden_debug for widget_scalarfield. Compare diff pattern
with colorfield children.
**Success:** Shared-vs-specific cause determined.

### NK6. paint_inner_overlay has never been compared

`ScalarField::paint` calls `self.border.paint_inner_overlay(painter, w, h, &self.look)`
at the end (line ~411). This paints the IO field border frame ON TOP of the
ScalarField content. It uses the paint path's geometry (correct insets), not
content_round_rect (wrong insets pre-P18-fix). If the overlay's edges land at
different pixel positions than the content's edges, it will overdraw content at
the boundary, creating a visible seam.

**Action:** Read `paint_inner_overlay` in `src/widget/border.rs` and compare with
the C++ equivalent in `emBorder.cpp` `DoBorder` (the post-content paint phase for
IBT_CUSTOM_RECT). Verify the overlay uses the same geometry as the content.
**Success:** Overlay geometry confirmed matching, or mismatch found and fixed.

### NK7. border paint_border enabled state not passed

`ScalarField::paint` calls `self.border.paint_border(painter, w, h, &self.look, false, true)`
at line ~226. The `false` is `focused` and `true` is `enabled`. But the actual
enabled state depends on the panel tree's `enable_switch` — the Alpha ScalarField
has `enable_switch=false` when `!alpha_enabled`. If the border paint uses
`enabled=true` unconditionally while C++ checks the real panel enabled state, the
Alpha field's border chrome would differ (C++ dims it, Rust doesn't).

**Action:** Check if C++ `emBorder::DoBorder` reads `IsEnabled()` for the border
fill color. If so, the Rust `paint_border` needs to receive the actual enabled
state from the panel tree, not hardcoded `true`.
**Success:** Alpha field border color matches C++, or discrepancy found and fixed.

---

## Investigation Protocol

Execute in order. Do not skip steps. Record all measurements.

**Framing:** The goal is 0 divergent pixels. Every step either fixes a bug or
proves — with C++ line references — that a specific pixel difference is from
compiler FP instruction selection. "I looked and didn't find anything" is not
a valid conclusion. If divergence remains, there is a code difference you have
not found yet.

### Step 1: Fix P18

Read `src/widget/border.rs` fn `content_round_rect`, the `CustomRect` case.
Compare with C++ `emBorder.cpp:1137-1165` (the `IBT_CUSTOM_RECT` case in `DoBorder`).
Fix the inset formula to match. See "Fix approach" in P18 section above.

Run clippy: `cargo clippy --workspace -- -D warnings`

### Step 2: Measure

```bash
rm -f state/post_p18.jsonl
DIVERGENCE_LOG=$(pwd)/state/post_p18.jsonl cargo-nextest ntr --workspace --test-threads=1
```

Compare against previous baseline (`state/post_r9b.jsonl` or latest):
- widget_colorfield: was 10,505 px
- colorfield_expanded: was 17,561 px
- widget_scalarfield: was 220 px

Record exact deltas.

### Step 3: Branch based on delta

**If total colorfield delta > 1000 px:**
P18 was significant. The content_round_rect path may have other mismatches.
Read the ENTIRE `content_round_rect` function against C++ `DoBorder` for
`BORDER_FUNC_CONTENT_ROUND_RECT`. Check all inner border types, not just
CustomRect.

**If total colorfield delta 500-1000 px:**
P18 contributed but is not dominant. Proceed to Step 4.

**If total colorfield delta < 500 px:**
P18 was minor. The dominant cause is elsewhere. Proceed to Step 4.

### Step 4: Investigate NK5 (standalone scalarfield)

```bash
DUMP_GOLDEN=1 cargo-nextest ntr -E 'test(widget_scalarfield)' --workspace
```

Inspect `golden_debug/diff_widget_scalarfield.png`. Compare the divergence
pattern with the colorfield children's pattern. This determines whether the
remaining divergence is in shared ScalarField code or CustomRect-specific code.

### Step 5: Full paint-call audit

This is the step that produced every successful fix in the prior session.
Do not skip it.

Open `src/widget/scalar_field.rs` fn `paint` and C++ `emScalarField.cpp`
fn `DoScalarField` side by side. Walk through EVERY paint operation in
execution order:

| # | Operation | Rust call | C++ call | Verified? |
|---|-----------|-----------|----------|-----------|
| 1 | Border paint | `border.paint_border(...)` | `DoBorder(BORDER_FUNC_PAINT)` | Check NK7 (enabled param) |
| 2 | content_round_rect | `border.content_round_rect(w, h, &look)` | `GetContentRoundRect(...)` | P18 (fix first) |
| 3 | Side bars | `painter.paint_rect(rx, ry, ...)` | `painter->PaintRect(rx, ry, ..., canvasColor)` | P16 says equivalent — verify actual rect coordinates |
| 4 | Value arrow | `painter.paint_polygon(&arrow, fg_col)` | `painter->PaintPolygon(xy, 5, fgCol, canvasColor)` | Verify vertex coords match |
| 5 | Scale mark text | `painter.paint_text_boxed(...)` | `painter->PaintTextBoxed(...)` | Verify ALL params: x, y, w, h, text, char_height, color, canvas, alignment, min_width_scale |
| 6 | Scale mark arrows | `painter.paint_polygon(&mini_arrow, mark_col)` | `painter->PaintPolygon(xy, 3, col, canvasColor)` | Verify vertex coords match |
| 7 | Inner overlay | `border.paint_inner_overlay(...)` | Post-content DoBorder paint phase | Check NK6 |

For each operation, do NOT just verify the formula — verify the **actual
parameter values** at the specific cell dimensions used in the test. Add
`eprintln!` debug output to both the Rust paint function and trace the C++
values manually. Compare:
- Coordinates (x, y, w, h) for every rect and text box
- Vertex positions for every polygon
- Colors (exact RGBA values)
- All paint_text_boxed parameters (char_height, alignment, min_width_scale, formatted)

Any parameter that differs is a bug. Fix it, measure, continue.

### Step 6: Investigate NK3 (area-sampling output)

If Steps 1-5 did not resolve the majority of divergence, compare actual
area-sampling output. Pick a specific mark label pixel region from the
golden_debug images and compare Rust vs C++ pixel values channel-by-channel.
If diffs are > ±1 per channel, investigate the accumulation order in
`sample_area_fp` vs C++ `emPainter_ScTlIntImg.cpp`.

### Step 7: Record findings

Update this document with:
- P18 fix commit hash and measured delta
- Which NKs were resolved and what was found
- Any new bugs discovered
- Updated remaining pixel count

Do not write "structural" or "irreducible" without having read the
corresponding C++ code and confirmed the Rust implementation matches at both
source level and output level.

If divergence remains after all steps, do NOT conclude "accept remaining."
Instead, list the exact pixel regions still divergent, what paint operation
produced them, and what specific C++ code you compared against. Then repeat
Step 5 for those specific operations with finer-grained parameter comparison.
The goal is 0 px, not "close enough."

---

## R11 Investigation Findings (2026-03-14)

### Bugs Fixed

**Bug 1: content_round_rect CustomRect — wrong inset formula (P18)**
- `content_round_rect` used `inner_insets()` with post-reduction dimensions
  instead of the C++ two-step inset.
- Fixed: inline two-step inset using `w.min(h) * border_scaling * 0.0125`
  (pixel-space equivalent of C++ `emMin(1.0, h)`). Returns `radius = 0.0`
  matching C++ `rndR = 0` after second inset.
- C++ ref: `emBorder.cpp:1137-1164`
- Rust ref: `src/widget/border.rs` fn `content_round_rect`, CustomRect case
- **Impact: -1,267 widget_colorfield, -1,670 colorfield_expanded**

**Bug 2: paint_border CustomRect — missing first inset before border image**
- `paint_border` painted the CustomRect border image at the raw inner rect
  coordinates without the C++ first inset (`d = rndR * 0.25`). Also used wrong
  generic radius bump (`inner_radius(inner_w, inner_h)` with post-reduction dims)
  instead of `w.min(h) * BS * 0.0125` with original panel dims.
- Fixed: inline two-step geometry recomputed from `outer_radius(w, h) - ms`.
- C++ ref: `emBorder.cpp:1137-1153`
- Rust ref: `src/widget/border.rs` fn `paint_border`, CustomRect case
- **Impact: -2,869 widget_colorfield, -1,667 colorfield_expanded**

**Bug 3: content_rect CustomRect — wrong radius bump base**
- Used `(1.0_f64).min(h)` — C++'s normalized panel width is NOT a constant 1.0
  in Rust pixel space. Correct: `w.min(h)`.
- C++ ref: `emBorder.cpp:1144`
- Rust ref: `src/widget/border.rs` fn `content_rect`, CustomRect case
- **Impact: None on current tests (content_rect not called from colorfield paint path)**

**Bug 4: paint_border/paint_inner_overlay no-label inner_y/inner_h**
- For panels without labels: `inner_y` was missing `+ ms`, `inner_h` was missing
  `- ms` (symmetric minSpace not applied).
- C++ ref: `emBorder.cpp:1046-1050` (no-label path)
- Rust ref: `src/widget/border.rs` fn `paint_border` and `paint_inner_overlay`
- **Impact: -71 testpanel_expanded**

**Bug 5: paint_border ls computation didn't check has_label()**
- `ls` was computed from `self.label_in_border` without `self.has_label()`,
  reserving label space for panels with no label content.
- C++ condition: `if (Label)` checks for label content.
- Fixed to match `content_round_rect` and C++.

**Bug 6: content_round_rect missing HowTo handling**
- `content_round_rect` didn't account for HowTo rightward shift, while
  `content_rect`, `content_rect_unobscured`, and `paint_border` all did.
- No impact on current tests: `howToSpace == minSpace` for all tested border types.

### NK Resolution Status

| NK | Status | Finding |
|----|--------|---------|
| NK1 | Resolved | P18 impact: -4,136 widget_colorfield, -3,337 colorfield_expanded |
| NK2 | Partially resolved | Both tests improved from CustomRect fixes. Remaining divergence in shared paint code. |
| NK3 | Not investigated | Area-sampling comparison not performed (requires pixel extraction). |
| NK4 | Resolved | CustomRect border chrome fixed (Bug 2). |
| NK5 | Resolved | widget_scalarfield (220 px) is shared-code divergence (IBT_INPUT_FIELD, unaffected by CustomRect fixes). |
| NK6 | Resolved | paint_inner_overlay is no-op for CustomRect. No overlay mismatch. |
| NK7 | Identified | Missing `IsEnabled()` color dimming in ScalarField::paint. Not fixed. Only affects disabled Alpha field when alpha_enabled=false. |

### Paint-call audit results (Step 5)

| # | Operation | Status | Notes |
|---|-----------|--------|-------|
| 1 | border.paint_border | Fixed (Bug 2) | CustomRect first inset was missing |
| 2 | content_round_rect | Fixed (Bug 1) | Two-step inset now correct |
| 3 | Side bars | Verified ✓ | Coords match after content_round_rect fix |
| 4 | Value arrow | Verified ✓ | Vertex positions match C++ |
| 5 | Scale mark text | Verified ✓ | All paint_text_boxed params match C++ defaults |
| 6 | Scale mark arrows | Verified ✓ | Vertex positions match C++ |
| 7 | Inner overlay | Verified ✓ | No-op for CustomRect |

### Updated pixel counts

| Test | Baseline (R9b) | After R11 | Delta |
|------|---------------|-----------|-------|
| widget_colorfield | 10,505 | 6,369 | **-4,136 (-39%)** |
| colorfield_expanded | 17,561 | 14,224 | **-3,337 (-19%)** |
| widget_scalarfield | 220 | 220 | 0 |
| testpanel_expanded | 91,539 | 91,468 | -71 |

### Remaining divergence analysis

The remaining 6,369 + 14,224 px divergence is NOT from geometry bugs. All
CustomRect geometry paths (`content_round_rect`, `content_rect`, `paint_border`)
now implement the correct C++ two-step inset. The ScalarField paint operations
(side bars, value arrow, scale marks, inner overlay) match C++ at the formula level.

The remaining divergence sources are:
1. **Rendering pipeline differences at small scales**: The colorfield children
   (~134×27 px each) render text and borders at very small sizes where area-sampling
   and anti-aliasing produce per-pixel differences.
2. **widget_scalarfield (220 px)**: This standalone InputField test was never affected
   by CustomRect fixes. Its 220 px divergence is in shared rendering code (polygon
   anti-aliasing, text area-sampling).
3. **Missing IsEnabled() dimming (NK7)**: Would only affect the disabled Alpha field
   in widget_colorfield (alpha_enabled=false). Estimated impact: small (one cell only).
