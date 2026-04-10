// Compare C++ PaintText vs Rust rust_paint_text for identical inputs.
// Links against libemCore.so (C++ reference) and libem_harness.so (Rust).
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -L target/debug \
//     -o harness/test_paint_text \
//     harness/test_paint_text.cpp \
//     -lemCore -lem_harness \
//     -Wl,-rpath,$HOME/git/eaglemode-0.96.4/lib \
//     -Wl,-rpath,target/debug

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cmath>
#include <cstdint>

#include <emCore/emScheduler.h>
#include <emCore/emContext.h>
#include <emCore/emPainter.h>
#include <emCore/emImage.h>

// Rust FFI
extern "C" int rust_paint_text(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    const char *text,
    double x, double y,
    double char_height, double width_scale,
    uint32_t color, uint32_t canvas_color
);

struct TestCase {
    const char* name;
    int canvas_w, canvas_h;
    double scale_x, scale_y, offset_x, offset_y;
    const char* text;
    double x, y;
    double char_height, width_scale;
    uint32_t color;        // packed RGBA
    uint32_t canvas_color; // packed RGBA
};

// Compare only RGB channels
static int compare_buffers(const char* name,
                           const uint8_t* cpp_buf, const uint8_t* rust_buf,
                           int w, int h) {
    int diffs = 0;
    int first_x = -1, first_y = -1;
    uint8_t first_cpp[4] = {}, first_rust[4] = {};
    int max_diff = 0;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            if (memcmp(cpp_buf + off, rust_buf + off, 3) != 0) {
                if (diffs == 0) {
                    first_x = x; first_y = y;
                    memcpy(first_cpp, cpp_buf + off, 4);
                    memcpy(first_rust, rust_buf + off, 4);
                }
                for (int c = 0; c < 3; c++) {
                    int d = abs((int)cpp_buf[off+c] - (int)rust_buf[off+c]);
                    if (d > max_diff) max_diff = d;
                }
                diffs++;
            }
        }
    }
    if (diffs == 0) {
        printf("  [PASS] %s: RGB-identical (%dx%d)\n", name, w, h);
    } else {
        printf("  [FAIL] %s: %d divergent pixels (max_diff=%d) out of %d\n",
               name, diffs, max_diff, w * h);
        printf("    first at (%d,%d): C++=[%d,%d,%d,%d] Rust=[%d,%d,%d,%d]\n",
               first_x, first_y,
               first_cpp[0], first_cpp[1], first_cpp[2], first_cpp[3],
               first_rust[0], first_rust[1], first_rust[2], first_rust[3]);
    }
    return diffs == 0 ? 0 : 1;
}

