# G1 Area Sampling Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `interpolate_scanline_area_inner` and `simulate_carry_chain` with a literal port of C++ `InterpolateImageAreaSampled`, fixing 23 golden tests to pass at tol=0.

**Architecture:** The C++ function processes an entire scanline in one call with carry state (`ox`, `pCy`) naturally maintained across pixels. The Rust callers batch scanlines into 256px chunks. The implementation must either (a) change callers to process full scanlines, or (b) thread carry state across batch calls. The implementer reads the C++ source, translates it preserving loop structure and arithmetic, and determines the correct batching adaptation.

**Tech Stack:** Rust (const generics for channel monomorphization), C++ reference at `~/git/eaglemode-0.96.4/`

**Key files:**
- C++ source: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlIntImg.cpp` lines 680-828 (function + all macros 200-558)
- Rust target: `crates/emcore/src/emPainterInterpolation.rs` lines 1205-1577
- Rust callers: `crates/emcore/src/emPainter.rs` (3 call sites around lines 1252, 1466, 2952)
- Test runner: `cargo test --test golden -- --test-threads=1`

---

### Task 1: Record the current baseline

Before touching any code, record exact pass/fail counts and max_diff values so regressions are immediately detectable.

**Files:**
- Read: golden test output

- [ ] **Step 1: Run the full golden suite and save the output**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tee /tmp/golden-baseline.txt
```

- [ ] **Step 2: Extract the baseline numbers**

```bash
echo "=== BASELINE ==="
grep 'test result:' /tmp/golden-baseline.txt
grep 'FAILED' /tmp/golden-baseline.txt | wc -l
```

Expected: 199 passed, 42 failed. This is the starting point. The pass count must never go below 199 during implementation.

- [ ] **Step 3: Run parallel_benchmark to confirm it passes**

```bash
cargo test --test golden parallel_benchmark -- --test-threads=1 2>&1 | tail -5
```

Expected: PASS. This must remain passing throughout.

---

### Task 2: Understand the C++ function end-to-end

This is a read-only research task. The implementer must understand every line of the C++ before writing any Rust. No code changes.

**Files:**
- Read: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlIntImg.cpp` lines 200-828
- Read: `crates/emcore/src/emPainterInterpolation.rs` lines 1200-1577

- [ ] **Step 1: Read the C++ macro definitions (lines 200-558)**

The C++ uses preprocessor macros that expand differently per CHANNELS (1/3/4). For CHANNELS=4 (the most complex case, RGBA), the key macro expansions are:

```
DEFINE_AND_SET_COLOR(cy, 0)
  → emUInt32 cyr=0, cyg=0, cyb=0, cya=0;

READ_PREMUL_MUL_COLOR(cy, p, oy1)
  → cya = p[3] * oy1;
    cyr = p[0] * cya;
    cyg = p[1] * cya;
    cyb = p[2] * cya;

DEFINE_AND_READ_PREMUL_COLOR(ctmp, p)
  → emUInt32 ctmpa = p[3];
    ctmpr = p[0] * ctmpa;
    ctmpg = p[1] * ctmpa;
    ctmpb = p[2] * ctmpa;

ADD_READ_PREMUL_COLOR(ctmp, p)
  → { DEFINE_AND_READ_PREMUL_COLOR(cTmp, p); ADD_COLOR(ctmp, cTmp); }
  → ctmpa += p[3]; ctmpr += p[0]*p[3]; ...

ADD_MUL_COLOR(cy, ctmp, ody)
  → cyr += ctmpr * ody; cyg += ctmpg * ody; ...

ADD_READ_PREMUL_MUL_COLOR(cy, p, oys)
  → { tmp_a = p[3]*oys; cya += tmp_a; cyr += p[0]*tmp_a; ... }

FINPREMUL_SHR_COLOR(cy, 8)  [CHANNELS=4]
  → cyr = (cyr + 0x7F7F) / 0xFF00;
    cyg = (cyg + 0x7F7F) / 0xFF00;
    cyb = (cyb + 0x7F7F) / 0xFF00;
    cya = (cya + 0x7F) >> 8;

