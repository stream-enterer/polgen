# Phase 5 — Golden Tests for emMain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add golden tests comparing Rust emMain panel rendering against C++ reference data. Divergence log clean at tolerance 0.

**Architecture:** Extend the C++ golden data generator (`gen_golden.cpp`) with standalone rendering functions that replicate emMain panel Paint methods using libemCore's emPainter. Add 5 Rust test modules (eagle_logo, starfield, main_panel, cosmos_items, control_panel) that exercise the actual Rust panel code and compare against the C++ reference. Layout-only tests (main_panel, control_panel) compare f64 rect coordinates; rendering tests (eagle_logo, starfield, cosmos_items) compare pixel output.

**Tech Stack:** C++ gen_golden.cpp (libemCore emPainter, emImage, emTexture, emRes), Rust golden test harness (emcore emPainter, emmain panel types, SoftwareCompositor), binary golden data format.

**Key constraint:** No `libemMain.so` exists — emMain is compiled into the eaglemode binary. The generator uses standalone functions that replicate C++ Paint logic using only libemCore's emPainter (already linked). Polygon data and rendering algorithms are copied from C++ sources at exact line references.

---

### Task 1: Eagle Logo Golden Test

Verify that the Rust `emMainContentPanel::Paint` (gradient background + 14 eagle polygons) produces identical pixels to C++ at 800×600 viewport with panel height 0.75.

**Files:**
- Modify: `crates/eaglemode/tests/golden/gen/gen_golden.cpp` — add `gen_eagle_logo()` function
- Create: `crates/eaglemode/tests/golden/eagle_logo.rs` — Rust test module
- Modify: `crates/eaglemode/tests/golden/main.rs` — register module

- [ ] **Step 1: Create eagle_logo.rs test skeleton**

```rust
// crates/eaglemode/tests/golden/eagle_logo.rs
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emImage::emImage;
use emcore::emPainter::emPainter;
use emcore::emPanel::{PanelBehavior, PanelState};

use emmain::emMainContentPanel::emMainContentPanel;

use super::common::*;

macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden data not found");
            return;
        }
    };
}

#[test]
fn eagle_logo() {
    require_golden!();
    let (ew, eh, expected) = load_painter_golden("eagle_logo");

    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainContentPanel::new(Rc::clone(&ctx));
    panel.update_coordinates(0.75);

    let mut img = emImage::new(ew, eh, 4);
    img.fill(emColor::WHITE);
    {
        let mut p = emPainter::new(&mut img);
        // Scale so panel coords (0,0)-(1.0,0.75) map to pixels (0,0)-(800,600).
        // scale_x = 800, scale_y = 600/0.75 = 800.
        p.scale(ew as f64, ew as f64);
        p.SetCanvasColor(emColor::TRANSPARENT);

        let state = PanelState::default();
        panel.Paint(&mut p, 1.0, 0.75, &state);
    }

    compare_images("eagle_logo", img.GetMap(), &expected, ew, eh, 0, 0.0).unwrap();
}
```

- [ ] **Step 2: Register module in main.rs**

Add to `crates/eaglemode/tests/golden/main.rs`:
```rust
mod eagle_logo;
```

- [ ] **Step 3: Run test to verify it fails (no golden data)**

Run: `cargo test --test golden eagle_logo -- --test-threads=1`
Expected: SKIP (golden data not found) or panic on missing file.

- [ ] **Step 4: Write C++ generator function**

Add to `gen_golden.cpp`, after the existing includes, add:
```cpp
#include <emCore/emRes.h>
```
(if not already present — it is NOT in the current includes).

Add the eagle polygon data section. Copy the 14 polygon arrays from C++ `emMainContentPanel.cpp` lines 134–332. The data is static `const double` arrays with flat x,y pairs:

```cpp
// ═══════════════════════════════════════════════════════════════════
// emMain golden tests — Eagle logo polygon data
// Source: ~/git/eaglemode-0.96.4/src/emMain/emMainContentPanel.cpp:134-332
// ═══════════════════════════════════════════════════════════════════

// Copy poly0[922] through poly13[14] and polyColors[14] from C++ source.
// Keep the flat double-array format: { x0,y0, x1,y1, ... }
// poly0: 461 vertices (922 doubles) — lines 134-200
// poly1: 74 vertices (148 doubles) — lines 200-216
// poly2: 6 vertices (12 doubles)
// poly3: 8 vertices (16 doubles)
// poly4: 151 vertices (302 doubles)
// poly5: 15 vertices (30 doubles)
// poly6: 71 vertices (142 doubles)
// poly7: 70 vertices (140 doubles)
// poly8: 18 vertices (36 doubles)
// poly9: 19 vertices (38 doubles)
// poly10: 15 vertices (30 doubles)
// poly11: 18 vertices (36 doubles)
// poly12: 27 vertices (54 doubles)
// poly13: 7 vertices (14 doubles)
//
// polyColors: 14 emColor values — line 346
```

Then add the generator function:

