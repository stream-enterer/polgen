# Zero-Tolerance Golden Parity — Failure Report

**Date:** 2026-04-01
**Spec:** `docs/superpowers/specs/2026-04-01-zero-tolerance-golden-parity-design.md`
**Plan:** `docs/superpowers/plans/2026-04-01-zero-tolerance-golden-parity.md`
**Outcome:** Failed. 42 failures at tol=0 before, 42 failures at tol=0 after.

## What was accomplished

1. **Tolerances dropped to zero** (commit `cceddbc`). All 241 golden tests now use `compare_images(name, actual, expected, w, h, 0, 0.0)`. This is permanent — the tolerances should not be restored.

2. **Gradient hash formula restored** (commit `3a2c1c1`). Replaced `GetBlended` calls in `sample_linear_gradient` and `sample_pixel_texture` (linear + radial) with the C++ hash formula `((a*(255-g)+b*g)*257+0x8073)>>16`. This fixed `gradient_h` and `gradient_v` (now pass at tol=0, max_diff=0).

3. **Session history investigation** traced the gradient regression to commit `b645fb3` which replaced the correct hash formula with `GetBlended`'s formula. Root cause fully understood and documented.

## Why the plan failed

The plan assumed the divergences were **formula-level** problems: wrong rounding, wrong precision, wrong formula in isolated Rust functions. The fix strategy was "find the Rust function, find the C++ equivalent, port the exact formula."

The actual problem is **architectural**. The C++ rendering pipeline is:

```
gradient_interpolation → 1-byte g value → hash_formula_index → SharedPixelFormat_hash_table_lookup → pixel_format_encoded_uint32 → direct pixel write
```

The Rust rendering pipeline is:

```
gradient_interpolation → 1-byte g value → hash_formula → RGBA channel values → InterpolationBuffer → blend_scanline → canvas_blend(BLEND_HASH) or source_over → per-channel byte write
```

These are structurally different. The C++ pipeline mediates ALL pixel writes through the `SharedPixelFormat` hash table, which introduces its own rounding (`(a1*a2*range+32512)/65025`). The Rust pipeline computes channel values with the hash formula, then passes them through a separate blend step. The two-step Rust process accumulates rounding differently than the single-step C++ process.

**Evidence:** A solid `PaintRect(emColor(145,171,242))` in C++ produces R=145. But the same color through the gradient pipeline (`g=0`, which should give pure color1) produces R=144. The hash formula correctly computes index=145, but the hash table lookup + pixel format encoding rounds to 144. The Rust gradient produces 145 because it doesn't go through the hash table round-trip.

This ±1 difference cannot be fixed by changing the gradient formula. It requires the Rust rendering pipeline to replicate the C++ hash table round-trip for every pixel write, which is an architectural change — exactly what the spec's "Non-Goals" section excluded.

## What the spec/plan got wrong

1. **Assumed formula-level fixes would suffice.** The spec said "port the exact C++ formula" for each divergence. But the C++ doesn't have a single formula per rendering path — it has a multi-stage pipeline where the hash table is integral to the output. Porting individual formulas without the hash table mediation produces close-but-not-identical results.

2. **Assumed Phase 5 test setups were correct or incorrect without verifying.** Time was spent changing eagle_logo's fill color (BLACK→WHITE→BLACK) based on comparing to the C++ generator setup, but the fill color is irrelevant at full opacity — the C++ `*p = pix` overwrites the pixel entirely.

3. **Underestimated the C++ pixel format abstraction.** The `SharedPixelFormat` hash table is not just a performance optimization — it's the mechanism that determines final pixel values. The Rust `BLEND_HASH` table exists for canvas blending but is not used in the gradient-to-pixel path.

4. **Iterative audit methodology was too slow.** The plan said "for each failure, audit against C++." With 42 failures and the real issue being architectural, auditing individual tests is the wrong granularity. The right approach is to understand the C++ pipeline end-to-end first, then fix the Rust pipeline to match.

## Anti-patterns exhibited during execution

1. **Hand-computing arithmetic instead of running the code.** Multiple rounds of manually computing hash formula values, quadrant mappings, and fixed-point arithmetic when a debug print in the C++ generator would have given the answer immediately.

2. **Guessing test setup changes.** Changed eagle_logo fill from BLACK→WHITE→BLACK based on incomplete understanding of canvas blending, causing worse failures before reverting.

3. **Continuing to look for formula fixes after evidence showed architectural mismatch.** After confirming that the hash formula gives the correct index (145) but the pixel reads differently (144), continued trying to find a formula-level explanation instead of recognizing the structural difference.

## What to do next

A new spec is needed that addresses the architectural mismatch. The key question: should the Rust rendering pipeline route gradient (and possibly all) pixel writes through the `BLEND_HASH` table to match the C++ `SharedPixelFormat` round-trip? This is the "Approach 1" (port the C++ scanline tool pipeline) that was rejected in brainstorming as too invasive. The evidence now suggests it may be necessary, at least for the hash table mediation layer.

Before writing that spec, the following needs to be understood:
- Exactly how the C++ `SharedPixelFormat` hash table mediates between computed color values and final pixel bytes
- Whether the Rust `BLEND_HASH` table is mathematically equivalent to the C++ hash table (it uses the same `(a1*a2*range+32512)/65025` formula)
- Whether routing gradient output through `BLEND_HASH` before writing pixels would achieve exact parity
- How many of the 42 failures are caused by this hash table round-trip vs other issues

## Commits on main

- `cceddbc` — test(golden): drop all tolerances to zero, add zero-tolerance parity spec
- `3a2c1c1` — fix(gradient): restore C++ hash formula for gradient blending
- `119de17` — docs: add zero-tolerance golden parity implementation plan

The tolerance zeroing and gradient hash fix are correct and should be kept. The eagle_logo test has an uncommitted edit to the comment (harmless).
