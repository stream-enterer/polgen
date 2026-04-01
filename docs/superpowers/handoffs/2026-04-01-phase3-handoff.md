Execute `/superpowers:writing-plans` for Phase 3 of @docs/superpowers/specs/2026-04-01-parity-completions-design.md

Phase 3 covers items 11 (Startup Animation) and 14 (Autoplay Panel Traversal). Save the plan to `docs/superpowers/plans/2026-04-01-phase3-startup-autoplay.md`.

**Context from prior phases:**

- Phase 1 (complete): emMainPanel rewritten with emSubViewPanel for control/content, full SliderPanel with drag/double-click/shift, StartupOverlayPanel, UpdateCoordinates exact port. `_DragSlider`/`_DoubleClickSlider` are _-prefixed (not yet wired from parent input dispatch). Timer wiring for slider auto-hide is stubbed (needs Cycle integration). `PaintBorderImageSrcRect` was added to emcore.

- Phase 2 (complete): Eagle logo polygons ported (461 vertices verified), Star.tga 3-tier rendering (color1/color2 args were initially inverted — fixed; DIVERGED comment added for Paint-vs-PaintOverlay), IPC client wired via emMiniIpcClient::TrySend, emMain engine struct added (server side stubbed with on_reception handling NewWindow/ReloadFiles/unknown), CreateControlWindow added, VcItem hex color parsing fixed. Star rendering fix resolved 181 lines of pre-existing golden divergence.

**Phase 3 should also complete these Phase 1 deferrals:**
- Remove `_` prefix from `_DragSlider`/`_DoubleClickSlider` and wire them from parent input dispatch
- Complete timer wiring for slider auto-hide (SliderTimer start/cancel in Cycle)

**Key things to verify before writing the plan:**
- Read the current state of `emMainWindow.rs`, `emMainPanel.rs`, `emAutoplay.rs`, `emMainControlPanel.rs`, `emMain.rs`, `main.rs`, `lib.rs` — these were all modified in Phases 1-2
- Read the C++ sources: `emMainWindow.cpp` (StartupEngineClass lines 330-485, window lifecycle lines 86-263), `emAutoplay.cpp` (full file), `emAutoplay.h`
- Check what emcore APIs exist: emVisitingViewAnimator (SetGoalFullsized, SetGoal, Activate, Deactivate, IsActive, SetAnimated), emView (Visit, RawZoomOut, SetActivePanel, Focus, ZoomOut, GetTitle), EngineScheduler, emGetClockMS, emPackGroup
- Check emMainControlPanel constructor — C++ takes `(view, name, mainWindow, contentView)`, Rust currently takes `(ctx)`. Phase 3 creates it inside control sub-view and it needs content view reference for bookmark navigation.
- The `PanelBehavior` trait has NO `PaintOverlay` method
- `emGetInsResImage` does NOT exist in Rust — use `include_bytes!` + `load_tga` pattern

**Non-obvious API divergences:**
- **Timer API:** C++ uses `emTimer` as a member field with `.Start(ms)` / `.Stop()` / `.GetSignal()`. Rust has NO `emTimer` struct — instead there's a centralized `TimerCentral` accessed via `EngineScheduler`, using `TimerId` handles: `create_timer(signal_id) -> TimerId`, `start_timer(id, interval_ms, periodic)`, `cancel_timer(id, abort_signal)`, `is_running(id)`. Phase 1's `emMainPanel` already has stubbed timer comments showing where this needs to go.
- **C++ emMain has no header:** `emMain` class is defined entirely in `emMain.cpp` (lines 66-99), not in a separate `.h` file. It extends both `emEngine` and `emMiniIpcServer` (multiple inheritance). The Rust version composes rather than inherits.

**Lessons from prior phases (for plan quality):**
- Don't include uncertain analysis in the plan (Phase 2 Task 1's polySizes debate was noise — just tell the implementer to count the C++ array)
- Verify PaintImageColored arg order against `lum_to_color` semantics before writing pseudocode (color1=foreground/white pixels, color2=background/black pixels)
- When writing pseudocode with API calls, verify the exact Rust signatures first — don't assume they match C++