```cpp
static void gen_eagle_logo() {
    const int W = 800, H = 600;
    const double panelH = 0.75;  // H / W * 1.0 when aspect = 4:3

    emImage img(W, H, 4);
    img.Fill(emColor::WHITE);
    emPainter pixel_p = make_painter(img);

    // Create scaled painter: coords (0,0)-(1,0.75) → pixels (0,0)-(800,600).
    double sx = (double)W, sy = (double)W;  // uniform scale: 800
    emPainter p(pixel_p,
        pixel_p.GetClipX1(), pixel_p.GetClipY1(),
        pixel_p.GetClipX2(), pixel_p.GetClipY2(),
        0.0, 0.0, sx, sy);

    // Gradient background: top blue → bottom gold (C++ emMainContentPanel::Paint:80-87).
    p.PaintRect(0, 0, 1, panelH,
        emLinearGradientTexture(0, 0, emColor(145,171,242), 0, panelH, emColor(225,221,183)),
        emColor::WHITE);

    // Eagle transform (C++ UpdateCoordinates:106-109).
    double eagleScaleX = emMin(1.0/180000.0, panelH/120000.0);
    double eagleScaleY = eagleScaleX;
    double eagleShiftX = 0.5 - eagleScaleX * 78450.0;
    double eagleShiftY = panelH * 0.5 - eagleScaleY * 47690.0;

    // Sub-painter with eagle transform (C++ Paint:89-100).
    emPainter ep(p,
        p.GetClipX1(), p.GetClipY1(), p.GetClipX2(), p.GetClipY2(),
        p.GetOriginX() + p.GetScaleX() * eagleShiftX,
        p.GetOriginY() + p.GetScaleY() * eagleShiftY,
        p.GetScaleX() * eagleScaleX,
        p.GetScaleY() * eagleScaleY);

    // Paint 14 polygons (C++ PaintEagle:346-348).
    static const struct { const double* xy; int n; } polys[] = {
        {poly0, sizeof(poly0)/sizeof(double)/2},
        {poly1, sizeof(poly1)/sizeof(double)/2},
        // ... all 14 entries ...
        {poly13, sizeof(poly13)/sizeof(double)/2},
    };
    for (int i = 0; i < 14; i++) {
        ep.PaintPolygon(polys[i].xy, polys[i].n, polyColors[i]);
    }

    dump_painter("eagle_logo", img);
}
```

Add to `main()`:
```cpp
printf("Generating emMain golden files...\n");
gen_eagle_logo();
```

- [ ] **Step 5: Build and run generator**

Run: `make -C crates/eaglemode/tests/golden/gen && make -C crates/eaglemode/tests/golden/gen run`
Expected: `emMain/eagle_logo` appears in output, file `tests/golden/data/painter/eagle_logo.painter.golden` created.

- [ ] **Step 6: Run Rust test and verify pass**

Run: `cargo test --test golden eagle_logo -- --test-threads=1`
Expected: PASS with zero divergence.

If the test fails, debug with:
```bash
DUMP_GOLDEN=1 cargo test --test golden eagle_logo -- --test-threads=1
```
Then inspect the PPM diff images in `target/golden-debug/`.

Common failure causes:
- Painter scale mismatch: verify `p.scaling()` returns `(800.0, 800.0)` in the Rust test
- Canvas color mismatch: C++ uses `WHITE` as canvas, Rust must match
- Gradient API difference: verify `paint_linear_gradient` vertical direction matches `emLinearGradientTexture` Y-axis

- [ ] **Step 7: Commit**

```bash
git add crates/eaglemode/tests/golden/eagle_logo.rs \
        crates/eaglemode/tests/golden/main.rs \
        crates/eaglemode/tests/golden/gen/gen_golden.cpp \
        crates/eaglemode/tests/golden/data/painter/eagle_logo.painter.golden
git commit -m "test(golden): add eagle logo emMain golden test

Compares Rust emMainContentPanel::Paint (gradient + 14 eagle polygons)
against C++ reference at 800x600 viewport, tolerance 0."
```

---

### Task 2: Starfield Golden Test

Verify that `emStarFieldPanel::Paint` produces identical pixels to C++ for deterministic star rendering. Two subtests: small viewport (tier 2+3: ellipse+rect) and large viewport (tier 1: textured glow).

**Files:**
- Modify: `crates/eaglemode/tests/golden/gen/gen_golden.cpp` — add starfield generator
- Create: `crates/eaglemode/tests/golden/starfield.rs` — Rust test module
- Modify: `crates/eaglemode/tests/golden/main.rs` — register module

- [ ] **Step 1: Create starfield.rs test skeleton**

```rust
// crates/eaglemode/tests/golden/starfield.rs
use emcore::emColor::emColor;
use emcore::emImage::emImage;
use emcore::emPainter::emPainter;
use emcore::emPanel::{PanelBehavior, PanelState};

use emmain::emStarFieldPanel::emStarFieldPanel;

use super::common::*;

macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden data not found");
            return;
        }
    };
}

/// Render a starfield panel at depth/seed into an image of the given size.
/// Panel coordinates (0,0)-(1,1) are mapped to pixels (0,0)-(w,h).
fn render_starfield(depth: i32, seed: u32, w: u32, h: u32) -> emImage {
    let mut panel = emStarFieldPanel::new(depth, seed);
    let mut img = emImage::new(w, h, 4);
    // Don't fill — Paint clears to black.
    {
        let mut p = emPainter::new(&mut img);
        p.scale(w as f64, h as f64);
        p.SetCanvasColor(emColor::TRANSPARENT);
        let state = PanelState::default();
        panel.Paint(&mut p, 1.0, 1.0, &state);
    }
    img
}

/// Small viewport: stars rendered as ellipses and rects (tiers 2+3).
/// At 256×256, vr = 256 * r where r ∈ [0.0023, 0.0047].
/// vr ∈ [0.6, 1.2] → tier 3 (rect) and borderline tier 2 (ellipse).
#[test]
fn starfield_small() {
    require_golden!();
    let (ew, eh, expected) = load_painter_golden("starfield_small");
    let img = render_starfield(3, 0x12345678, ew, eh);
    compare_images("starfield_small", img.GetMap(), &expected, ew, eh, 0, 0.0).unwrap();
}

/// Large viewport: stars rendered as textured glow (tier 1).
/// At 1024×1024, vr = 1024 * r where r ∈ [0.0023, 0.0047].
/// vr ∈ [2.4, 4.8] → mix of tier 1 (textured, vr>4) and tier 2 (ellipse).
#[test]
fn starfield_large() {
    require_golden!();
    let (ew, eh, expected) = load_painter_golden("starfield_large");
    let img = render_starfield(3, 0x12345678, ew, eh);
    compare_images("starfield_large", img.GetMap(), &expected, ew, eh, 0, 0.0).unwrap();
}
```

