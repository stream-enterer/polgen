# Golden Parity Test Harness

> Specification for all remaining panel-tree-level golden parity tests.
>
> Self-contained. Hand this document to an LLM and it can write the
> C++ generators and Rust tests autonomously, then build, generate,
> and verify.

---

## 1. Context

### 1.1 Current State

140 golden tests pass (4 ignored). Coverage by subsystem:

| Subsystem | Tests | Status |
|-----------|-------|--------|
| Painter | 42 | Complete |
| Layout | 13 | Complete |
| Compositor | 5 | Complete |
| Input dispatch | 4 | Complete |
| Notice dispatch | 8 | 7/10 C++ flags tested |
| Behavioral (focus/activation) | 13 | Core ops covered |
| Animator trajectory | 10 | Complete |
| Input filter (VIF) | 8 | Complete |
| Widget rendering | 18 (14 pass, 4 ignored) | At ceiling (9-slice) |
| Widget interaction | 14 | Complete |
| Scheduler | 11 (unit, no golden) | Complete |

### 1.2 What This Harness Covers

Remaining panel-tree-level golden tests — operations on `PanelTree`,
`View`, and `PanelBehavior` that have C++ equivalents but no golden
coverage. Excludes:

- Widget rendering (at 9-slice ceiling)
- Widget interaction (complete)
- Painter/compositor (complete)
- Animator/VIF trajectories (complete)
- Scheduler (unit tests sufficient)
- Rust-only features (`CANVAS_CHANGED`, `VIEW_CHANGED` flags)

### 1.3 Codebase Paths

```
C++ source:      ~/.local/git/eaglemode-0.96.4/
C++ generator:   golden_gen/gen_golden.cpp
Rust tests:      tests/golden_parity/
Golden data:     golden/  (gitignored, regenerated)
```

### 1.4 Build & Run

```bash
make -C golden_gen          # Build C++ generator
make -C golden_gen run      # Generate golden files
cargo test -p zuicchini     # Run all tests
cargo clippy --workspace -- -D warnings  # Must be clean
```

---

## 2. Anti-Pattern Rules

Carry forward from all prior phases. Violations → revert.

| ID | Rule | Enforcement |
|----|------|-------------|
| R1 | No speculative fixes | Every test motivated by a specific untested C++ API |
| R2 | One test at a time | Write C++ gen + Rust test, build, run, verify before next |
| R3 | Mandatory offramp | If C++ and Rust semantics diverge, skip test with `#[ignore]` and document why |
| R4 | Measure passing tests | Full `cargo test -p zuicchini` after every batch — no regressions |
| R5 | Match C++ exactly | C++ generator is the oracle. If Rust behavior differs, the Rust code has a gap |
| R6 | No scope creep | Do not fix Rust bugs found during test writing. Document them and move on |

---

## 3. Infrastructure Reference

### 3.1 Golden Format Types

| Format | Extension | Loader | Comparator |
|--------|-----------|--------|------------|
| Notice | `.notice.golden` | `load_notice_golden()` | `compare_notices()` with `translate_cpp_notice_flags()` |
| Behavioral | `.behavioral.golden` | `load_behavioral_golden()` | `compare_behavioral()` |
| Input | `.input.golden` | `load_input_golden()` | `compare_input()` |
| Layout | `.layout.golden` | `load_layout_golden()` | `compare_rects()` with `scale_golden_rects()` |

### 3.2 C++ Generator Patterns

**Notice test** (`RecordingPanel` accumulates flags):
```cpp
static void gen_notice_NAME() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    // GoldenViewPort vp(view);  // Only if SetViewFocused/InputToView needed

    auto* root = new RecordingPanel(view, "root");
    auto* child1 = new RecordingPanel(*root, "child1");
    // ...

    // Settle initial notices
    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();

    // ACTION
    // ...

    // Deliver new notices
    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_notice("notice_NAME", {root, child1});
}
```

**Behavioral test** (plain `emPanel`, check active/path state):
```cpp
static void gen_focus_NAME() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    // ...

    // ACTION (Focus, Activate, GetFocusable*, etc.)
    // ...

    dump_behavioral("focus_NAME", {root, child1, ...});
}
```

### 3.3 Rust Test Patterns

