use crate::foundation::{Image, Rect};
use crate::render::font_cache::FontCache;
use crate::render::{Painter, Stroke, TextAlignment};

use super::look::Look;

/// Height allocated for caption and description text, derived from font metrics.
/// Uses glyph height + 4px padding.
const TEXT_ROW_HEIGHT: f64 = FontCache::DEFAULT_SIZE_PX + 4.0;

/// 1 - 1/sqrt(2), used for round-rect corner inset computation.
const CORNER_INSET_FACTOR: f64 = 1.0 - std::f64::consts::FRAC_1_SQRT_2;

/// Outer border style.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OuterBorderType {
    None,
    Filled,
    Margin,
    MarginFilled,
    Rect,
    RoundRect,
    Group,
    Instrument,
    InstrumentMoreRound,
    PopupRoot,
}

/// Inner border style.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InnerBorderType {
    None,
    Group,
    InputField,
    OutputField,
    CustomRect,
}

/// Layout of icon, caption, and description within the label area.
struct LabelLayout {
    icon_rect: Option<Rect>,
    caption_rect: Option<Rect>,
    description_rect: Option<Rect>,
    total_height: f64,
}

/// Border chrome helper. Embedded in widgets to draw surrounding decoration.
pub struct Border {
    pub outer: OuterBorderType,
    pub inner: InnerBorderType,
    pub caption: String,
    pub description: String,
    pub border_scaling: f64,
    pub label_alignment: TextAlignment,
    pub caption_alignment: Option<TextAlignment>,
    pub description_alignment: Option<TextAlignment>,
    pub icon: Option<Image>,
    pub icon_above_caption: bool,
    pub max_icon_area_tallness: f64,
}

impl Border {
    pub fn new(outer: OuterBorderType) -> Self {
        Self {
            outer,
            inner: InnerBorderType::None,
            caption: String::new(),
            description: String::new(),
            border_scaling: 1.0,
            label_alignment: TextAlignment::Left,
            caption_alignment: None,
            description_alignment: None,
            icon: None,
            icon_above_caption: false,
            max_icon_area_tallness: 1.0,
        }
    }

    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = caption.to_string();
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_inner(mut self, inner: InnerBorderType) -> Self {
        self.inner = inner;
        self
    }

    pub fn with_border_scaling(mut self, s: f64) -> Self {
        self.border_scaling = s.max(1e-10);
        self
    }

    pub fn set_border_scaling(&mut self, s: f64) {
        self.border_scaling = s.max(1e-10);
    }

    pub fn with_label_alignment(mut self, a: TextAlignment) -> Self {
        self.label_alignment = a;
        self
    }

    pub fn set_label_alignment(&mut self, a: TextAlignment) {
        self.label_alignment = a;
    }

    pub fn with_caption_alignment(mut self, a: TextAlignment) -> Self {
        self.caption_alignment = Some(a);
        self
    }

    pub fn set_caption_alignment(&mut self, a: Option<TextAlignment>) {
        self.caption_alignment = a;
    }

    pub fn with_description_alignment(mut self, a: TextAlignment) -> Self {
        self.description_alignment = Some(a);
        self
    }

    pub fn set_description_alignment(&mut self, a: Option<TextAlignment>) {
        self.description_alignment = a;
    }

    pub fn with_icon(mut self, icon: Image) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn set_icon(&mut self, icon: Option<Image>) {
        self.icon = icon;
    }

    pub fn set_icon_above_caption(&mut self, above: bool) {
        self.icon_above_caption = above;
    }

    pub fn set_max_icon_area_tallness(&mut self, t: f64) {
        self.max_icon_area_tallness = t.max(1e-10);
    }

    fn has_label(&self) -> bool {
        !self.caption.is_empty() || !self.description.is_empty() || self.icon.is_some()
    }

    /// Base scaling unit for outer geometry.
    fn base_unit(&self, w: f64, h: f64) -> f64 {
        w.min(h) * self.border_scaling
    }

