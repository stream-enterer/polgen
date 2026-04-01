// Rust-only regression test for emMainControlPanel::LayoutChildren.
// Verifies that the simplified flat vertical layout produces correct button
// positions. No C++ golden data needed — expected values are computed from
// the algorithm constants.

use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emPanel::PanelBehavior;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelTree;

use emMain::emMainControlPanel::emMainControlPanel;

/// Expected layout constants (from emMainControlPanel.rs):
///   BUTTON_WEIGHT = 1.0, BOOKMARKS_WEIGHT = 6.5
///   n_buttons = 5
///   total_weight = 5 * 1.0 + 6.5 = 11.5
///   pad_x = 0.01
///   child_w = 1.0 - 2 * 0.01 = 0.98
///   gap_frac = 0.005
///   total_gaps = 6 * 0.005 = 0.03
///   usable_h = 1.0 - 0.03 = 0.97
///   btn_h = 0.97 * (1.0 / 11.5)
///   bm_h  = 0.97 * (6.5 / 11.5)
const EPS: f64 = 1e-12;

fn expected_layout() -> Vec<(f64, f64, f64, f64)> {
    let n_buttons: f64 = 5.0;
    let button_weight: f64 = 1.0;
    let bookmarks_weight: f64 = 6.5;
    let total_weight = n_buttons * button_weight + bookmarks_weight;
    let pad_x = 0.01_f64;
    let child_w = 1.0 - 2.0 * pad_x;
    let gap_frac = 0.005_f64;
    let total_gaps = (n_buttons as usize + 1) as f64 * gap_frac;
    let usable_h = 1.0 - total_gaps;

    let btn_h = usable_h * (button_weight / total_weight);
    let bm_h = usable_h * (bookmarks_weight / total_weight);

    let mut rects = Vec::new();
    let mut y = gap_frac;
    for _ in 0..n_buttons as usize {
        rects.push((pad_x, y, child_w, btn_h));
        y += btn_h + gap_frac;
    }
    rects.push((pad_x, y, child_w, bm_h));
    rects
}

#[test]
fn control_panel_layout_children() {
    let ctx = emContext::NewRoot();
    let mut panel = emMainControlPanel::new(Rc::clone(&ctx));

    let mut tree = PanelTree::new();
    let root = tree.create_root("ctrl_root");
    // Give root a 1:1 layout so normalized coordinates are [0,1] x [0,1].
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);

    // Call LayoutChildren — this creates children AND positions them.
    {
        let mut pctx = PanelCtx::new(&mut tree, root);
        panel.LayoutChildren(&mut pctx);
    }

    // Collect actual child layout rects.
    let children: Vec<_> = tree.children(root).collect();
    let expected = expected_layout();

    assert_eq!(
        children.len(),
        expected.len(),
        "Expected {} children (5 buttons + 1 bookmarks), got {}",
        expected.len(),
        children.len()
    );

    for (i, (&child_id, &(ex, ey, ew, eh))) in children.iter().zip(expected.iter()).enumerate() {
        let rect = tree
            .layout_rect(child_id)
            .unwrap_or_else(|| panic!("child {i} has no layout rect"));

        let label = if i < 5 {
            format!("btn_{i}")
        } else {
            "bookmarks".to_string()
        };

        assert!(
            (rect.x - ex).abs() < EPS,
            "{label}: x mismatch: actual={} expected={}",
            rect.x,
            ex
        );
        assert!(
            (rect.y - ey).abs() < EPS,
            "{label}: y mismatch: actual={} expected={}",
            rect.y,
            ey
        );
        assert!(
            (rect.w - ew).abs() < EPS,
            "{label}: w mismatch: actual={} expected={}",
            rect.w,
            ew
        );
        assert!(
            (rect.h - eh).abs() < EPS,
            "{label}: h mismatch: actual={} expected={}",
            rect.h,
            eh
        );
    }
}

#[test]
fn control_panel_child_names() {
    let ctx = emContext::NewRoot();
    let mut panel = emMainControlPanel::new(Rc::clone(&ctx));

    let mut tree = PanelTree::new();
    let root = tree.create_root("ctrl_root");
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);

    {
        let mut pctx = PanelCtx::new(&mut tree, root);
        panel.LayoutChildren(&mut pctx);
    }

    let children: Vec<_> = tree.children(root).collect();
    let names: Vec<&str> = children
        .iter()
        .map(|&id| tree.name(id).unwrap())
        .collect();

    assert_eq!(
        names,
        vec!["btn_0", "btn_1", "btn_2", "btn_3", "btn_4", "bookmarks"]
    );
}