**Notice test** (in `tests/golden_parity/notice.rs`):
```rust
#[test]
fn notice_NAME() {
    require_golden!();
    let expected = load_notice_golden("notice_NAME");
    // Build tree, attach NoticeBehavior, settle, reset
    // ACTION
    // settle, collect bits, compare_notices
}
```

**Behavioral test** (in `tests/golden_parity/interaction.rs`):
```rust
#[test]
fn interaction_NAME() {
    require_golden!();
    let expected = load_behavioral_golden("NAME");
    // Build tree, setup view
    // ACTION
    // Collect panel_state(), compare_behavioral
}
```

### 3.4 C++ API Reference

All public on `emPanel` (line 35–535 in `emPanel.h`):

| C++ Method | Rust Equivalent | Used In Tests |
|------------|----------------|---------------|
| `Activate()` | `view.set_active_panel(tree, id, false)` | ✓ |
| `Focus()` | `view.focus_panel(tree, id)` or `view.set_active_panel(tree, id, true)` | ✓ |
| `SetFocusable(bool)` | `tree.set_focusable(id, bool)` | ✓ |
| `SetEnableSwitch(bool)` | `tree.set_enable_switch(id, bool)` | ✓ |
| `GetFocusableNext()` | `tree.focusable_next(id)` | ✓ |
| `GetFocusablePrev()` | `tree.focusable_prev(id)` | ✓ |
| `GetFocusableParent()` | `tree.focusable_ancestor(id)` | ✓ |
| `GetFocusableFirstChild()` | `tree.focusable_first_child(id)` | ✓ |
| `GetFocusableLastChild()` | `tree.focusable_last_child(id)` | Not yet |
| `Layout(x,y,w,h,cc)` | `tree.set_layout_rect(id,x,y,w,h)` | ✓ |
| `IsEnabled()` | `ctx.is_enabled()` / `tree.get(id).enabled` | Not yet |
| `IsViewed()` | `tree.get(id).viewed` (if field exists) | Not yet |
| `delete panel` | `view.remove_panel(tree, id)` | ✓ |

C++ `emView` visit methods (all have Rust equivalents):

| C++ Method | Rust Equivalent | Used In Tests |
|------------|----------------|---------------|
| `VisitNext()` | `view.visit_next(tree)` | ✓ |
| `VisitPrev()` | `view.visit_prev(tree)` | ✓ |
| `VisitIn()` | `view.visit_in(tree)` | ✓ |
| `VisitOut()` | `view.visit_out(tree)` | ✓ |
| `VisitFirst()` | `view.visit_first(tree)` | Not yet |
| `VisitLast()` | `view.visit_last(tree)` | Not yet |
| `VisitLeft()` | `view.visit_left(tree)` | Not yet |
| `VisitRight()` | `view.visit_right(tree)` | Not yet |
| `VisitUp()` | `view.visit_up(tree)` | Not yet |
| `VisitDown()` | `view.visit_down(tree)` | Not yet |

---

## 4. Test Specifications

Each test specifies:
- **Golden name:** filename prefix for golden data
- **Format:** which golden format to use
- **C++ generator:** exact code for `gen_golden.cpp`
- **Rust test:** exact code for the test file
- **What it verifies:** the untested API/behavior

Tests are grouped into phases. Complete one phase before starting the next.

---

### Phase 1: Focus Navigation (Behavioral)

Tests for `VisitFirst`, `VisitLast`, `VisitLeft`, `VisitRight`, `VisitUp`,
`VisitDown` — the 6 visit operations with no golden coverage.

#### Test 1.1: `focus_visit_first`

**Golden name:** `focus_visit_first`
**Format:** behavioral
**Verifies:** `VisitFirst()` — from middle child, jump to first focusable sibling

**C++ generator:**
```cpp
// VisitFirst: from child2, jump to first focusable sibling (child1).
static void gen_focus_visit_first() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* child2 = new emPanel(*root, "child2");
    emPanel* child3 = new emPanel(*root, "child3");

    child2->Focus();
    // C++ VisitFirst goes to parent's first focusable child
    emPanel* first = child2->GetParent()->GetFocusableFirstChild();
    if (first) first->Focus();

    dump_behavioral("focus_visit_first", {root, child1, child2, child3});
}
```