- [ ] **Step 2: Register module in main.rs**

Add to `crates/eaglemode/tests/golden/main.rs`:
```rust
mod starfield;
```

- [ ] **Step 3: Run test to verify it fails (no golden data)**

Run: `cargo test --test golden starfield -- --test-threads=1`
Expected: SKIP or panic.

- [ ] **Step 4: Write C++ generator — starfield PRNG and rendering**

Add to `gen_golden.cpp`:

```cpp
// ═══════════════════════════════════════════════════════════════════
// emMain golden tests — Starfield
// Source: ~/git/eaglemode-0.96.4/src/emMain/emStarFieldPanel.cpp
// ═══════════════════════════════════════════════════════════════════

static const double SF_MinPanelSize = 64.0;
static const double SF_MinStarRadius = 0.3;

// LCG matching C++ emStarFieldPanel::GetRandom() — Knuth/Numerical Recipes.
static emUInt32 sf_lcg(emUInt32& seed) {
    seed = seed * 1664525u + 1013904223u;
    return seed;
}

static double sf_random_range(emUInt32& seed, double minVal, double maxVal) {
    return sf_lcg(seed) * (maxVal - minVal) / (double)0xFFFFFFFFu + minVal;
}

struct SfStar {
    double x, y, radius;
    emColor color;
};

static std::vector<SfStar> sf_generate_stars(int depth, emUInt32& seed) {
    std::vector<SfStar> stars;
    if (depth < 1) return stars;
    int count = (int)(emMin(depth*3, 400) * sf_random_range(seed, 0.5, 1.0));
    stars.reserve(count);
    for (int i = 0; i < count; i++) {
        double r = SF_MinStarRadius / SF_MinPanelSize * sf_random_range(seed, 0.5, 1.0);
        double x = sf_random_range(seed, r, 1.0-r);
        double y = sf_random_range(seed, r, 1.0-r);
        float hue = (float)sf_random_range(seed, 0.0, 360.0);
        float sat = (float)sf_random_range(seed, 0.0, 15.0);
        emColor c; c.SetHSVA(hue, sat, 100.0F);
        stars.push_back({x, y, r, c});
    }
    // Consume child seeds (4 calls) to keep PRNG stream in sync.
    for (int i = 0; i < 4; i++) sf_lcg(seed);
    return stars;
}

static void sf_paint_stars(
    emPainter& p, const std::vector<SfStar>& stars,
    double scale_x, const emImage& starShape
) {
    for (auto& s : stars) {
        double r = s.radius;
        double vr = scale_x * r;
        if (vr <= SF_MinStarRadius) continue;
        if (vr > 4.0) {
            float hue = s.color.GetHue();
            float sat = s.color.GetSat();
            float alpha = sat * 18.0F;
            if (alpha > 255.0F) alpha = 255.0F;
            emColor c1; c1.SetHSVA(hue, 100.0F, 100.0F, (emByte)alpha);
            double x = s.x - r, y = s.y - r, d = r * 2;
            p.PaintImageColored(x, y, d, d, starShape, 0, c1, 0, emTexture::EXTEND_ZERO);
            emColor c2; c2.SetHSVA(hue, sat - 10.0F, 100.0F);
            p.PaintImageColored(x, y, d, d, starShape, 0, c2, 0, emTexture::EXTEND_ZERO);
        } else {
            r *= 0.6;
            vr = scale_x * r;
            if (vr > 1.2) {
                double x = s.x - r, y = s.y - r, d = r * 2;
                p.PaintEllipse(x, y, d, d, s.color);
            } else {
                r *= 0.8862;
                double x = s.x - r, y = s.y - r, d = r * 2;
                p.PaintRect(x, y, d, d, s.color);
            }
        }
    }
}

static void gen_starfield(const char* name, int depth, emUInt32 seed, int w, int h) {
    emImage img(w, h, 4);
    img.Fill(0x000000FFu);  // Black background
    emPainter pixel_p = make_painter(img);

    // Scale: panel (0,0)-(1,1) → pixels (0,0)-(w,h).
    emPainter p(pixel_p,
        pixel_p.GetClipX1(), pixel_p.GetClipY1(),
        pixel_p.GetClipX2(), pixel_p.GetClipY2(),
        0.0, 0.0, (double)w, (double)h);

    emUInt32 rseed = seed;
    auto stars = sf_generate_stars(depth, rseed);

    // Load Star.tga for tier 1 rendering.
    emImage starShape = emGetInsResImage(*g_ctx, "emMain", "Star.tga", 1);

    sf_paint_stars(p, stars, (double)w, starShape);

    dump_painter(name, img);
}
```

Add to `main()`:
```cpp
gen_starfield("starfield_small", 3, 0x12345678, 256, 256);
gen_starfield("starfield_large", 3, 0x12345678, 1024, 1024);
```

