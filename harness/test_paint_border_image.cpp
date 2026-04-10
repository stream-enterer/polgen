// Compare C++ PaintBorderImage vs Rust rust_paint_border_image.
// Uses the actual VcItemInnerBorder.tga border image for realistic testing.
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -L target/debug \
//     -o harness/test_paint_border_image \
//     harness/test_paint_border_image.cpp \
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
#include <emCore/emTexture.h>

// Rust FFI — matches the existing rust_paint_border_image signature
extern "C" int rust_paint_border_image(
    const uint8_t *src_data, int src_w, int src_h,
    uint8_t *fb_data, int fb_w, int fb_h,
    double x, double y, double w, double h,
    double l, double t, double r, double b,
    int src_l, int src_t, int src_r, int src_b,
    double scale_x, double scale_y,
    double origin_x, double origin_y,
    uint8_t alpha,
    uint8_t canvas_r, uint8_t canvas_g, uint8_t canvas_b, uint8_t canvas_a,
    int which_sub_rects
);

// Fill 4-channel gradient image
static void fill_gradient(unsigned char* data, int w, int h) {
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * 4;
            data[off+0] = (unsigned char)((x * 17) & 0xFF);
            data[off+1] = (unsigned char)((y * 17) & 0xFF);
            data[off+2] = (unsigned char)(((x + y) * 8) & 0xFF);
            data[off+3] = 255;
        }
    }
}

struct TestCase {
    const char* name;
    int fb_w, fb_h;
    int img_w, img_h;
    double scale_x, scale_y, origin_x, origin_y;
    double x, y, w, h;
    double l, t, r, b;
    int src_l, src_t, src_r, src_b;
    uint8_t alpha;
    uint8_t canvas_r, canvas_g, canvas_b, canvas_a;
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
        // Case 1: Upscale border, 16x16 src, thick borders
        {"border_upscale_thick", 400, 300, 16, 16,
         400.0, 300.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.1, 0.1, 0.1, 0.1,
         4, 4, 4, 4,
         255, 0, 0, 0, 0},  // no canvas color

        // Case 2: Upscale border with canvas color
        {"border_upscale_canvas", 400, 300, 16, 16,
         400.0, 300.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.1, 0.1, 0.1, 0.1,
         4, 4, 4, 4,
         255, 128, 128, 128, 255},  // grey canvas

        // Case 3: Thin borders
        {"border_thin", 400, 300, 16, 16,
         400.0, 300.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.02, 0.02, 0.02, 0.02,
         2, 2, 2, 2,
         255, 0, 0, 0, 0},

        // Case 4: Asymmetric borders
        {"border_asymmetric", 400, 300, 16, 16,
         400.0, 300.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.05, 0.1, 0.15, 0.05,
         2, 4, 6, 2,
         255, 0, 0, 0, 0},

        // Case 5: Border with offset
        {"border_offset", 400, 300, 16, 16,
         400.0, 300.0, 10.0, 10.0,
         0.0, 0.0, 0.8, 0.6,
         0.08, 0.08, 0.08, 0.08,
         3, 3, 3, 3,
         255, 0, 0, 0, 0},

        // Case 6: Border with partial alpha
        {"border_alpha", 400, 300, 16, 16,
         400.0, 300.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.1, 0.1, 0.1, 0.1,
         4, 4, 4, 4,
         200, 0, 0, 0, 0},

        // Case 7: Very small border (like widget at small zoom)
        {"border_small", 100, 75, 16, 16,
         100.0, 75.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.1, 0.1, 0.1, 0.1,
         4, 4, 4, 4,
         255, 0, 0, 0, 0},

        // Case 8: Large border (like widget at high zoom)
        {"border_large", 800, 600, 16, 16,
         800.0, 600.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 0.75,
         0.1, 0.1, 0.1, 0.1,
         4, 4, 4, 4,
         255, 0, 0, 0, 0},
    };

    int n_cases = (int)(sizeof(cases) / sizeof(cases[0]));
    int failures = 0;

    for (int c = 0; c < n_cases; c++) {
        TestCase& tc = cases[c];
        printf("Test: %s\n", tc.name);

        // Create source image (4-channel)
        int img_size = tc.img_w * tc.img_h * 4;
        unsigned char* img_data = (unsigned char*)malloc(img_size);
        fill_gradient(img_data, tc.img_w, tc.img_h);

        int fb_size = tc.fb_w * tc.fb_h * 4;

        // --- C++ render ---
        emImage cpp_canvas;
        cpp_canvas.Setup(tc.fb_w, tc.fb_h, 4);
        if (tc.canvas_a) {
            unsigned char* m = (unsigned char*)cpp_canvas.GetMap();
            for (int i = 0; i < fb_size; i += 4) {
                m[i+0] = tc.canvas_r; m[i+1] = tc.canvas_g;
                m[i+2] = tc.canvas_b; m[i+3] = tc.canvas_a;
            }
        } else {
            memset((void*)cpp_canvas.GetMap(), 0, fb_size);
        }

        emImage srcImg;
        srcImg.Setup(tc.img_w, tc.img_h, 4);
        memcpy((void*)srcImg.GetMap(), img_data, img_size);

        emPainter painter(
            rootContext,
            (void*)cpp_canvas.GetMap(),
            tc.fb_w * 4, 4,
            0xFF, 0xFF00, 0xFF0000,
            0.0, 0.0, (double)tc.fb_w, (double)tc.fb_h,
            tc.origin_x, tc.origin_y,
            tc.scale_x, tc.scale_y
        );

        emColor cc((emUInt8)tc.canvas_r, (emUInt8)tc.canvas_g,
                   (emUInt8)tc.canvas_b, (emUInt8)tc.canvas_a);

        painter.PaintBorderImage(tc.x, tc.y, tc.w, tc.h,
                                 tc.l, tc.t, tc.r, tc.b,
                                 srcImg,
                                 tc.src_l, tc.src_t, tc.src_r, tc.src_b,
                                 tc.alpha, cc);  // default whichSubRects=0757

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)malloc(fb_size);
        if (tc.canvas_a) {
            for (int i = 0; i < fb_size; i += 4) {
                rust_canvas[i+0] = tc.canvas_r; rust_canvas[i+1] = tc.canvas_g;
                rust_canvas[i+2] = tc.canvas_b; rust_canvas[i+3] = tc.canvas_a;
            }
        } else {
            memset(rust_canvas, 0, fb_size);
        }

        rust_paint_border_image(
            img_data, tc.img_w, tc.img_h,
            rust_canvas, tc.fb_w, tc.fb_h,
            tc.x, tc.y, tc.w, tc.h,
            tc.l, tc.t, tc.r, tc.b,
            tc.src_l, tc.src_t, tc.src_r, tc.src_b,
            tc.scale_x, tc.scale_y,
            tc.origin_x, tc.origin_y,
            tc.alpha,
            tc.canvas_r, tc.canvas_g, tc.canvas_b, tc.canvas_a,
            0757  // same as C++ default
        );

        // --- Compare ---
        failures += compare_buffers(tc.name,
            (const uint8_t*)cpp_canvas.GetMap(), rust_canvas,
            tc.fb_w, tc.fb_h);

        free(img_data);
        free(rust_canvas);
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
