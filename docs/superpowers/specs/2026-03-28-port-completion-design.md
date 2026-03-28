# Port Completion & Testing Expansion Design

Date: 2026-03-28

## Objective

Complete the eaglemode-rs emCore port to full C++ header parity and close all
NOT VERIFIED items in the marker files. NOT VERIFIED items fall into three
categories, each with a different resolution:

1. **Behavioral properties** (COW, stable iterators, BreakCrossPtrs timing,
   encoding): Port into Rust equivalents. C++ source investigation informs
   the design, not whether to build it.
2. **Usage coverage** (whether outside-emCore files use a type in ways the
   stdlib replacement doesn't cover, whether all API methods are needed):
   Investigate C++ source to determine the API surface, then port that
   surface.
3. **Implementation correctness** (whether Rust code correctly handles edge
   cases like concurrent IPC pings, rect convention conversions, HashMap
   iteration ordering): Investigate and fix. These are bugs or risks in
   existing code, not missing ports.

Expand the 7-layer testing infrastructure to cover all newly ported code and
all correctness fixes. Work is LLM-driven with human review.

## Scope

- 15 .no_rs files to resolve (port new Rust types for Category 2/3, confirm
  stdlib sufficiency for Category 1)
- 5 .rust_only files to eliminate (fold back into C++ header counterparts)
- Concrete rendering gaps to close (ImgTunnel, ImgDir, ImgDirUp, PanelPointerCache, OverwriteDialog)
- NOT VERIFIED items to close: behavioral properties to port, usage coverage
  to investigate and port, implementation correctness to investigate and fix
- Test expansion for all new and refactored code

---

## Section 1: Principles & Constraints

### Port-don't-skip

Every .no_rs type gets a Rust equivalent that replicates the C++ behavioral
contract (COW sharing, stable iteration, explicit invalidation timing, ordered
access) unless the C++ behavior literally cannot exist in safe Rust. "Rust
stdlib covers it" is not sufficient justification for skipping if the stdlib
type does not replicate the behavioral contract.

Exception: types where the C++ behavior is fully and completely replaced by
Rust stdlib with no behavioral gap. These remain as .no_rs with reviewed
evidence. The three categories:

- **Category 1 (stdlib sufficient):** `emRef` -> `Rc`, `emOwnPtr` -> `Box`,
  `emString` -> `String`, `emThread` -> `std::thread`, `emToolkit` -> explicit
  module imports. No new type, no refactoring. Existing code using these stdlib
  types is correct. `emString` encoding risk (UTF-8 vs byte-oriented paths) is
  addressed in Phase 3 via emFileStream's `PathBuf` design and a codebase-wide
  audit of file-path-in-String usage.
- **Category 2 (stdlib insufficient):** `emArray` -> `Vec` misses COW and
  stable iterators. `emAvlTree` -> `HashMap` misses ordered access. New Rust
  types built with full behavioral parity.
- **Category 3 (timing/semantics):** `emCrossPtr` -> `Weak` misses explicit
  invalidation timing. New Rust type built, call sites audited for dependence
  on the specific behavioral difference.

### Fold-back rule

All 5 .rust_only files are eliminated. Code folds into the .rs file
corresponding to its C++ header, marked with
`// RUST_ONLY: <origin_file> -- <reason>` at the insertion point. A file stays
separate only if folding would break functionality (not convention).

### Verify-then-port

NOT VERIFIED items are resolved according to their category:

- **Behavioral properties** (COW, stable iterators, BreakCrossPtrs,
  encoding): Port into Rust equivalents unconditionally. Investigation informs
  the design, not whether to build it. The investigation answers "how does C++
  use this behavior" not "does anything need this behavior."
- **Usage coverage** (whether outside-emCore files need specific API methods
  or type features): Investigate C++ source to determine the full API surface
  required. Port that surface. Do not assume a subset is sufficient because
  the current Rust code only uses a subset.
- **Implementation correctness** (whether existing Rust code handles edge
  cases correctly): Investigate, write a test that demonstrates the edge
  case, and fix if broken. These are bugs, not design questions.

### Blast radius rule

When porting a type, audit existing call sites that use the stdlib stand-in.
If a call site depends on behavior the stdlib type doesn't provide (COW,
stable iteration, ordered access, explicit invalidation), refactor it to use
the new type. If the call site only uses basic functionality that stdlib
covers, leave it alone. The investigation is per-site, not blanket
replacement.

