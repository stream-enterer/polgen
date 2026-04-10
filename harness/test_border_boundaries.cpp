// test_border_boundaries.cpp
// Compares C++ PaintBorderImage boundary computation against Rust FFI.
// Self-contained: reimplements RoundX/RoundY + 9-slice formula from
// emPainter.cpp:275-284, 1903-1982. No emCore dependency.

#include <cstdio>
#include <cstring>
#include <cmath>
#include <cstdlib>

// --- C-compatible structs matching harness/src/lib.rs ---

struct CBorderImageParams {
    double x, y, w, h;
    double l, t, r, b;
    int img_w, img_h;
    int src_l, src_t, src_r, src_b;
    double scale_x, scale_y;
    double origin_x, origin_y;
    unsigned char canvas_r, canvas_g, canvas_b, canvas_a;
};

struct CBorderImageBoundaries {
    double adj_l, adj_t, adj_r, adj_b;
    double target_rects[9][4];  // [i][x,y,w,h]
    int source_rects[9][4];     // [i][sx,sy,sw,sh]
};

// --- Rust FFI ---
extern "C" int rust_border_image_boundaries(
    const CBorderImageParams* params,
    CBorderImageBoundaries* out
);

// --- C++ reference: RoundX/RoundY (emPainter.cpp:275-284) ---
static double CppRoundX(double x, double ScaleX, double OriginX) {
    return (floor(x * ScaleX + OriginX + 0.5) - OriginX) / ScaleX;
}

static double CppRoundY(double y, double ScaleY, double OriginY) {
    return (floor(y * ScaleY + OriginY + 0.5) - OriginY) / ScaleY;
}

// --- C++ reference: boundary computation (emPainter.cpp:1892-1982) ---
static void CppBorderImageBoundaries(
    const CBorderImageParams& p,
    CBorderImageBoundaries& out
) {
    double l = p.l, t = p.t, r = p.r, b = p.b;

    // Pixel-round when canvas is non-opaque (emPainter.cpp:1903-1908)
    bool canvas_opaque = (p.canvas_a == 255);
    if (!canvas_opaque) {
        double f;
        f = CppRoundX(p.x + l, p.scale_x, p.origin_x) - p.x;
        if (f > 0 && f < p.w - r) l = f;
        f = p.x + p.w - CppRoundX(p.x + p.w - r, p.scale_x, p.origin_x);
        if (f > 0 && f < p.w - l) r = f;
        f = CppRoundY(p.y + t, p.scale_y, p.origin_y) - p.y;
        if (f > 0 && f < p.h - b) t = f;
        f = p.y + p.h - CppRoundY(p.y + p.h - b, p.scale_y, p.origin_y);
        if (f > 0 && f < p.h - t) b = f;
    }

    out.adj_l = l;
    out.adj_t = t;
    out.adj_r = r;
    out.adj_b = b;

    int src_cx = p.img_w - p.src_l - p.src_r;
    int src_cy = p.img_h - p.src_t - p.src_b;
    double dst_cx = p.w - l - r;
    double dst_cy = p.h - t - b;

    double x = p.x, y = p.y, w = p.w, h = p.h;
    int iw = p.img_w, ih = p.img_h;
    int sl = p.src_l, st = p.src_t, sr = p.src_r, sb = p.src_b;

    // Target rects: UL, U, UR, L, C, R, LL, B, LR
    double tr[9][4] = {
        {x,         y,         l,      t},
        {x + l,     y,         dst_cx, t},
        {x + w - r, y,         r,      t},
        {x,         y + t,     l,      dst_cy},
        {x + l,     y + t,     dst_cx, dst_cy},
        {x + w - r, y + t,     r,      dst_cy},
        {x,         y + h - b, l,      b},
        {x + l,     y + h - b, dst_cx, b},
        {x + w - r, y + h - b, r,      b},
    };
    memcpy(out.target_rects, tr, sizeof(tr));

    // Source rects
    int sr_arr[9][4] = {
        {0,          0,          sl,     st},
        {sl,         0,          src_cx, st},
        {iw - sr,    0,          sr,     st},
        {0,          st,         sl,     src_cy},
        {sl,         st,         src_cx, src_cy},
        {iw - sr,    st,         sr,     src_cy},
        {0,          ih - sb,    sl,     sb},
        {sl,         ih - sb,    src_cx, sb},
        {iw - sr,    ih - sb,    sr,     sb},
    };
    memcpy(out.source_rects, sr_arr, sizeof(sr_arr));
}

// --- Test cases ---

struct TestCase {
    const char* name;
    CBorderImageParams params;
};

