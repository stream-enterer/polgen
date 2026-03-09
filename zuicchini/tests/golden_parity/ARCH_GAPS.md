# Architectural Gaps

Differences between C++ emPainter and Rust zuicchini that are caused by
fundamentally different algorithms, not bugs or tunable parameters.

All measurements taken at `ch_tol=1` (any pixel with channel diff > 1 counts).

## Status summary

The **C1: polygon-coverage** gap has been **CLOSED** by porting the C++
polynomial AA coverage rasterizer (A0/A1/A2 scan entries). This brought
9 tests to near-exact parity (ch_tol=1).

Remaining gaps are C2 (stroke expansion geometry), C3 (image interpolation),
C4 (stroke end decoration geometry), and C6 (bezier flattening).

---

## ~~C1: polygon-coverage~~ — CLOSED

**Resolved** by porting the C++ polynomial coverage rasterizer. The
`rasterize_polynomial()` function in `scanline.rs` now matches the C++
`emPainter::PaintPolygon` algorithm exactly.

Tests now passing at (ch_tol=1, 0.5%):
- `ellipse_basic`, `ellipse_sector`, `ellipse_small`
- `polygon_tri`, `polygon_star`, `polygon_complex`
- `clip_basic`, `multi_compose`, `line_basic`

## C2: stroke-expansion — Stroke polygon construction

Stroked shapes are expanded into filled polygons before rasterization.
The stroke expansion geometry differs between C++ and Rust (different
normal offsets, join calculations, etc.). Polygon AA coverage is now
identical; remaining diffs are purely stroke geometry.

- **Root cause:** Stroke expansion geometry, NOT coverage model.
- **Affected tests:**
  - `line_dashed` — max_diff=255, 1.92% differ
  - `outline_rect` — max_diff=255, 4.25% differ
  - `outline_ellipse` — max_diff=255, 3.47% differ
  - `outline_polygon` — max_diff=255, 0.85% differ
  - `outline_round_rect` — max_diff=255, 4.33% differ
  - `bezier_stroked` — max_diff=255, 3.17% differ
  - `polyline` — max_diff=255, 3.99% differ
- **Could narrow with:** Match C++ stroke expansion geometry exactly.
- **Assessment:** Acceptable at ch_tol=80. All within 5% budget.

## C3: interpolation — Image scaling filter

- **Root cause:** Different interpolation algorithm (Rust uses bilinear).
- **Affected tests:**
  - `image_scaled` — max_diff=118, 30.68% differ at ch_tol=1
- **Assessment:** Acceptable. Independent of polygon coverage.

## C4: stroke-ends — Stroke end decoration rendering

- **Root cause:** Different decoration geometry generation + sizing.
- **Affected tests:**
  - `line_ends_all` — max_diff=255, 13.28% differ
- **Assessment:** **Needs future work.** Above 5% budget.

## ~~C5: compound~~ — CLOSED

Previously tracked as downstream effect of C1. Now that polygon coverage
is exact, `multi_compose` passes at (ch_tol=1, 0.5%).

## C6: bezier-flattening — Bezier curve approximation

- **Root cause:** Bezier-to-polygon flattening produces different vertex
  positions between C++ and Rust.
- **Affected tests:**
  - `bezier_filled` — max_diff=255, 4.41% differ
- **Could narrow with:** Match C++ bezier flattening algorithm.
- **Assessment:** Acceptable at ch_tol=80.

## C7: gradient-rounding — Gradient texturing at boundaries

- **Root cause:** Gradient interpolation rounding at polygon boundaries
  produces small per-pixel diffs across many pixels.
- **Affected tests:**
  - `gradient_radial` — max_diff=50, 25.08% differ at ch_tol=1
    (previously max_diff=248 before polynomial coverage port)
- **Could narrow with:** Match C++ gradient interpolation rounding.
- **Assessment:** Acceptable at ch_tol=50. The high fail_pct reflects
  many small (1-50) channel diffs, not structural differences.

---

## Review needed

Only `line_ends_all` exceeds the 5% budget at ch_tol=80 (~13%).
This should be split into per-end-type sub-tests and fixed individually.
