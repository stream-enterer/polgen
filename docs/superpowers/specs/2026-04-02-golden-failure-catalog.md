# Golden Failure Catalog (2026-04-02)

Supersedes the 2026-04-01 catalog. 37 tests across 12 groups.

**Changes from 2026-04-01:**
- 5 tests fixed (area sampling inner loop literal port + tdx_init fix)
- G1 hypothesis (area sampling carry-over) **disproven** — 23 former-G1 tests reclassified into 4 new groups (A-D)
- G2-G9 hypotheses **re-validated** — all confirmed with identical divergence patterns
- widget_listbox max_diff dropped 136→25 (IO field overlay fixed, remaining divergence is HowTo text)
- **HowTo text fix applied** — 7 widget types now populate `how_to_text` in `Paint()`. Groups A+B root cause reclassified from "missing HowTo text" to "PaintTextBoxed text rendering divergence". 11 tests improved (max_diff reduced), 4 slightly regressed (text rendering divergence > old flat-background divergence at some glyph positions), 5 composite tests unchanged.

## Summary

| Group | Code Path | Tests | max_diff range | Status | Likely cause |
|-------|-----------|-------|----------------|--------|--------------|
| A | `PaintTextBoxed` via `emBorder::paint_border` | 15 | 13-54 | verified | HowTo text populated (fixed), residual divergence is `PaintTextBoxed` rendering vs C++ |
| B | Same as A (composite) | 5 | 153-255 | verified | Composite widgets aggregating Sub-group A child divergences |
| G2 | `fill_polygon_aa` / `rasterize_polynomial` | 6 | 12-255 | carried forward | Polygon rasterizer FP edge-crossing accumulation differs from C++ |
| C | `PaintEllipse` / `PaintImageColored` | 2 | 53-69 | verified | Star rendering sub-pixel interpolation differs from C++ |
| G3 | `ADAPTIVE_TABLE` / `interpolate_scanline_adaptive_premul` | 2 | 1 | carried forward | Runtime f64 Hermite factor table rounds differently from C++ compile-time table |
| D | `PaintBorderImage` (splitter grip) | 1 | 19 | verified | Grip 9-slice sub-pixel boundary sampling |
| G4 | `PaintRoundRectOutline` inner polygon | 1 | 24 | carried forward | Inner polygon vertex ordering / bridge construction differs from C++ |
| G5 | `fill_span_blended` direct division | 1 | 1 | carried forward | `(c*a+127)/255` vs `blend_hash_lookup(c, a)` for source premul term |
| G6 | Radial gradient polygon AA boundary | 1 | 1 | carried forward | Sub-pixel coverage at ellipse polygon edge differs from C++ |
| G7 | `paint_linear_gradient` / `sample_linear_gradient` | 1 | 175 | carried forward | f64 gradient parameter vs C++ 24-bit integer fixed-point walk |
| G8 | `emVirtualCosmosItemPanel::Paint` structural | 1 | 130 | carried forward | 4 PaintRect strips with wrong canvas_color vs C++ 10-vertex PaintPolygon |
| G9 | `PaintSolidPolyline` checkmark stroke | 1 | 236 | carried forward | Stroke polygon construction diverges from C++ PaintPolylineWithoutArrows |

**Total: 15 + 5 + 6 + 2 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 37**

---

## Group A: HowTo Text Rendering Divergence — 15 tests

**Priority:** 1 (largest group — needs `PaintTextBoxed` fix to resolve)

**Status: PARTIALLY FIXED.** HowTo text is now populated (7 widgets wired to call `GetHowTo()` in `Paint()`). Residual divergence is `PaintTextBoxed` producing different glyph pixels from C++.

**Tests (15):**

