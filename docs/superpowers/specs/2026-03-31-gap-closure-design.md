# Gap Closure: Maximalist C++ Parity Across All Modules

**Date:** 2026-03-31
**Scope:** emCore, emFileMan, emStocks — all DIVERGED, TODO, stub, and omitted-method gaps
**Approach:** Option A — port everything without `unsafe`; maximize API surface and behavioral parity; accept Rust-safe backing structures

## Motivation

The codebase has 139 DIVERGED comments, ~15 TODOs, and multiple stub implementations. All C++ headers have corresponding Rust files — the gaps are *inside* the files. This spec traces every gap to its transitive closure and specifies the work to close it.

Guiding principle: any Rust "intended divergence" is fixable and replaceable by default, not an opportunity for simplification. Exceptions require proof that matching C++ is impossible without `unsafe`.

## Phase 1 — emCore Foundations (~50 items)

### 1.1 emColor API Restoration

#### Scale Changes (breaking — ~40 caller updates)

**GetBlended:** Change parameter `t` from `[0.0, 1.0]` to `[0.0, 100.0]` matching C++ `weight` semantics. C++ implementation:

```cpp
if (weight<=0.0F) return *this;
if (weight>=100.0F) return color;
w2=(int)(weight*655.36F+0.5F);
w1=65536-w2;
```

All ~40 Rust callers currently pass normalized decimals (0.25, 0.4, 0.5, 0.66, 0.8). Each must be multiplied by 100 (→ 25.0, 40.0, 50.0, 66.0, 80.0).

**GetLighted:** Reunify `lighten(amount)` + `darken(amount)` into single `GetLighted(light: f32) -> emColor` with C++ range `[-100.0, 100.0]`. Negative = darken (blend toward black), positive = lighten (blend toward white). Update all callers of `lighten(x)` → `GetLighted(x * 100.0)` and `darken(x)` → `GetLighted(-x * 100.0)`. Remove `lighten`/`darken` methods. Remove DIVERGED comments on lines 220 and 227.

#### Method Restoration (non-breaking additions)

| Method | Signature | Notes |
|--------|-----------|-------|
| `GetHue` | `fn GetHue(self) -> f32` | Returns `[0.0, 360.0)`. Exists alongside `GetHSV()`. |
| `GetSat` | `fn GetSat(self) -> f32` | Returns `[0.0, 100.0]`. |
| `GetVal` | `fn GetVal(self) -> f32` | Returns `[0.0, 100.0]`. |
| `SetHSVA` (4-param) | `fn SetHSVA(h: f32, s: f32, v: f32, alpha: u8) -> Self` | Existing 3-param form calls this with `alpha=255`. |
| `SetGrey` (2-param) | `fn SetGrey(grey: u8, alpha: u8) -> Self` | Existing 1-param form calls this with `alpha=255`. |

Remove DIVERGED comments on lines 125, 163, 337 once these methods exist.

#### Naming (no change needed)

- `GetPacked`: C++ `Get()` is an implicit `operator emUInt32()`. No Rust equivalent exists. DIVERGED comment stays — this is genuinely impossible without unsafe conversion traits.
- `Set` overloads: C++ mutates in place. Rust `emColor` is `Copy` — returning new values is the correct pattern. `SetRed`, `SetGreen`, `SetBlue`, `SetAlpha`, `SetHue`, `SetSat`, `SetVal` all keep their current return-new-value signatures. DIVERGED comments on these stay — they document an unavoidable Rust idiom difference.

### 1.2 emATMatrix Method Restoration

Add individual methods alongside existing tuple-returning methods:

| Method | Signature |
|--------|-----------|
| `TransX` | `fn TransX(&self, sx: f64, sy: f64) -> f64` |
| `TransY` | `fn TransY(&self, sx: f64, sy: f64) -> f64` |
| `InverseTransX` | `fn InverseTransX(&self, dx: f64, dy: f64) -> Option<f64>` |
| `InverseTransY` | `fn InverseTransY(&self, dx: f64, dy: f64) -> Option<f64>` |

Remove DIVERGED comments on lines 287 and 296. The existing tuple-returning methods stay as Rust-side conveniences (add DIVERGED comment noting they have no C++ equivalent).

### 1.3 emArray Method Additions

#### Custom comparator overloads

The C++ versions of `Sort`, `BinarySearch`, `BinaryInsert`, `BinaryInsertIfNew`, `BinaryInsertOrReplace`, `BinaryRemove` all accept a function pointer comparator. The Rust versions use `Ord` trait bounds. Add `_by` variants:

