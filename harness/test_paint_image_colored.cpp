// Compare C++ PaintImageColored vs Rust rust_paint_image_colored for identical inputs.
// Links against libemCore.so (C++ reference) and libem_harness.so (Rust).
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -L target/debug \
//     -o harness/test_paint_image_colored \
//     harness/test_paint_image_colored.cpp \
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

// Rust FFI
extern "C" int rust_paint_image_colored(
    uint8_t *canvas, int canvas_w, int canvas_h,
    double scale_x, double scale_y,
    double offset_x, double offset_y,
    const uint8_t *img_data, int img_w, int img_h, int img_ch,
    double x, double y, double w, double h,
    int src_x, int src_y, int src_w, int src_h,
    uint32_t color1, uint32_t color2,
    uint32_t canvas_color, int extension
);

// Fill a 1-channel luminance gradient: pixel (x,y) = (x*17 + y*13) & 0xFF
static void fill_lum_gradient(unsigned char* data, int w, int h, int ch) {
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int off = (y * w + x) * ch;
            unsigned char lum = (unsigned char)((x * 17 + y * 13) & 0xFF);
            if (ch == 1) {
                data[off] = lum;
            } else {
                // For multi-channel, set RGB to same luminance, alpha=255
                data[off+0] = lum;
                if (ch >= 2) data[off+1] = lum;
                if (ch >= 3) data[off+2] = lum;
                if (ch >= 4) data[off+3] = 255;
            }
        }
    }
}

struct TestCase {
    const char* name;
    int canvas_w, canvas_h;
    int img_w, img_h, img_ch;
    double scale_x, scale_y, offset_x, offset_y;
    double x, y, w, h;
    int src_x, src_y, src_w, src_h;
    uint32_t color1;   // packed RGBA (R<<24 | G<<16 | B<<8 | A)
    uint32_t color2;   // packed RGBA
    uint32_t canvas_color;
    int extension;     // 0=TILED,1=EDGE,2=ZERO,3=EDGE_OR_ZERO
};