**Rust test** (in `interaction.rs`):
```rust
#[test]
fn interaction_focus_visit_first() {
    require_golden!();
    let expected = load_behavioral_golden("focus_visit_first");

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
    let child1 = tree.create_child(root, "child1");
    tree.set_layout_rect(child1, 0.0, 0.0, 0.33, 1.0);
    let child2 = tree.create_child(root, "child2");
    tree.set_layout_rect(child2, 0.33, 0.0, 0.33, 1.0);
    let child3 = tree.create_child(root, "child3");
    tree.set_layout_rect(child3, 0.66, 0.0, 0.34, 1.0);

    let mut view = View::new(root, 100.0, 100.0);
    view.update_viewing(&mut tree);

    view.set_window_focused(&mut tree, true);
    view.set_active_panel(&mut tree, child2, true);
    view.visit_first(&mut tree);

    let actual = vec![
        panel_state(&tree, root),
        panel_state(&tree, child1),
        panel_state(&tree, child2),
        panel_state(&tree, child3),
    ];
    compare_behavioral(&actual, &expected, &["root", "child1", "child2", "child3"]).unwrap();
}
```

#### Test 1.2: `focus_visit_last`

**Golden name:** `focus_visit_last`
**Format:** behavioral
**Verifies:** `VisitLast()` — from middle child, jump to last focusable sibling

**C++ generator:**
```cpp
// VisitLast: from child1, jump to last focusable sibling (child3).
static void gen_focus_visit_last() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* child2 = new emPanel(*root, "child2");
    emPanel* child3 = new emPanel(*root, "child3");

    child1->Focus();
    // C++ VisitLast goes to parent's last focusable child
    emPanel* last = child1->GetParent()->GetFocusableLastChild();
    if (last) last->Focus();

    dump_behavioral("focus_visit_last", {root, child1, child2, child3});
}
```

**Rust test** (in `interaction.rs`):
```rust
#[test]
fn interaction_focus_visit_last() {
    require_golden!();
    let expected = load_behavioral_golden("focus_visit_last");

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
    let child1 = tree.create_child(root, "child1");
    tree.set_layout_rect(child1, 0.0, 0.0, 0.33, 1.0);
    let child2 = tree.create_child(root, "child2");
    tree.set_layout_rect(child2, 0.33, 0.0, 0.33, 1.0);
    let child3 = tree.create_child(root, "child3");
    tree.set_layout_rect(child3, 0.66, 0.0, 0.34, 1.0);

    let mut view = View::new(root, 100.0, 100.0);
    view.update_viewing(&mut tree);

    view.set_window_focused(&mut tree, true);
    view.set_active_panel(&mut tree, child1, true);
    view.visit_last(&mut tree);

    let actual = vec![
        panel_state(&tree, root),
        panel_state(&tree, child1),
        panel_state(&tree, child2),
        panel_state(&tree, child3),
    ];
    compare_behavioral(&actual, &expected, &["root", "child1", "child2", "child3"]).unwrap();
}
```

#### Test 1.3: `focus_visit_left`

**Golden name:** `focus_visit_left`
**Format:** behavioral
**Verifies:** `VisitLeft()` — spatial navigation to left neighbour

**C++ generator:**
```cpp
// VisitLeft: 3 children side by side, focus child3, visit left → child2.
static void gen_focus_visit_left() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    GoldenViewPort vp(view);

    emPanel* root = new emPanel(view, "root");
    root->Layout(0, 0, 1, 0.75);
    emPanel* child1 = new emPanel(*root, "child1");
    child1->Layout(0, 0, 0.33, 1);
    emPanel* child2 = new emPanel(*root, "child2");
    child2->Layout(0.33, 0, 0.33, 1);
    emPanel* child3 = new emPanel(*root, "child3");
    child3->Layout(0.66, 0, 0.34, 1);

    // Settle to establish viewing
    { TerminateEngine ctrl(sched, 30); sched.Run(); }

    child3->Focus();
    // C++ VisitLeft uses spatial neighbour lookup
    view.VisitLeft();

    dump_behavioral("focus_visit_left", {root, child1, child2, child3});
}
```