| Test | pre-fix max_diff | post-fix max_diff | Change |
|------|-----------------|-------------------|--------|
| colorfield_expanded | 54 | 54 | — (dominated by IO field ±1-5 LSB) |
| listbox_expanded | 33 | 36 | +3 |
| widget_button_normal | 31 | 14 | -17 |
| widget_radiobutton | 31 | 26 | -5 |
| widget_textfield_content | 26 | 31 | +5 |
| widget_textfield_empty | 26 | 31 | +5 |
| widget_textfield_single_char_square | 26 | 31 | +5 |
| widget_listbox_single | 25 | 24 | -1 |
| widget_listbox_empty | 25 | 24 | -1 |
| widget_listbox | 25 | 24 | -1 |
| widget_colorfield | 24 | 14 | -10 |
| widget_colorfield_alpha_near | 24 | 14 | -10 |
| widget_colorfield_alpha_opaque | 24 | 14 | -10 |
| widget_colorfield_alpha_zero | 24 | 14 | -10 |
| widget_checkbox_unchecked | 22 | 13 | -9 |

**Divergent code path:** `emBorder::paint_border()` → `PaintTextBoxed()` for the HowTo pill text. Both Rust and C++ now render the same text content onto the pill, but `PaintTextBoxed` produces different glyph anti-aliasing pixels.

**C++ reference:** `emBorder.cpp:904-928` (HowTo pill rendering), `emPainter.cpp` `PaintTextBoxed` (text rasterization).

**Spatial pattern:** Divergent pixels cluster in the HowTo pill region (left edge of border, y≈288-295). Both sides render text — the divergence is in glyph edge anti-aliasing, not missing content.

**Root cause:** Two layers:
1. ~~Missing `how_to_text`~~ **(FIXED)** — 7 widget types now call `self.border.how_to_text = self.GetHowTo(enabled, true)` before `paint_border()`.
2. **`PaintTextBoxed` rendering divergence (REMAINING)** — Rust text rasterization produces different anti-aliased glyph pixels from C++. This is a sub-pixel precision issue in text layout and glyph rendering, not a missing feature. At some glyph positions the Rust rendering differs more from C++ than the old flat background did, explaining the 4 tests that show slightly increased max_diff.

**Note:** colorfield_expanded (max_diff=54) also has ±1-5 LSB divergences in IO field overlay content beyond the HowTo region, keeping its max_diff unchanged.

---

## Group B: Composite Widget HowTo Text — 5 tests