DEFINE_AND_SET_COLOR(cyx, 0x7fffff)
  → emUInt32 cyxr=0x7fffff, cyxg=0x7fffff, cyxb=0x7fffff, cyxa=0x7fffff;

ADD_MUL_COLOR(cyx, cy, ox)
  → cyxr += cyr*ox; cyxg += cyg*ox; ...

WRITE_NO_ROUND_SHR_COLOR(buf, cyx, 24)
  → buf[0] = (emByte)(cyxr >> 24);
    buf[1] = (emByte)(cyxg >> 24);
    buf[2] = (emByte)(cyxb >> 24);
    buf[3] = (emByte)(cyxa >> 24);

WRITE_ZERO_COLOR(buf)
  → buf[0]=0; buf[1]=0; buf[2]=0; buf[3]=0;
```

For CHANNELS=3 (RGB, no alpha):
- `READ_PREMUL_MUL_COLOR`: `cyr = p[0]*S; cyg = p[1]*S; cyb = p[2]*S;` (no alpha)
- `FINPREMUL_SHR_COLOR(cy,8)`: `cyr = (cyr + 0x7F) >> 8;` (shift, NOT division)
- `WRITE_NO_ROUND_SHR_COLOR`: same as 4ch but only 3 bytes

For CHANNELS=1 (grayscale):
- Only `cyg` variable, same shift rounding as 3ch

- [ ] **Step 2: Read the C++ function body (lines 680-828)**

The function structure is:

```
Y setup (lines 686-725):
  ty1, ty2, ody, oy1, oy1n, row0 — all hoisted

X loop — two levels:
  OUTER do...while (buf < bufEnd):     // lines 735-826
    Compute tx1, tx2, odx, txStop for current chunk
    Compute ox (first column weight)
    Check pCy cache (pointer comparison)

    INNER do...while (tx < txStop):    // lines 790-825
      cyx = 0x7fffff (output accumulator with rounding bias)
      oxs = 0x10000 (remaining weight for this output pixel)

      while (ox < oxs):               // column accumulation
        cyx += cy * ox
        oxs -= ox
        pCy = p0                       // mark this column as cached
        Y-accumulate this column → cy  // READ_PREMUL + row loop + FINPREMUL
        p0 += imgDX                    // advance to next source column
        ox = ox1; ox1 = odx            // load next column weight

      cyx += cy * oxs                  // last column (partial weight)
      WRITE(cyx >> 24)                 // output pixel
      ox -= oxs                        // CARRY to next pixel
      tx += tdx