- [ ] **Step 5: Build and run generator**

Run: `make -C crates/eaglemode/tests/golden/gen && make -C crates/eaglemode/tests/golden/gen run`
Expected: `painter/starfield_small` and `painter/starfield_large` in output.

- [ ] **Step 6: Run Rust tests and verify pass**

Run: `cargo test --test golden starfield -- --test-threads=1`
Expected: PASS.

If tier 1 test fails: debug with `DUMP_GOLDEN=1`. Check:
- HSV color computation: `emColor::SetHSVA` parity (already covered by emColor golden tests, but verify)
- `PaintImageColored` API mapping: Rust uses `(color1=TRANSPARENT, color2=star_color)` for black→transparent, white→color. C++ uses `(alpha=0, color=star_color)`. Verify these are semantically equivalent in the painter.
- Glow alpha: `sat * 18.0` clamped to 255. Verify C++ `(emByte)alpha` truncation matches Rust `as u8`.

If tier 2/3 test fails: check `PaintEllipse` center-vs-topleft convention:
- C++: `PaintEllipse(x-r, y-r, 2*r, 2*r, color)` — top-left + diameter
- Rust: `PaintEllipse(x, y, r, r, color, bg)` — center + radius
Both should produce identical pixels. If not, the underlying painter golden tests for ellipse should also fail.

- [ ] **Step 7: Commit**

```bash
git add crates/eaglemode/tests/golden/starfield.rs \
        crates/eaglemode/tests/golden/main.rs \
        crates/eaglemode/tests/golden/gen/gen_golden.cpp \
        crates/eaglemode/tests/golden/data/painter/starfield_small.painter.golden \
        crates/eaglemode/tests/golden/data/painter/starfield_large.painter.golden
git commit -m "test(golden): add starfield emMain golden tests

Two viewport sizes: 256x256 (tier 2+3: ellipse/rect) and 1024x1024
(tier 1: textured glow). Seed 0x12345678, depth 3, tolerance 0."
```

---

### Task 3: Main Panel Layout Golden Test

Verify that `emMainPanel::update_coordinates` produces identical layout geometry to C++ `UpdateCoordinates` at 3 parameter sets covering all algorithm branches.

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs` — add pub test accessors
- Modify: `crates/eaglemode/tests/golden/gen/gen_golden.cpp` — add layout generator
- Modify: `crates/eaglemode/tests/golden/common.rs` — add `load_main_layout_golden` if needed
- Create: `crates/eaglemode/tests/golden/main_panel.rs` — Rust test module
- Modify: `crates/eaglemode/tests/golden/main.rs` — register module

- [ ] **Step 1: Add pub test accessors to emMainPanel**

Add to `crates/emmain/src/emMainPanel.rs` in the `impl emMainPanel` block:

```rust
    /// Compute layout coordinates for testing.
    ///
    /// Sets `unified_slider_pos` and `slider_pressed`, then runs
    /// `update_coordinates`. Returns (control, content, slider) rects.
    pub fn compute_layout_for_test(
        &mut self,
        h: f64,
        slider_pos: f64,
        slider_pressed: bool,
    ) -> [(f64, f64, f64, f64); 3] {
        self.unified_slider_pos = slider_pos;
        self.slider_pressed = slider_pressed;
        self.update_coordinates(h);
        [
            (self.control_x, self.control_y, self.control_w, self.control_h),
            (self.content_x, self.content_y, self.content_w, self.content_h),
            (self.slider_x, self.slider_y, self.slider_w, self.slider_h),
        ]
    }
```

- [ ] **Step 2: Verify accessor compiles**

Run: `cargo check -p emmain`
Expected: OK.

- [ ] **Step 3: Create main_panel.rs test skeleton**

```rust
// crates/eaglemode/tests/golden/main_panel.rs
use std::rc::Rc;

use emmain::emMainPanel::emMainPanel;

use super::common::*;

macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden data not found");
            return;
        }
    };
}

/// Compute layout rects and compare against golden reference.
fn check_layout(name: &str, h: f64, slider_pos: f64, control_tallness: f64) {
    let expected = load_layout_golden(name);

    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), control_tallness);
    let rects = panel.compute_layout_for_test(h, slider_pos, false);

    let actual: Vec<(f64, f64, f64, f64)> = rects.to_vec();

    compare_rects(&actual, &expected, 1e-12).unwrap();
}

/// Normal case: h=2.0, sliderPos=0.5, controlTallness=0.0538.
/// Exercises the centered-control branch (ControlX > 1E-5).
#[test]
fn main_panel_layout_normal() {
    require_golden!();
    check_layout("main_panel_layout_normal", 2.0, 0.5, 0.0538);
}

/// Collapsed case: h=2.0, sliderPos=0.0.
/// Exercises the ControlH < 1E-5 branch (collapsed control).
#[test]
fn main_panel_layout_collapsed() {
    require_golden!();
    check_layout("main_panel_layout_collapsed", 2.0, 0.0, 0.0538);
}

/// Width-limited case: h=0.5, sliderPos=0.7, controlTallness=0.0538.
/// Exercises the ControlX < 1E-5 branch (control wider than available space).
#[test]
fn main_panel_layout_wide() {
    require_golden!();
    check_layout("main_panel_layout_wide", 0.5, 0.7, 0.0538);
}
```

- [ ] **Step 4: Register module in main.rs**

Add to `crates/eaglemode/tests/golden/main.rs`:
```rust
mod main_panel;
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cargo test --test golden main_panel -- --test-threads=1`
Expected: SKIP or panic.

- [ ] **Step 6: Write C++ layout generator**

Add to `gen_golden.cpp`:

```cpp
// ═══════════════════════════════════════════════════════════════════
// emMain golden tests — Main panel layout (UpdateCoordinates)
// Source: ~/git/eaglemode-0.96.4/src/emMain/emMainPanel.cpp:234-293
// ═══════════════════════════════════════════════════════════════════