| Method | Signature |
|--------|-----------|
| `Sort_by` | `fn Sort_by(&mut self, compare: impl FnMut(&T, &T) -> Ordering)` |
| `BinarySearch_by` | `fn BinarySearch_by(&self, compare: impl FnMut(&T) -> Ordering) -> Result<usize, usize>` |
| `BinaryInsert_by` | `fn BinaryInsert_by(&mut self, element: T, compare: impl FnMut(&T, &T) -> Ordering)` |
| `BinaryInsertIfNew_by` | `fn BinaryInsertIfNew_by(&mut self, element: T, compare: impl FnMut(&T, &T) -> Ordering) -> bool` |
| `BinaryInsertOrReplace_by` | `fn BinaryInsertOrReplace_by(&mut self, element: T, compare: impl FnMut(&T, &T) -> Ordering)` |
| `BinaryRemove_by` | `fn BinaryRemove_by(&mut self, compare: impl FnMut(&T) -> Ordering) -> bool` |

#### Key-based search

| Method | Signature |
|--------|-----------|
| `BinarySearchByKey` | `fn BinarySearchByKey<K: Ord>(&self, key: &K, extract: impl Fn(&T) -> K) -> Result<usize, usize>` |
| `BinaryReplace` | `fn BinaryReplace(&mut self, element: T, compare: impl FnMut(&T, &T) -> Ordering) -> bool` |
| `BinaryRemoveByKey` | `fn BinaryRemoveByKey<K: Ord>(&self, key: &K, extract: impl Fn(&T) -> K) -> bool` |

#### Default-value insertion

| Method | Signature | Notes |
|--------|-----------|-------|
| `AddNew` | `fn AddNew(&mut self) where T: Default` | Appends `T::default()` |
| `InsertNew` | `fn InsertNew(&mut self, index: usize) where T: Default` | Inserts `T::default()` at index |
| `ReplaceByNew` | `fn ReplaceByNew(&mut self, index: usize, count: usize) where T: Default` | Replaces range with defaults |

#### TuningLevel (API correspondence, no-op)

Add `tuning_level: u8` field, `GetTuningLevel() -> u8`, `SetTuningLevel(level: u8)`. The field is stored but has no effect on allocation. DIVERGED comment explains: "Rust ownership model makes COW tuning unnecessary; field exists for API correspondence only."

#### Omitted (per Option A — unsafe/pointer-based)

- `PointerToIndex`: requires pointer arithmetic into Vec backing. DIVERGED comment stays.
- `GetWritable(ptr)`: requires raw pointer resolution. DIVERGED comment stays.

#### Iterator auto-adjustment

C++ emArray::Iterator auto-adjusts index when elements are inserted/removed before the cursor position. The Rust `Cursor` does not. This is a behavioral divergence affecting any consumer that mutates the array while iterating.

Port approach: Add an `adjustments` list to emArray (a `Vec<(isize, usize)>` recording (delta, at_index) for each mutation). Each `Cursor` checks adjustments on `Get()`/`SetNext()`/`SetPrev()` and adjusts its stored index. The adjustment list is cleared when no cursors are alive (track cursor count via `Rc<Cell<usize>>`). This is safe Rust — no raw pointers needed.

Remove DIVERGED comment on line 33.

### 1.4 emList Method Additions

#### Mutable navigation

| Method | Signature |
|--------|-----------|
| `GetNextWritable` | `fn GetNextWritable(&mut self, index: usize) -> Option<(usize, &mut T)>` |
| `GetPrevWritable` | `fn GetPrevWritable(&mut self, index: usize) -> Option<(usize, &mut T)>` |

#### Move operations (O(n) in Vec, documented)

| Method | Signature |
|--------|-----------|
| `MoveToBeg` | `fn MoveToBeg(&mut self, index: usize)` |
| `MoveToEnd` | `fn MoveToEnd(&mut self, index: usize)` |
| `MoveBefore` | `fn MoveBefore(&mut self, src: usize, dst: usize)` |
| `MoveAfter` | `fn MoveAfter(&mut self, src: usize, dst: usize)` |

Each Move method: remove element from `src`, insert at target position. DIVERGED comment stays on line 17 documenting O(n) vs O(1) complexity difference, but methods now exist.

#### SubList operations

| Method | Signature |
|--------|-----------|
| `GetSubList` | `fn GetSubList(&self, first: usize, last: usize) -> emList<T> where T: Clone` |
| `GetSubListOfFirst` | `fn GetSubListOfFirst(&self, count: usize) -> emList<T> where T: Clone` |
| `GetSubListOfLast` | `fn GetSubListOfLast(&self, count: usize) -> emList<T> where T: Clone` |
| `Extract` | `fn Extract(&mut self, first: usize, last: usize) -> emList<T>` |

Remove DIVERGED comment on line 20.

#### Multi-variant insertion overloads

For each of `InsertAtBeg`, `InsertAtEnd`, `InsertBefore`, `InsertAfter`, `Add`, add:
- `_slice` variant: `fn InsertAtBeg_slice(&mut self, elements: &[T]) where T: Clone`
- `_list` variant: `fn InsertAtBeg_list(&mut self, other: &emList<T>) where T: Clone`
- `_fill` variant: `fn InsertAtBeg_fill(&mut self, element: T, count: usize) where T: Clone`