```

Key observations:
1. `pCy` is a **pointer** comparison (`pCy != p0`), not a column index comparison. It's initialized to NULL. When a column is Y-accumulated, `pCy = p0` saves the pointer. Next pixel checks if its `p0` matches.
2. `ox` carries between pixels via `ox -= oxs` (line 823). This is the carry that must persist.
3. The outer loop groups pixels into "chunks" where `odx` is the same. At chunk boundaries (edge pixels where `odx` differs from `odx0`), `txStop = tx` forces the inner loop to process only one pixel, then the outer loop recomputes `odx`.
4. `p0` is `row0 + (tx1>>24) * imgDX` — a pointer offset, not a column index. The Rust equivalent uses `read_area_pixel(image, sec, col, row, xfm)`.

- [ ] **Step 3: Identify the Rust divergence**

Compare the C++ inner loop (Step 2) against the current Rust `interpolate_scanline_area_inner` (lines 1352-1577). The key differences:

1. **pCy**: C++ uses pointer comparison. Rust uses `prev_cy_col` (column index comparison). These should be equivalent but may diverge at section boundaries where `read_area_pixel` clamps coordinates.
2. **Carry initialization**: C++ starts with `pCy=NULL`, `cy=0`. First pixel always gets `ox1=ox; ox=0` (lines 781-783). Rust uses `simulate_carry_chain` to compute initial carry from `carry_origin_x`.
3. **Chunk structure**: C++ outer loop computes `txStop` to batch pixels with the same `odx`. Rust processes one pixel at a time with `at_chunk_boundary = odx != carry_odx`.
4. **ox initialization at chunk boundary**: C++ always computes `ox` fresh at line 777 per outer loop iteration. Rust's `simulate_carry_chain` tries to reproduce this but may diverge.

---

### Task 3: Write the literal port

Replace `interpolate_scanline_area_inner` and `simulate_carry_chain` with a literal translation of the C++ function. This is the core task.

**Files:**
- Modify: `crates/emcore/src/emPainterInterpolation.rs`
- Possibly modify: `crates/emcore/src/emPainter.rs` (callers)

- [ ] **Step 1: Decide the batching strategy**

The C++ processes all pixels in one call (`buf` to `bufEnd`). Rust callers batch into 256px chunks. Options:

**Option A:** Change the Rust function signature to accept mutable carry state (`&mut CarryState`) that persists across batch calls. The caller creates it once per scanline row and passes it to each batch call.

**Option B:** Change callers to pass the full scanline width instead of batching. This requires a larger buffer or processing directly into the destination.

**Option C:** Keep `simulate_carry_chain` but rewrite the pixel-processing inner loop to exactly match C++.

Read the callers (emPainter.rs lines 1248-1260, 1461-1468, 2948-2960) to understand what constrains the batch size. The `InterpolationBuffer` is 1024 bytes (256 RGBA pixels). The buffer is stack-allocated. Decide which option is simplest and most correct.

- [ ] **Step 2: Write the new function**

Replace `interpolate_scanline_area_inner` with a literal translation. Preserve:
- `const CH: usize` generic for channel monomorphization
- `AreaSampleTransform`, `SectionBounds`, `ImageExtension` parameter types
- `InterpolationBuffer` output

The translation must match the C++ line-by-line. Use the macro expansions from Task 2 Step 1 as the reference. Key translation patterns:

- C++ `const emByte * p0 = row0 + (tx1>>24) * imgDX` → Rust `read_area_pixel(image, sec, col, row0_idx, xfm)` where `col = (tx1 >> 24) as i32` and `row0_idx = (ty1 >> 24) as i32`
- C++ `p0 += imgDX` → Rust `col += 1`
- C++ `pCy != p0` → Rust column tracking (the exact mechanism depends on how `p0` is represented)
- C++ `p += imgDY` → Rust `row += 1` (next row in same column)
- C++ `emUInt32` accumulators → Rust `u64` (Rust already uses u64 for these)
- C++ `while (buf < bufEnd)` → Rust `for pixel_idx in 0..count`

- [ ] **Step 3: Handle FINPREMUL_SHR_COLOR correctly per channel count**

This is a critical detail. The FINPREMUL rounding differs between CHANNELS=4 and CHANNELS=1/3:

```rust
// CHANNELS=4: RGB uses integer division, alpha uses shift
if CH == 4 {
    cr = (cr + 0x7F7F) / 0xFF00;  // NOT >> 8
    cg = (cg + 0x7F7F) / 0xFF00;
    cb = (cb + 0x7F7F) / 0xFF00;
    ca = (ca + 0x7F) >> 8;
}
// CHANNELS=1 or 3: all channels use shift
else {
    cr = (cr + 0x7F) >> 8;
    cg = (cg + 0x7F) >> 8;
    cb = (cb + 0x7F) >> 8;
}
```

Verify the existing Rust `y_accumulate_4ch` already does this correctly (it does — lines 492-495). But the new literal port must preserve this.

- [ ] **Step 4: Delete `simulate_carry_chain` if no longer needed**

If the batching strategy (Step 1) eliminates the need for `simulate_carry_chain`, delete it. If carry state is threaded via `&mut CarryState`, delete `simulate_carry_chain` and the `carry_origin_x` field from `AreaSampleTransform` (and update the 3 call sites that set it).

If `simulate_carry_chain` is kept (Option C), rewrite it to exactly match the C++ outer loop's `ox` and `pCy` initialization.

- [ ] **Step 5: Run the full golden suite**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep -E 'test result:|FAILED' | head -50
```