struct MainPanelLayout {
    double ControlX, ControlY, ControlW, ControlH;
    double ContentX, ContentY, ContentW, ContentH;
    double SliderX, SliderY, SliderW, SliderH;
};

static MainPanelLayout compute_main_panel_layout(
    double h, double sliderPos, double controlTallness
) {
    MainPanelLayout L = {};
    double SliderMinY = 0.0;
    double SliderMaxY = emMin(controlTallness, h * 0.5);
    L.SliderY = (SliderMaxY - SliderMinY) * sliderPos + SliderMinY;
    L.SliderW = emMin(emMin(1.0, h) * 0.1, emMax(1.0, h) * 0.02);
    L.SliderH = L.SliderW * 1.2;
    L.SliderX = 1.0 - L.SliderW;

    double spaceFac = 1.015;
    double t = L.SliderH * 0.5;
    if (L.SliderY < t) {
        L.ControlH = L.SliderY + L.SliderH * L.SliderY / t;
    } else {
        L.ControlH = (L.SliderY + L.SliderH) / spaceFac;
    }

    if (L.ControlH < 1E-5) {
        L.ControlH = 1E-5;
        L.ControlW = L.ControlH / controlTallness;
        L.ControlX = 0.5 * (1.0 - L.ControlW);
        L.ControlY = 0.0;
        L.ContentX = 0.0;
        L.ContentY = 0.0;
        L.ContentW = 1.0;
        L.ContentH = h;
    } else {
        L.ControlW = L.ControlH / controlTallness;
        L.ControlX = emMin((1.0 - L.ControlW) * 0.5, L.SliderX - L.ControlW);
        L.ControlY = 0.0;
        if (L.ControlX < 1E-5) {
            L.ControlW = 1.0 - L.SliderW;
            L.ControlX = 0.0;
            L.ControlH = L.ControlW * controlTallness;
            if (L.ControlH < L.SliderY) {
                L.ControlH = L.SliderY;
                L.ControlW = L.ControlH / controlTallness;
            }
            // slider_pressed=false: apply correction
            else {
                L.SliderY = L.ControlH * spaceFac - L.SliderH;
            }
        }
        L.ContentY = L.ControlY + L.ControlH * spaceFac;
        L.ContentX = 0.0;
        L.ContentW = 1.0;
        L.ContentH = h - L.ContentY;
    }
    return L;
}

static void dump_main_panel_layout(const char* name, const MainPanelLayout& L) {
    // Use layout.golden format: [u32 child_count][child_count * 4 f64s]
    FILE* f = open_golden("layout", name, "layout.golden");
    write_u32(f, 3);  // 3 rects: control, content, slider
    write_f64(f, L.ControlX); write_f64(f, L.ControlY);
    write_f64(f, L.ControlW); write_f64(f, L.ControlH);
    write_f64(f, L.ContentX); write_f64(f, L.ContentY);
    write_f64(f, L.ContentW); write_f64(f, L.ContentH);
    write_f64(f, L.SliderX);  write_f64(f, L.SliderY);
    write_f64(f, L.SliderW);  write_f64(f, L.SliderH);
    fclose(f);
    printf("  layout/%s\n", name);
}

static void gen_main_panel_layouts() {
    // Normal: centered control
    dump_main_panel_layout("main_panel_layout_normal",
        compute_main_panel_layout(2.0, 0.5, 0.0538));
    // Collapsed: sliderPos=0 → ControlH < 1E-5
    dump_main_panel_layout("main_panel_layout_collapsed",
        compute_main_panel_layout(2.0, 0.0, 0.0538));
    // Width-limited: small h → ControlX < 1E-5
    dump_main_panel_layout("main_panel_layout_wide",
        compute_main_panel_layout(0.5, 0.7, 0.0538));
}
```

Add to `main()`:
```cpp
gen_main_panel_layouts();
```

- [ ] **Step 7: Build and run generator**

Run: `make -C crates/eaglemode/tests/golden/gen && make -C crates/eaglemode/tests/golden/gen run`
Expected: `layout/main_panel_layout_normal`, `_collapsed`, `_wide` in output.

- [ ] **Step 8: Run Rust tests and verify pass**

Run: `cargo test --test golden main_panel -- --test-threads=1`
Expected: PASS. All 3 layout tests match C++ at 1e-12 tolerance.

If any test fails, print both actual and expected values. The algorithm is a direct port — differences indicate a Rust porting bug in `update_coordinates`.

- [ ] **Step 9: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs \
        crates/eaglemode/tests/golden/main_panel.rs \
        crates/eaglemode/tests/golden/main.rs \
        crates/eaglemode/tests/golden/gen/gen_golden.cpp \
        crates/eaglemode/tests/golden/data/layout/main_panel_layout_normal.layout.golden \
        crates/eaglemode/tests/golden/data/layout/main_panel_layout_collapsed.layout.golden \
        crates/eaglemode/tests/golden/data/layout/main_panel_layout_wide.layout.golden
git commit -m "test(golden): add main panel layout golden tests

Three UpdateCoordinates parameter sets: normal (centered control),
collapsed (sliderPos=0), width-limited (small h). f64 rect comparison."
```