**Rust test** (in `interaction.rs`):
```rust
#[test]
fn interaction_focus_visit_left() {
    require_golden!();
    let expected = load_behavioral_golden("focus_visit_left");

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 0.75);
    let child1 = tree.create_child(root, "child1");
    tree.set_layout_rect(child1, 0.0, 0.0, 0.33, 1.0);
    let child2 = tree.create_child(root, "child2");
    tree.set_layout_rect(child2, 0.33, 0.0, 0.33, 1.0);
    let child3 = tree.create_child(root, "child3");
    tree.set_layout_rect(child3, 0.66, 0.0, 0.34, 1.0);

    let mut view = View::new(root, 800.0, 600.0);
    view.update_viewing(&mut tree);

    view.set_window_focused(&mut tree, true);
    view.set_active_panel(&mut tree, child3, true);
    view.visit_left(&mut tree);

    let actual = vec![
        panel_state(&tree, root),
        panel_state(&tree, child1),
        panel_state(&tree, child2),
        panel_state(&tree, child3),
    ];
    compare_behavioral(&actual, &expected, &["root", "child1", "child2", "child3"]).unwrap();
}
```

**Note:** `VisitLeft/Right/Up/Down` use spatial neighbour lookup based on
panel layout positions. The C++ generator MUST set layout rects via
`Layout()` and settle to establish viewing state before calling the visit
method. If the Rust spatial lookup implementation differs from C++
(e.g., different tie-breaking), take offramp R3.

#### Test 1.4: `focus_visit_right`

**Golden name:** `focus_visit_right`
**Format:** behavioral
**Verifies:** `VisitRight()` — spatial navigation to right neighbour

Same tree as 1.3, but focus child1, then `VisitRight()` → child2.

**C++ generator:** Same as 1.3 but:
```cpp
    child1->Focus();
    view.VisitRight();
    dump_behavioral("focus_visit_right", {root, child1, child2, child3});
```

**Rust test:** Same as 1.3 but:
```rust
    view.set_active_panel(&mut tree, child1, true);
    view.visit_right(&mut tree);
```

#### Test 1.5: `focus_visit_down`

**Golden name:** `focus_visit_down`
**Format:** behavioral
**Verifies:** `VisitDown()` — spatial navigation downward

**C++ generator:**
```cpp
// VisitDown: 3 children stacked vertically, focus child1, visit down → child2.
static void gen_focus_visit_down() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    GoldenViewPort vp(view);

    emPanel* root = new emPanel(view, "root");
    root->Layout(0, 0, 1, 0.75);
    emPanel* child1 = new emPanel(*root, "child1");
    child1->Layout(0, 0, 1, 0.33);
    emPanel* child2 = new emPanel(*root, "child2");
    child2->Layout(0, 0.33, 1, 0.33);
    emPanel* child3 = new emPanel(*root, "child3");
    child3->Layout(0, 0.66, 1, 0.34);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }

    child1->Focus();
    view.VisitDown();

    dump_behavioral("focus_visit_down", {root, child1, child2, child3});
}
```

**Rust test:** Vertical layout (children stacked), focus child1, visit_down.

#### Test 1.6: `focus_visit_up`

Same tree as 1.5, but focus child3, then `VisitUp()` → child2.

---

### Phase 2: Enable/Disable State (Notice + Behavioral)

Tests for recursive enable propagation and its interaction with
focus traversal.

#### Test 2.1: `notice_recursive_enable`

**Golden name:** `notice_recursive_enable`
**Format:** notice
**Verifies:** Disabling a parent fires `ENABLE_CHANGED` on children too

**C++ generator:**
```cpp
// Disable parent → children also get NF_ENABLE_CHANGED.
static void gen_notice_recursive_enable() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    auto* root = new RecordingPanel(view, "root");
    auto* child1 = new RecordingPanel(*root, "child1");
    auto* gc = new RecordingPanel(*child1, "gc");
    auto* child2 = new RecordingPanel(*root, "child2");

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();
    gc->ResetRecording();
    child2->ResetRecording();

    // Action: disable child1 → gc should also get ENABLE_CHANGED
    child1->SetEnableSwitch(false);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_notice("notice_recursive_enable", {root, child1, gc, child2});
}
```