static const TestCase cases[] = {
    // Case 1: Checkbox-like (286x286 border image, transparent canvas, 800x600 viewport)
    {
        "checkbox_286x286_transparent",
        {
            0.0, 0.0, 1.0, 0.75,
            286.0/286.0 * 0.3, 286.0/286.0 * 0.3, 286.0/286.0 * 0.3, 286.0/286.0 * 0.3,
            286, 286,
            286, 286, 286, 286,
            800.0, 800.0,
            0.0, 0.0,
            0, 0, 0, 0
        }
    },
    // Case 2: Button-like (asymmetric insets, opaque canvas)
    {
        "button_asymmetric_opaque",
        {
            0.1, 0.05, 0.8, 0.4,
            278.0/264.0 * 0.15, 278.0/264.0 * 0.15, 278.0/264.0 * 0.15, 278.0/264.0 * 0.15,
            920, 920,
            278, 278, 278, 278,
            800.0, 800.0,
            0.0, 0.0,
            255, 255, 255, 255
        }
    },
    // Case 3: Splitter-like (non-square, non-transparent canvas)
    {
        "splitter_colored_canvas",
        {
            0.2, 0.1, 0.6, 0.3,
            0.05, 0.05, 0.05, 0.05,
            600, 600,
            150, 150, 149, 149,
            1024.0, 1024.0,
            10.0, 5.0,
            128, 128, 128, 200
        }
    },
    // Case 4: IO field (asymmetric src insets)
    {
        "iofield_asymmetric_src",
        {
            0.0, 0.0, 1.0, 0.5,
            300.0/216.0 * 0.1, 346.0/216.0 * 0.1, 0.1, 0.1,
            592, 592,
            300, 346, 216, 216,
            800.0, 800.0,
            0.0, 0.0,
            0, 0, 0, 0
        }
    },
    // Case 5: Sub-pixel edge case (fractional scale, non-zero origin)
    {
        "subpixel_fractional_scale",
        {
            0.15, 0.08, 0.7, 0.35,
            0.12, 0.12, 0.12, 0.12,
            400, 400,
            100, 100, 100, 100,
            753.0, 753.0,
            3.7, 2.1,
            0, 0, 0, 128
        }
    },
};

static const int NUM_CASES = sizeof(cases) / sizeof(cases[0]);

int main() {
    int total_pass = 0, total_fail = 0;

    for (int c = 0; c < NUM_CASES; c++) {
        const TestCase& tc = cases[c];

        CBorderImageBoundaries cpp_out, rust_out;
        memset(&cpp_out, 0, sizeof(cpp_out));
        memset(&rust_out, 0, sizeof(rust_out));

        CppBorderImageBoundaries(tc.params, cpp_out);
        int rc = rust_border_image_boundaries(&tc.params, &rust_out);

        if (rc != 0) {
            printf("FAIL %s: Rust returned %d (skipped)\n", tc.name, rc);
            total_fail++;
            continue;
        }

        bool pass = true;
        const double eps = 1e-12;

        // Compare adjusted insets
        if (fabs(cpp_out.adj_l - rust_out.adj_l) > eps ||
            fabs(cpp_out.adj_t - rust_out.adj_t) > eps ||
            fabs(cpp_out.adj_r - rust_out.adj_r) > eps ||
            fabs(cpp_out.adj_b - rust_out.adj_b) > eps) {
            printf("  DIVERGE %s adjusted insets:\n", tc.name);
            printf("    C++:  l=%.15f t=%.15f r=%.15f b=%.15f\n",
                   cpp_out.adj_l, cpp_out.adj_t, cpp_out.adj_r, cpp_out.adj_b);
            printf("    Rust: l=%.15f t=%.15f r=%.15f b=%.15f\n",
                   rust_out.adj_l, rust_out.adj_t, rust_out.adj_r, rust_out.adj_b);
            pass = false;
        }

        // Compare 9 target rects
        const char* rect_names[] = {"UL","U","UR","L","C","R","LL","B","LR"};
        for (int i = 0; i < 9; i++) {
            for (int j = 0; j < 4; j++) {
                if (fabs(cpp_out.target_rects[i][j] - rust_out.target_rects[i][j]) > eps) {
                    const char* dim[] = {"x","y","w","h"};
                    printf("  DIVERGE %s target_rect[%s].%s: C++=%.15f Rust=%.15f\n",
                           tc.name, rect_names[i], dim[j],
                           cpp_out.target_rects[i][j], rust_out.target_rects[i][j]);
                    pass = false;
                }
            }
        }

        // Compare 9 source rects
        for (int i = 0; i < 9; i++) {
            for (int j = 0; j < 4; j++) {
                if (cpp_out.source_rects[i][j] != rust_out.source_rects[i][j]) {
                    const char* dim[] = {"sx","sy","sw","sh"};
                    printf("  DIVERGE %s source_rect[%s].%s: C++=%d Rust=%d\n",
                           tc.name, rect_names[i], dim[j],
                           cpp_out.source_rects[i][j], rust_out.source_rects[i][j]);
                    pass = false;
                }
            }
        }

        if (pass) {
            printf("PASS %s\n", tc.name);
            total_pass++;
        } else {
            total_fail++;
        }
    }

    printf("\n--- Results: %d PASS, %d FAIL ---\n", total_pass, total_fail);
    return total_fail > 0 ? 1 : 0;
}