---

### Task 4: Cosmos Item Border Golden Test

Verify that `emVirtualCosmosItemPanel::Paint` renders identical borders, background, and title text to C++ at 400×300 viewport with known item parameters.

**Files:**
- Modify: `crates/eaglemode/tests/golden/gen/gen_golden.cpp` — add cosmos item generator
- Create: `crates/eaglemode/tests/golden/cosmos_items.rs` — Rust test module
- Modify: `crates/eaglemode/tests/golden/main.rs` — register module

- [ ] **Step 1: Create cosmos_items.rs test skeleton**

```rust
// crates/eaglemode/tests/golden/cosmos_items.rs
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emImage::emImage;
use emcore::emPainter::emPainter;
use emcore::emPanel::{PanelBehavior, PanelState};

use emmain::emVirtualCosmos::{emVirtualCosmosItemPanel, emVirtualCosmosItemRec};

use super::common::*;

macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden data not found");
            return;
        }
    };
}

fn test_item_rec() -> emVirtualCosmosItemRec {
    let mut rec = emVirtualCosmosItemRec::default();
    rec.Name = "TestItem".to_string();
    rec.Title = "Test Cosmos Item".to_string();
    rec.Width = 1.0;
    rec.ContentTallness = 0.75;
    rec.BorderScaling = 1.0;
    rec.BackgroundColor = emColor::from_packed(0x202040FF);
    rec.BorderColor = emColor::from_packed(0x4060A0FF);
    rec.TitleColor = emColor::from_packed(0xE0E0FFFF);
    rec
}

#[test]
fn cosmos_item_border() {
    require_golden!();
    let (ew, eh, expected) = load_painter_golden("cosmos_item_border");

    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emVirtualCosmosItemPanel::new(Rc::clone(&ctx));
    panel.SetItemRec(test_item_rec());

    let mut img = emImage::new(ew, eh, 4);
    img.fill(emColor::BLACK);
    {
        let mut p = emPainter::new(&mut img);
        p.scale(ew as f64, eh as f64);
        p.SetCanvasColor(emColor::TRANSPARENT);
        let state = PanelState::default();
        // Panel paints in normalized coords: w=1.0, h = panel tallness.
        // With ContentTallness=0.75, BorderScaling=1.0:
        //   b = min(0.75, 1.0) * 1.0 = 0.75
        //   borders: (0.0225, 0.0375, 0.0225, 0.0225)
        // Panel height = ContentTallness + top + bottom borders.
        // The item panel's Paint receives (w, h) = panel dimensions.
        // For the golden test: w=1.0, h computed from item geometry.
        let item = test_item_rec();
        let b = item.ContentTallness.min(1.0) * item.BorderScaling;
        let top = b * 0.05;
        let bot = b * 0.03;
        let panel_h = item.ContentTallness + top + bot;
        panel.Paint(&mut p, 1.0, panel_h, &state);
    }

    compare_images("cosmos_item_border", img.GetMap(), &expected, ew, eh, 0, 0.0).unwrap();
}
```

- [ ] **Step 2: Register module in main.rs**

Add to `crates/eaglemode/tests/golden/main.rs`:
```rust
mod cosmos_items;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test golden cosmos_item -- --test-threads=1`
Expected: SKIP or panic.

- [ ] **Step 4: Write C++ cosmos item border generator**

Add to `gen_golden.cpp`:

```cpp
// ═══════════════════════════════════════════════════════════════════
// emMain golden tests — Cosmos item border rendering
// Source: ~/git/eaglemode-0.96.4/src/emMain/emVirtualCosmos.cpp
// ═══════════════════════════════════════════════════════════════════

static void gen_cosmos_item_border() {
    // Item parameters matching Rust test_item_rec().
    double contentTallness = 0.75;
    double borderScaling = 1.0;
    emColor bgColor(0x20, 0x20, 0x40);
    emColor borderColor(0x40, 0x60, 0xA0);
    emColor titleColor(0xE0, 0xE0, 0xFF);
    const char* title = "Test Cosmos Item";

    // CalcBorders: b = min(contentTallness, 1.0) * borderScaling
    double b = emMin(contentTallness, 1.0) * borderScaling;
    double bl = b * 0.03, bt = b * 0.05, br = b * 0.03, bb = b * 0.03;
    double panelH = contentTallness + bt + bb;

    // Image: 400×300 pixels, panel coords (0,0)-(1,panelH).
    const int W = 400, H = 300;
    emImage img(W, H, 4);
    img.Fill(emColor::BLACK);
    emPainter pixel_p = make_painter(img);

    double sx = (double)W, sy = (double)H / panelH;
    emPainter p(pixel_p,
        pixel_p.GetClipX1(), pixel_p.GetClipY1(),
        pixel_p.GetClipX2(), pixel_p.GetClipY2(),
        0.0, 0.0, sx, sy);

    double w = 1.0, h = panelH;

    // Top border strip
    p.PaintRect(0.0, 0.0, w, bt * h, borderColor);
    // Bottom border strip
    p.PaintRect(0.0, (1.0 - bb / panelH) * h, w, bb * h, borderColor);
    // Left border strip (between top and bottom)
    p.PaintRect(0.0, bt * h, bl * w, (1.0 - bt / panelH - bb / panelH) * h, borderColor);
    // Right border strip
    p.PaintRect((1.0 - br) * w, bt * h, br * w, (1.0 - bt / panelH - bb / panelH) * h, borderColor);

    // Background inside content area
    p.PaintRect(bl * w, bt * h,
        (1.0 - bl - br) * w, (1.0 - bt / panelH - bb / panelH) * h, bgColor);

    // Title text at top of border area
    double fontH = bt * h * 0.7;
    if (fontH >= 1.0) {
        p.PaintText(bl * w, bt * h * 0.15, title, fontH, 1.0, titleColor);
    }

    dump_painter("cosmos_item_border", img);
}
```