### emPainter firewall

Do not refactor any `emPainter*.rs` file as blast radius of another change. If
a new type's introduction would require changes to emPainter code, log the
finding to `docs/empainter-deferred-refactors.log` with the type, affected
lines, and what would need to change. Alert the user. Exception: additive-only
appends (e.g., appending a type definition with zero modifications to existing
lines) are allowed.

### No standalone reimplementations

If Rust code reimplements part of an unported C++ type's functionality
(different name, different approach, subset of features), it must be refactored
to use the ported type once it exists. The standalone reimplementation is debt
to be retired, not an alternative design. This applies even if the
reimplementation is technically sufficient for its current callers.

Known instances:
- `emResTga.rs` decodes TGA from `&[u8]`, working around missing emFileStream
- `emFontCache.rs` uses `OnceLock<emImage>` single atlas, replacing C++
  `emOwnPtrArray<Entry>` dynamic cache + `emRef`/`emModel` shared ownership
  (hits emPainter firewall -- logged, not touched)

### No test assumed correct

When touching any code path that has existing tests, the tests themselves must
be analyzed for correctness before relying on them as a regression gate. A
passing test is not evidence of correctness -- it's evidence that the test's
assertions match current behavior, which may itself be wrong.

- Golden test reference data: verify it was generated from C++ output, not from
  the Rust implementation being tested.
- Behavioral test assertions: verify they match C++ behavioral contracts, not
  just current Rust behavior.
- Test coverage: verify the test actually exercises the code path affected by
  the change.

This applies to all phases. When we port a new type and refactor call sites,
every test that touches those call sites gets audited.

### Testing floor

Every new port gets inline unit tests + behavioral tests at minimum. Layer
escalation:
- Golden tests where output feeds rendering.
- Kani proofs where integer arithmetic must match C++ exactly.
- Pipeline tests for widgets.
- Integration tests for panel-tree interactions.

### File correspondence

New .rs files follow existing naming rules: `emFoo.h` -> `emFoo.rs`. The
.no_rs marker file is deleted when the .rs file is created.
CORRESPONDENCE.md is updated to reflect the change.

---

## Section 2: Phase Structure

### Phase 1 -- Rendering Gaps & .rust_only Fold-back

**Goal:** Close visible rendering gaps and eliminate all .rust_only files.

#### .rust_only fold-back

| File | Target | Action |
|------|--------|--------|
| `toolkit_images.rs` | `emBorder.rs` | Fold ToolkitImages struct + with_toolkit_images accessor. C++ correspondence: `emBorder::TkResources`. Update 8 callers' import paths. |
| `widget_utils.rs` | 8 callers | Inline `check_mouse_round_rect` into each caller (emButton, emCheckBox, emCheckButton, emRadioBox, emRadioButton, emColorField, emListBox, emTextField). Inline `trace_input_enabled` into each caller (emButton, emCheckBox, emCheckButton, emRadioBox, emRadioButton, emWindow). Matches C++ where each widget has its own copy. Delete file. |
| `fixed.rs` | `emPainter.rs` | Additive-only append of Fixed12 newtype + impl block. Zero modifications to existing emPainter lines. If fold requires changing existing code, log as deferred instead. |
| `rect.rs` | Determined during Phase 1 | Audit PixelRect for dead code (remove if dead). Trace Rect usage to identify which C++ header(s) the `GetLayoutX/Y/W/H` pattern comes from. Fold into the corresponding .rs file. |
| `emPainterDrawList.rs` | Deferred to Phase 4 | Part of emThread investigation. |

#### Rendering gaps

- Port `emCrossPtr` as a Rust type with explicit invalidation (see Section 3).
  Investigate C++ destructor cross-pointer checks first; build explicit
  invalidation mechanism regardless.
- Implement emBorder `PanelPointerCache` using emCrossPtr.
- Implement emFileDialog `OverwriteDialog` using emCrossPtr.
- Add missing toolkit images: ImgTunnel, ImgDir, ImgDirUp.

#### Cleanup

- Delete all resolved .rust_only marker files.
- Update CORRESPONDENCE.md.

#### Testing