int main() {
    emStandardScheduler scheduler;
    emRootContext rootContext(scheduler);

    TestCase cases[] = {
        // Case 1: Simple text, upscaled, no canvas color
        {"text_upscale_simple", 400, 100, 400.0, 100.0, 0.0, 0.0,
         "Hello", 0.0, 0.0, 0.5, 1.0,
         0xFFFFFFFF, 0},  // white text, no canvas

        // Case 2: Large text, single char
        {"text_large_A", 200, 200, 200.0, 200.0, 0.0, 0.0,
         "A", 0.0, 0.0, 1.0, 1.0,
         0xFF0000FF, 0},  // red text

        // Case 3: Text with canvas color
        {"text_canvas", 400, 100, 400.0, 100.0, 0.0, 0.0,
         "Test", 0.0, 0.0, 0.5, 1.0,
         0x000000FF, 0xFFFFFFFF},  // black on white canvas

        // Case 4: Small text (near tiny threshold)
        {"text_small", 200, 50, 200.0, 50.0, 0.0, 0.0,
         "tiny", 0.0, 0.0, 0.04, 1.0,
         0xFFFFFFFF, 0},

        // Case 5: Multi-character with offset
        {"text_offset", 400, 100, 400.0, 100.0, 10.0, 5.0,
         "Offset", 0.0, 0.0, 0.5, 1.0,
         0x00FF00FF, 0},  // green text

        // Case 6: Text with sub-pixel char_height (triggers exact downscale)
        {"text_medium", 400, 200, 400.0, 200.0, 0.0, 0.0,
         "Medium", 0.0, 0.0, 0.25, 1.0,
         0xFFFFFFFF, 0},

        // Case 7: Typical widget label size (realistic)
        {"text_label", 800, 600, 800.0, 600.0, 0.0, 0.0,
         "Label", 0.1, 0.1, 0.05, 1.0,
         0x000000FF, 0x808080FF},  // black on grey

        // Case 8: Full alphabet
        {"text_alphabet", 1000, 100, 1000.0, 100.0, 0.0, 0.0,
         "ABCDEFGHIJKLMNOPQRSTUVWXYZ", 0.0, 0.0, 0.8, 1.0,
         0xFFFFFFFF, 0},

        // Case 9: Width scale != 1.0
        {"text_wide", 400, 100, 400.0, 100.0, 0.0, 0.0,
         "Wide", 0.0, 0.0, 0.5, 1.5,
         0xFFFFFFFF, 0},

        // Case 10: Digits and punctuation
        {"text_digits", 600, 100, 600.0, 100.0, 0.0, 0.0,
         "0123456789!@#$%", 0.0, 0.0, 0.5, 1.0,
         0xFFFFFFFF, 0},
    };

    int n_cases = (int)(sizeof(cases) / sizeof(cases[0]));
    int failures = 0;

    for (int c = 0; c < n_cases; c++) {
        TestCase& tc = cases[c];
        printf("Test: %s\n", tc.name);

        int fb_size = tc.canvas_w * tc.canvas_h * 4;

        // --- C++ render ---
        emImage cpp_canvas;
        cpp_canvas.Setup(tc.canvas_w, tc.canvas_h, 4);
        if (tc.canvas_color & 0xFF) {
            unsigned char cr = (tc.canvas_color >> 24) & 0xFF;
            unsigned char cg = (tc.canvas_color >> 16) & 0xFF;
            unsigned char cb = (tc.canvas_color >> 8) & 0xFF;
            unsigned char* m = (unsigned char*)cpp_canvas.GetMap();
            for (int i = 0; i < fb_size; i += 4) {
                m[i+0] = cr; m[i+1] = cg; m[i+2] = cb; m[i+3] = 0xFF;
            }
        } else {
            memset((void*)cpp_canvas.GetMap(), 0, fb_size);
        }

        emPainter painter(
            rootContext,
            (void*)cpp_canvas.GetMap(),
            tc.canvas_w * 4,
            4,
            0x000000FF,
            0x0000FF00,
            0x00FF0000,
            0.0, 0.0,
            (double)tc.canvas_w, (double)tc.canvas_h,
            tc.offset_x, tc.offset_y,
            tc.scale_x, tc.scale_y
        );

        emColor cc = (emUInt32)tc.color;
        emColor ccv = (emUInt32)tc.canvas_color;

        painter.PaintText(tc.x, tc.y, tc.text, tc.char_height, tc.width_scale,
                         cc, ccv, strlen(tc.text));

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)malloc(fb_size);
        if (tc.canvas_color & 0xFF) {
            unsigned char cr = (tc.canvas_color >> 24) & 0xFF;
            unsigned char cg = (tc.canvas_color >> 16) & 0xFF;
            unsigned char cb = (tc.canvas_color >> 8) & 0xFF;
            for (int i = 0; i < fb_size; i += 4) {
                rust_canvas[i+0] = cr; rust_canvas[i+1] = cg;
                rust_canvas[i+2] = cb; rust_canvas[i+3] = 0xFF;
            }
        } else {
            memset(rust_canvas, 0, fb_size);
        }

        rust_paint_text(
            rust_canvas, tc.canvas_w, tc.canvas_h,
            tc.scale_x, tc.scale_y, tc.offset_x, tc.offset_y,
            tc.text,
            tc.x, tc.y, tc.char_height, tc.width_scale,
            tc.color, tc.canvas_color
        );

        // --- Compare ---
        failures += compare_buffers(tc.name,
            (const uint8_t*)cpp_canvas.GetMap(), rust_canvas,
            tc.canvas_w, tc.canvas_h);

        free(rust_canvas);
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
