// Compare C++ PaintRoundRect vs Rust rust_paint_round_rect.
// Uses exact parameters from the border_roundrect_thin golden test.
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -L target/debug \
//     -o harness/test_paint_round_rect \
//     harness/test_paint_round_rect.cpp \
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
extern "C" int rust_paint_round_rect(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    double x, double y, double w, double h,
    double radius, uint32_t color
);

struct TestCase {
    const char* name;
    int canvas_w, canvas_h;
    double scale_x, scale_y, offset_x, offset_y;
    double x, y, w, h;
    double radius;
    uint32_t color; // packed RGBA
    uint32_t bg_color; // pre-fill color (packed RGBA, 0=black)
};

// Compare only RGB channels
static int compare_buffers(const char* name,
                           const uint8_t* cpp_buf, const uint8_t* rust_buf,
                           int w, int h,
                           bool verbose = false) {
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
                if (verbose && diffs < 20) {
                    printf("    diff at (%d,%d): C++=[%d,%d,%d] Rust=[%d,%d,%d]\n",
                           x, y,
                           cpp_buf[off], cpp_buf[off+1], cpp_buf[off+2],
                           rust_buf[off], rust_buf[off+1], rust_buf[off+2]);
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
        if (!verbose)
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

    // emColor packing: (r << 24) | (g << 16) | (b << 8) | a
    // border fill color from draw ops: emColor(1365148927) = 0x515E84FF = rgb(81,94,132)
    // background fill: emColor(2155905279) = 0x808080FF = rgb(128,128,128)

    TestCase cases[] = {
        // Case 1: Exact parameters from border_roundrect_thin golden test
        // SetOffset(0.0, 299.2), ClipRect(0, 0, 800, 1.6)
        // PaintRoundRect(0.0368, 0.0368, 799.9264, 1.5264, radius=0.352, color=0x515E84FF)
        {"border_thin_exact", 800, 600,
         1.0, 1.0, 0.0, 299.2,
         0.0368, 0.0368, 799.9264, 1.5264000000000002,
         0.35200000000000004,
         0x515E84FF, 0x808080FF},

        // Case 2: Simple large round rect
        {"large_roundrect", 200, 200,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0.1,
         0xFF0000FF, 0},

        // Case 3: Small round rect with sub-pixel positioning
        {"small_subpixel", 100, 100,
         100.0, 100.0, 0.3, 0.7,
         0.0, 0.0, 0.5, 0.5,
         0.05,
         0x00FF00FF, 0},

        // Case 4: Tiny round rect (like the border case)
        {"tiny_roundrect", 20, 4,
         1.0, 1.0, 0.0, 0.0,
         0.5, 0.5, 19.0, 3.0,
         0.5,
         0x515E84FF, 0x808080FF},

        // Case 5: Round rect with very small radius
        {"small_radius", 200, 200,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0.01,
         0xFFFFFFFF, 0},

        // Case 6: Round rect that spans most of the canvas
        {"full_width", 800, 10,
         1.0, 1.0, 0.0, 0.0,
         0.0, 0.0, 800.0, 10.0,
         2.0,
         0xFF8000FF, 0},
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
        if (tc.bg_color) {
            unsigned char cr = (tc.bg_color >> 24) & 0xFF;
            unsigned char cg = (tc.bg_color >> 16) & 0xFF;
            unsigned char cb = (tc.bg_color >> 8) & 0xFF;
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
            tc.canvas_w * 4, 4,
            0xFF, 0xFF00, 0xFF0000,
            0.0, 0.0, (double)tc.canvas_w, (double)tc.canvas_h,
            tc.offset_x, tc.offset_y,
            tc.scale_x, tc.scale_y
        );

        emColor color = (emUInt32)tc.color;
        painter.PaintRoundRect(tc.x, tc.y, tc.w, tc.h, tc.radius, tc.radius, color, 0);

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)malloc(fb_size);
        if (tc.bg_color) {
            unsigned char cr = (tc.bg_color >> 24) & 0xFF;
            unsigned char cg = (tc.bg_color >> 16) & 0xFF;
            unsigned char cb = (tc.bg_color >> 8) & 0xFF;
            for (int i = 0; i < fb_size; i += 4) {
                rust_canvas[i+0] = cr; rust_canvas[i+1] = cg;
                rust_canvas[i+2] = cb; rust_canvas[i+3] = 0xFF;
            }
        } else {
            memset(rust_canvas, 0, fb_size);
        }

        rust_paint_round_rect(
            rust_canvas, tc.canvas_w, tc.canvas_h,
            tc.scale_x, tc.scale_y, tc.offset_x, tc.offset_y,
            tc.x, tc.y, tc.w, tc.h, tc.radius, tc.color
        );

        // --- Compare ---
        bool verbose = (c == 0); // verbose for the exact border test case
        failures += compare_buffers(tc.name,
            (const uint8_t*)cpp_canvas.GetMap(), rust_canvas,
            tc.canvas_w, tc.canvas_h, verbose);

        // Dump specific pixels for case 1
        if (c == 0) {
            const unsigned char* cm = (const unsigned char*)cpp_canvas.GetMap();
            int positions[][2] = {{0,0}, {1,0}, {798,0}, {799,0}, {0,1}, {1,1}, {798,1}, {799,1}};
            printf("  C++ pixels (offset y=299.2 applied by painter):\n");
            for (auto& pos : positions) {
                int x = pos[0], y = pos[1];
                int off = (y * tc.canvas_w + x) * 4;
                printf("    (%d,%d): rgb(%d,%d,%d)\n", x, y, cm[off], cm[off+1], cm[off+2]);
            }
            printf("  Rust pixels:\n");
            for (auto& pos : positions) {
                int x = pos[0], y = pos[1];
                int off = (y * tc.canvas_w + x) * 4;
                printf("    (%d,%d): rgb(%d,%d,%d)\n", x, y, rust_canvas[off], rust_canvas[off+1], rust_canvas[off+2]);
            }
        }

        free(rust_canvas);
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