- Golden tests for ImgTunnel, ImgDir, ImgDirUp rendering.
- Behavioral tests for emCrossPtr invalidation timing.
- Unit tests for PanelPointerCache and OverwriteDialog.
- Pipeline tests for widget behavior changes.
- Audit all existing tests touched by fold-back refactoring.

### Phase 2 -- COW Collection Family (Bottom-up)

**Goal:** Build collection types with full C++ behavioral parity (COW, stable
iteration, ordered access).

#### Dependency order

1. `emArray<T>` -- COW (Rc backing + clone-on-mutate), stable cursors,
   BinaryInsert/BinaryRemove/BinaryInsertIfNew. Foundation type.
2. `emAvlTree` -- Intrusive AVL tree with COW and stable iterators. Ordered
   access (GetFirst/Last/GetNearest*).
3. `emAvlTreeMap<K,V>` -- Built on emAvlTree. Ordered map with COW.
4. `emAvlTreeSet<T>` -- Built on emAvlTree. Ordered set with COW.
5. `emList<T>` -- Doubly-linked list with COW and stable iterators.

#### Per-type work

- Port full C++ API surface.
- Audit call sites per blast radius rule (category 2): refactor where
  behavior gap is proven, leave alone where stdlib is sufficient.
- Close all NOT VERIFIED items per type: port behavioral properties (COW,
  stable iteration) unconditionally, investigate usage coverage to determine
  full API surface, investigate and fix implementation correctness issues.

#### Testing

- Unit tests for every public method matching C++ API.
- Behavioral tests for COW semantics (shared state, clone-on-mutate, refcount).
- Behavioral tests for stable iteration (iterate while mutating, cursor
  survival across COW clone).
- Kani proofs for AVL tree balance factor arithmetic, cursor index arithmetic.
- Integration tests for refactored call sites.
- Audit all existing tests at affected call sites.

### Phase 3 -- Outside-Consumer Types

**Goal:** Port types needed by eaglemode app modules outside emCore.

#### Work items

- **emFileStream** -- Buffered I/O with endian-aware read/write. File paths
  stored as `PathBuf`/`&Path` (not String) to handle UTF-8 encoding risk.
  Investigate: verify CORRESPONDENCE.md claims about emResTga reimplementation
  (including claim that Rust implemented its own TGA loader before discovering
  C++ had one). Refactor emResTga to use emFileStream per no-standalone-
  reimplementations rule. Audit 13 outside image-loader consumers to define
  API surface.
- **emTmpFile** -- Port C++ IPC-based cleanup approach (not tempfile crate
  wrapper). 2 outside consumers (emTmpConv).
- **emOwnPtrArray<T>** -- Audit 2 emCore usages (emFontCache -- firewall,
  emFpPlugin) for behavioral gaps vs Vec<T>. Build if gaps exist.
- **emAnything** -- Audit for shared-copy semantics dependence vs Box<dyn Any>.
  Build if gap exists.
- **emString encoding audit** -- Grep codebase for file paths stored in
  `String` (not `PathBuf`). For each site, determine if non-UTF-8 paths are
  possible. Refactor to `PathBuf`/`OsString` where they are. This closes the
  encoding risk flagged in emString.no_rs and CORRESPONDENCE.md.

#### Testing

- Unit + behavioral tests for emFileStream (buffered I/O, endian read/write,
  path handling).
- Golden test for emResTga after refactor: verify identical output. Audit
  golden test reference data for correctness first (was it generated from C++
  or from Rust?).
- Behavioral tests for emTmpFile lifecycle and IPC cleanup.
- Unit tests for emOwnPtrArray if built.
- Audit all existing tests at affected call sites.

### Phase 4 -- emThread Investigation & Cleanup

**Goal:** Resolve emPainterDrawList/emThread question, close remaining items.

#### Work items

- **Investigate:** Does emPainterDrawList.rs represent the Rust replacement for
  emThread's threading role? Trace C++ rendering pipeline (emThread
  mutex-protected direct render) vs Rust pipeline (record-replay). If yes,
  rename emPainterDrawList.rs -> emThread.rs with DIVERGED: comment explaining
  the architectural change.
- Port any remaining NOT VERIFIED behavioral properties deferred from earlier
  phases.
- Final CORRESPONDENCE.md update reflecting completed state.
- Review and action any entries in `docs/empainter-deferred-refactors.log`.

#### Testing

- If rename happens, verify all existing tests still pass (no behavioral
  change, just file reorganization).