// Helper to convert packed RGBA to emColor
static emColor packed_to_emColor(uint32_t packed) {
    // emColor packing: (r << 24) | (g << 16) | (b << 8) | a
    return (emUInt32)packed;
}

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

    // Color packing: (R << 24) | (G << 16) | (B << 8) | A
    // Red opaque:    0xFF0000FF
    // Blue opaque:   0x0000FFFF
    // White opaque:  0xFFFFFFFF
    // Green opaque:  0x00FF00FF
    // Black opaque:  0x000000FF
    // Transparent:   0x00000000
    // Grey opaque:   0x808080FF

    TestCase cases[] = {
        // Case 1: G1G2 upscale, 1-channel source, no canvas color
        {"g1g2_upscale_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0xFF0000FF, 0x0000FFFF,  // red→blue gradient
         0, 1},  // EXTEND_EDGE

        // Case 2: G1G2 downscale, 1-channel source, no canvas color
        {"g1g2_downscale_1ch", 200, 200, 32, 32, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 0.05, 0.05,
         0, 0, 32, 32,
         0xFF0000FF, 0x0000FFFF,
         0, 1},

        // Case 3: G2 only (color1 transparent), upscale
        {"g2_upscale_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0x00000000, 0x0000FFFF,  // transparent→blue
         0, 1},

        // Case 4: G1 only (color2 transparent), upscale
        {"g1_upscale_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0xFF0000FF, 0x00000000,  // red→transparent
         0, 1},

        // Case 5: G1G2 with canvas color (HAVE_CVC)
        {"g1g2_canvas_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0xFF0000FF, 0x0000FFFF,
         0xFFFFFFFF, 1},  // white canvas

        // Case 6: G2 with canvas color
        {"g2_canvas_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0x00000000, 0x0000FFFF,
         0x808080FF, 1},  // grey canvas

        // Case 7: G1G2 exact 1:1 (16x16 → 16x16)
        {"g1g2_exact_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 0.08, 0.08,
         0, 0, 16, 16,
         0x00FF00FF, 0xFF00FFFF,  // green→magenta
         0, 3},  // EXTEND_EDGE_OR_ZERO

        // Case 8: G1G2 with sub-pixel offset
        {"g1g2_subpixel_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.3, 0.7,
         0.0, 0.0, 0.5, 0.5,
         0, 0, 16, 16,
         0xFF0000FF, 0x0000FFFF,
         0, 1},

        // Case 9: G1G2 upscale, 4-channel source (RGB luminance extraction)
        {"g1g2_upscale_4ch", 200, 200, 16, 16, 4,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0xFF0000FF, 0x0000FFFF,
         0, 1},

        // Case 10: G1G2 with white→black (typical border colors)
        {"g1g2_white_black_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 16, 16,
         0xFFFFFFFF, 0x000000FF,  // white→black
         0, 1},

        // Case 11: G1G2 with canvas color + sub-pixel (hardest case)
        {"g1g2_canvas_subpixel_1ch", 200, 200, 16, 16, 1,
         200.0, 200.0, 0.3, 0.7,
         0.0, 0.0, 0.5, 0.5,
         0, 0, 16, 16,
         0xFF0000FF, 0x0000FFFF,
         0xFFFFFFFF, 1},

        // Case 12: Small upscale (8x8→~100x100), 1-channel
        {"g1g2_small_upscale_1ch", 200, 200, 8, 8, 1,
         200.0, 200.0, 0.0, 0.0,
         0.0, 0.0, 1.0, 1.0,
         0, 0, 8, 8,
         0x808080FF, 0xC0C0C0FF,  // grey→lighter grey
         0, 1},
    };

    int n_cases = (int)(sizeof(cases) / sizeof(cases[0]));
    int failures = 0;

    for (int c = 0; c < n_cases; c++) {
        TestCase& tc = cases[c];
        printf("Test: %s\n", tc.name);

        // Create source image
        int img_size = tc.img_w * tc.img_h * tc.img_ch;
        unsigned char* img_data = (unsigned char*)malloc(img_size);
        fill_lum_gradient(img_data, tc.img_w, tc.img_h, tc.img_ch);

        int fb_size = tc.canvas_w * tc.canvas_h * 4;

        // --- C++ render ---
        emImage cpp_canvas;
        cpp_canvas.Setup(tc.canvas_w, tc.canvas_h, 4);
        // Pre-fill with canvas color if opaque (canvas blend expects it)
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

        emImage srcImg;
        srcImg.Setup(tc.img_w, tc.img_h, tc.img_ch);
        memcpy((void*)srcImg.GetMap(), img_data, img_size);

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

        emColor cc1 = packed_to_emColor(tc.color1);
        emColor cc2 = packed_to_emColor(tc.color2);
        emColor ccv = packed_to_emColor(tc.canvas_color);
        emTexture::ExtensionType ext;
        switch(tc.extension) {
            case 0: ext = emTexture::EXTEND_TILED; break;
            case 1: ext = emTexture::EXTEND_EDGE; break;
            case 2: ext = emTexture::EXTEND_ZERO; break;
            default: ext = emTexture::EXTEND_EDGE_OR_ZERO; break;
        }

        painter.PaintImageColored(tc.x, tc.y, tc.w, tc.h, srcImg,
                                  tc.src_x, tc.src_y, tc.src_w, tc.src_h,
                                  cc1, cc2, ccv, ext);

        // --- Rust render ---
        unsigned char* rust_canvas = (unsigned char*)malloc(fb_size);
        // Pre-fill with canvas color if opaque (same as C++ canvas)
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

        rust_paint_image_colored(
            rust_canvas, tc.canvas_w, tc.canvas_h,
            tc.scale_x, tc.scale_y, tc.offset_x, tc.offset_y,
            img_data, tc.img_w, tc.img_h, tc.img_ch,
            tc.x, tc.y, tc.w, tc.h,
            tc.src_x, tc.src_y, tc.src_w, tc.src_h,
            tc.color1, tc.color2, tc.canvas_color, tc.extension
        );

        // --- Compare ---
        failures += compare_buffers(tc.name,
            (const uint8_t*)cpp_canvas.GetMap(), rust_canvas,
            tc.canvas_w, tc.canvas_h);

        free(img_data);
        free(rust_canvas);
    }

    printf("\nResult: %d/%d passed\n", n_cases - failures, n_cases);
    return failures > 0 ? 1 : 0;
}