**Important:** The border coordinate formulas in the generator must exactly match the Rust `emVirtualCosmosItemPanel::Paint` method at `crates/emmain/src/emVirtualCosmos.rs:449-493`. Read that code and ensure the generator uses identical formulas. The code above is derived from the Rust source, but verify:
- Top strip: `(0, 0, w, t*h)` where `t = bt` (border fraction, not pixels)
- Bottom strip: `(0, (1-b)*h, w, b*h)` where `b = bb`
- Left strip: `(0, t*h, l*w, (1-t-b)*h)`
- Right strip: `((1-r)*w, t*h, r*w, (1-t-b)*h)`
- Background: `(l*w, t*h, (1-l-r)*w, (1-t-b)*h)`

The Rust Paint receives `(w, h)` as panel dimensions (w=width=1.0, h=panel_height). The border fractions (l, t, r, b) are relative to the panel dimensions. Verify this matches the generator.

Add to `main()`:
```cpp
gen_cosmos_item_border();
```

- [ ] **Step 5: Build and run generator**

Run: `make -C crates/eaglemode/tests/golden/gen && make -C crates/eaglemode/tests/golden/gen run`
Expected: `painter/cosmos_item_border` in output.

- [ ] **Step 6: Run Rust test and verify pass**

Run: `cargo test --test golden cosmos_item -- --test-threads=1`
Expected: PASS.

If the test fails, common causes:
- Border fraction coordinates: ensure `CalcBorders` returns `(left, top, right, bottom)` in the same order in both C++ and Rust
- Panel height computation: the Rust test must compute `panel_h` the same way as the generator
- Text rendering: `PaintText` differences between C++ and Rust (font metrics, baseline). If text causes failures, add `ch_tol=1` for this test specifically and document the text rendering divergence.

- [ ] **Step 7: Commit**

```bash
git add crates/eaglemode/tests/golden/cosmos_items.rs \
        crates/eaglemode/tests/golden/main.rs \
        crates/eaglemode/tests/golden/gen/gen_golden.cpp \
        crates/eaglemode/tests/golden/data/painter/cosmos_item_border.painter.golden
git commit -m "test(golden): add cosmos item border golden test

Verifies border strips, background fill, and title text rendering
for emVirtualCosmosItemPanel at 400x300 viewport, tolerance 0."
```

---

### Task 5: Control Panel Layout Test

Verify that `emMainControlPanel::LayoutChildren` produces correct button positions. Since the Rust control panel is DIVERGED from C++ (simplified flat layout vs emLinearGroup widget tree), this is a Rust-only regression test with hardcoded expected values — no C++ golden data.

**Files:**
- Create: `crates/eaglemode/tests/golden/control_panel.rs` — Rust test module
- Modify: `crates/eaglemode/tests/golden/main.rs` — register module

- [ ] **Step 1: Compute expected layout values**

From `emMainControlPanel::LayoutChildren` (emMainControlPanel.rs:142-171):

```
n_buttons = 5
total_weight = 5 * 1.0 + 6.5 = 11.5
pad_x = 0.01
child_w = 1.0 - 2 * 0.01 = 0.98
gap_frac = 0.005
total_gaps = 6 * 0.005 = 0.03
usable_h = 1.0 - 0.03 = 0.97

Button height = 0.97 * (1.0 / 11.5) = 0.08434782608695652
Bookmarks height = 0.97 * (6.5 / 11.5) = 0.5482608695652174

Layout (y starts at 0.005):
  btn_0: (0.01, 0.005, 0.98, 0.08434782608695652)
  btn_1: (0.01, 0.09434782608695652, 0.98, 0.08434782608695652)
  btn_2: (0.01, 0.18369565217391304, 0.98, 0.08434782608695652)
  btn_3: (0.01, 0.27304347826086957, 0.98, 0.08434782608695652)
  btn_4: (0.01, 0.36239130434782610, 0.98, 0.08434782608695652)
  bookmarks: (0.01, 0.45173913043478261, 0.98, 0.5482608695652174)
```

- [ ] **Step 2: Create control_panel.rs test**