**Rust test** (in `notice.rs`):
```rust
#[test]
fn notice_recursive_enable() {
    require_golden!();
    let expected = load_notice_golden("notice_recursive_enable");

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
    let child1 = tree.create_child(root, "child1");
    tree.set_layout_rect(child1, 0.0, 0.0, 0.5, 1.0);
    let gc = tree.create_child(child1, "gc");
    tree.set_layout_rect(gc, 0.0, 0.0, 1.0, 1.0);
    let child2 = tree.create_child(root, "child2");
    tree.set_layout_rect(child2, 0.5, 0.0, 0.5, 1.0);

    let mut view = View::new(root, 800.0, 600.0);
    view.set_window_focused(&mut tree, false);

    let acc_root = attach_notice(&mut tree, root);
    let acc_child1 = attach_notice(&mut tree, child1);
    let acc_gc = attach_notice(&mut tree, gc);
    let acc_child2 = attach_notice(&mut tree, child2);

    settle(&mut tree, &mut view);
    reset(&acc_root);
    reset(&acc_child1);
    reset(&acc_gc);
    reset(&acc_child2);

    tree.set_enable_switch(child1, false);

    settle(&mut tree, &mut view);

    let actual = vec![
        acc_root.borrow().bits(),
        acc_child1.borrow().bits(),
        acc_gc.borrow().bits(),
        acc_child2.borrow().bits(),
    ];
    compare_notices(
        &actual, &expected,
        &["root", "child1", "gc", "child2"],
        NOTICE_FULL_MASK,
    ).unwrap();
}
```

#### Test 2.2: `notice_re_enable`

**Golden name:** `notice_re_enable`
**Format:** notice
**Verifies:** Re-enabling a parent fires `ENABLE_CHANGED` again

Same tree as 2.1, but after disabling and settling, reset recordings,
then `SetEnableSwitch(true)`. Children should get `ENABLE_CHANGED` again.

#### Test 2.3: `focus_disabled_panel`

**Golden name:** `focus_disabled_panel`
**Format:** behavioral
**Verifies:** What happens when the active panel is disabled — does
focus walk to a focusable ancestor? C++ behavior may vary.

**Note:** This test may hit offramp R3 if C++ allows focusing disabled
panels (they are still focusable, just disabled for input). Investigate
C++ `emPanel::IsEnabled()` interaction with `GetFocusableNext()` before
implementing.

---

### Phase 3: Panel Lifecycle (Behavioral)

Tests for removal edge cases.

#### Test 3.1: `activate_remove_middle`

**Golden name:** `activate_remove_middle`
**Format:** behavioral
**Verifies:** Remove a non-active panel from the middle of the tree

**C++ generator:**
```cpp
// Remove non-active middle child → remaining panels unaffected.
static void gen_activate_remove_middle() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* child2 = new emPanel(*root, "child2");
    emPanel* child3 = new emPanel(*root, "child3");

    child1->Focus();
    delete child2;  // Remove non-active panel

    dump_behavioral("activate_remove_middle", {root, child1, child3});
}
```

**Rust test:**
```rust
#[test]
fn interaction_activate_remove_middle() {
    require_golden!();
    let expected = load_behavioral_golden("activate_remove_middle");

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
    let child1 = tree.create_child(root, "child1");
    tree.set_layout_rect(child1, 0.0, 0.0, 0.33, 1.0);
    let child2 = tree.create_child(root, "child2");
    tree.set_layout_rect(child2, 0.33, 0.0, 0.33, 1.0);
    let child3 = tree.create_child(root, "child3");
    tree.set_layout_rect(child3, 0.66, 0.0, 0.34, 1.0);

    let mut view = View::new(root, 100.0, 100.0);
    view.update_viewing(&mut tree);

    view.set_window_focused(&mut tree, true);
    view.set_active_panel(&mut tree, child1, true);
    view.remove_panel(&mut tree, child2);

    let actual = vec![
        panel_state(&tree, root),
        panel_state(&tree, child1),
        panel_state(&tree, child3),
    ];
    compare_behavioral(&actual, &expected, &["root", "child1", "child3"]).unwrap();
}
```

#### Test 3.2: `activate_remove_in_path`

**Golden name:** `activate_remove_in_path`
**Format:** behavioral
**Verifies:** Remove a panel that is in the active path (not the active
panel itself, but its parent)

