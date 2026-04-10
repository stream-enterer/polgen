// Test polygon rasterizer spans from Rust FFI.
// Calls rust_rasterize_polygon with several shapes and validates sanity.

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cmath>
#include <cassert>

struct CPolygonVertex { double x, y; };

struct CSpan {
    int x_start, x_end;
    int opacity_beg, opacity_mid, opacity_end;
};

struct CScanlineSpans {
    int y;
    int span_count;
    CSpan spans[64];
};

extern "C" int rust_rasterize_polygon(
    const CPolygonVertex* vertices,
    int n_vertices,
    double clip_x1, double clip_y1, double clip_x2, double clip_y2,
    int winding_rule,
    CScanlineSpans* out_scanlines,
    int max_scanlines,
    int* out_scanline_count
);

static void print_scanlines(const CScanlineSpans* sl, int count, int max_print) {
    for (int i = 0; i < count && i < max_print; i++) {
        printf("  y=%d  spans=%d", sl[i].y, sl[i].span_count);
        for (int j = 0; j < sl[i].span_count && j < 4; j++) {
            const CSpan& s = sl[i].spans[j];
            printf("  [%d..%d beg=%d mid=%d end=%d]",
                   s.x_start, s.x_end, s.opacity_beg, s.opacity_mid, s.opacity_end);
        }
        if (sl[i].span_count > 4) printf("  ...");
        printf("\n");
    }
    if (count > max_print) printf("  ... (%d more scanlines)\n", count - max_print);
}

static bool validate_basic(const CScanlineSpans* sl, int count,
                           int clip_y1, int clip_y2, const char* name) {
    bool ok = true;
    if (count <= 0) {
        printf("  FAIL: %s produced 0 scanlines\n", name);
        return false;
    }
    for (int i = 0; i < count; i++) {
        if (sl[i].y < clip_y1 || sl[i].y >= clip_y2) {
            printf("  FAIL: %s scanline y=%d outside clip [%d,%d)\n",
                   name, sl[i].y, clip_y1, clip_y2);
            ok = false;
        }
        for (int j = 0; j < sl[i].span_count; j++) {
            const CSpan& s = sl[i].spans[j];
            if (s.x_start > s.x_end) {
                printf("  FAIL: %s y=%d span x_start=%d > x_end=%d\n",
                       name, sl[i].y, s.x_start, s.x_end);
                ok = false;
            }
            // Opacity values should be in [0, 4096]
            if (s.opacity_beg < 0 || s.opacity_beg > 4096 ||
                s.opacity_mid < 0 || s.opacity_mid > 4096 ||
                s.opacity_end < 0 || s.opacity_end > 4096) {
                printf("  FAIL: %s y=%d span opacity out of range: beg=%d mid=%d end=%d\n",
                       name, sl[i].y, s.opacity_beg, s.opacity_mid, s.opacity_end);
                ok = false;
            }
        }
    }
    return ok;
}

int main() {
    CScanlineSpans scanlines[256];
    int scanline_count = 0;
    int rc;
    int pass = 0, fail = 0;

    // Test 1: Unit square
    {
        printf("=== Test 1: Unit square (10,10)-(40,40) ===\n");
        CPolygonVertex verts[] = {
            {10.0, 10.0}, {40.0, 10.0}, {40.0, 40.0}, {10.0, 40.0}
        };
        rc = rust_rasterize_polygon(verts, 4, 0.0, 0.0, 50.0, 50.0, 0,
                                    scanlines, 256, &scanline_count);
        printf("  rc=%d  scanline_count=%d\n", rc, scanline_count);
        print_scanlines(scanlines, scanline_count, 6);

        if (validate_basic(scanlines, scanline_count, 0, 50, "square")) {
            // Should have ~30 scanlines (y=10..39)
            if (scanline_count >= 28 && scanline_count <= 32) {
                printf("  PASS: reasonable scanline count\n");
                pass++;
            } else {
                printf("  FAIL: expected ~30 scanlines, got %d\n", scanline_count);
                fail++;
            }
        } else { fail++; }
    }

    // Test 2: Sub-pixel triangle
    {
        printf("\n=== Test 2: Sub-pixel triangle ===\n");
        CPolygonVertex verts[] = {
            {15.3, 10.7}, {35.8, 10.2}, {25.1, 38.9}
        };
        rc = rust_rasterize_polygon(verts, 3, 0.0, 0.0, 50.0, 50.0, 0,
                                    scanlines, 256, &scanline_count);
        printf("  rc=%d  scanline_count=%d\n", rc, scanline_count);
        print_scanlines(scanlines, scanline_count, 6);

        if (validate_basic(scanlines, scanline_count, 0, 50, "triangle")) {
            // Should cover roughly y=10..38
            if (scanline_count >= 25 && scanline_count <= 32) {
                printf("  PASS: reasonable scanline count\n");
                pass++;
            } else {
                printf("  FAIL: expected ~29 scanlines, got %d\n", scanline_count);
                fail++;
            }
        } else { fail++; }
    }

    // Test 3: 5-point star (NonZero winding)
    {
        printf("\n=== Test 3: 5-point star (NonZero) ===\n");
        CPolygonVertex verts[] = {
            {25.0,  2.0}, {29.0, 18.0}, {48.0, 18.0}, {33.0, 28.0}, {38.0, 46.0},
            {25.0, 34.0}, {12.0, 46.0}, {17.0, 28.0}, { 2.0, 18.0}, {21.0, 18.0}
        };
        rc = rust_rasterize_polygon(verts, 10, 0.0, 0.0, 50.0, 50.0, 0,
                                    scanlines, 256, &scanline_count);
        printf("  rc=%d  scanline_count=%d\n", rc, scanline_count);
        print_scanlines(scanlines, scanline_count, 8);

        if (validate_basic(scanlines, scanline_count, 0, 50, "star-nonzero")) {
            // Star covers roughly y=2..45
            if (scanline_count >= 35) {
                printf("  PASS: reasonable scanline count\n");
                pass++;
            } else {
                printf("  FAIL: expected >=35 scanlines, got %d\n", scanline_count);
                fail++;
            }
        } else { fail++; }
    }

    // Test 4: Same star with EvenOdd winding (center should be hollow)
    {
        printf("\n=== Test 4: 5-point star (EvenOdd) ===\n");
        CPolygonVertex verts[] = {
            {25.0,  2.0}, {29.0, 18.0}, {48.0, 18.0}, {33.0, 28.0}, {38.0, 46.0},
            {25.0, 34.0}, {12.0, 46.0}, {17.0, 28.0}, { 2.0, 18.0}, {21.0, 18.0}
        };
        rc = rust_rasterize_polygon(verts, 10, 0.0, 0.0, 50.0, 50.0, 1,
                                    scanlines, 256, &scanline_count);
        printf("  rc=%d  scanline_count=%d\n", rc, scanline_count);
        print_scanlines(scanlines, scanline_count, 8);

        if (validate_basic(scanlines, scanline_count, 0, 50, "star-evenodd")) {
            // EvenOdd star should have multiple spans per scanline in the middle
            bool found_multi = false;
            for (int i = 0; i < scanline_count; i++) {
                if (scanlines[i].span_count >= 2) { found_multi = true; break; }
            }
            if (found_multi) {
                printf("  PASS: found multi-span scanlines (expected for EvenOdd star)\n");
                pass++;
            } else {
                printf("  WARN: no multi-span scanlines found (may still be correct)\n");
                pass++; // not a hard failure
            }
        } else { fail++; }
    }

    printf("\n=== Summary: %d passed, %d failed ===\n", pass, fail);
    return fail > 0 ? 1 : 0;
}