#### Constructor variants

| Constructor | Signature |
|-------------|-----------|
| `from_two` | `fn from_two(a: &emList<T>, b: &emList<T>) -> Self where T: Clone` |
| `from_sublist` | `fn from_sublist(src: &emList<T>, first: usize, last: usize) -> Self where T: Clone` |

#### Custom comparator sort

| Method | Signature |
|--------|-----------|
| `Sort_by` | `fn Sort_by(&mut self, compare: impl FnMut(&T, &T) -> Ordering)` |

### 1.5 emAvlTreeMap Additions

#### Index trait

Implement `std::ops::Index<&K>` for `emAvlTreeMap<K, V>` returning `&V`. Panics if key not found (matching C++ `operator[]` which creates default entry — Rust version panics instead since we can't create entries without `Default` bound on `V`). Add DIVERGED comment on the panic-vs-default-insert difference.

### 1.6 emAvlTreeSet Operator Trait Impls

Implement `std::ops` traits matching C++ operators:

| Rust Trait | C++ Operator | Semantics |
|------------|-------------|-----------|
| `BitOr<&Self>` | `\|` | Union (new set) |
| `BitOrAssign<&Self>` | `\|=` | Union (in-place, calls `InsertSet`) |
| `BitAnd<&Self>` | `&` | Intersection (new set) |
| `BitAndAssign<&Self>` | `&=` | Intersection (in-place, calls `Intersect`) |
| `Sub<&Self>` | `-` | Difference (new set) |
| `SubAssign<&Self>` | `-=` | Difference (in-place, calls `RemoveSet`) |
| `Add<T>` | `+ obj` | Single-element union (new set) |
| `AddAssign<T>` | `+= obj` | Single-element insert |

Remove DIVERGED comment on line 22. Named methods (`InsertSet`, `RemoveSet`, `Intersect`) remain as the canonical implementations; operators delegate to them.

### 1.7 emCursor Restoration

- Add `Get() -> Self` method. C++ returns `int` id; Rust returns the enum variant (which is the identity for an enum). Trivially `fn Get(self) -> Self { self }`. Remove DIVERGED comment on line 28.
- Keep `as_str()` name. This is a genuine Rust constraint: implementing `Display` auto-provides a `ToString` trait method, so an inherent `ToString()` would shadow it and cause confusion. DIVERGED comment on line 29 stays, updated to: "C++ name is ToString. Renamed to as_str because Rust's Display trait auto-provides ToString, and an inherent method with the same name would shadow it."

### 1.8 emButton EOI Signal

C++ `emButton` fires an End-Of-Interaction signal after the click completes (mouse released over button). Port:

- Add `eoi_signal: SignalId` field (or callback: `on_eoi: Option<Box<dyn Fn()>>` matching existing callback pattern)
- Fire after `Clicked()` completes in `Input()` when mouse button is released
- `GetEOISignal() -> &SignalId` accessor (or callback setter matching existing pattern)

Remove the "EOI signal not implemented" comment in `emButton.rs:401`.

### 1.9 emTmpFileMaster

C++ `emTmpFileMaster` is a singleton per temp directory that:
- Acquires exclusive ownership via IPC (emMiniIpc server name based on temp dir path)
- Cleans up orphaned temp files from crashed processes
- Registers/unregisters temp files created by emTmpFile instances
- Runs periodic cleanup in its `Cycle()` method

Port approach (safe Rust, no IPC needed):
- Use `std::fs::File` + `flock(LOCK_EX | LOCK_NB)` on a lock file in the temp directory as the singleton mechanism
- On acquisition: scan directory for orphaned temp files (matching emTmpFile naming pattern) and remove them
- Maintain a `HashSet<PathBuf>` of registered temp file paths
- `Cycle()`: periodically verify registered files still exist, remove stale entries
- `Drop`: release lock file, clean up registered temp files

Wire into `emTmpFile`: when `emTmpFile::New()` is called, register with the master. When dropped, unregister.

Remove DIVERGED comment in `emTmpFile.rs:6`.

## Phase 2 — emCore Behavioral (~15 items)

### 2.1 emDefaultTouchVIF 18-State Gesture Machine

Replace the `panic!("C++ emDefaultTouchVIF 17-state gesture machine not yet ported")` at `emViewInputFilter.rs:3010` with a full port of the C++ implementation (`emViewInputFilter.cpp:823-1302`, ~480 lines).

#### Data structures

```rust
struct Touch {
    id: u64,
    ms_total: i32,
    ms_since_prev: i32,
    down: bool,
    x: f64,
    y: f64,
    prev_down: bool,
    prev_x: f64,
    prev_y: f64,
    down_x: f64,
    down_y: f64,
}
```

Maximum 16 simultaneous touches (`const MAX_TOUCH_COUNT: usize = 16`).

#### State enum

```rust
enum GestureState {
    Ready,
    FirstDown,
    Scroll,
    ZoomIn,
    ZoomOut,
    FirstDownUp,
    DoubleDown,
    DoubleDownUp,
    TripleDown,
    TripleDownUp,
    SecondDown,
    EmuMouse1,  // left button (swipe right)
    EmuMouse2,  // right button (swipe left)
    EmuMouse3,  // shift+left (swipe down)
    EmuMouse4,  // ctrl+left (swipe up)
    ThirdDown,
    FourthDown,
    Finish,
}
```

#### Core methods

- `Input()`: updates touch array from input events, calls `DoGesture()`, consumes handled events
- `Cycle()`: calls `DoGesture()` for time-based transitions (250ms hold thresholds)
- `DoGesture()`: the state machine — `match self.gesture_state { ... }` with transitions as documented in Phase 2 design section
- `GetTouchEventPriority()`: returns 2.0 or 3.0 based on state (higher priority when actively gesturing)

#### Gesture behaviors

| Gesture | State Flow | Action |
|---------|-----------|--------|
| Single-finger scroll | FirstDown → Scroll (>20px move) | `view.Scroll(dx, dy)` |
| Single-finger hold | FirstDown → ZoomIn (>250ms) | `view.Zoom(x, y, exp(0.002*ms))` |
| Two-finger hold | DoubleDown → ZoomOut (>250ms) | `view.Zoom(x, y, exp(-0.002*ms))` |
| Double-tap | FirstDownUp → DoubleDown → DoubleDownUp → Finish | `view.VisitFullsized(panel, false)` |
| Triple-tap | TripleDownUp → Finish | `view.VisitFullsized(panel, true)` (toggle) |
| Two-finger directional | SecondDown → EmuMouse1/2/3/4 | Emit synthetic mouse events |
| Three-finger lift | ThirdDown → Finish | Emit `EM_KEY_MENU` |
| Four-finger lift | FourthDown → Finish | `view.ShowSoftKeyboard(!shown)` |

#### Dependencies

All dependencies exist in Rust:
- `emView::Scroll`, `Zoom`, `GetFocusablePanelAt`, `VisitFullsized`, `ShowSoftKeyboard`, `GetInputClockMS`
- `emInputEvent`, `emInputState` for touch event data
- `emViewInputFilter` base class for `GetTouchEventPriority` and event consumption

### 2.2 Magnetic View Animator Wiring

Check whether `emMagneticViewAnimator` exists in Rust emView. If it does:
- In `emDefaultTouchVIF`, after scroll/zoom gestures complete (transition to `Finish`), call `view.activate_magnetic_view_animator()` when `!self.magnetism_avoidance`
- In `emCheatVIF`, same wiring for cheat-triggered view changes
- Remove TODO comment at `emViewInputFilter.rs:786`

If `emMagneticViewAnimator` is not yet ported, port it as a subtask. C++ implementation is in `emView.cpp` (~200 lines). It is an `emEngine` subclass owned by `emView` with:
- `Cycle()` method that runs each frame while animating
- `Activate(panel)`: sets target panel, starts animation
- `DoAnimation()`: computes interpolated view rect between current position and target panel bounds using exponential decay (`factor = exp(-dt * speed)`)
- Fields: `TargetPanel` (weak ref), `Speed` (decay rate), `Active` (bool)
- Integration: `emView` creates it in constructor, calls `Activate()` from `VisitFullsized()` and touch VIF

### 2.3 Cheat Dispatch on emView

C++ `emCheatVIF::Input()` accumulates typed characters into a cheat buffer and calls `emView::DoCheat(cheatCode)` when a recognized sequence is completed. `emView::DoCheat()` dispatches to registered cheat handlers.

Port:
- Add `DoCheat(code: &str)` method to `emView`
- Add cheat handler registration: `RegisterCheat(code: &str, handler: Box<dyn Fn(&mut emView)>)`
- Wire from `emCheatVIF::Input()` — the existing Rust code already accumulates characters; only the dispatch call is missing
- Remove TODO at `emViewInputFilter.rs:2261`

### 2.4 emScreen MoveMousePointer

Current state: no-op stub in `emScreen.rs:118-123`.

winit `Window::set_cursor_position(LogicalPosition)` exists since winit 0.20+. The Rust codebase uses winit. Implementation:

- In `MoveMousePointer(dx: f64, dy: f64)`:
  1. Get current cursor position via `window.cursor_position()` (if available) or tracked state
  2. Compute new position: `(current_x + dx, current_y + dy)`
  3. Call `window.set_cursor_position(LogicalPosition::new(new_x, new_y))`
  4. Handle `Err` (some platforms/compositors reject cursor warping) — log warning, do not panic

- Remove "Stub" comment and "Not supported by winit core" DIVERGED comment. Add DIVERGED comment only if a specific platform rejects the call.

## Phase 3 — emFileMan Behavioral (~8 items)

### 3.1 Shift-Range Selection

Replace simplified single-entry selection at `emDirEntryPanel.rs:244` with full sibling walk.

C++ algorithm:
1. On shift-click, read `emFileManModel::ShiftTgtSelPath` (the anchor entry)
2. Find the parent `emDirPanel`
3. Iterate the parent's children (all `emDirEntryPanel`s) from anchor to clicked entry (inclusive)
4. Call `emFileManModel::SelectEntryByPath(child.GetPath())` for each child in range
5. Update `ShiftTgtSelPath` to the clicked entry

Rust implementation:
- In `emDirEntryPanel::Input()`, when shift+click detected:
  1. Get parent panel (the `emDirPanel`) via panel tree traversal
  2. Get sorted child list from parent
  3. Find indices of anchor and current entry in the child list
  4. Iterate `min_idx..=max_idx`, selecting each entry
- Requires: parent panel access (available via `Weak<RefCell<emPanel>>` parent ref), child iteration (available via panel tree)

Remove DIVERGED comment on line 244.

### 3.2 SelectAll via Content View

Replace the TODO at `emFileManControlPanel.rs:460`.

Implementation:
- Add `content_view: Option<Weak<RefCell<emView>>>` field to `emFileManControlPanel`
- Set during construction — the parent that creates the control panel also creates the content view; pass the view reference
- `SelectAll()` implementation:
  1. Upgrade weak reference to get the view
  2. Call `view.GetFocusedPanel()` or `view.GetRootPanel()`
  3. Walk ancestor chain to find the nearest `emDirPanel` (check panel type)
  4. Call `dir_panel.SelectAll()` which iterates its `emDirEntryPanel` children and selects each via `emFileManModel`

Remove the `log::debug!("SelectAll: TODO")` line.

### 3.3 Scroll-to-Entry

Replace the TODO at `emDirPanel.rs:214`.

C++ calls `emPanel::Layout()` followed by `emView::Visit()` to scroll to a named child panel. Rust implementation:

- After a child `emDirEntryPanel` is created (e.g., on directory navigation or `SetFileModel` load completion):
  1. Find the child panel by name in the panel tree
  2. Call `view.Visit(child_panel, ...)` or equivalent scroll-into-view method
- The view's visit infrastructure already exists for view animation

Remove the TODO comment.

### 3.4 emDirModel Composing emFileModel

Replace the standalone data wrapper at `emDirModel.rs:201` with proper `FileModelState` composition.

Changes to `emDirModel`:
- Add `signal_id: SignalId` field, obtained from scheduler at construction
- Implement `FileModelState` trait:
  - `get_state() -> FileState` maps existing load states to `FileState` enum
  - `get_update_signal() -> SignalId` returns the signal_id
  - `get_file_path() -> &Path` returns the directory path
  - `TryStartLoading()` / `TryContinueLoading()` wrap existing `start_loading()` / `try_continue_loading()`
- Add `FileModelClientList` field for client registration
- Fire `update_signal` when load state changes

Remove DIVERGED comment on line 201.

### 3.5 emDirPanel Using FileModelState

Replace manual loading in `emDirPanel::Cycle()` at `emDirPanel.rs:116` with `emFilePanel::SetFileModel` integration.

Changes to `emDirPanel`:
- Remove manual `try_continue_loading()` calls from `Cycle()`
- In construction or when directory changes, call `self.SetFileModel(dir_model)` where `dir_model` implements `FileModelState`
- Let inherited `emFilePanel::Cycle()` drive the load state machine
- Gate child panel creation on `IsContentReady()`
- `SortChildren()` and child panel layout happen after `IsContentReady()` returns true

Remove DIVERGED comment on line 116.

### 3.6 emFileLinkPanel Deferred Update

`emFileLinkPanel.rs:242` documents that C++ calls `UpdateDataAndChildPanel` from `Cycle()` and `Notice()`, while Rust defers to `LayoutChildren()`.

This is actually correct Rust borrow-safety behavior and matches the pattern established in `emDirEntryPanel`. However, verify that the deferred update doesn't cause a visible frame delay. If it does, investigate calling the update from `Cycle()` with appropriate borrow management (drop the RefCell borrow before calling child-creating methods).

DIVERGED comment stays if the deferral is necessary for borrow safety, but should be updated to explain the timing difference is at most one frame.

## Phase 4 — emStocks Rendering (~6 panels + process integration)

### 4.1 emStocksItemChart — Full Paint Pipeline

Replace stubbed painting with full port of C++ `emStocksItemChart::PaintContent()` (~1,000 lines, 7 sub-methods).

#### Fields to add

```rust
// Coordinate transformation
x_offset: f64,
x_factor: f64,
y_offset: f64,
y_factor: f64,

// Aggregated price data
prices: Vec<f64>,       // aggregated prices per interval
days_per_price: i32,    // adaptive: 1 at high zoom, up to 1000+ at low zoom
```

#### Methods to port

| Method | Lines | Description |
|--------|-------|-------------|
| `PaintContent` | ~20 | Orchestrator: calls 7 sub-methods in order |
| `PaintXScaleLines` | ~98 | Vertical grid at day/month/year/decade intervals |
| `PaintXScaleLabels` | ~117 | Multi-level date labels with adaptive text sizing |
| `PaintYScaleLines` | ~44 | Horizontal price grid with log-scale levels |
| `PaintYScaleLabels` | ~55 | Price labels with dynamic decimal formatting |
| `PaintPriceBar` | ~65 | Gradient rectangle: green/red (owning), magenta/cyan (selling) |
| `PaintDesiredPrice` | ~37 | Yellow horizontal line at target price |
| `PaintGraph` | ~89 | Price polyline with point markers at high zoom |
| `UpdatePrices1` | ~30 | Extract trade/current/desired prices from StockRec |
| `UpdatePrices2` | ~73 | Aggregate daily prices into DaysPerPrice intervals |
| `UpdateTransformation` | ~40 | Compute coordinate transform from view rect |
| `CalculateYScaleLevelRange` | ~32 | Log-scale range for Y axis (powers of 10, 5, 2) |

#### Rendering primitives used

All exist in Rust `emPainter`:
- `PaintRect` — price bar, grid lines
- `PaintEllipse` — point markers on graph
- `PaintTextLayout` — axis labels, annotations
- `PaintBorderImage` / color fills — gradient textures on price bar

#### PanelBehavior implementation

- `is_opaque()`: return based on content state
- `paint()`: call `PaintContent` with painter and content rect
- `notice()`: respond to view geometry changes, trigger `UpdateTransformation`
- `Cycle()`: check for data model changes, call `UpdatePrices1`/`UpdatePrices2`

#### View-dependent behavior

C++ uses `IsViewed()` and `GetContentRect()` for adaptive rendering. The Rust version currently uses a unit rect (DIVERGED at line 418). Fix:
- Use actual `GetContentRect()` from PanelBehavior context
- Use `IsViewed()` / view state for `DaysPerPrice` calculation (DIVERGED at line 211)

Remove DIVERGED comments on lines 29, 211, 418.

### 4.2 emStocksControlPanel — Full Widget Tree

Replace stub at `emStocksControlPanel.rs:6` with full widget creation.

#### Widget hierarchy (from C++ AutoExpand)

```
ControlPanel (emLinearGroup)
├── AboutLabel (emLabel) — introduction text
├── PreferencesGroup (emLinearLayout)
│   ├── ApiScript (FileFieldPanel)
│   ├── ApiScriptInterpreter (FileFieldPanel)
│   ├── ApiKey (emTextField)
│   ├── WebBrowser (FileFieldPanel)
│   ├── AutoUpdateDates (emCheckBox)
│   ├── TriggeringOpensWebPage (emCheckBox)
│   └── ChartPeriod (emScalarField, custom formatter)
├── FiltersGroup (emLinearLayout)
│   ├── MinVisibleInterest (emRadioButton::LinearGroup)
│   ├── VisibleCountries (CategoryPanel)
│   ├── VisibleSectors (CategoryPanel)
│   └── VisibleCollections (CategoryPanel)
├── SortingGroup (emLinearLayout)
│   ├── Sorting (emRadioButton::RasterGroup, 12 options)
│   └── OwnedSharesFirst (emCheckButton)
├── PriceHistoryGroup (emLinearLayout)
│   ├── FetchSharePrices (emButton)
│   ├── DeleteSharePrices (emButton)
│   ├── GoBackInHistory / GoForwardInHistory (emButton)
│   └── SelectedDate (emTextField, validated)
├── TotalsGroup (emLinearLayout, read-only)
│   ├── TotalPurchaseValue (emTextField)
│   ├── TotalCurrentValue (emTextField)
│   └── TotalDifferenceValue (emTextField)
├── StockEditGroup (emLinearLayout)
│   ├── NewStock, CutStocks, CopyStocks, PasteStocks, DeleteStocks (emButton)
│   └── SelectAll, ClearSelection (emButton)
├── InterestGroup (emLinearLayout)
│   └── SetHighInterest, SetMediumInterest, SetLowInterest (emButton)
├── WebPagesGroup (emLinearLayout)
│   └── ShowFirstWebPages, ShowAllWebPages (emButton)
└── SearchGroup (emLinearLayout)
    ├── FindSelected (emButton)
    ├── SearchText (emTextField)
    └── FindNext, FindPrevious (emButton)
```

#### Inner classes to port

- `FileFieldPanel`: `emLinearGroup` with `emTextField` + file selection `emButton`. Stores path string, fires change callback.
- `CategoryPanel`: extends `emListBox` with checkable items. Dynamically populated from `emStocksConfig` category lists. Each item toggle updates the corresponding visibility set in `emStocksRec`.

#### AutoExpand/AutoShrink lifecycle

Widgets created in `LayoutChildren()` when panel is viewed at sufficient zoom. Destroyed when scrolled away. The existing `emLinearGroup` AutoExpand pattern in Rust handles this — each widget is created as `Option<WidgetType>` and populated/cleared based on view state.

#### Data wiring

Each widget callback writes to `emStocksRec` or `emStocksConfig` and triggers `emRecListener` notification. The `Cycle()` method watches for external data changes (other panels editing the same record) and updates widget display values.

Remove DIVERGED comments on lines 6, 22, 52.

### 4.3 emStocksItemPanel — Nested Stock Editor

Replace stub at `emStocksItemPanel.rs:6` with full widget hierarchy.

#### Key implementation details

- Extends `emLinearGroup` — auto-layout container with vertical/horizontal switching
- Implements `emListBox::ItemPanelInterface` for list integration
- Listens to `emRecListener` for data change notifications from the model
- Adaptive orientation: switches based on aspect ratio thresholds (0.3, 0.5, 1.0)

#### Data validation

Port C++ validator functions:
- `ValidateNumber(text) -> Option<f64>`: parse decimal number, reject non-numeric
- `ValidateDate(text) -> Option<Date>`: parse YYYY-MM-DD format, reject invalid dates

#### Special behaviors

- `OwningShares` checkbox toggle: swaps `TradePrice`↔`SalePrice` and `TradeDate`↔`SaleDate` in the underlying `StockRec`
- Computed values auto-update: `TradeValue = TradePrice * OwnShares`, `DifferenceValue = CurrentValue - TradeValue`
- Selected item highlight: blue background tint when item is selected in `emStocksListBox`

#### Inner class

- `CategoryPanel` (different from ControlPanel's): editable per-stock category assignment. Text search field + ListBox for selection from config-defined categories.

Remove DIVERGED comments on lines 6, 20, 40.

### 4.4 emStocksListBox — Stock List Container

Replace stub at `emStocksListBox.rs:9` with full `emListBox` integration.

#### Core methods to port

| Method | Description |
|--------|-------------|
| `CreateItemPanel` | Factory returning `emStocksItemPanel` for each visible stock |
| `UpdateItems` | Sync visible items with model based on active filters |
| `SortItems` | 12 sort comparators (name, trade date, inquiry date, achievement %, rise %, dividend, values, owned-first) |
| `Paint` | Call parent paint + "empty stock list" message if no items |

#### User operations

| Operation | Implementation |
|-----------|---------------|
| Cut | Serialize selected `StockRec`s to clipboard, remove from model |
| Copy | Serialize selected `StockRec`s to clipboard |
| Paste | Deserialize from clipboard, insert into model |
| Delete | Confirmation dialog via `emDialog`, then remove on confirm |
| New Stock | Create default `StockRec`, insert, select |
| Find | `FindSelected` scrolls to selected item; `FindNext`/`FindPrevious` search by name substring |

#### Dialog management

C++ uses `emCrossPtr<emDialog>` for confirmation dialogs (polled in `Cycle()`). Rust uses `Option<Rc<RefCell<emDialog>>>` with the existing emDialog pattern — check dialog state in `Cycle()`, act on result, drop reference when done.

Remove DIVERGED comment on line 9.

### 4.5 emStocksFilePanel — Top-Level Container

Replace stub at `emStocksFilePanel.rs:6` with full rendering.

#### PanelBehavior implementation

- `paint()`: fill background with `emColor::rgba(0x13, 0x15, 0x20, 0xFF)`
- `is_opaque()`: return false
- `LayoutChildren()`: position `emStocksListBox` to fill entire content rect
- `Input()`: keyboard shortcuts:
  - `Ctrl+N` → New stock
  - `Ctrl+X` → Cut
  - `Ctrl+C` → Copy
  - `Ctrl+V` → Paste
  - `Shift+Alt+H/M/L` → Set interest filter
  - `Ctrl+F` → Find
  - `F3` / `Shift+F3` → FindNext / FindPrevious

#### Child management

Creates and owns:
- `emStocksListBox` — main content
- `emStocksConfig` — configuration model (loaded from config file)
- `emStocksControlPanel` — settings sidebar (created as control panel via `CreateControlPanel`)

#### File model integration

Extends `emFilePanel`. Uses `emStocksFileModel` via `SetFileModel()`. Content becomes ready when the `.emStocks` file is loaded and parsed into `emStocksRec`.

Remove DIVERGED comment on lines 6, 12.

### 4.6 emStocksFetchPricesDialog — Progress Dialog

Replace stub at `emStocksFetchPricesDialog.rs:4` with full dialog UI.

#### Structure

- Extends `emDialog` with non-modal window flags (`GetWindowFlags() & ~WF_MODAL`)
- Auto-sizes based on parent window: width = parent_width * 0.4, height = width * 0.15

#### Child widgets

- `emLabel`: shows current stock name being fetched, or status/error message
- `ProgressBarPanel`: custom `emBorder` subclass with:
  - `PaintContent()`: filled rectangle proportional to `ProgressInPercent` (0.0–100.0)
  - Rectangle color: green fill on dark background
  - Margin: 10% on each side

#### Lifecycle

- Created by `emStocksListBox` when user triggers "Fetch Share Prices"
- `Cycle()`: polls `emStocksPricesFetcher` for progress updates, updates label and progress bar
- On completion: auto-close dialog
- On error: show error message in label, keep dialog open for user to dismiss

Remove DIVERGED comments on lines 4, 26.

### 4.7 emStocksPricesFetcher — Process Integration

Replace stubbed process methods with real `emProcess` integration.

#### Changes

- `StartProcess()`: use `emProcess::TryStart()` to spawn the API script interpreter with:
  - argv: `[interpreter_path, script_path, api_key, stock_symbol]`
  - stdin: closed
  - stdout: piped (for reading price data)
  - stderr: piped (for error messages)

- `PollProcess()`: use `emProcess::TryRead()` to read stdout line by line. Parse format: `SYMBOL\tDATE\tPRICE\n`. Feed parsed prices into `emStocksRec` stock price history via `StockRec::AddPrice()`.

- `CurrentProcess.Terminate()` (line 115): call `emProcess::SendTerminationSignal()` or `SendKillSignal()`.

- ListBox date-selection update (line 262): now that `emStocksListBox` is fully ported, call `list_box.UpdateSelectedDate()` after prices are fetched.

Remove DIVERGED comments on lines 2, 3, 4, 115, 262.

## DIVERGED Comments Disposition Summary

After all phases complete:

| Category | Count | Action |
|----------|-------|--------|
| Removed (gap closed) | ~95 | Method added, behavior matched, stub replaced |
| Retained (genuine Rust constraint) | ~30 | Copy semantics, no null refs, no implicit conversions, no pointer arithmetic |
| Updated (clearer reasoning) | ~14 | Better explanation of why Rust can't match C++ |

### Retained DIVERGED comments (genuine constraints, per Option A)

- emColor `Set*` methods returning `Self` instead of `void` (Copy type)
- emColor `GetPacked` vs C++ implicit `operator emUInt32()`
- emCrossPtr `Set`/`Reset` split (no null references)
- emArray/emList pointer-based methods (`PointerToIndex`, `GetWritable(ptr)`)
- emAvlTreeMap/Set `GetWritable`/`GetKeyWritable` (safety — mutating sorted container keys)
- emAvlTreeMap/Set element-pointer overloads (no raw pointer API)
- emList Vec-backed O(n) moves vs C++ O(1) pointer relinks
- emList/emArray cursor index-based vs C++ pointer-based (but auto-adjustment is now ported)
- emFileStream `PathBuf`/`&Path` instead of `String`

## Testing Strategy

Each phase has its own verification:

**Phase 1:** Unit tests for every new method on collections and emColor. Golden tests for emColor scale changes (existing golden tests will catch any rendering regressions from the GetBlended/GetLighted scale change — if golden tests fail, the caller update was wrong).

**Phase 2:** Integration tests for touch gesture recognition (synthetic touch event sequences → verify correct view.Scroll/Zoom calls). Manual testing for magnetic animator and cheat dispatch.

**Phase 3:** Pipeline tests for shift-range selection (create DirPanel with N entries, shift-click, verify selection set). Unit tests for emDirModel FileModelState implementation.

**Phase 4:** Golden tests for emStocksItemChart rendering (generate reference images from C++ build). Pipeline tests for ListBox operations (cut/copy/paste round-trip, sort order verification). Manual testing for dialog lifecycle.

## Dependencies Between Phases

```
Phase 1 ──→ Phase 2 (touch VIF uses emColor for potential UI feedback)
Phase 1 ──→ Phase 3 (emDirModel uses collection types)
Phase 1 ──→ Phase 4 (emStocks uses emColor with correct scales)
Phase 3.4 ──→ Phase 3.5 (emDirPanel FileModelState requires emDirModel FileModelState)
Phase 4.4 ──→ Phase 4.7 (PricesFetcher ListBox update requires ListBox)
Phase 4.1-4.6 are otherwise independent of each other
```

Phase 2.2 (magnetic animator) depends on verifying whether the animator is already ported — if not, porting it is a subtask within 2.2.
