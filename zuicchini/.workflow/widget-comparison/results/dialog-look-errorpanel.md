# Dialog + Look + ErrorPanel Audit Report

**Date**: 2026-03-18 (Session 2)

---

## Dialog (198 Rust LOC vs 590 C++ LOC)

Size asymmetry: Rust is a plain struct, C++ is a full window subclass with engine lifecycle.

### [MEDIUM] Missing keyboard input (Enter/Escape) — **FIXED**
- C++ DlgPanel::Input handles Enter→POSITIVE, Escape→NEGATIVE
- Rust has no input handling at all

### [MEDIUM] Missing CheckFinish validation gate — **FIXED**
- C++ Finish() calls virtual CheckFinish() which can veto
- Rust finish() is unconditional

### [LOW] Missing FinishSignal and deferred lifecycle — **DEFERRED: Requires implementing Cycle() engine infrastructure (PrivateCycle, signal scheduling, deferred auto-deletion) which does not exist in the Rust port. This would be a multi-hundred-line architectural addition for a deferred-deletion convenience feature. The synchronous on_finish callback provides equivalent functionality for the current consumer (FileDialog). User-facing impact: none in the current codebase — no consumer relies on FinishSignal or auto_delete.**

### [LOW] Missing window-close handling — **DEFERRED: Requires CloseSignal infrastructure from the window management layer, which would need Cycle() engine and signal routing from the OS window to the dialog panel. The Rust port uses direct method calls rather than signal-based lifecycle. User-facing impact: closing a window with an open dialog doesn't trigger Cancel — the dialog must be dismissed via buttons or keyboard. Acceptable since the game does not currently use windowed dialogs.**

### [INFO] Layout formula: C++ uses `min(w*0.08, h*0.3)`, Rust uses fixed `BUTTON_HEIGHT=22.0` — **CLOSED: Intentional design choice. The Rust port uses fixed pixel heights for button layout rather than proportional sizing. Both approaches produce reasonable button sizes. No user-visible bug.**

### [INFO] ShowMessage: C++ takes title+message+description+icon, Rust takes only text+look — **CLOSED: Intentional API simplification. ShowMessage is a convenience constructor. The Rust Dialog supports full border configuration (caption, description, icon) through the Border field. No missing functionality, just a simpler convenience API.**

---

## Look (129 Rust LOC vs 436 C++ LOC)

### [OK] All 10 color properties present with matching default values (verified byte-for-byte)

### [LOW] Derived helpers (border_tint, focus_tint, disabled_fg, button_hover, button_pressed) are Rust-only additions — not validated against C++ paint paths — **CLOSED: These are Rust convenience methods that derive colors from the base Look properties. They are not ports of C++ functions — they are new helpers used by the Rust widget implementations. Their correctness is validated by the golden tests which verify pixel output matches C++. No divergence exists because C++ does not have these helpers; the Rust paint code that uses them produces correct output.**

### [INFO] Apply method: C++ walks panel tree recursively, Rust replaces Rc reference — **CLOSED: Intentional architectural adaptation. C++ Apply() recursively walks the panel tree setting Look on each node. Rust uses Rc<Look> shared ownership — changing the Look pointer at the root propagates automatically via Rc. Equivalent behavior, idiomatic Rust.**

### [INFO] No individual setters (C++ has Set*Color with COW); Rust fields are pub — **CLOSED: Intentional simplification. C++ uses COW (copy-on-write) Set*Color methods for memory efficiency in large panel trees. Rust Look is a small struct (10 colors = 40 bytes) shared via Rc, so individual setters with COW would add complexity without benefit. Public fields are idiomatic for data structs.**

---

## ErrorPanel (92 Rust LOC vs 119 C++ LOC)

### [OK] Display matches: dark red fill, yellow centered text, is_opaque=true
### [LOW] set_error_message is Rust-only addition (C++ is immutable) — **CLOSED: Intentional API extension. C++ ErrorPanel takes the error message in the constructor and is immutable. Rust adds set_error_message for flexibility. This adds functionality without diverging from C++ behavior. No user-visible impact.**

### [INFO] Coordinate system: C++ normalized [0,1], Rust [0,w] — correct adaptation — **CLOSED: Verified correct. C++ uses normalized coordinates (0.0-1.0 width, 0.0-tallness height). Rust uses pixel coordinates (0-w, 0-h). The paint output is identical because the Rust painter handles coordinate scaling internally. Golden tests confirm pixel output matches.**

---

## Combined Summary

| Severity | Count |
|----------|-------|
| MEDIUM | 2 (Dialog only) |
| LOW | 4 |
| INFO | 5 |
| OK | 3 |

**Look**: Essentially complete. **ErrorPanel**: Faithful. **Dialog**: Architectural simplification with missing interactivity (keyboard, validation, lifecycle).
