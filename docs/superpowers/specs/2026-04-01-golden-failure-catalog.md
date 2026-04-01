# Golden Failure Catalog

Classifies all 42 golden test failures (measured 2026-04-01, after gradient hash fix `3a2c1c1`) into 9 groups by shared rendering code path. Each group identifies the divergent Rust function, the corresponding C++ reference, and a root cause hypothesis.

## Summary

| Group | Code Path | Tests | max_diff range | Likely cause |
|-------|-----------|-------|----------------|--------------|
| G1 | `interpolate_scanline_area_sampled` | 23 | 19-255 | Area sampling carry-over / column-reuse accumulation diverges from C++ ScanlineTool |
| G2 | `fill_polygon_aa` / `rasterize_polynomial` | 6 | 12-255 | Polygon rasterizer FP edge-crossing accumulation differs from C++ |
| G3 | `ADAPTIVE_TABLE` / `interpolate_scanline_adaptive_premul` | 6 | 1 | Runtime f64 Hermite factor table rounds differently from C++ compile-time table |
| G4 | `PaintRoundRectOutline` inner polygon | 2 | 24-79 | Inner polygon vertex ordering / bridge construction differs from C++ |
| G5 | `fill_span_blended` direct division | 1 | 1 | `(c*a+127)/255` instead of `blend_hash_lookup(c, a)` for source premul term |
| G6 | Radial gradient polygon AA boundary | 1 | 1 | Sub-pixel coverage at ellipse polygon edge differs from C++ |
| G7 | `paint_linear_gradient` / `sample_linear_gradient` | 1 | 175 | f64 gradient parameter vs C++ 24-bit integer fixed-point walk |
| G8 | `emVirtualCosmosItemPanel::Paint` structural | 1 | 130 | 4 PaintRect strips with wrong canvas_color vs C++ 10-vertex PaintPolygon |
| G9 | `PaintSolidPolyline` checkmark stroke | 1 | 236 | Stroke polygon construction diverges from C++ PaintPolylineWithoutArrows |

**Total: 42 tests across 9 groups.**

---

## G1: Area Sampling (`interpolate_scanline_area_sampled`) — 23 tests

**Priority:** 1 (highest — fixes 23 of 42 tests)

**Tests (23):**

| Test | max_diff | fail% | Entry point |
|------|----------|-------|-------------|
| testpanel_expanded | 255 | 4.56% | PaintBorderImage (child widget borders) |
| composition_tktest_1x | 239 | 8.76% | PaintBorderImage (Group + widget borders) |
| composition_tktest_2x | 239 | 2.09% | PaintBorderImage (Group + widget borders at 2x) |
| widget_file_selection_box | 237 | 2.96% | PaintBorderImage (Group + Instrument borders) |
| composed_border_nest | 153 | 2.07% | PaintBorderImage (inner Group border) |
| widget_listbox | 136 | 0.04% | PaintBorderImage IO field overlay over dark bg |
| starfield_small | 69 | 0.03% | PaintImageColored (star glow texture) |
| colorfield_expanded | 54 | 0.75% | PaintBorderImage (border edge) + colorfield gradient |
| starfield_large | 53 | 0.02% | PaintImageColored (star glow texture) |
| listbox_expanded | 33 | 0.07% | PaintBorderImage (rightmost border column) |
| widget_button_normal | 31 | 0.03% | PaintBorderImage (Instrument border) |
| widget_radiobutton | 31 | 0.04% | PaintBorderImage (InstrumentMoreRound border) |
| widget_textfield_content | 26 | 0.04% | PaintBorderImage (Instrument border) |
| widget_textfield_empty | 26 | 0.04% | PaintBorderImage (Instrument border) |
| widget_textfield_single_char_square | 26 | 0.05% | PaintBorderImage (Instrument border) |
| widget_listbox_single | 25 | 0.08% | PaintBorderImage (Instrument border) |
| widget_listbox_empty | 25 | 0.03% | PaintBorderImage (Instrument border) |
| widget_colorfield | 24 | 0.27% | PaintBorderImage (border + IO field overlay) |
| widget_colorfield_alpha_near | 24 | 0.71% | PaintBorderImage IO field overlay |
| widget_colorfield_alpha_opaque | 24 | 0.27% | PaintBorderImage IO field overlay |
| widget_colorfield_alpha_zero | 24 | 0.57% | PaintBorderImage IO field overlay |
| widget_checkbox_unchecked | 22 | 0.04% | paint_image_full (CheckBox 380px image) |
| widget_splitter_v_extreme_tall | 19 | 0.02% | PaintBorderImage (splitter grip at extreme aspect) |

