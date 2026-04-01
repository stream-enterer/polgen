# Gap: Deferred Work from Main App Launch

**Filed:** 2026-03-31
**Affects:** Full Eagle Mode feature parity
**Severity:** Low-Medium — app is functional without these, but they're needed for complete parity

## Context

The 2026-03-31 main app launch design intentionally excludes the following work to keep scope focused on getting eaglemode running with the ported apps (emStocks, emFileMan, emDirPanel). This document tracks everything deferred so nothing is lost.

## Deferred Items

### 1. Viewer Plugins (No Ported Plugins for Common File Types)

The C++ Eagle Mode ships 30+ file format plugins. None of these are ported. Navigating the filesystem shows "unsupported format" for any non-directory, non-.emStocks file.

**Plugins not yet ported (grouped by priority):**

**High value (basic browsing):**
- `emText` — plain text viewer (`.txt`, `.log`, `.conf`, etc.)
- `emPng` — PNG image viewer
- `emJpeg` — JPEG image viewer

**Medium value (common formats):**
- `emBmp` — BMP images
- `emGif` — GIF images
- `emSvg` — SVG vector graphics
- `emPdf` — PDF viewer
- `emJson` — JSON viewer
- `emWebp` — WebP images

**Low value (niche formats):**
- `emTga` — TGA images
- `emPcx` — PCX images
- `emPnm` — PNM images
- `emRas` — RAS images
- `emRgb` — RGB images
- `emTiff` — TIFF images
- `emIlbm` — ILBM images
- `emXbm` — XBM images
- `emXpm` — XPM images

**C++ source:** `~/git/eaglemode-0.96.4/include/em{Text,Png,Jpeg,...}/`

### 2. App Plugins (Interactive Applications)

The C++ cosmos includes interactive apps. None are ported.

- `emClock` — analog/digital clock
- `emMines` — Minesweeper game
- `emNetwalk` — network puzzle game
- `SilChess` — chess game
- `emFractal` — fractal explorer
- `emHmiDemo` — HMI demonstration

**C++ source:** `~/git/eaglemode-0.96.4/include/em{Clock,Mines,Netwalk,...}/`

### 3. Dynamic Plugin Loading

The current design uses static linking — all plugin functions are compiled into the binary and registered via a static lookup table. The C++ version loads `.so` libraries at runtime from `.emFpPlugin` config files.

**What's needed later:**
- `emFpPluginList` reads `.emFpPlugin` files (already implemented)
- `emTryResolveSymbol` does dynamic symbol lookup (already implemented)
- Wire these together so plugins can be separate `.so` crates loaded at runtime
- Enables third-party plugins and hot-reloading

### 4. Audio/Video Support

- `emAv` — audio/video player plugin
- `emTmpConv` — temporary file conversion pipeline

These depend on external libraries (ffmpeg, etc.) and are the most complex plugins.

### 5. Platform Ports

- `emWnds` — Windows platform abstraction (currently Linux-only via winit)
- `emX11` — X11-specific features beyond what winit provides

The Rust port uses winit which abstracts most of this, but some C++ features (like specific X11 atom handling) may need attention.

### 6. Cosmos Items for Unported Apps

The C++ default cosmos has ~47 `.emVcItem` files. We only ship items for filesystem + stocks. When more apps are ported, their cosmos items should be added back at their original C++ positions (positions are preserved in the C++ reference files).

**C++ item files:** `~/git/eaglemode-0.96.4/etc/emMain/VcItems/`

### 7. emTreeDump and emOsm

- `emTreeDump` — debug tree visualization
- `emOsm` — OpenStreetMap viewer

Both are specialized and lower priority.

### 8. Golden Tests for emMain

The current golden test infrastructure covers emCore rendering. No golden tests exist for emMain-level panels (starfield, cosmos items, control panel). These should be added once the panels are stable to prevent regressions.

### 9. Eagle Logo Rendering

