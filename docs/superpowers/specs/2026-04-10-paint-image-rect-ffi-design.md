# paint_image_rect FFI Comparison Design

## Problem

36 golden tests fail. All flow through `paint_image_rect` — the orchestration layer that converts
high-level image paint calls into per-scanline interpolation + blend. Individual sub-operations
(layers 1-7 pixel ops, layer 8 boundary computation, layer 10 gradient walk) are proven byte-perfect
via FFI harness. The orchestration itself is unproven.

## Goal

Mechanically compare Rust `paint_image_rect` output against C++ `PaintRect(emImageTexture)` output
for identical inputs. Identify the exact pixel and scanline where divergence first appears.

## Architecture

A C++ test binary (`test_paint_image_rect`) that links against:
- `libemCore.so` from `~/git/eaglemode-0.96.4/lib/` — C++ reference implementation
- `libem_harness.so` from Rust cdylib — Rust implementation via FFI

### Test flow per case

1. Create a source image (synthetic RGBA pattern or loaded from `.tga`)
2. Allocate two canvas buffers (C++ and Rust), same dimensions, zeroed
3. Initialize C++ `emPainter` with known state (Map, BytesPerRow, PixelFormat, Clip, Origin, Scale)
4. Call C++ `PaintImage(x, y, w, h, img, srcX, srcY, srcW, srcH, alpha, canvasColor, ext)`
5. Call Rust `rust_paint_image_rect(canvas, cw, ch, scale_x, scale_y, offset_x, offset_y, clip_x1, clip_y1, clip_x2, clip_y2, img_data, img_w, img_h, img_ch, x, y, w, h, src_x, src_y, src_w, src_h, alpha, canvas_color, extension)`
6. Compare C++ and Rust canvas buffers byte-by-byte
7. On first divergence: print pixel (x,y), C++ RGBA, Rust RGBA, scanline number

### Rust FFI export: `rust_paint_image_rect`

```c
extern "C" int rust_paint_image_rect(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    double clip_x1, double clip_y1, double clip_x2, double clip_y2,
    const uint8_t *img_data, int img_w, int img_h, int img_channels,
    double x, double y, double w, double h,
    int src_x, int src_y, int src_w, int src_h,
    int alpha, uint32_t canvas_color, int extension
);
```

Returns 0 on success, non-zero on error. The canvas buffer is mutated in-place.

Internally: creates an `emImage` wrapping the canvas buffer, creates an `emPainter`, sets state
(offset, scale, clip, alpha, canvas_color), calls `paint_image_rect`, returns.

### Rust FFI export: `rust_paint_image_rect_colored`

Same pattern but for `PaintImageColored` (two-color luminance mapping). Takes additional
`color1` and `color2` parameters. Many widget borders use colored painting.

### Intermediate diagnostics: `rust_get_paint_transforms`

Export the transform parameters that `paint_image_rect` computes internally:
- SubPixelEdges: ix1, iy1, ix2, frac_left, frac_right
- Y boundaries: cpp_iy1, cpp_iy2, cpp_ay1, cpp_ay2
- AreaSampleTransform (if downscaling): tdx, tdy, off_x, off_y, stride_x, stride_y
- ScaleTransform (if upscaling): sdx, sdy

This allows bisecting: if transforms match but output diverges, the bug is in the interpolation
loop or blend dispatch. If transforms diverge, the bug is in boundary/transform computation.

## C++ emPainter setup

From `test_real_tga_e2e.cpp` pattern:

```cpp
emPainter p;
static emPainter::SharedPixelFormat pf;
setup_pixel_format(pf);  // Initialize blend hash tables (one-time)

p.Map = (void*)canvas;
p.BytesPerRow = cw * 4;
p.PixelFormat = &pf;
p.ClipX1 = clip_x1; p.ClipY1 = clip_y1;
p.ClipX2 = clip_x2; p.ClipY2 = clip_y2;
p.OriginX = offset_x; p.OriginY = offset_y;
p.ScaleX = scale_x;   p.ScaleY = scale_y;
p.UserSpaceMutex = NULL;
```

Then call `p.PaintImage(x, y, w, h, img, srcX, srcY, srcW, srcH, alpha, canvasColor, ext)`.

## Test cases

Derive parameters from representative failing golden tests:

1. **Upscale case** — `painter_image_scaled`: small source image painted into larger canvas rect.
   Uses a synthetic 4x4 gradient pattern. Exercises adaptive interpolation path.

2. **Border slice case** — `widget_checkbox_unchecked`: one of the 9 PaintBorderImage slices.
   Exercises EXTEND_EDGE extension and sub-pixel boundary computation for typical widget sizes.

3. **Downscale case** — from `starfield_small` or similar: large source into small dest.
   Exercises area sampling path with pre-reduction strides.

4. **Colored case** — `widget_button_normal`: PaintImageColored with border image.
   Exercises the colored blend path (luminance mapping + two-color interpolation).

For each test case, extract the exact parameters by adding temporary debug prints to the golden
test runner, capturing the arguments to `paint_image_rect` / `PaintImageColored`.

## Build

```bash
# Build Rust harness
cargo build -p em-harness

# Compile C++ test
g++ -std=c++11 -O2 \
    -I ~/git/eaglemode-0.96.4/include \
    -L ~/git/eaglemode-0.96.4/lib \
    -L target/debug \
    -o harness/test_paint_image_rect \
    harness/test_paint_image_rect.cpp \
    -lemCore -lem_harness \
    -Wl,-rpath,~/git/eaglemode-0.96.4/lib \
    -Wl,-rpath,target/debug

# Run
LD_LIBRARY_PATH=~/git/eaglemode-0.96.4/lib:target/debug \
    harness/test_paint_image_rect
```

## Success criteria

- At least one test case produces byte-identical output between C++ and Rust: confirms the FFI
  comparison framework works correctly
- For divergent cases: exact pixel coordinates and values of first divergence, plus whether
  the transforms matched (bisects the bug to orchestration vs sub-operation)

## Non-goals

- Fixing divergences in this spec (that's a separate task after diagnosis)
- Testing all 36 failing cases (3-4 representative cases are enough to identify the pattern)
- PaintTextColored or PaintPolygon (separate diagnostic paths)