**Divergent code path:** Multiple callers (`PaintBorderImage` → `paint_9slice_section`, `PaintImageColored`, `paint_image_full`) all route through `emPainterInterpolation::interpolate_scanline_area_sampled` when the source image is larger than the destination.

**C++ reference:** `emPainter_ScTlIntImg.cpp` — `InterpolateImageAreaSampled` in the ScanlineTool template. The C++ area sampling uses inline accumulation within the ScanlineTool's per-scanline loop, with carry-over state managed by the template infrastructure.

**Spatial pattern:**
- **Outer borders (Instrument/Group):** Narrow horizontal band at bottom border (y~288 at 800x600), right edge column (x~767 in expanded views). The 286-340px border source images are downscaled 10-20x into ~15-30px border widths.
- **IO field overlay:** Symmetric left/right edges of colorfield inner region (x~87, x~712). IO field rendered with canvas=TRANSPARENT (source-over blend), where ±1 errors amplify against dark backgrounds (widget_listbox max_diff=136).
- **Star glows:** Isolated star-edge pixels where PaintImageColored downscales star texture.
- **CheckBox image:** Edge pixels of 380x380 CheckBox image downscaled to ~60px.
- **Composite panels:** Union of all child widget border divergences.

**Root cause hypothesis:** `interpolate_scanline_area_sampled` carry-over / column-reuse accumulation logic produces different intermediate accumulator values from C++ ScanlineTool. The C++ template infrastructure manages per-column state differently (inline within a monolithic scanline loop) compared to Rust's separated function calls. At high downscaling ratios (10-20x), these accumulation differences produce ±1 per-channel errors that can amplify to max_diff=255 when composited over contrasting backgrounds via source-over blending.

**Sub-groups by entry point:**
- **Border area sampling (16 tests):** PaintBorderImage → paint_9slice_section → area_sampled. The dominant sub-group. All Instrument/InstrumentMoreRound/Group outer borders and IO field inner overlays.
- **Image area sampling (3 tests):** paint_image_full or PaintImageColored → area_sampled. Includes checkbox_unchecked, starfield_small, starfield_large.
- **Composite (4 tests):** testpanel_expanded, composition_tktest_1x/2x, widget_file_selection_box — aggregate child widget border divergences.

---

## G2: Polygon Rasterizer FP Accumulation — 6 tests

**Priority:** 2

**Tests (6):**

| Test | max_diff | fail% | Primitive |
|------|----------|-------|-----------|
| testpanel_root | 255 | 2.79% | PaintRectOutline, PaintPolygon, PaintEllipse, PaintBezier, PaintPolyline |
| bezier_stroked | 53 | 0.18% | PaintBezierLine → PaintSolidPolyline → PaintPolygon |
| widget_scalarfield | 12 | 0.25% | PaintPolygon (5-point value arrow + 3-point scale marks) |
| widget_scalarfield_zero_range | 12 | 0.20% | PaintPolygon (same) |
| widget_scalarfield_min_value | 12 | 0.07% | PaintPolygon (arrow at min position) |
| widget_scalarfield_max_value | 12 | 0.06% | PaintPolygon (arrow at max position) |

**Divergent code path:** `PaintPolygon` / `PaintRectOutline` / `PaintEllipse` → `fill_polygon_aa` → `rasterize_polynomial` (in `emPainterScanline.rs`). The rasterizer computes per-scanline x-coordinate edge crossings using `x_cur += dx_per_row` accumulation.

**C++ reference:** `emPainter.cpp:591-612` — in-place `x1 += dx/dy` per scanline row within `PaintPolygon`.

**Spatial pattern:**
- **Scalarfield:** Diagonal edges of the value arrow polygon at y~146-160, symmetric about widget center. Position varies with value (x=342/457 for center, x=67 for min, x=674/732 for max).
- **testpanel_root:** Starting at (22,26) — just inside PaintRectOutline inner edge. 27,878 pixels spanning all primitive types rendered by TestPanel::Paint.
- **bezier_stroked:** Bezier curve edge pixels at rows 167-168, grayscale (R=G=B), ±1-5.