- Final test audit across all phases.

---

## Section 3: Type Design Principles

### COW semantics

Types with COW in C++ get COW in Rust. Implementation: `Rc<Inner<T>>` backing
store, `Rc::make_mut` for clone-on-mutate. Check `Rc::strong_count` to decide
whether to clone before mutation. Public API matches C++ method names per file
correspondence rules.

### Stable iterators

Types with stable iterators in C++ get cursor-based iteration in Rust. A
cursor is an opaque handle (index or key) that survives mutations, including
mutations that trigger a COW clone. Cursors are not Rust `Iterator` trait
objects (those borrow immutably). Instead: a `Cursor` type with `next()` /
`prev()` / `get()` methods that take `&self` on the collection. C++ iterator
API names (`GetFirst`, `GetLast`, `GetNext`, `GetPrev`, `GetAtKey`) map
directly to cursor methods.

### emCrossPtr explicit invalidation

Wraps `Weak<RefCell<T>>` but adds an `invalidate()` method that sets an
internal flag before the Rc drops. Callers checking `is_valid()` see
invalidation at the same time as C++ (during target's destructor-equivalent,
not after last Rc drop). Built regardless of investigation outcome (default to
port, not skip).

### emFileStream

Wraps `std::fs::File` with buffered I/O and endian-aware read/write methods
matching C++ API (`ReadUInt32LE`, `WriteFloat64BE`, etc.). File paths stored as
`PathBuf` / `&Path` (not String) to handle UTF-8 encoding risk on Unix. This
is one place where the Rust type intentionally differs from C++ `emString`-
based paths to avoid data loss.

### emTmpFile

Port the C++ IPC-based cleanup approach. The C++ design has specific behavior
around dead-directory detection that the tempfile crate doesn't replicate.

### Construction pattern

All new types follow codebase convention: `new()` primary constructor, builder
`with_*` for optional config. Names match C++ per file correspondence.
`pub(crate)` visibility default.

---

## Section 4: Testing Strategy

### Floor for every new port

- Inline unit tests in the .rs file covering every public method.
- Behavioral test file in `tests/behavioral/` covering API contracts and state
  transitions.

### Layer escalation

| Type | Unit | Behavioral | Golden | Kani | Integration | Pipeline |
|------|------|-----------|--------|------|-------------|----------|
| emArray | x | x (COW, cursors) | | x (if int math) | x (refactored sites) | |
| emAvlTree | x | x (COW, cursors, order) | | x (AVL balance) | | |
| emAvlTreeMap | x | x (COW, ordered) | | | | |
| emAvlTreeSet | x | x (COW, ordered) | | | | |
| emList | x | x (COW, cursors) | | | | |
| emCrossPtr | x | x (invalidation) | | | x (cache, dialog) | x (widgets) |
| emFileStream | x | x (I/O, endian) | x (emResTga refactor) | | | |
| emTmpFile | x | x (lifecycle, IPC) | | | | |
| emOwnPtrArray | x | x (if built) | | | | |
| emAnything | x | x (if built) | | | | |
| toolkit_images fold | | | x (new images) | | | |
| emCrossPtr features | | | x (PanelPointerCache) | | | x (OverwriteDialog) |

### COW behavioral test pattern

1. Create instance A, insert data.
2. Clone to B (verify shared backing store, not deep copy).
3. Mutate B -- verify A unchanged (clone-on-mutate).
4. Verify strong_count reflects sharing/separation.

### Stable cursor test pattern

1. Create collection, obtain cursor to element N.
2. Insert/remove other elements.
3. Verify cursor still points to element N (or reports invalidation).
4. Trigger COW clone while cursor is live -- verify cursor tracks correctly.

### emCrossPtr invalidation timing test

1. Create target T, create emCrossPtr to T.
2. Begin dropping T (trigger invalidation).
3. Verify emCrossPtr reports invalid during drop, not after.

### Kani proof targets

- AVL tree balance factor arithmetic (rotation correctness).
- Any div255 or fixed-point math in new types.
- Cursor index arithmetic (no overflow/underflow on insert/remove adjustment).

### Test audit protocol

Before relying on any existing test as a regression gate:
1. Check golden reference data provenance (C++ generated vs Rust generated).
2. Check behavioral assertions against C++ contracts.
3. Check that the test exercises the code path being changed.