**C++ generator:**
```cpp
// Focus gc (grandchild), then remove child1 (its parent, which is in active path).
static void gen_activate_remove_in_path() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* gc = new emPanel(*child1, "gc");
    emPanel* child2 = new emPanel(*root, "child2");

    gc->Focus();
    delete child1;  // Removes child1 + gc (entire subtree)

    dump_behavioral("activate_remove_in_path", {root, child2});
}
```

**Rust test:** Focus gc, remove child1 subtree, check root+child2 state.

#### Test 3.3: `notice_remove_child`

**Golden name:** `notice_remove_child`
**Format:** notice
**Verifies:** Removing a child fires `CHILDREN_CHANGED` on parent

**C++ generator:**
```cpp
static void gen_notice_remove_child() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    auto* root = new RecordingPanel(view, "root");
    auto* child1 = new RecordingPanel(*root, "child1");
    auto* child2 = new RecordingPanel(*root, "child2");

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();
    child2->ResetRecording();

    // Action: remove child2
    delete child2;

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    // Only root and child1 remain
    dump_notice("notice_remove_child", {root, child1});
}
```

**Rust test:** Build tree, settle, reset, remove child2, settle, compare
root+child1 notice flags (expect `CHILDREN_CHANGED` on root).

---

### Phase 4: Deep Focus Traversal (Behavioral)

Tests for focus traversal in deeper trees (3+ levels).

#### Test 4.1: `focus_tab_deep`

**Golden name:** `focus_tab_deep`
**Format:** behavioral
**Verifies:** `VisitNext` from a grandchild when siblings exist at
multiple levels

**C++ generator:**
```cpp
// Tree: root → child1 → gc1, gc2; root → child2
// Focus gc1, VisitNext → gc2 (same-level sibling).
static void gen_focus_tab_deep() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* gc1 = new emPanel(*child1, "gc1");
    emPanel* gc2 = new emPanel(*child1, "gc2");
    emPanel* child2 = new emPanel(*root, "child2");

    gc1->Focus();
    emPanel* next = gc1->GetFocusableNext();
    if (next) next->Focus();

    dump_behavioral("focus_tab_deep", {root, child1, gc1, gc2, child2});
}
```

**Rust test:** Build deep tree, focus gc1, visit_next, compare state.

#### Test 4.2: `focus_tab_ascend`

**Golden name:** `focus_tab_ascend`
**Format:** behavioral
**Verifies:** `VisitNext` from the last grandchild ascends and wraps
to the parent's first focusable child

**C++ generator:**
```cpp
// Tree: root → child1 → gc1, gc2
// Focus gc2 (last), VisitNext → should wrap to gc1 (parent's first).
static void gen_focus_tab_ascend() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* gc1 = new emPanel(*child1, "gc1");
    emPanel* gc2 = new emPanel(*child1, "gc2");

    gc2->Focus();
    emPanel* next = gc2->GetFocusableNext();
    if (next) {
        next->Focus();
    } else {
        emPanel* p = gc2->GetFocusableParent();
        if (p) {
            emPanel* fc = p->GetFocusableFirstChild();
            if (fc) fc->Focus();
        }
    }

    dump_behavioral("focus_tab_ascend", {root, child1, gc1, gc2});
}
```

**Rust test:** Focus gc2, visit_next, compare state.

#### Test 4.3: `focus_visit_out_to_root`

**Golden name:** `focus_visit_out_to_root`
**Format:** behavioral
**Verifies:** `VisitOut` from a root-level child (no focusable parent
except root itself)

**C++ generator:**
```cpp
// Focus child1, VisitOut → should go to root (parent).
static void gen_focus_visit_out_to_root() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    emPanel* root = new emPanel(view, "root");
    emPanel* child1 = new emPanel(*root, "child1");
    emPanel* child2 = new emPanel(*root, "child2");

    child1->Focus();
    emPanel* parent = child1->GetFocusableParent();
    if (parent) parent->Focus();

    dump_behavioral("focus_visit_out_to_root", {root, child1, child2});
}
```

**Rust test:** Focus child1, visit_out, compare state (root should be active).

---

### Phase 5: Input Dispatch Edge Cases (Input)