**Root cause hypothesis:** C++ does `dx /= dy` then `x1 += dx` per row (in-place mutation). Rust computes `dx_per_row = dx / dy` separately and advances `x_cur += dx_per_row`. For long polygon edges, floating-point non-associativity causes the accumulated `x_cur` to differ from C++'s `x1` by sub-pixel epsilon. When this epsilon straddles a `floor()` boundary, the pixel column of the edge shifts by ±1, producing a ±1 coverage difference in `blit_span` → `blend_with_coverage`. Round cap/join vertices (bezier_stroked) are computed with identical trig but rasterized through the same divergent accumulator.

---

## G3: Adaptive Hermite Interpolation FP Table — 6 tests

**Priority:** 3

**Tests (6):**

| Test | max_diff | fail% | Entry point |
|------|----------|-------|-------------|
| multi_compose... | | | |

Wait — multi_compose is G5 (fill_span_blended). Let me list G3 correctly:

| Test | max_diff | fail% | Entry point |
|------|----------|-------|-------------|
| image_scaled | 1 | 0.75% | paint_image_full → adaptive interpolation |
| composed_splitter_content | 1 | 0.002% | PaintBorderImage → paint_9slice_section corners + splitter grip |
| widget_splitter_h | 1 | 0.0002% | PaintBorderImage → splitter grip single pixel |
| widget_splitter_h_pos0 | 1 | 0.0002% | PaintBorderImage → splitter grip single pixel |
| widget_splitter_h_pos1 | 1 | 0.0002% | PaintBorderImage → splitter grip single pixel |
| widget_error_panel | 1 | 0.0006% | PaintText → PaintImageColored → font glyph upscaling |

**Divergent code path:** `paint_image_full` / `paint_9slice_section` / `PaintImageColored` → `interpolate_scanline_adaptive_premul` / `interpolate_scanline_adaptive_premul_section` → `ADAPTIVE_TABLE` (runtime f64 polynomial evaluation).

**C++ reference:** `emPainter_ScTlIntImg.cpp:1391` — hardcoded `FactorsTable[257]` (generated offline via `round()` in a separate program).

**Spatial pattern:**
- **image_scaled:** 493 scattered pixels at sub-pixel boundary positions in scaled RGBA image.
- **splitter_h variants:** Single pixel each at (402,596), (8,596), (796,596) — the rendered grip corner.
- **composed_splitter_content:** 8 border corner pixels at (x in {9,384,415,790}, y in {9,590}) + 1 splitter pixel at (402,596).
- **widget_error_panel:** 3 pixels at (185,298), (680,298), (408,307) — font glyph edge pixels in yellow error text.

**Root cause hypothesis:** Rust builds `ADAPTIVE_TABLE` at runtime using f64 polynomial evaluation (`adaptive_factors()` at `emPainterInterpolation.rs`). C++ uses a compile-time hardcoded `FactorsTable[257]`. For some of the 257 entries, runtime f64 evaluation produces a different `round()` result from C++'s offline-generated table — the test comment for `image_scaled` explicitly identifies this as "FP rounding in Hermite factor table computation." The ±1 difference in a single table entry propagates through the 4-tap Hermite filter to produce ±1 in the output pixel channel.

---

## G4: PaintRoundRectOutline Inner Polygon Vertex Ordering — 2 tests

**Priority:** 4

**Tests (2):**

| Test | max_diff | fail% |
|------|----------|-------|
| widget_border_round_rect | 79 | 0.003% |
| golden_widget_border_roundrect_thin | 24 | 0.0008% |

**Divergent code path:** `emBorder::paint_border` (`OuterBorderType::RoundRect`) → `emPainter::PaintRoundRectOutline` → inner polygon vertex construction.

**C++ reference:** `emPainter.cpp:1777` — `PaintRoundRectOutline` builds a combined outer+inner polygon where inner vertices are traversed via a specific bridge construction.

**Spatial pattern:**
- **widget_border_round_rect:** 15 pixels at top rounded corners (y=59-61 at x=166,228). Only corner pixels diverge, not straight edges.
- **golden_widget_border_roundrect_thin:** 4 extreme-corner pixels at (0,299), (799,299), (0,300), (799,300) — the very corners of a thin roundrect.

