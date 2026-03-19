# FilePanel + FileDialog + CoreConfigPanel Audit Report

**Date**: 2026-03-18 (Session 2)

---

## FilePanel (479 Rust LOC vs 606 C++ LOC)

### [LOW] Saving progress always shows 0.0% — **FIXED**
- Changed paint_status() Saving arm to display "Saving..." without percentage. Removed dead file_state_progress() helper.

### [LOW] Missing IsContentReady port — **CLOSED: The equivalent functionality exists as VirtualFileState::is_good() (file_panel.rs:26-29) which returns true for Loaded or Unsaved states — matching C++ IsContentReady semantics exactly. The method name differs but the behavior is identical.**
### [LOW] Missing GetIconFileName port — **DEFERRED: C++ GetIconFileName is a virtual method that subclasses override to provide a per-file-type icon filename. The Rust port doesn't have icon rendering in panel headers. Implementing this would require: (1) a trait method on PanelBehavior or a new IconProvider trait, (2) icon loading infrastructure, (3) integration with border's icon rendering. This is a feature addition (~80 LOC) that depends on icon asset availability. User-facing impact: file panels show without type-specific icons in their headers.**
### [LOW] Missing ancestor-sharing guard in SetFileModel — **CLOSED: The Rust FilePanel doesn't have a SetFileModel method — it uses set_has_model(bool) to track model presence and set_file_state() to receive state updates. The C++ ancestor-sharing guard prevents two panels from sharing the same FileModel instance (which would cause double-free on destruction). The Rust ownership model prevents this structurally — FileState is an enum copied into the panel, not a shared reference. No guard needed.**

### [OK] Paint output matches C++ colors and layout (all color values verified identical)
### [OK] VirtualFileState enum and is_good/is_hope_for_seeking match

---

## FileDialog (341 Rust LOC vs 514 C++ LOC)

### [LOW] set_mode doesn't update dialog title/button text after construction — **FIXED**
- Added set_caption to Border, set_title and set_button_label_for_result to Dialog. set_mode now calls mode_title_and_ok() and updates both.

### [OK] All three modes (Select/Open/Save) with correct titles
### [OK] Filter support fully forwarded
### [OK] Multi-selection forwarded
### [OK] Check-finish validation logic matches C++ (overwrite confirmation pattern)
### [OK] Result handling (GetSelectedPath/Name/Names) forwarded correctly

---

## CoreConfigPanel (1569 Rust LOC vs 1079 C++ LOC)

Size asymmetry explained: Rust lacks inheritance, requires boilerplate PanelBehavior impls. No extra features.

### [LOW] 3 factor fields missing on_value callbacks (wheelaccel, kinetic, magnetism radius — changes silently discarded) — **FIXED**
- Added on_value callbacks to wheelaccel, kinetic_zooming_and_scrolling, and magnetism_radius ScalarFields matching the pattern of other factor fields.
### [LOW] StickPossible not checked — stick checkbox always enabled — **DEFERRED: C++ disables the stick checkbox when emScreen::CanMoveMousePointer() returns false (platform query). The Rust port has no platform capability query for mouse pointer warping. Implementing this requires OS-specific cursor warp detection (X11/Wayland/Win32). User-facing impact: the stick checkbox is always clickable even if the platform cannot warp the mouse cursor. Added TODO comment in code.**
### [LOW] MaxMemGroup label text shorter (6 vs 15 lines) — **FIXED**
- Updated label text to match full C++ warning including IMPORTANT, RECOMMENDATION, WARNING, and NOTE sections.
### [LOW] Upscale quality range excludes "Nearest Pixel" (Rust min=1, C++ min=0) — **FIXED**
- Changed ScalarField min from 1.0 to 0.0 and updated callback clamp from (1,5) to (0,5).
### [LOW] Downscale quality range hardcoded (2.0..6.0 vs dynamic from config) — **DEFERRED: C++ reads min/max from the emCoreConfig record properties at runtime (emRec field constraints). The Rust config system uses a fixed CoreConfig struct without runtime-queryable field metadata. Implementing dynamic ranges would require: (1) adding min/max metadata to the config record definition, (2) querying it at panel construction time, (3) updating ScalarField ranges when config reloads. This is ~60 LOC of config infrastructure. User-facing impact: quality slider has slightly different range than C++ when config defines non-default bounds — unlikely in practice since the hardcoded values match the C++ defaults.**
### [LOW] Factor field ranges hardcoded vs dynamic from config record — **DEFERRED: Same root cause as downscale quality range — C++ emRec fields carry min/max constraints that the config panel reads at runtime. Rust CoreConfig doesn't expose field metadata. The hardcoded ranges (0.25-4.0 for most factors) match the C++ default constraints. User-facing impact: none with default config; would matter only if a custom config record redefined field bounds.**

### [OK] All config options present (MouseWheel*, Zoom*, Scroll*, Kinetic*, MaxMem, Threads, SIMD, Quality)
### [OK] Factor conversion math identical (Val2Cfg/Cfg2Val pow/log formulas)
### [OK] Memory conversion math identical
### [OK] Layout structure matches (vertical split, 4 groups, nested tunnels)
### [OK] Reset button present

---

## Combined Summary

| Severity | Count |
|----------|-------|
| LOW | 11 |
| INFO | 3 |
| OK | 12 |

**All three are structurally faithful ports with no HIGH or MEDIUM findings.** Main gaps: missing callbacks in CoreConfigPanel, saving progress bug in FilePanel, hardcoded ranges.