Additional input routing tests.

#### Test 5.1: `input_mouse_miss`

**Golden name:** `input_mouse_miss`
**Format:** input
**Verifies:** Click in empty space (no panel at coordinates) — no panel
receives input, root may activate

**C++ generator:**
```cpp
// Click at (400, 599) — below all panels if root tallness < 1.0.
static void gen_input_mouse_miss() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    GoldenViewPort vp(view);

    auto* root = new RecordingPanel(view, "root");
    root->Layout(0, 0, 1, 0.5);  // Only covers top half
    auto* child1 = new RecordingPanel(*root, "child1");
    child1->Layout(0, 0, 1, 1);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();

    // Click below the panel area
    emInputEvent event;
    emInputState state;
    state.SetMouse(400, 500);  // Below root (root ends at ~375 in 800x600 viewport)
    event.Setup(EM_KEY_LEFT_BUTTON, emString(), 1, 0);
    vp.DoInputToView(event, state);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_input("input_mouse_miss", {root, child1});
}
```

**Rust test:** Create tree with limited height, click below it, check
that no panel receives input.

#### Test 5.2: `input_nested_hit`

**Golden name:** `input_nested_hit`
**Format:** input
**Verifies:** Click on a grandchild panel — deepest panel receives input

**C++ generator:**
```cpp
// Tree: root → child1 → gc. Click at gc's position → gc receives input.
static void gen_input_nested_hit() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    GoldenViewPort vp(view);

    auto* root = new RecordingPanel(view, "root");
    root->Layout(0, 0, 1, 0.75);
    auto* child1 = new RecordingPanel(*root, "child1");
    child1->Layout(0, 0, 0.5, 1);
    auto* gc = new RecordingPanel(*child1, "gc");
    gc->Layout(0, 0, 1, 1);
    auto* child2 = new RecordingPanel(*root, "child2");
    child2->Layout(0.5, 0, 0.5, 1);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();
    gc->ResetRecording();
    child2->ResetRecording();

    // Click at (100, 300) → inside gc (which fills child1's left half)
    emInputEvent event;
    emInputState state;
    state.SetMouse(100, 300);
    event.Setup(EM_KEY_LEFT_BUTTON, emString(), 1, 0);
    vp.DoInputToView(event, state);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_input("input_nested_hit", {root, child1, gc, child2});
}
```

**Rust test:** Build nested tree, click at gc coordinates, check gc
receives input and becomes active.

---

### Phase 6: Notice Combinations (Notice)

Tests for multiple notice flags fired by a single operation.

#### Test 6.1: `notice_focus_and_layout`

**Golden name:** `notice_focus_and_layout`
**Format:** notice
**Verifies:** Focus a panel AND change its layout in the same settle
cycle — both flags should appear

**C++ generator:**
```cpp
static void gen_notice_focus_and_layout() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);
    GoldenViewPort vp(view);

    auto* root = new RecordingPanel(view, "root");
    auto* child1 = new RecordingPanel(*root, "child1");
    auto* child2 = new RecordingPanel(*root, "child2");

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();
    child2->ResetRecording();

    // Two actions before settle: focus + layout change
    child1->Focus();
    child1->Layout(0.1, 0.1, 0.3, 0.5);

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_notice("notice_focus_and_layout", {root, child1, child2});
}
```

**Rust test:** Focus child1 + change its layout rect before settling.
Both `FOCUS_CHANGED` + `LAYOUT_CHANGED` (plus `ACTIVE_CHANGED`,
`VIEW_FOCUS_CHANGED`) should appear in child1's accumulated flags.

#### Test 6.2: `notice_add_and_activate`

**Golden name:** `notice_add_and_activate`
**Format:** notice
**Verifies:** Add a new child and immediately activate it before settle

**C++ generator:**
```cpp
static void gen_notice_add_and_activate() {
    emStandardScheduler sched;
    emRootContext ctx(sched);
    emView view(ctx, 0);

    auto* root = new RecordingPanel(view, "root");
    auto* child1 = new RecordingPanel(*root, "child1");

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    root->ResetRecording();
    child1->ResetRecording();

    // Add new child and activate it before settling
    auto* child2 = new RecordingPanel(*root, "child2");
    child2->Activate();

    { TerminateEngine ctrl(sched, 30); sched.Run(); }
    dump_notice("notice_add_and_activate", {root, child1, child2});
}
```