**Root cause hypothesis:** C++ builds a combined outer+inner polygon where: (1) outer polygon has `4n+4` vertices with segment count `n` from outer radii, (2) inner polygon has independently computed segment count `m` from inner radii, (3) bridge connects via `xy[4n+4]=outer[0]` then `xy[4n+5]=inner_start` (reversed inner). Rust uses `outer.push(outer[0]); outer.push(inner[0]); outer.extend(inner.iter().rev())`. The different bridge construction causes subtly different AA coverage at corners where the inner polygon's first/last vertex positions matter.

---

## G5: `fill_span_blended` Direct Division vs Hash Lookup — 1 test

**Priority:** 5

**Tests (1):** multi_compose

| Test | max_diff | fail% |
|------|----------|-------|
| multi_compose | 1 | 7.18% |

**Divergent code path:** `emPainter::fill_span_blended` → canvas blend branch → `(color_ch * alpha + 127) / 255`.

**C++ reference:** `emPainter_ScTlPSCol.cpp:119` — uses `h1R[alpha]` hash lookup for source color term.

**Spatial pattern:** 4703/65536 pixels distributed across large overlapping semi-transparent shape regions. Affects interior bulk-span pixels, not polygon edges.

**Root cause hypothesis:** `fill_span_blended` computes the source premul term as `(color_ch * alpha + 127) / 255` directly, but C++ uses `blend_hash_lookup(color_ch, alpha)` (the SharedPixelFormat hash table). The hash table rounds its quadrant terms independently, producing ±1 from direct computation for ~0.2% of `(color, alpha)` pairs.

---

## G6: Radial Gradient Polygon AA Boundary — 1 test

**Priority:** 6

**Tests (1):** gradient_radial

| Test | max_diff | fail% |
|------|----------|-------|
| gradient_radial | 1 | 0.05% |

**Divergent code path:** `emPainter::paint_radial_gradient` → `blit_span_textured` → `blend_with_coverage_unchecked` → `blend_pixel_unchecked` → `emColor::canvas_blend` → `blend_hash_lookup`.

**C++ reference:** `emPainter_ScTlPSCol.cpp:119` — hash lookup for solid color with coverage.

**Spatial pattern:** 32 pixels at ellipse polygon boundary AA rows (y=1 to y~4), symmetric about horizontal center x=128. Pairs at ±17-32 pixels from center. Rust always +1.

**Root cause hypothesis:** At the polygon AA edge, partial coverage from the scanline rasterizer produces a marginally different effective alpha. The different alpha, when passed through the hash table, produces ±1 output. Alternatively, sub-pixel differences in polygon vertex computation shift the coverage boundary by sub-LSB amounts. Related to G2 (polygon rasterizer FP), but here the ±1 manifests through the hash lookup rather than through coverage accumulation.

---

## G7: Linear Gradient Integer vs f64 Computation — 1 test

**Priority:** 7

**Tests (1):** eagle_logo

| Test | max_diff | fail% |
|------|----------|-------|
| eagle_logo | 175 | 55.23% |

**Divergent code path:** `emMainContentPanel::Paint` → `paint_linear_gradient` → `emPainterInterpolation::sample_linear_gradient`.

**C++ reference:** `emPainter_ScTlIntGra.cpp:24-38` (`InterpolateLinearGradient`), `emPainter_ScTl.cpp:174-188` (setup).

**Spatial pattern:** Nearly every gradient pixel differs by ±1 in one channel (55% of 480k pixels). One isolated outlier at (0,1) is completely wrong (rgb(145,171,242) vs rgb(192,228,67)) — likely a golden-generator artifact from different canvas fill color (WHITE vs BLACK initial condition).

**Root cause hypothesis:** C++ `InterpolateLinearGradient` computes the gradient parameter `u` (0-255) using 24-bit integer fixed-point arithmetic: `TX = (int64)((tx1-0.5)*nx + (ty1-0.5)*ny) - 0x7fffff; u = (x*TDX + y*TDY - TX) >> 24`. This is a truncating integer walk with a non-symmetric bias (`0x7fffff` not `0x800000`). Rust `sample_linear_gradient` computes `g = (t * 255.0 + 0.5) as i32` via f64 division, which rounds differently. The ±1 gradient parameter difference propagates through the hash formula `((c1*(255-g) + c2*g)*257 + 0x8073) >> 16` to produce ±1 in the output channel.

---