**Priority:** 2 (resolves when Group A's PaintTextBoxed divergence is fixed)

**Status: PARTIALLY FIXED.** HowTo text populated in child widgets. Composite max_diff unchanged because it's dominated by child PaintTextBoxed divergences composited onto dark backgrounds.

**Tests (5):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| testpanel_expanded | 255 | 45395 | 4.54% |
| composition_tktest_1x | 239 | 41521 | 8.65% |
| composition_tktest_2x | 239 | 10007 | 2.08% |
| widget_file_selection_box | 237 | 14190 | 2.96% |
| composed_border_nest | 153 | 9944 | 2.07% |

**Divergent code path:** Same as Group A. These tests render multiple child widgets, each contributing HowTo-text-sized blocks of divergent pixels.

**C++ reference:** Same as Group A.

**Spatial pattern:** Large max_diff (153-255) because composited HowTo text rendering divergences on dark backgrounds produce high contrast. Both sides now render text, but glyph pixel differences amplify through compositing.

**Root cause:** Aggregates of Group A divergences. testpanel_expanded renders 4 TkTestPanels containing all widget types; composition_tktest_1x/2x render all widget types in a raster grid; widget_file_selection_box contains child text fields + buttons; composed_border_nest contains Button + TextField children.

---

## G2: Polygon Rasterizer FP Accumulation — 6 tests

**Priority:** 3

**Tests (6):**

| Test | max_diff | fail_px | fail% | Primitive |
|------|----------|---------|-------|-----------|
| testpanel_root | 255 | 27878 | 2.79% | PaintRectOutline, PaintPolygon, PaintEllipse, PaintBezier, PaintPolyline |
| bezier_stroked | 53 | 119 | 0.18% | PaintBezierLine → PaintSolidPolyline → PaintPolygon |
| widget_scalarfield | 12 | 1192 | 0.25% | PaintPolygon (5-point value arrow + 3-point scale marks) |
| widget_scalarfield_zero_range | 12 | 975 | 0.20% | PaintPolygon (same) |
| widget_scalarfield_min_value | 12 | 332 | 0.07% | PaintPolygon (arrow at min position) |
| widget_scalarfield_max_value | 12 | 275 | 0.06% | PaintPolygon (arrow at max position) |

**Status:** Carried forward — re-validated, all patterns identical to original catalog.

**Divergent code path:** `PaintPolygon` / `PaintRectOutline` / `PaintEllipse` → `fill_polygon_aa` → `rasterize_polynomial`. The rasterizer computes per-scanline x-coordinate edge crossings using `x_cur += dx_per_row` accumulation.

**C++ reference:** `emPainter.cpp:591-612` — in-place `x1 += dx/dy` per scanline row within `PaintPolygon`.

**Spatial pattern:**
- testpanel_root: Starting at (22,26) — just inside PaintRectOutline inner edge. 27,878 pixels spanning all primitive types.
- bezier_stroked: Bezier curve edge pixels at rows 167-168, grayscale, ±1-5.
- widget_scalarfield (×4): Diagonal edges of the value arrow polygon at y~146-160.

**Root cause hypothesis:** C++ does `dx /= dy` then `x1 += dx` per row (in-place mutation). Rust computes `dx_per_row = dx / dy` separately and advances `x_cur += dx_per_row`. For long polygon edges, floating-point non-associativity causes accumulated `x_cur` to differ by sub-pixel epsilon, shifting pixel column by ±1 at `floor()` boundaries.

---

## Group C: Starfield Rendering Precision — 2 tests

**Priority:** 4

**Tests (2):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| starfield_small | 69 | 21 | 0.03% |
| starfield_large | 53 | 233 | 0.02% |

**Status:** Verified (new group, split from former G1).

**Divergent code path:** `emStarFieldPanel::Paint()` → `PaintEllipse` (star body AA polygon) / `PaintImageColored` (star glow texture bilinear interpolation).

**C++ reference:** `emStarFieldPanel.cpp`

**Spatial pattern:** Divergent pixels are scattered at individual star positions (not contiguous blocks). Color channel differences suggest different sub-pixel sampling/interpolation rounding at star edges.

**Root cause hypothesis (VERIFIED):** Two contributing factors: (1) PaintEllipse polygon AA approximation produces slightly different sub-pixel coverage from C++ at star body edges. (2) PaintImageColored bilinear interpolation rounds differently at star glow texture boundaries. Both are sub-pixel precision issues specific to star rendering geometry.

---

## G3: Adaptive Hermite Interpolation FP Table — 2 tests

**Priority:** 5

**Tests (2):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| image_scaled | 1 | 493 | 0.75% |
| composed_splitter_content | 1 | 8 | 0.00% |

**Status:** Carried forward — re-validated. (4 tests fixed by area sampling/tdx_init: widget_splitter_h ×3, widget_error_panel)

**Divergent code path:** `paint_image_full` / `paint_9slice_section` → `interpolate_scanline_adaptive_premul` → `ADAPTIVE_TABLE`.

**C++ reference:** `emPainter_ScTlIntImg.cpp:1391` — hardcoded `FactorsTable[257]`.

**Spatial pattern:** image_scaled: 493 scattered pixels. composed_splitter_content: 8 border corner pixels at (x∈{9,384,415,790}, y∈{9,590}).

**Root cause hypothesis:** Rust builds `ADAPTIVE_TABLE` at runtime using f64 polynomial evaluation. C++ uses a compile-time hardcoded table. For some entries, runtime f64 `round()` differs from C++'s offline-generated values. ±1 in a table entry propagates through the 4-tap Hermite filter to ±1 in output.

---

## Group D: Splitter Grip Border Image Boundary — 1 test

**Priority:** 6

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| widget_splitter_v_extreme_tall | 19 | 84 | 0.02% |

**Status:** Verified (new group, split from former G1).

**Divergent code path:** `emSplitter::Paint()` → `painter.PaintBorderImage()` at `emSplitter.rs:135`.

**C++ reference:** `emSplitter.cpp`

**Spatial pattern:** All 84 divergent pixels at the grip boundary (x=362, y=295-304 + x=362-436, y=304). Single-pixel-wide vertical strip at the grip edge.

**Root cause hypothesis (VERIFIED):** The splitter grip uses PaintBorderImage for its visual overlay. The ±19 max_diff across 10 y-coordinates is a sub-pixel boundary sampling difference in the 9-slice grip image rendering. This splitter never sets `has_how_to`, so HowTo text is not a factor.

---

## G4: PaintRoundRectOutline Inner Polygon — 1 test

**Priority:** 7

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| golden_widget_border_roundrect_thin | 24 | 4 | 0.00% |

**Status:** Carried forward — re-validated. (widget_border_round_rect now passes, was fixed by area sampling/tdx_init)

**Divergent code path:** `PaintRoundRectOutline` inner polygon vertex construction.

**C++ reference:** `emPainter.cpp:1777`

**Spatial pattern:** 4 extreme-corner pixels at (0,299), (799,299), (0,300), (799,300).

**Root cause hypothesis:** Bridge construction between outer and inner polygon vertices differs from C++, causing subtly different AA coverage at corners.

---

## G5: `fill_span_blended` Direct Division vs Hash Lookup — 1 test

**Priority:** 8

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| multi_compose | 1 | 4703 | 7.18% |

**Status:** Carried forward — re-validated.

**Divergent code path:** `emPainter::fill_span_blended` → `(color_ch * alpha + 127) / 255`.

**C++ reference:** `emPainter_ScTlPSCol.cpp:119` — uses `h1R[alpha]` hash lookup.

**Spatial pattern:** 4703/65536 pixels distributed across overlapping semi-transparent regions. Interior bulk spans, not polygon edges.

**Root cause hypothesis:** Direct division vs hash table lookup produces ±1 for ~0.2% of `(color, alpha)` pairs.

---

## G6: Radial Gradient Polygon AA Boundary — 1 test

**Priority:** 9

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| gradient_radial | 1 | 32 | 0.05% |

**Status:** Carried forward — re-validated.

**Divergent code path:** `paint_radial_gradient` → `blit_span_textured` → `blend_with_coverage_unchecked`.

**C++ reference:** `emPainter_ScTlPSCol.cpp:119`

**Spatial pattern:** 32 pixels at ellipse polygon boundary AA rows, symmetric pairs at y=1-45.

**Root cause hypothesis:** Sub-pixel coverage at polygon AA edge differs from C++, producing ±1 through hash lookup. Related to G2 (polygon rasterizer FP).

---

## G7: Linear Gradient Integer vs f64 Computation — 1 test

**Priority:** 10

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| eagle_logo | 175 | 265111 | 55.23% |

**Status:** Carried forward — re-validated.

**Divergent code path:** `paint_linear_gradient` → `sample_linear_gradient`.

**C++ reference:** `emPainter_ScTlIntGra.cpp:24-38`, `emPainter_ScTl.cpp:174-188`.

**Spatial pattern:** 55% of 480k pixels differ by ±1 in one channel. Structural outlier at (0,1): `actual=rgb(145,171,242) expected=rgb(192,228,67)`.

**Root cause hypothesis:** C++ uses 24-bit integer fixed-point walk with truncating integer arithmetic. Rust uses f64 division, rounding differently. ±1 gradient parameter difference propagates through hash formula.

---

## G8: Cosmos Item Border Structural — 1 test

**Priority:** 11

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| cosmos_item_border | 130 | 800 | 0.67% |

**Status:** Carried forward — re-validated.

**Divergent code path:** `emVirtualCosmosItemPanel::Paint` → 4 `PaintRect` calls.

**C++ reference:** `emVirtualCosmos.cpp:361-409` — C++ uses single 10-vertex `PaintPolygon`.

**Spatial pattern:** Row 11, columns 0-9 (10 pixels wide). BLACK in Rust where C++ has blended border.

**Root cause hypothesis:** Two issues: (1) Structural: Rust uses 4 PaintRect calls vs C++ single 10-vertex polygon. (2) Wrong canvas_color: Rust passes `canvas_color = border_color` instead of TRANSPARENT.

---

## G9: CheckBox Checkmark PaintSolidPolyline — 1 test

**Priority:** 12 (lowest)

**Tests (1):**

| Test | max_diff | fail_px | fail% |
|------|----------|---------|-------|
| widget_checkbox_checked | 236 | 339 | 0.07% |

**Status:** Carried forward — re-validated.

**Divergent code path:** `emCheckBox::Paint` → `PaintSolidPolyline` → stroke polygon construction.

**C++ reference:** `emButton.cpp:160-184`, `emPainter.cpp:3280-3582`.

**Spatial pattern:** 339 pixels at checkmark stroke interior (x=117-122, y=271). Actual shows face bg color where C++ shows checkmark stroke pixels.

**Root cause hypothesis:** Stroke polygon construction in Rust diverges from C++ `PaintPolylineWithoutArrows` — likely in round join miter/bevel transition point or polygon winding bridge between segments.

---

## Coverage Verification

All 37 failing tests are accounted for, each in exactly one group:

- **A (15):** colorfield_expanded, listbox_expanded, widget_button_normal, widget_radiobutton, widget_textfield_content, widget_textfield_empty, widget_textfield_single_char_square, widget_listbox_single, widget_listbox_empty, widget_listbox, widget_colorfield, widget_colorfield_alpha_near, widget_colorfield_alpha_opaque, widget_colorfield_alpha_zero, widget_checkbox_unchecked
- **B (5):** testpanel_expanded, composition_tktest_1x, composition_tktest_2x, widget_file_selection_box, composed_border_nest
- **G2 (6):** testpanel_root, bezier_stroked, widget_scalarfield, widget_scalarfield_zero_range, widget_scalarfield_min_value, widget_scalarfield_max_value
- **C (2):** starfield_small, starfield_large
- **G3 (2):** image_scaled, composed_splitter_content
- **D (1):** widget_splitter_v_extreme_tall
- **G4 (1):** golden_widget_border_roundrect_thin
- **G5 (1):** multi_compose
- **G6 (1):** gradient_radial
- **G7 (1):** eagle_logo
- **G8 (1):** cosmos_item_border
- **G9 (1):** widget_checkbox_checked

**Total: 15 + 5 + 6 + 2 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 37 ✓**

---

## Fix Priority Summary

| Priority | Group(s) | Tests | Effort | Notes |
|----------|----------|-------|--------|-------|
| 1 | A + B | 20 | Medium | ~~Wire `GetHowTo()`~~ (DONE). Remaining: fix `PaintTextBoxed` glyph rendering |
| 2 | G2 | 6 | Medium | Match C++ in-place `dx/dy` accumulation in polygon rasterizer |
| 3 | C | 2 | Medium | Per-function investigation of PaintEllipse/PaintImageColored |
| 4 | G3 | 2 | Low | Port C++ compile-time Hermite factor table literally |
| 5 | D | 1 | Low | Investigate splitter grip 9-slice boundary |
| 6 | G4 | 1 | Low | Fix bridge construction in PaintRoundRectOutline |
| 7 | G5 | 1 | Low | Switch to hash table lookup for source premul |
| 8 | G6 | 1 | Low | May be fixed by G2 polygon rasterizer fix |
| 9 | G7 | 1 | Medium | Port C++ 24-bit integer fixed-point gradient walk |
| 10 | G8 | 1 | Low | Switch to single PaintPolygon + canvas_color=0 |
| 11 | G9 | 1 | Medium | Port C++ stroke polygon construction literally |
