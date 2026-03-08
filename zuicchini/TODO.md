# Deferred Items

Tracked here so they don't get forgotten. Sourced from EMCORE_FEATURE_CONTRACT.md
and the parity convergence pipeline.

---

## Crate Separation (deferred)

**Context:** The project hit a classic porting failure mode — simultaneous
redesign and reimplementation caused semantic drift, making bugs
indistinguishable from intentional adaptations. A parity-first pivot
(commit `3c70ad7`) fixed this by matching C++ behavior exactly, then
preserving the 5 original design decisions as mechanism-layer adaptations.

The 5 design decisions (arena panel tree, typed singletons, KDL
serialization, CPU raster + wgpu compositor, winit + wgpu platform) are
all viable and non-conflicting with the parity-verified behavioral code.
They operate at different layers: decisions change *how* things are stored
and connected, parity ensures *what* the code does matches C++.

**Current state:** Everything lives in one crate (`zuicchini`). The
separation already exists logically — the framework (panel, layout, widget,
scheduler) has zero imports from `window/`, and painter has zero knowledge
of tiles or wgpu. The dependency arrow is strictly one-way.

**The split when ready:**

```
zuicchini-core/          (pure CPU, no platform deps)
  foundation/            Color, Image, Rect, Fixed12, TGA
  scheduler/             EngineScheduler, Signal, Timer
  model/                 Context, Record, ConfigModel, FileModel
  input/                 InputEvent, InputState, Cursor, Hotkey
  panel/                 PanelTree, PanelBehavior, View, Animators, VIFs
  layout/                Linear, Raster, Pack
  widget/                Border, Button, TextField, etc.
  render/painter.rs      CPU rasterizer (3,906 lines, parity-verified)
  render/scanline.rs     AA scanline edge rasterizer
  render/interpolation.rs  Image sampling pipeline
  render/bitmap_font.rs  8x14 VGA bitmap font

zuicchini/               (platform integration, thin)
  window/app.rs          winit ApplicationHandler, GpuContext
  window/zui_window.rs   winit Window + wgpu Surface + rendering orchestration
  window/screen.rs       Monitor enumeration via winit
  window/state_saver.rs  Window geometry persistence
  render/compositor.rs   wgpu tile upload + textured quad display (~300 lines)
  render/tile_cache.rs   Tile grid management, dirty tracking (~144 lines)
```

**When to split:** After Phase 5 and 6 are done. Phase 5 adds winit wrappers
that need to expose capabilities to panel behaviors (DPI, mouse warping). Those
API boundaries are easier to iterate on within one crate (`pub(crate)`), then
promote to `pub` across the crate boundary once stable.

**Why split matters:** wgpu is the heaviest dependency. Once past convergence
and into building egopol, you'll iterate on panel behaviors and widget logic
constantly while rarely touching the compositor. Separate compilation saves
rebuild time on every change.

---

## View Animators

- [ ] `SwipingViewAnimator` — touch-drag with spring physics and momentum (needs touch input infrastructure)
- [ ] `MagneticViewAnimator` — snaps view to "best" panel alignment (needs working UI for tuning)

## Widgets

- [ ] `FileSelectionBox` — file browser (only if game needs file open/save)
- [ ] `FileDialog` — wraps FileSelectionBox in a dialog window
- [ ] `CoreConfigPanel` — core settings editor (needs config system fully working)
- [ ] `ErrorPanel` — simple error text display (small effort, useful for debugging)

## Structural Refactors

- [x] Restrict PanelData field visibility — make computed fields (`enabled`, `pending_notices`) and tree-managed fields (`parent`, `first_child`, etc.) non-public after the fix pass settles their access patterns

## Rendering

- [ ] Multi-threaded tile rasterization — parallelize independent dirty tiles across threads (benchmark-driven, threading boundary is well-defined)
- [ ] Sub-pixel AA for non-text operations — route rects/gradients/images through polygon rasterizer with Fixed12 edge coverage; add axis-aligned rect fast path; fix coverage rounding in `make_span` (edge seams) and clamp minimum coverage for thin rects. See `.workflow/dialectic/convergence_ledger.md` for full analysis
- [ ] Glyph content sub-pixel positioning — edge coverage solves rect wiggle but glyphs remain pixel-snapped inside their bounding box. Investigate SDF rendering, boundary-only bilinear, or accept integer glyph positioning as C++ Eagle Mode does
- [ ] 4K paint profiling — `bench_interaction 3840 2160` to check if paint exceeds 16ms budget; if so, scanline rasterizer needs optimization
- [ ] Glyph rasterization cost under complex panel trees — single TestPanel is cheap, but multiple panels with diverse text sizes may stress the glyph cache LRU eviction path

## Window Integration

- [ ] Wire `WindowStateSaver::cycle()` into `App::about_to_wait` — method has correct logic (preserves normal geometry when maximized/fullscreen, saves on change when focused) but is never called. C++ equivalent is auto-driven by `emEngine::Cycle` via the scheduler. Needs an architecture decision: store state savers in `App`, in `ZuiWindow`, or let user code manage them.

## Font System Follow-ups

- [x] Hinted rasterization — skrifa's `HintingInstance` requires per-size instances; currently using `DrawSettings::unhinted`. Add hinting for crisper text at small sizes (no API changes needed)
- [ ] Thread FontCache through PanelBehavior/PanelCtx — when widgets start implementing `PanelBehavior::preferred_size` via the trait (not just inherent methods), the trait signature and PanelCtx need `&FontCache`
- [ ] Variable font weight selection — Inter Variable is embedded but always renders at default weight; expose weight axis via `skrifa::instance::Location`
- [x] Text scroll in TextField — `scroll_x` updated in `paint()` to keep cursor visible
- [ ] i18n shaping verification — rustybuzz handles Arabic/Devanagari/CJK but needs testing with actual multilingual text