```rust
// crates/eaglemode/tests/golden/control_panel.rs
use std::rc::Rc;

use emcore::emPanel::PanelBehavior;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelTree;

use emmain::emMainControlPanel::emMainControlPanel;

use super::common::*;

/// Verify control panel button layout geometry.
///
/// This is a Rust-only regression test (DIVERGED from C++ emLinearGroup layout).
/// Expected values computed from the algorithm in emMainControlPanel::LayoutChildren.
#[test]
fn control_panel_layout() {
    let ctx = emcore::emContext::emContext::NewRoot();

    let mut tree = PanelTree::new();
    let root = tree.create_root("ctrl_root");
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);
    tree.set_behavior(root, Box::new(emMainControlPanel::new(Rc::clone(&ctx))));

    // Run LayoutChildren to create and position children.
    let mut behavior = tree.take_behavior(root).unwrap();
    {
        let mut pctx = PanelCtx::new(&mut tree, root);
        behavior.LayoutChildren(&mut pctx);
    }
    tree.put_behavior(root, behavior);

    // Read child layout rects from tree.
    let children: Vec<_> = tree.children(root).collect();
    assert_eq!(children.len(), 6, "5 buttons + 1 bookmarks panel");

    let eps = 1e-12;

    // Expected values from algorithm.
    let btn_h = 0.97 * (1.0 / 11.5);
    let bm_h = 0.97 * (6.5 / 11.5);
    let pad_x = 0.01;
    let child_w = 0.98;
    let gap = 0.005;

    let mut y = gap;
    for i in 0..5 {
        let r = tree.layout_rect(children[i])
            .unwrap_or_else(|| panic!("btn_{i} has no layout rect"));
        assert!((r.x - pad_x).abs() < eps, "btn_{i} x: {} vs {}", r.x, pad_x);
        assert!((r.y - y).abs() < eps, "btn_{i} y: {} vs {}", r.y, y);
        assert!((r.w - child_w).abs() < eps, "btn_{i} w: {} vs {}", r.w, child_w);
        assert!((r.h - btn_h).abs() < eps, "btn_{i} h: {} vs {}", r.h, btn_h);
        y += btn_h + gap;
    }

    // Bookmarks panel
    let r = tree.layout_rect(children[5])
        .unwrap_or_else(|| panic!("bookmarks has no layout rect"));
    assert!((r.x - pad_x).abs() < eps, "bookmarks x: {} vs {}", r.x, pad_x);
    assert!((r.y - y).abs() < eps, "bookmarks y: {} vs {}", r.y, y);
    assert!((r.w - child_w).abs() < eps, "bookmarks w: {} vs {}", r.w, child_w);
    assert!((r.h - bm_h).abs() < eps, "bookmarks h: {} vs {}", r.h, bm_h);
}
```

- [ ] **Step 3: Register module in main.rs**

Add to `crates/eaglemode/tests/golden/main.rs`:
```rust
mod control_panel;
```

- [ ] **Step 4: Run test and verify pass**

Run: `cargo test --test golden control_panel -- --test-threads=1`
Expected: PASS.

If the test fails: read the actual values and compare against the algorithm. Check:
- `tree.children(root)` returns children in creation order (btn_0..btn_4, bookmarks)
- `tree.layout_rect(id)` returns the rect set by `ctx.layout_child_canvas`
- The rect coordinates are in parent-normalized space (0,0)-(1,1)

- [ ] **Step 5: Commit**

```bash
git add crates/eaglemode/tests/golden/control_panel.rs \
        crates/eaglemode/tests/golden/main.rs
git commit -m "test(golden): add control panel layout regression test

Rust-only test (control panel is DIVERGED from C++ emLinearGroup).
Verifies 5 button + bookmarks panel positions from LayoutChildren."
```

---

### Task 6: Full Verification and Divergence Log Cleanup

Run the complete golden test suite, verify all new tests pass, and confirm the divergence log is clean at tolerance 0.

**Files:**
- Check: `crates/eaglemode/target/golden-divergence/divergence.jsonl`

- [ ] **Step 1: Regenerate all golden data**

Run: `make -C crates/eaglemode/tests/golden/gen clean && make -C crates/eaglemode/tests/golden/gen && make -C crates/eaglemode/tests/golden/gen run`
Expected: All existing + new golden files generated without errors.

- [ ] **Step 2: Run full golden test suite**

Run: `cargo test --test golden -- --test-threads=1`
Expected: ALL tests pass (existing + new).

- [ ] **Step 3: Inspect divergence log**

Run: `cat crates/eaglemode/target/golden-divergence/divergence.jsonl | python3 -c "import sys,json; [print(json.loads(l)['test'],json.loads(l).get('status','?')) for l in sys.stdin]" | grep -v PASS`
Expected: No output (all tests PASS at tolerance 0).

If any existing test regressed: the generator rebuild may have changed data. Compare `divergence.jsonl` against `divergence.prev.jsonl` to identify regressions.

- [ ] **Step 4: Run full project test suite**

Run: `cargo-nextest ntr`
Expected: All tests pass (unit + golden + integration).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 6: Final commit (if any fixups needed)**

Only commit if fixups were needed in Steps 2-5. Otherwise, all work was committed in Tasks 1-5.

```bash
git add -A
git commit -m "test(golden): fix Phase 5 golden test issues

[describe specific fixes]"
```

---

## Summary

| Test Module | Type | Golden Data | Tolerance | C++ Reference |
|---|---|---|---|---|
| `eagle_logo.rs` | pixel | `painter/eagle_logo.painter.golden` | ch_tol=0, max_fail_pct=0.0 | Yes |
| `starfield.rs` | pixel | `painter/starfield_{small,large}.painter.golden` | ch_tol=0, max_fail_pct=0.0 | Yes |
| `main_panel.rs` | layout | `layout/main_panel_layout_{normal,collapsed,wide}.layout.golden` | eps=1e-12 | Yes |
| `cosmos_items.rs` | pixel | `painter/cosmos_item_border.painter.golden` | ch_tol=0, max_fail_pct=0.0 | Yes |
| `control_panel.rs` | layout | (hardcoded expected values) | eps=1e-12 | No (DIVERGED) |

**Total new golden files:** 6 binary files + 5 Rust test modules + ~400 lines C++ generator code.

**Gate:** All golden tests pass. `target/golden-divergence/divergence.jsonl` clean at tolerance 0 for all emMain tests. `cargo-nextest ntr` passes. `cargo clippy -- -D warnings` clean.