**Rust test:** Settle, reset, create child2 + activate it, settle, compare.

---

## 5. Execution Protocol

### 5.1 Phase Ordering

Execute phases 1–6 in order. Within each phase, implement tests
sequentially (R2).

### 5.2 Per-Test Workflow

For each test:

1. **Write C++ generator** in `golden_gen/gen_golden.cpp`
   - Add generator function near existing similar functions
   - Add call to `main()` in the appropriate section
2. **Build generator:** `make -C golden_gen`
   - Must compile with no errors (warnings from `golden_format.h` OK)
3. **Generate golden data:** `make -C golden_gen run`
   - Confirm the new golden file appears in output
4. **Write Rust test** in the appropriate test file
   - `notice.rs` for notice tests
   - `interaction.rs` for behavioral tests
   - `input.rs` for input tests
5. **Run single test:** `cargo test -p zuicchini TEST_NAME -- --nocapture`
   - Must pass
6. **Run full suite:** `cargo test -p zuicchini`
   - No regressions (same pass/ignore count or better)
7. **Run clippy:** `cargo clippy --workspace -- -D warnings`
   - Must be clean

### 5.3 Offramp Criteria (R3)

Skip a test with `#[ignore = "reason"]` if:

- C++ and Rust produce different golden data AND the difference
  is a known design divergence (cite from MEMORY.md or CLAUDE.md)
- The C++ API being tested does not exist in Rust (MISSING)
- The C++ behavior depends on viewing/viewport state that the Rust
  test harness cannot reproduce deterministically

When skipping, add a comment in both the C++ generator and Rust test
explaining why. Update the summary table in this document.

### 5.4 Commit Protocol

Commit after each completed phase (not per-test). Message format:

```
Golden harness phase N: <summary>

<N> new golden tests: <test list>
<total> golden tests pass (<ignored> ignored).
```

---

## 6. Summary Table

Track progress here. Update after each phase.

| Phase | Tests Planned | Tests Pass | Tests Skipped | Status |
|-------|--------------|------------|---------------|--------|
| 1. Focus Navigation | 6 | 6 | 0 | DONE |
| 2. Enable/Disable | 3 | 3 | 0 | DONE |
| 3. Panel Lifecycle | 3 | 3 | 0 | DONE |
| 4. Deep Focus | 3 | 3 | 0 | DONE |
| 5. Input Edge Cases | 2 | 1 | 1 | DONE |
| 6. Notice Combos | 2 | 2 | 0 | DONE |
| **Total** | **19** | **18** | **1** | |

Starting baseline: 140 golden tests pass, 4 ignored.
Final: 158 golden tests pass (140 + 18), 5 ignored (4 + 1).

Skipped test: `input_mouse_miss` — C++ activates root on empty-space click, Rust does not (activation fallback divergence).

Bug found: Rust `visit_down`/`visit_up` direction transforms were swapped. Fixed: Down→(dy,-dx), Up→(-dy,dx) matching C++ VisitNeighbour.

---

## 7. Post-Harness Gaps

These are known gaps that are NOT addressed by this harness because
they lack C++ golden-test equivalents or require infrastructure changes:

| Gap | Why Not Tested |
|-----|---------------|
| `CANVAS_CHANGED` notice | Rust-only flag (no C++ `NF_CANVAS_CHANGED`) |
| `VIEW_CHANGED` notice | Rust-only flag |
| `SOUGHT_NAME_CHANGED` notice | No `SetSoughtName` API exercised in zuicchini |
| `MEMORY_LIMIT_CHANGED` notice | Only used in `file_model.rs`, not panel tree |
| `NF_VIEWING_CHANGED` notice | Requires viewport manipulation to trigger |
| Panel reparenting | No `reparent()` API in zuicchini |
| Auto-expansion | Requires multi-panel test harness with viewing |
| Clipboard operations | `StubClipboard` only, no real clipboard |
| Multi-view | Single-view only in test harness |
| View coordinate conversion | Internal math, not golden-testable |
| Widget rendering (4 ignored) | 9-slice interpolation precision ceiling |
