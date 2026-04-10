// Intercept PaintBorderImage calls from a C++ emBorder rendering.
// Uses a custom emPainter wrapper to log all paint calls.
//
// Build:
//   g++ -std=c++11 -O2 \
//     -I ~/git/eaglemode-0.96.4/include \
//     -L ~/git/eaglemode-0.96.4/lib \
//     -o harness/dump_border_calls \
//     harness/dump_border_calls.cpp \
//     -lemCore \
//     -Wl,-rpath,$HOME/git/eaglemode-0.96.4/lib

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <cstdint>

#include <emCore/emScheduler.h>
#include <emCore/emContext.h>
#include <emCore/emPainter.h>
#include <emCore/emImage.h>
#include <emCore/emView.h>
#include <emCore/emBorder.h>

// A testable emBorder subclass
class TestBorder : public emBorder {
public:
    TestBorder(emView& view, const emString& name, const emString& caption)
        : emBorder(view, name, caption) {}

    // Override to make DoLayout accessible
    void DoLayout(double x, double y, double w, double h) {
        SetViewCondition(emViewCondition(
            emViewCondition::VCT_AREA, 1.0
        ));
        LayoutPanel(x, y, w, h, 0);
    }
};

// Custom viewport to access Paint
class TestViewPort : public emViewPort {
public:
    TestViewPort(emView& view) : emViewPort(view) {
        SetViewGeometry(0, 0, 800, 600, 1.0);
    }

    void DoPaint(emPainter& p) {
        PaintView(p, 0);
    }
};

int main() {
    emStandardScheduler sched;
    emRootContext ctx(sched);

    // Create view
    emView view(ctx, emView::VF_NO_ACTIVE_HIGHLIGHT);
    TestViewPort vp(view);

    // Create border widget
    TestBorder* border = new TestBorder(view, "test", "Test");
    border->SetBorderType(emBorder::OBT_ROUND_RECT, emBorder::IBT_NONE);
    border->Layout(0, 0, 1.0, 0.002);

    // Run scheduler
    struct TermCtrl : public emEngine {
        int cnt;
        TermCtrl(emScheduler& s, int n) : emEngine(s), cnt(n) { WakeUp(); }
        virtual bool Cycle() { return --cnt > 0; }
    };
    TermCtrl ctrl(sched, 30);
    sched.Run();

    // Render
    emImage img(800, 600, 4);
    memset((void*)img.GetMap(), 0, 800*600*4);

    emPainter p;
    if (!img.PreparePainter(&p, ctx, 0.0, 0.0, 800.0, 600.0)) {
        fprintf(stderr, "PreparePainter failed\n");
        return 1;
    }

    vp.DoPaint(p);

    // Dump specific pixel values
    const unsigned char* map = (const unsigned char*)img.GetMap();
    printf("C++ pixel dump for border_roundrect_thin:\n");
    int positions[][2] = {
        {0, 298}, {0, 299}, {0, 300}, {0, 301},
        {799, 298}, {799, 299}, {799, 300}, {799, 301},
        {400, 298}, {400, 299}, {400, 300}, {400, 301},
    };
    for (auto& pos : positions) {
        int x = pos[0], y = pos[1];
        int off = (y * 800 + x) * 4;
        printf("  (%d,%d): rgb(%d,%d,%d) a=%d\n",
               x, y, map[off], map[off+1], map[off+2], map[off+3]);
    }

    return 0;
}