The C++ `emMainContentPanel::PaintEagle()` draws the eagle shape using hundreds of polygon coordinate pairs. The Rust port replaces this with a text placeholder ("Eagle Mode"). The full polygon data is in `~/git/eaglemode-0.96.4/src/emMain/emMainContentPanel.cpp` lines 132-400+.

### 10. Star.tga Textured Star Rendering

The C++ starfield loads `Star.tga` from resources and renders large stars as textured images with HSV-shifted glow layers. The Rust port uses `PaintEllipse` for all star sizes. Needs `emGetInsResImage` / resource loading infrastructure.

**C++ source:** `emStarFieldPanel::PaintOverlay()` in `~/git/eaglemode-0.96.4/src/emMain/emStarFieldPanel.cpp` lines 102-147

### 11. Startup Animation

The C++ `emMainWindow::StartupEngineClass` runs a ~2-second choreographed zoom-in animation from overview to the default visit target (3 phases: fade-in, zoom, settle). The Rust port skips this entirely — the window opens directly at the root view.

**C++ source:** `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp`

### 12. Detached Control Window

The C++ `emMainWindow` creates a separate OS window for the floating sidebar (`emWindow` for detached control panel). The Rust port embeds the control panel directly in the main panel's split layout. The C++ behavior allows the sidebar to be in its own OS window with independent drag/resize.

### 13. Slider Drag Interaction

The `emMainPanel` slider panel renders as a solid color strip but does not respond to mouse drag. The C++ `SliderPanel::Input()` tracks mouse press/drag/release to resize the control/content split, and `DragSlider()` updates `UnifiedSliderPos` and saves to config. Also missing: double-click-to-reset and auto-hide timer (5s delay in fullscreen mode).

**C++ source:** `~/git/eaglemode-0.96.4/src/emMain/emMainPanel.cpp`

### 14. Autoplay View Animator Panel Traversal

The `emAutoplayViewAnimator` state machine structure is ported but all panel-dependent methods are stubs (`SetGoalToItemAt`, `SetGoalToPreviousItemOf`, `SetGoalToNextItemOf`, `SkipToPreviousItem`, `SkipToNextItem`). These need the panel tree traversal API (`GetFirstChild`, `GetNext`, etc.) and `emVisitingViewAnimator` integration.

**C++ source:** `~/git/eaglemode-0.96.4/src/emMain/emAutoplay.cpp`

### 15. IPC Single-Instance

`emMain::CalcServerName()` is ported and derives the server name from hostname + DISPLAY. However, `try_ipc_client()` always returns false (no actual IPC attempt). Needs `emMiniIpc::emMiniIpcClient::TrySend()` and `emMiniIpcServer` to be wired for single-instance behavior.

### 16. emSubViewPanel Integration

The C++ `emMainPanel` uses `emSubViewPanel` for both control and content sides, giving each independent zoom/pan navigation. The Rust port creates `emMainControlPanel` and `emMainContentPanel` as direct children without independent views. This means the control panel zooms with the content — not C++ behavior.

### 17. Items Requiring Review

**emRec color round-trip fidelity:** The bookmark and cosmos item Record implementations serialize colors as `{R G B A}` int sub-structs. This may not match the C++ `emColorRec` format exactly — worth testing with actual `.emVcItem` and `.emBookmarks` files loaded from the C++ installation.

**emBookmarksGroupPanel ownership divergence:** C++ `emBookmarksPanel` is recursive (creates child `emBookmarksPanel` for groups). Rust ownership rules prevent a type from creating children of itself in `LayoutChildren`, so a separate `emBookmarksGroupPanel` type was introduced. This means groups only render one level deep — nested groups within groups won't show their children.

**TicTacToe Easter egg:** The C++ starfield creates a TicTacToe panel at depth > 50 with 1/11213 probability. This is noted in comments but not implemented.

## How to Use This Document

When picking the next work after the main app launch, use this as a menu. Priority order for maximum user impact:

1. `emText` (immediately makes filesystem browsing useful)
2. `emPng` + `emJpeg` (image browsing)
3. `emClock` (visible in cosmos, simple app)
4. Dynamic plugin loading (enables modular builds)
5. Remaining viewers and apps as needed