    /// Outer border insets `(x, y, w_total, h_total)` — proportional to dimensions.
    fn outer_insets(&self, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let s = self.base_unit(w, h);
        let d = match self.outer {
            OuterBorderType::None | OuterBorderType::Filled => 0.0,
            OuterBorderType::Margin | OuterBorderType::MarginFilled => s * 0.04,
            OuterBorderType::Rect => s * 0.023,
            OuterBorderType::RoundRect => s * 0.22 * CORNER_INSET_FACTOR + s * 0.02,
            OuterBorderType::Group => s * 0.0104,
            OuterBorderType::Instrument => s * 0.052,
            OuterBorderType::InstrumentMoreRound => s * 0.223 * CORNER_INSET_FACTOR + s * 0.02,
            OuterBorderType::PopupRoot => s * 0.006,
        };
        if d == 0.0 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            (d, d, 2.0 * d, 2.0 * d)
        }
    }

    /// Inner border insets, computed from the area after outer+label.
    fn inner_insets(&self, iw: f64, ih: f64) -> (f64, f64, f64, f64) {
        let s = iw.min(ih) * self.border_scaling;
        let d = match self.inner {
            InnerBorderType::None => 0.0,
            InnerBorderType::Group => s * 0.0188,
            InnerBorderType::InputField | InnerBorderType::OutputField => s * 0.094,
            InnerBorderType::CustomRect => s * 0.0125,
        };
        if d == 0.0 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            (d, d, 2.0 * d, 2.0 * d)
        }
    }

    /// Corner radius for outer border types.
    fn outer_radius(&self, w: f64, h: f64) -> f64 {
        let s = self.base_unit(w, h);
        match self.outer {
            OuterBorderType::RoundRect => s * 0.22,
            OuterBorderType::Group => s * 0.0188,
            OuterBorderType::Instrument => s * 0.094,
            OuterBorderType::InstrumentMoreRound => s * 0.223,
            _ => 0.0,
        }
    }

    /// Corner radius for inner border types.
    fn inner_radius(&self, iw: f64, ih: f64) -> f64 {
        let s = iw.min(ih) * self.border_scaling;
        match self.inner {
            InnerBorderType::Group => s * 0.0188,
            InnerBorderType::InputField | InnerBorderType::OutputField => s * 0.094,
            InnerBorderType::CustomRect => s * 0.0125,
            InnerBorderType::None => 0.0,
        }
    }

    /// Stroke width for outer border outlines.
    fn outer_stroke_width(&self, w: f64, h: f64) -> f64 {
        let s = self.base_unit(w, h);
        match self.outer {
            OuterBorderType::PopupRoot => s * 0.012,
            OuterBorderType::None
            | OuterBorderType::Filled
            | OuterBorderType::Margin
            | OuterBorderType::MarginFilled => 0.0,
            _ => (s * 0.006).max(0.5),
        }
    }

    /// Compute label layout within the given area.
    fn label_layout(&self, area_x: f64, area_y: f64, area_w: f64, _area_h: f64) -> LabelLayout {
        let cap_h = if self.caption.is_empty() {
            0.0
        } else {
            TEXT_ROW_HEIGHT
        };
        let desc_h = if self.description.is_empty() {
            0.0
        } else {
            TEXT_ROW_HEIGHT
        };

        let icon = self.icon.as_ref().filter(|img| !img.is_empty());

        if icon.is_none() {
            // Simple text-only layout.
            let cap_rect = if cap_h > 0.0 {
                Some(Rect {
                    x: area_x,
                    y: area_y,
                    w: area_w,
                    h: cap_h,
                })
            } else {
                None
            };
            let desc_rect = if desc_h > 0.0 {
                Some(Rect {
                    x: area_x,
                    y: area_y + cap_h,
                    w: area_w,
                    h: desc_h,
                })
            } else {
                None
            };
            return LabelLayout {
                icon_rect: None,
                caption_rect: cap_rect,
                description_rect: desc_rect,
                total_height: cap_h + desc_h,
            };
        }

        let img = icon.expect("checked above");
        let img_w = img.width().max(1) as f64;
        let img_h = img.height().max(1) as f64;
        let icon_tallness = (img_h / img_w).min(self.max_icon_area_tallness);
        let gap = 0.1 * TEXT_ROW_HEIGHT;

        if self.icon_above_caption {
            let icon_h = 3.0 * TEXT_ROW_HEIGHT;
            let icon_w = icon_h / icon_tallness;
            let icon_rect = Rect {
                x: area_x + (area_w - icon_w) / 2.0,
                y: area_y,
                w: icon_w,
                h: icon_h,
            };
            let mut y = area_y + icon_h + gap;
            let cap_rect = if cap_h > 0.0 {
                let r = Rect {
                    x: area_x,
                    y,
                    w: area_w,
                    h: cap_h,
                };
                y += cap_h;
                Some(r)
            } else {
                None
            };
            let desc_rect = if desc_h > 0.0 {
                Some(Rect {
                    x: area_x,
                    y,
                    w: area_w,
                    h: desc_h,
                })
            } else {
                None
            };
            let total = icon_h + gap + cap_h + desc_h;
            LabelLayout {
                icon_rect: Some(icon_rect),
                caption_rect: cap_rect,
                description_rect: desc_rect,
                total_height: total,
            }
        } else {
            // Icon beside caption.
            let icon_h = TEXT_ROW_HEIGHT;
            let icon_w = icon_h / icon_tallness;
            let icon_rect = Rect {
                x: area_x,
                y: area_y,
                w: icon_w,
                h: icon_h,
            };
            let text_x = area_x + icon_w + gap;
            let text_w = (area_w - icon_w - gap).max(0.0);
            let cap_rect = if cap_h > 0.0 {
                Some(Rect {
                    x: text_x,
                    y: area_y,
                    w: text_w,
                    h: cap_h,
                })
            } else {
                None
            };
            let desc_rect = if desc_h > 0.0 {
                Some(Rect {
                    x: text_x,
                    y: area_y + cap_h,
                    w: text_w,
                    h: desc_h,
                })
            } else {
                None
            };
            let total = (icon_h).max(cap_h + desc_h);
            LabelLayout {
                icon_rect: Some(icon_rect),
                caption_rect: cap_rect,
                description_rect: desc_rect,
                total_height: total,
            }
        }
    }

    /// Returns `true` if this border type fully fills its rect so nothing behind
    /// it is visible. Only the border types that paint a solid background over the
    /// entire panel area qualify, and only when the background color is opaque.
    ///
    /// C++ equivalent: `emBorder::IsOpaque`.
    pub fn is_opaque(&self, look: &Look) -> bool {
        match self.outer {
            OuterBorderType::Filled
            | OuterBorderType::MarginFilled
            | OuterBorderType::PopupRoot => look.bg_color.is_opaque(),
            _ => false,
        }
    }

    /// Returns the *substance* round rectangle -- the outermost region where this
    /// border actually paints pixels. For simple rect-based types the radius is 0.
    /// For round types the radius matches the outer corner radius. For
    /// group/instrument types the rect is slightly expanded (per C++ TGA ratios)
    /// to cover the border-image area even though the Rust port paints simple
    /// fills.
    ///
    /// Returns `(rect, corner_radius)`.
    ///
    /// C++ equivalent: `emBorder::GetSubstanceRect`
    /// (via `DoBorder(BORDER_FUNC_SUBSTANCE_ROUND_RECT)`).
    pub fn substance_round_rect(&self, w: f64, h: f64) -> (Rect, f64) {
        let s = self.base_unit(w, h);
        match self.outer {
            OuterBorderType::None | OuterBorderType::Filled => (
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                },
                0.0,
            ),
            OuterBorderType::Margin | OuterBorderType::MarginFilled => {
                let d = s * 0.04;
                (
                    Rect {
                        x: d,
                        y: d,
                        w: (w - 2.0 * d).max(0.0),
                        h: (h - 2.0 * d).max(0.0),
                    },
                    0.0,
                )
            }
            OuterBorderType::Rect => {
                // Substance rect at the stroke center line.
                let d = s * 0.006;
                (
                    Rect {
                        x: d,
                        y: d,
                        w: (w - 2.0 * d).max(0.0),
                        h: (h - 2.0 * d).max(0.0),
                    },
                    0.0,
                )
            }
            OuterBorderType::RoundRect => {
                let d = s * 0.006; // half-stroke
                let f = s * 0.22; // outer radius
                (
                    Rect {
                        x: d,
                        y: d,
                        w: (w - 2.0 * d).max(0.0),
                        h: (h - 2.0 * d).max(0.0),
                    },
                    (f - d).max(0.0),
                )
            }
            OuterBorderType::Group => {
                let d = s * 0.0104;
                let rnd_r = s * 0.0188;
                let r = rnd_r * 280.0 / 209.0;
                let e = r - rnd_r;
                (
                    Rect {
                        x: (d - e).max(0.0),
                        y: (d - e).max(0.0),
                        w: (w - 2.0 * d + 2.0 * e).max(0.0),
                        h: (h - 2.0 * d + 2.0 * e).max(0.0),
                    },
                    r,
                )
            }
            OuterBorderType::Instrument => {
                let d = s * 0.052;
                let rnd_r = s * 0.094;
                let r = rnd_r * 280.0 / 209.0;
                let e = r - rnd_r;
                (
                    Rect {
                        x: (d - e).max(0.0),
                        y: (d - e).max(0.0),
                        w: (w - 2.0 * d + 2.0 * e).max(0.0),
                        h: (h - 2.0 * d + 2.0 * e).max(0.0),
                    },
                    r,
                )
            }
            OuterBorderType::InstrumentMoreRound => {
                let d = s * 0.02;
                let rnd_r = s * 0.223;
                let r = rnd_r * 336.0 / 293.4;
                let e = r - rnd_r;
                (
                    Rect {
                        x: (d - e).max(0.0),
                        y: (d - e).max(0.0),
                        w: (w - 2.0 * d + 2.0 * e).max(0.0),
                        h: (h - 2.0 * d + 2.0 * e).max(0.0),
                    },
                    r,
                )
            }
            OuterBorderType::PopupRoot => (
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                },
                0.0,
            ),
        }
    }

    /// Returns the content area as a round rectangle with corner radius.
    /// Unlike [`content_rect`](Self::content_rect) which returns the axis-aligned
    /// inscribed rectangle, this returns the round-rect boundary and its radius so
    /// callers can perform round-rect hit-testing or clipping.
    ///
    /// Returns `(rect, corner_radius)`.
    ///
    /// C++ equivalent: `emBorder::GetContentRoundRect`
    /// (via `DoBorder(BORDER_FUNC_CONTENT_ROUND_RECT)`).
    pub fn content_round_rect(&self, w: f64, h: f64, _look: &Look) -> (Rect, f64) {
        let (ox, oy, ow, oh) = self.outer_insets(w, h);
        let label_area_w = (w - ow).max(0.0);
        let label_h = if self.has_label() {
            self.label_layout(ox, oy, label_area_w, h).total_height
        } else {
            0.0
        };

        // Round rect after outer insets + label.
        let rnd_x = ox;
        let rnd_y = oy + label_h;
        let rnd_w = (w - ow).max(0.0);
        let rnd_h = (h - oh - label_h).max(0.0);
        let mut rnd_r = self.outer_radius(w, h);

        // Inner border processing: adjust round rect.
        let inner_s = rnd_w.min(rnd_h) * self.border_scaling;
        match self.inner {
            InnerBorderType::None => {}
            InnerBorderType::Group => {
                let r = inner_s * 0.0188;
                if rnd_r < r {
                    rnd_r = r;
                }
            }
            InnerBorderType::InputField | InnerBorderType::OutputField => {
                let r = inner_s * 0.094;
                // For IO fields, the content round rect is the inner field area.
                let (ix, iy, iw, ih) = self.inner_insets(rnd_w, rnd_h);
                return (
                    Rect {
                        x: rnd_x + ix,
                        y: rnd_y + iy,
                        w: (rnd_w - iw).max(0.0),
                        h: (rnd_h - ih).max(0.0),
                    },
                    r,
                );
            }
            InnerBorderType::CustomRect => {
                let r = inner_s * 0.0125;
                if rnd_r < r {
                    rnd_r = r;
                }
            }
        }

        let (ix, iy, iw, ih) = self.inner_insets(rnd_w, rnd_h);
        let ir = self.inner_radius(rnd_w, rnd_h);
        let final_r = if self.inner != InnerBorderType::None {
            ir
        } else {
            rnd_r
        };
        (
            Rect {
                x: rnd_x + ix,
                y: rnd_y + iy,
                w: (rnd_w - iw).max(0.0),
                h: (rnd_h - ih).max(0.0),
            },
            final_r.max(0.0),
        )
    }

    /// Returns the content rect with areas obscured by inner-border overlays
    /// removed. For input/output field inner borders, this is slightly smaller
    /// than [`content_rect`](Self::content_rect) because the field shadow/border
    /// images paint over the edges of the content area. For all other inner
    /// border types the result equals `content_rect`.
    ///
    /// C++ equivalent: `emBorder::GetContentRectUnobscured`
    /// (via `DoBorder(BORDER_FUNC_CONTENT_RECT_UNOBSCURED)`).
    pub fn content_rect_unobscured(&self, w: f64, h: f64, look: &Look) -> Rect {
        match self.inner {
            InnerBorderType::InputField | InnerBorderType::OutputField => {
                // IO fields have an overlay border that obscures a strip along
                // each edge. C++ computes d = 220/216 * rndR, then insets by d.
                let cr = self.content_rect(w, h, look);
                let inner_s = cr.w.min(cr.h) * self.border_scaling;
                let rnd_r = inner_s * 0.094;
                let d = rnd_r * 220.0 / 216.0;
                Rect {
                    x: cr.x + d,
                    y: cr.y + d,
                    w: (cr.w - 2.0 * d).max(0.0),
                    h: (cr.h - 2.0 * d).max(0.0),
                }
            }
            _ => self.content_rect(w, h, look),
        }
    }

    /// Compute the content area after border and label insets.
    pub fn content_rect(&self, w: f64, h: f64, _look: &Look) -> Rect {
        let (ox, oy, ow, oh) = self.outer_insets(w, h);
        let label_area_w = (w - ow).max(0.0);
        let label_h = if self.has_label() {
            self.label_layout(ox, oy, label_area_w, h).total_height
        } else {
            0.0
        };
        let iw = (w - ow).max(0.0);
        let ih = (h - oh - label_h).max(0.0);
        let (ix, iy, inner_w, inner_h) = self.inner_insets(iw, ih);

        Rect {
            x: ox + ix,
            y: oy + label_h + iy,
            w: (w - ow - inner_w).max(0.0),
            h: (h - oh - label_h - inner_h).max(0.0),
        }
    }

    /// Preferred size to fit the given content size.
    pub fn preferred_size_for_content(&self, cw: f64, ch: f64) -> (f64, f64) {
        let (_, _, ow, oh) = self.outer_insets(cw, ch);
        let label_area_w = cw;
        let label_h = if self.has_label() {
            self.label_layout(0.0, 0.0, label_area_w, ch).total_height
        } else {
            0.0
        };
        let (_, _, iw, ih) = self.inner_insets(cw, ch);
        (cw + ow + iw, ch + oh + label_h + ih)
    }

    /// Minimum size to fit any content.
    pub fn min_size_for_content(&self, min_cw: f64, min_ch: f64) -> (f64, f64) {
        self.preferred_size_for_content(min_cw, min_ch)
    }

    /// Paint the border chrome.
    pub fn paint_border(
        &self,
        painter: &mut Painter,
        w: f64,
        h: f64,
        look: &Look,
        focused: bool,
        enabled: bool,
    ) {
        // Dimming for disabled state: C++ "GetTransparented(75.0)" ~ alpha * 0.25.
        let dim_color = |c: crate::foundation::Color| -> crate::foundation::Color {
            if enabled {
                c
            } else {
                c.with_alpha((c.a() as u16 * 64 / 255) as u8)
            }
        };

        let outer_r = self.outer_radius(w, h);
        let stroke_w = self.outer_stroke_width(w, h);

        // Outer border
        match self.outer {
            OuterBorderType::None => {}
            OuterBorderType::Filled => {
                painter.paint_rect(0.0, 0.0, w, h, look.bg_color);
            }
            OuterBorderType::Margin => {}
            OuterBorderType::MarginFilled => {
                let (ox, oy, _, _) = self.outer_insets(w, h);
                painter.paint_rect(ox, oy, w - 2.0 * ox, h - 2.0 * oy, look.bg_color);
            }
            OuterBorderType::Rect => {
                let color = dim_color(if focused {
                    look.focus_tint()
                } else {
                    look.border_tint()
                });
                painter.paint_rect(0.0, 0.0, w, h, look.bg_color);
                painter.paint_rect_outlined(0.0, 0.0, w, h, &Stroke::new(color, stroke_w));
            }
            OuterBorderType::RoundRect => {
                let color = dim_color(if focused {
                    look.focus_tint()
                } else {
                    look.border_tint()
                });
                painter.paint_round_rect(0.0, 0.0, w, h, outer_r, look.bg_color);
                painter.paint_round_rect_outlined(
                    0.0,
                    0.0,
                    w,
                    h,
                    outer_r,
                    &Stroke::new(color, stroke_w),
                );
            }
            OuterBorderType::Group => {
                let color = dim_color(look.border_tint());
                painter.paint_round_rect(0.0, 0.0, w, h, outer_r, look.bg_color);
                painter.paint_round_rect_outlined(
                    0.0,
                    0.0,
                    w,
                    h,
                    outer_r,
                    &Stroke::new(color, stroke_w),
                );
            }
            OuterBorderType::Instrument => {
                painter.paint_round_rect(0.0, 0.0, w, h, outer_r, look.bg_color);
                let color = dim_color(if focused {
                    look.focus_tint()
                } else {
                    look.border_tint()
                });
                painter.paint_round_rect_outlined(
                    0.0,
                    0.0,
                    w,
                    h,
                    outer_r,
                    &Stroke::new(color, stroke_w),
                );
            }
            OuterBorderType::InstrumentMoreRound => {
                painter.paint_round_rect(0.0, 0.0, w, h, outer_r, look.bg_color);
                let color = dim_color(if focused {
                    look.focus_tint()
                } else {
                    look.border_tint()
                });
                painter.paint_round_rect_outlined(
                    0.0,
                    0.0,
                    w,
                    h,
                    outer_r,
                    &Stroke::new(color, stroke_w),
                );
            }
            OuterBorderType::PopupRoot => {
                painter.paint_rect(0.0, 0.0, w, h, look.bg_color);
                painter.paint_rect_outlined(
                    0.0,
                    0.0,
                    w,
                    h,
                    &Stroke::new(dim_color(look.border_tint()), stroke_w),
                );
            }
        }

        // Label area
        let (ox, oy, ow, _oh) = self.outer_insets(w, h);
        let label_area_w = (w - ow).max(0.0);
        let label = self.label_layout(ox, oy, label_area_w, h);

        let cap_align = self.caption_alignment.unwrap_or(self.label_alignment);
        let desc_align = self.description_alignment.unwrap_or(self.label_alignment);

        // Icon
        if let Some(ref icon_rect) = label.icon_rect {
            if let Some(ref img) = self.icon {
                if !img.is_empty() {
                    if img.channel_count() == 1 {
                        painter.paint_image_colored(
                            icon_rect.x,
                            icon_rect.y,
                            icon_rect.w,
                            icon_rect.h,
                            img,
                            0,
                            0,
                            img.width(),
                            img.height(),
                            dim_color(look.fg_color),
                        );
                    } else {
                        painter.paint_image_scaled(
                            icon_rect.x,
                            icon_rect.y,
                            icon_rect.w,
                            icon_rect.h,
                            img,
                            crate::render::ImageQuality::Bilinear,
                            crate::render::ImageExtension::Clamp,
                        );
                    }
                }
            }
        }

        // Caption
        if let Some(ref cap_rect) = label.caption_rect {
            painter.paint_text_boxed(
                cap_rect.x,
                cap_rect.y + 2.0,
                cap_rect.w,
                cap_rect.h,
                &self.caption,
                FontCache::DEFAULT_SIZE_PX,
                dim_color(look.fg_color),
                cap_align,
            );
        }

        // Description
        if let Some(ref desc_rect) = label.description_rect {
            painter.paint_text_boxed(
                desc_rect.x,
                desc_rect.y + 2.0,
                desc_rect.w,
                desc_rect.h,
                &self.description,
                FontCache::DEFAULT_SIZE_PX,
                dim_color(look.disabled_fg()),
                desc_align,
            );
        }

        // Inner border
        let inner_x = ox;
        let inner_y = oy + label.total_height;
        let inner_w = (w - ox * 2.0).max(0.0);
        let inner_h = (h - oy * 2.0 - label.total_height).max(0.0);
        let inner_r = self.inner_radius(inner_w, inner_h);
        let inner_stroke_w = {
            let s = inner_w.min(inner_h) * self.border_scaling;
            (s * 0.006).max(0.5)
        };

        match self.inner {
            InnerBorderType::None => {}
            InnerBorderType::Group => {
                painter.paint_round_rect_outlined(
                    inner_x,
                    inner_y,
                    inner_w,
                    inner_h,
                    inner_r,
                    &Stroke::new(dim_color(look.border_tint()), inner_stroke_w),
                );
            }
            InnerBorderType::InputField => {
                let bg = if enabled {
                    look.input_bg_color
                } else {
                    look.input_bg_color.lerp(look.bg_color, 0.80)
                };
                painter.paint_round_rect(inner_x, inner_y, inner_w, inner_h, inner_r, bg);
                painter.paint_round_rect_outlined(
                    inner_x,
                    inner_y,
                    inner_w,
                    inner_h,
                    inner_r,
                    &Stroke::new(dim_color(look.border_tint()), inner_stroke_w),
                );
            }
            InnerBorderType::OutputField => {
                let bg = if enabled {
                    look.output_bg_color
                } else {
                    look.output_bg_color.lerp(look.bg_color, 0.80)
                };
                painter.paint_round_rect(inner_x, inner_y, inner_w, inner_h, inner_r, bg);
                painter.paint_round_rect_outlined(
                    inner_x,
                    inner_y,
                    inner_w,
                    inner_h,
                    inner_r,
                    &Stroke::new(dim_color(look.border_tint()), inner_stroke_w),
                );
            }
            InnerBorderType::CustomRect => {
                painter.paint_round_rect_outlined(
                    inner_x,
                    inner_y,
                    inner_w,
                    inner_h,
                    inner_r,
                    &Stroke::new(dim_color(look.border_tint()), inner_stroke_w),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_look() -> Look {
        Look::default()
    }

    #[test]
    fn content_rect_none_border() {
        let border = Border::new(OuterBorderType::None);
        let Rect { x, y, w: cw, h: ch } = border.content_rect(100.0, 50.0, &test_look());
        assert!((x - 0.0).abs() < 0.01);
        assert!((y - 0.0).abs() < 0.01);
        assert!((cw - 100.0).abs() < 0.01);
        assert!((ch - 50.0).abs() < 0.01);
    }

    #[test]
    fn content_rect_rect_border() {
        let border = Border::new(OuterBorderType::Rect);
        let Rect { x, y, w: cw, h: ch } = border.content_rect(100.0, 50.0, &test_look());
        // s = 50 * 1.0 = 50, d = 50 * 0.023 = 1.15
        let d = 50.0 * 0.023;
        assert!((x - d).abs() < 0.01);
        assert!((y - d).abs() < 0.01);
        assert!((cw - (100.0 - 2.0 * d)).abs() < 0.01);
        assert!((ch - (50.0 - 2.0 * d)).abs() < 0.01);
    }

    #[test]
    fn content_rect_with_caption() {
        let border = Border::new(OuterBorderType::Rect).with_caption("Test");
        let Rect { x, y, w: cw, h: ch } = border.content_rect(100.0, 50.0, &test_look());
        let d = 50.0 * 0.023;
        assert!((x - d).abs() < 0.01);
        assert!((y - d - TEXT_ROW_HEIGHT).abs() < 0.01);
        assert!((cw - (100.0 - 2.0 * d)).abs() < 0.01);
        assert!((ch - (50.0 - 2.0 * d - TEXT_ROW_HEIGHT)).abs() < 0.01);
    }

    #[test]
    fn content_rect_with_inner_input_field() {
        let border = Border::new(OuterBorderType::None).with_inner(InnerBorderType::InputField);
        let Rect { x, y, w: cw, h: ch } = border.content_rect(100.0, 50.0, &test_look());
        // inner s = min(100, 50) * 1.0 = 50, d = 50 * 0.094 = 4.7
        let d = 50.0 * 0.094;
        assert!((x - d).abs() < 0.01);
        assert!((y - d).abs() < 0.01);
        assert!((cw - (100.0 - 2.0 * d)).abs() < 0.01);
        assert!((ch - (50.0 - 2.0 * d)).abs() < 0.01);
    }

    #[test]
    fn content_rect_instrument_with_caption_and_inner() {
        let border = Border::new(OuterBorderType::Instrument)
            .with_caption("Cap")
            .with_inner(InnerBorderType::InputField);
        let r = border.content_rect(100.0, 80.0, &test_look());
        // Outer s = 80*0.052 = 4.16, label = TEXT_ROW_HEIGHT
        let od = 80.0 * 0.052;
        let iw = 100.0 - 2.0 * od;
        let ih = 80.0 - 2.0 * od - TEXT_ROW_HEIGHT;
        let is = iw.min(ih);
        let id = is * 0.094;
        assert!((r.x - (od + id)).abs() < 0.5);
        assert!((r.y - (od + TEXT_ROW_HEIGHT + id)).abs() < 0.5);
        assert!((r.w - (100.0 - 2.0 * od - 2.0 * id)).abs() < 0.5);
        assert!((r.h - (80.0 - 2.0 * od - TEXT_ROW_HEIGHT - 2.0 * id)).abs() < 0.5);
    }

    #[test]
    fn preferred_size_round_trips() {
        let border = Border::new(OuterBorderType::RoundRect)
            .with_caption("Title")
            .with_inner(InnerBorderType::Group);
        let (pw, ph) = border.preferred_size_for_content(50.0, 30.0);
        let Rect { w: cw, h: ch, .. } = border.content_rect(pw, ph, &test_look());
        // Approximate round-trip: proportional insets differ when computed from
        // content size vs total size, so we allow broader tolerance.
        assert!((cw - 50.0).abs() < 5.0, "cw={cw}");
        assert!((ch - 30.0).abs() < 5.0, "ch={ch}");
    }

    #[test]
    fn border_scaling_doubles_insets() {
        let border1 = Border::new(OuterBorderType::Rect);
        let border2 = Border::new(OuterBorderType::Rect).with_border_scaling(2.0);
        let (ox1, _, _, _) = border1.outer_insets(100.0, 100.0);
        let (ox2, _, _, _) = border2.outer_insets(100.0, 100.0);
        assert!((ox2 - 2.0 * ox1).abs() < 0.01);
    }

    #[test]
    fn zero_size_clamping() {
        let border = Border::new(OuterBorderType::Instrument)
            .with_caption("Cap")
            .with_inner(InnerBorderType::InputField);
        let r = border.content_rect(1.0, 1.0, &test_look());
        assert!(r.w >= 0.0);
        assert!(r.h >= 0.0);
    }

    #[test]
    fn disabled_dimming_alpha() {
        use crate::foundation::Color;
        let c = Color::rgba(100, 150, 200, 255);
        let dimmed = c.with_alpha((c.a() as u16 * 64 / 255) as u8);
        // 255 * 64 / 255 = 64
        assert_eq!(dimmed.a(), 64);
        assert_eq!(dimmed.r(), 100);
    }

    #[test]
    fn with_alpha_preserves_rgb() {
        use crate::foundation::Color;
        let c = Color::rgb(10, 20, 30);
        let c2 = c.with_alpha(128);
        assert_eq!(c2.r(), 10);
        assert_eq!(c2.g(), 20);
        assert_eq!(c2.b(), 30);
        assert_eq!(c2.a(), 128);
    }

    #[test]
    fn has_label_with_icon_only() {
        let img = Image::new(16, 16, 4);
        let border = Border::new(OuterBorderType::None).with_icon(img);
        assert!(border.has_label());
    }

    #[test]
    fn label_height_icon_above() {
        let img = Image::new(16, 16, 4);
        let mut border = Border::new(OuterBorderType::None)
            .with_caption("Cap")
            .with_icon(img);
        border.set_icon_above_caption(true);
        let layout = border.label_layout(0.0, 0.0, 200.0, 200.0);
        let expected = 3.0 * TEXT_ROW_HEIGHT + 0.1 * TEXT_ROW_HEIGHT + TEXT_ROW_HEIGHT;
        assert!((layout.total_height - expected).abs() < 0.01);
    }

    #[test]
    fn content_rect_accounts_for_icon_height() {
        let img = Image::new(16, 16, 4);
        let mut border = Border::new(OuterBorderType::None)
            .with_caption("Cap")
            .with_icon(img);
        border.set_icon_above_caption(true);
        let r = border.content_rect(200.0, 200.0, &test_look());
        let layout = border.label_layout(0.0, 0.0, 200.0, 200.0);
        assert!((r.y - layout.total_height).abs() < 0.01);
    }

    #[test]
    fn image_is_empty() {
        let empty = Image::new(0, 0, 1);
        assert!(empty.is_empty());
        let nonempty = Image::new(1, 1, 1);
        assert!(!nonempty.is_empty());
    }

    // --- is_opaque tests ---

    #[test]
    fn is_opaque_filled_opaque_bg() {
        let look = test_look();
        assert!(
            look.bg_color.is_opaque(),
            "default look bg should be opaque"
        );
        let border = Border::new(OuterBorderType::Filled);
        assert!(border.is_opaque(&look));
    }

    #[test]
    fn is_opaque_margin_filled() {
        let border = Border::new(OuterBorderType::MarginFilled);
        assert!(border.is_opaque(&test_look()));
    }

    #[test]
    fn is_opaque_popup_root() {
        let border = Border::new(OuterBorderType::PopupRoot);
        assert!(border.is_opaque(&test_look()));
    }

    #[test]
    fn is_opaque_false_for_non_filled() {
        let look = test_look();
        for outer in [
            OuterBorderType::None,
            OuterBorderType::Margin,
            OuterBorderType::Rect,
            OuterBorderType::RoundRect,
            OuterBorderType::Group,
            OuterBorderType::Instrument,
            OuterBorderType::InstrumentMoreRound,
        ] {
            let border = Border::new(outer);
            assert!(!border.is_opaque(&look), "expected false for {outer:?}");
        }
    }

    #[test]
    fn is_opaque_transparent_bg() {
        use crate::foundation::Color;
        let mut look = test_look();
        look.bg_color = Color::rgba(100, 100, 100, 128);
        let border = Border::new(OuterBorderType::Filled);
        assert!(!border.is_opaque(&look));
    }

    // --- substance_round_rect tests ---

    #[test]
    fn substance_none_is_full_rect() {
        let border = Border::new(OuterBorderType::None);
        let (rect, r) = border.substance_round_rect(200.0, 100.0);
        assert!(rect.x.abs() < 0.001);
        assert!(rect.y.abs() < 0.001);
        assert!((rect.w - 200.0).abs() < 0.001);
        assert!((rect.h - 100.0).abs() < 0.001);
        assert!(r.abs() < 0.001);
    }

    #[test]
    fn substance_filled_is_full_rect() {
        let border = Border::new(OuterBorderType::Filled);
        let (rect, r) = border.substance_round_rect(200.0, 100.0);
        assert!((rect.w - 200.0).abs() < 0.001);
        assert!((rect.h - 100.0).abs() < 0.001);
        assert!(r.abs() < 0.001);
    }

    #[test]
    fn substance_margin_is_inset() {
        let border = Border::new(OuterBorderType::Margin);
        let (rect, r) = border.substance_round_rect(100.0, 100.0);
        let d = 100.0 * 0.04;
        assert!((rect.x - d).abs() < 0.01);
        assert!((rect.y - d).abs() < 0.01);
        assert!((rect.w - (100.0 - 2.0 * d)).abs() < 0.01);
        assert!((rect.h - (100.0 - 2.0 * d)).abs() < 0.01);
        assert!(r.abs() < 0.001);
    }

    #[test]
    fn substance_round_rect_has_radius() {
        let border = Border::new(OuterBorderType::RoundRect);
        let (rect, r) = border.substance_round_rect(200.0, 100.0);
        assert!(r > 0.0, "round rect substance should have positive radius");
        assert!(rect.w < 200.0, "should be inset from full width");
    }

    #[test]
    fn substance_group_expanded_from_rnd() {
        let border = Border::new(OuterBorderType::Group);
        let (rect, r) = border.substance_round_rect(200.0, 100.0);
        let s = 100.0; // min(200,100) * 1.0
        let d = s * 0.0104; // outer inset
        let rnd_r = s * 0.0188;
        let expanded_r = rnd_r * 280.0 / 209.0;
        let e = expanded_r - rnd_r;
        assert!((r - expanded_r).abs() < 0.01);
        assert!((rect.x - (d - e)).abs() < 0.01);
    }

    #[test]
    fn substance_popup_root_is_full_rect() {
        let border = Border::new(OuterBorderType::PopupRoot);
        let (rect, r) = border.substance_round_rect(200.0, 100.0);
        assert!(rect.x.abs() < 0.001);
        assert!((rect.w - 200.0).abs() < 0.001);
        assert!(r.abs() < 0.001);
    }

    // --- content_round_rect tests ---

    #[test]
    fn content_round_rect_none_border() {
        let border = Border::new(OuterBorderType::None);
        let look = test_look();
        let (rect, r) = border.content_round_rect(100.0, 50.0, &look);
        assert!(rect.x.abs() < 0.01);
        assert!(rect.y.abs() < 0.01);
        assert!((rect.w - 100.0).abs() < 0.01);
        assert!((rect.h - 50.0).abs() < 0.01);
        assert!(r.abs() < 0.01);
    }

    #[test]
    fn content_round_rect_with_inner_input_field() {
        let border = Border::new(OuterBorderType::None).with_inner(InnerBorderType::InputField);
        let look = test_look();
        let (rect, r) = border.content_round_rect(100.0, 50.0, &look);
        // Should be inset by input field inset
        let d = 50.0 * 0.094;
        assert!((rect.x - d).abs() < 0.5);
        assert!((rect.y - d).abs() < 0.5);
        assert!(r > 0.0, "IO field should have positive radius");
    }

    #[test]
    fn content_round_rect_matches_content_rect_position() {
        // For non-IO inner borders, the rect position should match content_rect.
        let border = Border::new(OuterBorderType::Rect).with_inner(InnerBorderType::Group);
        let look = test_look();
        let (rr, _radius) = border.content_round_rect(100.0, 60.0, &look);
        let cr = border.content_rect(100.0, 60.0, &look);
        assert!((rr.x - cr.x).abs() < 0.5);
        assert!((rr.y - cr.y).abs() < 0.5);
        assert!((rr.w - cr.w).abs() < 0.5);
        assert!((rr.h - cr.h).abs() < 0.5);
    }

    // --- content_rect_unobscured tests ---

    #[test]
    fn content_rect_unobscured_equals_content_rect_for_none() {
        let border = Border::new(OuterBorderType::Rect);
        let look = test_look();
        let cr = border.content_rect(100.0, 50.0, &look);
        let cu = border.content_rect_unobscured(100.0, 50.0, &look);
        assert!((cr.x - cu.x).abs() < 0.001);
        assert!((cr.y - cu.y).abs() < 0.001);
        assert!((cr.w - cu.w).abs() < 0.001);
        assert!((cr.h - cu.h).abs() < 0.001);
    }

    #[test]
    fn content_rect_unobscured_smaller_for_input_field() {
        let border = Border::new(OuterBorderType::None).with_inner(InnerBorderType::InputField);
        let look = test_look();
        let cr = border.content_rect(200.0, 100.0, &look);
        let cu = border.content_rect_unobscured(200.0, 100.0, &look);
        // Unobscured should be strictly inset from content rect.
        assert!(
            cu.x > cr.x,
            "unobscured x ({}) > content x ({})",
            cu.x,
            cr.x
        );
        assert!(
            cu.y > cr.y,
            "unobscured y ({}) > content y ({})",
            cu.y,
            cr.y
        );
        assert!(
            cu.w < cr.w,
            "unobscured w ({}) < content w ({})",
            cu.w,
            cr.w
        );
        assert!(
            cu.h < cr.h,
            "unobscured h ({}) < content h ({})",
            cu.h,
            cr.h
        );
    }

    #[test]
    fn content_rect_unobscured_equals_content_rect_for_group_inner() {
        let border = Border::new(OuterBorderType::None).with_inner(InnerBorderType::Group);
        let look = test_look();
        let cr = border.content_rect(200.0, 100.0, &look);
        let cu = border.content_rect_unobscured(200.0, 100.0, &look);
        assert!((cr.x - cu.x).abs() < 0.001);
        assert!((cr.w - cu.w).abs() < 0.001);
    }
}