## G8: Cosmos Item Border Structural Algorithm Difference — 1 test

**Priority:** 8

**Tests (1):** cosmos_item_border

| Test | max_diff | fail% |
|------|----------|-------|
| cosmos_item_border | 130 | 0.67% |

**Divergent code path:** `emVirtualCosmosItemPanel::Paint` → `emPainter::PaintRect` (4 separate strip calls) → `fill_span_blended` / canvas-blend.

**C++ reference:** `emVirtualCosmos.cpp:361-409` — C++ uses `PaintPolygon(10-vertex frame, canvas_color=0)`.

**Spatial pattern:** Row 11, columns 0-9 (10 pixels wide). Row 11 is the bottom AA edge of the top border strip (top strip ends at pixel 11.25). Entire row is BLACK in Rust where C++ produces blended border color.

**Root cause hypothesis:** Two issues: (1) **Structural algorithm:** Rust uses 4 separate `PaintRect` calls for the border frame. C++ uses a single 10-vertex polygon ring (outer rect minus inner rect). This eliminates the AA-boundary interaction at corners. (2) **Wrong canvas_color:** Rust passes `canvas_color = border_color` (same as paint color). The canvas-blend formula `output = hash(paint, a) - hash(canvas, a) + target` becomes a no-op when paint = canvas, leaving the initial BLACK fill unchanged. C++ uses `canvas_color = 0` (TRANSPARENT = standard alpha blend), producing `border_color * coverage + BLACK * (1-coverage)` = non-zero values at the AA edge.

---

## G9: CheckBox Checkmark PaintSolidPolyline — 1 test

**Priority:** 9 (lowest — structural, single test)

**Tests (1):** widget_checkbox_checked

| Test | max_diff | fail% |
|------|----------|-------|
| widget_checkbox_checked | 236 | 0.07% |

**Divergent code path:** `emCheckBox::Paint` → `emPainter::PaintSolidPolyline` (checkmark 3-vertex polyline with round cap/join) → stroke polygon construction.

**C++ reference:** `emButton.cpp:160-184` (`PaintBoxSymbol`), `emPainter.cpp:3280-3582` (`PaintPolylineWithoutArrows`).

**Spatial pattern:** 339 pixels at the interior of the checkbox face region (x=117-122, y=271). Actual shows uniform face bg color (rgb(224,225,231)) where C++ shows checkmark stroke pixels — the checkmark stroke is missing or shifted at those pixel positions.

**Root cause hypothesis:** The checkmark uses 3 vertices forming a check shape. `PaintSolidPolyline` with round cap/join builds a stroke polygon then calls `PaintPolygon`. The stroke polygon construction in Rust diverges from C++'s `PaintPolylineWithoutArrows` — likely in how round joins compute the miter/bevel transition point or how the polygon winding bridges between segments. The face geometry and stroke thickness match C++, so the divergence is in the stroke polygon shape at the join vertex.

---

## Coverage Verification

All 42 failing tests are accounted for, each in exactly one group:

- **G1 (23):** testpanel_expanded, composition_tktest_1x, composition_tktest_2x, widget_file_selection_box, composed_border_nest, widget_listbox, starfield_small, colorfield_expanded, starfield_large, listbox_expanded, widget_button_normal, widget_radiobutton, widget_textfield_content, widget_textfield_empty, widget_textfield_single_char_square, widget_listbox_single, widget_listbox_empty, widget_colorfield, widget_colorfield_alpha_near, widget_colorfield_alpha_opaque, widget_colorfield_alpha_zero, widget_checkbox_unchecked, widget_splitter_v_extreme_tall
- **G2 (6):** testpanel_root, bezier_stroked, widget_scalarfield, widget_scalarfield_zero_range, widget_scalarfield_min_value, widget_scalarfield_max_value
- **G3 (6):** image_scaled, composed_splitter_content, widget_splitter_h, widget_splitter_h_pos0, widget_splitter_h_pos1, widget_error_panel
- **G4 (2):** widget_border_round_rect, golden_widget_border_roundrect_thin
- **G5 (1):** multi_compose
- **G6 (1):** gradient_radial
- **G7 (1):** eagle_logo
- **G8 (1):** cosmos_item_border
- **G9 (1):** widget_checkbox_checked

**Total: 23 + 6 + 6 + 2 + 1 + 1 + 1 + 1 + 1 = 42**