Compare against baseline from Task 1. The pass count must be >= 199. Check which G1 tests now pass.

- [ ] **Step 6: Run parallel_benchmark**

```bash
cargo test --test golden parallel_benchmark -- --test-threads=1
```

Must PASS. If it fails, the batching strategy breaks carry state across tiles.

---

### Task 4: Debug remaining failures (if any)

If some G1 tests still fail after Task 3, debug them individually.

**Files:**
- Read: golden test output, diff images
- Modify: `crates/emcore/src/emPainterInterpolation.rs`

- [ ] **Step 1: Identify which G1 tests still fail**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep FAILED
```

Cross-reference against the 23 G1 tests listed in the spec. Any G1 test still failing needs investigation.

- [ ] **Step 2: For each remaining failure, generate diff images**

```bash
DUMP_GOLDEN=1 cargo test --test golden <test_name> -- --test-threads=1
```

Examine `target/golden-debug/diff_<name>.ppm`. Look at the divergent pixel coordinates from the error output.

- [ ] **Step 3: Trace the divergence**

For each divergent pixel, compute what the C++ would produce (manually expanding the formula with the pixel's specific `tx`, `odx`, `ox`, `pCy` values) and compare against what the Rust produces. Use `_trace_pixel` from `common.rs` or add debug prints.

The most likely sources of remaining divergence:
1. **FINPREMUL rounding**: Division vs shift, wrong constant
2. **pCy cache mismatch**: Column identity check doesn't match C++ pointer comparison
3. **Section bounds clamping**: `read_area_pixel` clamps differently from C++ `row0 + col * imgDX`
4. **Carry across batch boundary**: The batching strategy doesn't correctly thread `ox` and `pCy`

- [ ] **Step 4: Fix and re-run full suite after each fix**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

Pass count must never decrease.

---

### Task 5: Final verification and commit

- [ ] **Step 1: Run the full golden suite**

```bash
cargo test --test golden -- --test-threads=1
```

Expected: >= 222 passed (199 original + 23 G1), <= 19 failed (non-G1 only).

- [ ] **Step 2: Run clippy and nextest**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

Both must pass clean.

- [ ] **Step 3: Run parallel_benchmark one more time**

```bash
cargo test --test golden parallel_benchmark -- --test-threads=1
```

Must PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emPainterInterpolation.rs crates/emcore/src/emPainter.rs
git commit -m "fix(area-sampling): literal port of C++ InterpolateImageAreaSampled

Replace interpolate_scanline_area_inner and simulate_carry_chain with a
direct translation of emPainter_ScTlIntImg.cpp lines 735-826. The
previous Rust implementation diverged on carry-over weight management
and pCy column-reuse caching, causing ±1 to ±255 pixel differences in
23 golden tests that downscale border/image textures via area sampling.

C++ reference: emPainter_ScTlIntImg.cpp:InterpolateImageAreaSampled
Fixes: 23 G1 tests (PaintBorderImage, PaintImageColored, paint_image_full)"
```

---

## Critical Rules

1. **Full suite after every code change.** Not "after every task" — after every edit. `cargo test --test golden -- --test-threads=1`.
2. **Pass count must never decrease.** If it drops below 199, back out the change immediately.
3. **Read the actual C++ source.** The macro expansions in Task 2 are a guide, but the C++ file at `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlIntImg.cpp` is the single source of truth. If the plan and the C++ disagree, the C++ is right.
4. **Do not modify `GetBlended`, `canvas_blend`, or `blend_hash_lookup`.** These are unrelated to area sampling.
5. **The batching constraint is real.** The `InterpolationBuffer` is 1024 bytes. Do not assume the function is called once per scanline — verify by reading the callers.
