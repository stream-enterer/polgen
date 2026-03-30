//! Port of C++ emDirPanel grid layout algorithm (LayoutChildren).

pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Port of C++ emDirPanel::LayoutChildren grid algorithm.
/// theme_height is the theme's Height value.
/// panel_height is GetHeight() (the panel's actual height, typically GetHeight()).
/// pad_l/t/r/b are DirPaddingL/T/R/B from the theme.
pub fn compute_grid_layout(
    count: usize,
    theme_height: f64,
    panel_height: f64,
    pad_l: f64,
    pad_t: f64,
    pad_r: f64,
    pad_b: f64,
) -> Vec<LayoutRect> {
    if count == 0 {
        return Vec::new();
    }

    let t = theme_height;
    let h = panel_height;

    // Find minimum rows such that rows*cols >= count
    let mut rows = 1;
    loop {
        let mut cols = (rows as f64 * t / (h * (1.0 - 0.05 / rows as f64))) as i32;
        if cols <= 0 {
            cols = 1;
        }
        if (rows * cols as usize) >= count {
            break;
        }
        rows += 1;
    }
    let cols = count.div_ceil(rows);

    // Cell dimensions with padding
    let mut cw = 1.0 / (pad_l + cols as f64 + pad_r);
    let mut ch = h / (pad_t / t + rows as f64 + pad_b / t);
    if ch > cw * t {
        ch = cw * t;
    } else {
        cw = ch / t;
    }
    let mut cx = cw * pad_l;
    let cy = cw * pad_t;

    // Gap calculation
    let f = 1.0 - cw * (pad_l + pad_r);
    let n = (f / cw + 0.001) as i32;
    let mut gap = ((pad_t + pad_b) / t - (pad_l + pad_r)) * cw;
    gap = gap.min(f - n as f64 * cw);
    if gap < 0.0 {
        gap = 0.0;
    }
    gap /= (n + 1) as f64;
    cx += gap;

    // Column-major layout
    let mut rects = Vec::with_capacity(count);
    let mut col = 0;
    let mut row = 0;
    for _ in 0..count {
        rects.push(LayoutRect {
            x: cx + (cw + gap) * col as f64,
            y: cy + ch * row as f64,
            w: cw,
            h: ch,
        });
        row += 1;
        if row >= rows {
            col += 1;
            row = 0;
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout_single_entry() {
        let rects = compute_grid_layout(1, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].x >= 0.0);
        assert!(rects[0].y >= 0.0);
        assert!(rects[0].w > 0.0);
        assert!(rects[0].h > 0.0);
    }

    #[test]
    fn grid_layout_many_entries() {
        let rects = compute_grid_layout(20, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 20);
        for r in &rects {
            assert!(r.x >= 0.0);
            assert!(r.x + r.w <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn grid_layout_column_major() {
        let rects = compute_grid_layout(4, 1.5, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(rects.len(), 4);
        // Column-major: entries[0] and [1] share same column (x)
        assert!((rects[0].x - rects[1].x).abs() < 1e-9);
    }

    #[test]
    fn grid_layout_empty() {
        let rects = compute_grid_layout(0, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert!(rects.is_empty());
    }
}
