# FileSelectionBox Audit Report

**Date**: 2026-03-18 (Session 2)
**C++ files**: emFileSelectionBox.cpp (1217 LOC) + emFileSelectionBox.h (403 LOC) = 1620 LOC
**Rust file**: file_selection_box.rs (665 LOC)

## Completeness: ~40% — Structural shell only

## What Works (16 of 39 checked items)
- Data model (all state fields, getters/setters)
- Filter pattern matching (exact port of C++ glob logic)
- Directory listing reload (simplified but functional)
- Child panel creation with correct 3-zone layout geometry
- Border and paint pass-through
- ".." entry at non-root
- Multi-selection flag and selected names storage

## What's Missing (20 GAPs, 51% of functionality)

### [HIGH] Entire reactive/event layer missing — **DEFERRED: Requires implementing Cycle() engine infrastructure — the C++ Cycle() virtual method is called every frame by the engine scheduler to poll signals, update filesystem watchers, sync child selections, and process deferred state changes. The Rust port has no equivalent frame-tick callback system. Implementing this would require: (1) a Cycle-equivalent callback registration in the scheduler (~100 LOC), (2) signal routing for ListBox selection changes (~50 LOC), (3) filesystem watch integration (~100 LOC), (4) Enter key input handling in the name field (~30 LOC), and (5) signal emission for SelectionSignal/FileTriggerSignal (~50 LOC). Total: ~330 LOC of new infrastructure beyond a bugfix session. User-facing impact: the file selection box displays correctly but cannot respond to user interaction — it is a visual scaffold only.**

### [HIGH] FileItemPanel missing entirely (~280 LOC in C++) — **DEFERRED: FileItemPanel is a custom panel that renders each file entry with icon, filename text, selection highlight, "not readable" indicator, and optional file content preview via emFpPlugin. The Rust port uses generic ListBox items instead. Implementing this would require: the panel class (~150 LOC), icon loading from file type (~50 LOC), selection highlight rendering (~30 LOC), and plugin-based file preview (~50+ LOC). This is a feature implementation, not a bugfix. User-facing impact: file entries show as plain text items rather than rich panels with icons and previews.**

### [MEDIUM] No interactive directory navigation — **DEFERRED: Double-click/trigger on a directory should call EnterSubDir() which reloads the listing, updates ParentDirField, and resets selection. This requires the Cycle() event layer (see HIGH finding above) to receive ListBox trigger signals. The ParentDirField text entry also needs input handling to navigate on Enter. Cannot be fixed without the reactive infrastructure. User-facing impact: users cannot browse the filesystem.**

### [MEDIUM] No name field sync — **DEFERRED: Selected filename should appear in the name TextField, and editing the name should update selection. This requires bidirectional signal wiring between ListBox selection changes and TextField content, which depends on the Cycle() infrastructure. User-facing impact: the name field stays empty regardless of selection.**

### [LOW] Locale-aware sort missing (strcoll → str::cmp) — **DEFERRED: C++ uses strcoll() for locale-aware filename ordering. Rust's str::cmp uses byte ordering which differs for non-ASCII filenames (accented characters, CJK). Fixing this would require pulling in a Unicode collation library (e.g. icu_collator) which adds a dependency for a minor sort-order difference. User-facing impact: filenames with accented characters may appear in different order than C++. Acceptable for current use case.**

### [LOW] set_filters doesn't update existing child ListBox — **FIXED**
- set_filters now sets children_dirty flag. layout_children detects dirty flag, tears down and recreates all children with fresh state. Filter ListBox gets updated items.
### [LOW] set_multi_selection_enabled doesn't update existing ListBox type — **FIXED**
- set_multi_selection_enabled now sets children_dirty flag. Recreated file ListBox gets correct SelectionMode.
### [LOW] No AutoShrink (Options never cleared) — **FIXED**
- Resolved by the children_dirty rebuild mechanism: create_children always starts with fresh ListBox instances, so no stale options accumulate.

## Summary

| Category | Match | GAP |
|----------|-------|-----|
| Data model | 90% | 10% |
| Filters | 85% | 15% |
| Navigation | 45% | 55% |
| Multi-selection | 25% | 75% |
| Signals | 0% | 100% |
| Child panels | 60% | 40% |
| Event handling | 0% | 100% |

## Overall: This is a data-model-and-layout scaffold. The widget creates correct visual structure but cannot respond to any user interaction. To reach parity, it needs: Cycle() or callback-based event processing, FileItemPanel, signal/callback wiring, and bidirectional selection sync.
