use crate::foundation::Color;

/// Line join style.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// Line cap style.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// Stroke end type. The contract defines 16 variants; essential ones are implemented,
/// the rest fall back to Butt behavior.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StrokeEnd {
    Butt,
    Round,
    Square,
    Arrow,
    Diamond,
    Circle,
    // Extended variants — rendered as Butt for now
    TriangleIn,
    TriangleOut,
    HookLeft,
    HookRight,
    ForkLeft,
    ForkRight,
    CrossBar,
    Dot,
    Ring,
    Flat,
}

impl StrokeEnd {
    /// Get the effective rendering style. Extended variants fall back to Butt.
    pub fn effective(self) -> Self {
        match self {
            StrokeEnd::Butt
            | StrokeEnd::Round
            | StrokeEnd::Square
            | StrokeEnd::Arrow
            | StrokeEnd::Diamond
            | StrokeEnd::Circle => self,
            _ => StrokeEnd::Butt,
        }
    }
}

/// Stroke properties for outlined shapes.
#[derive(Clone, Debug)]
pub struct Stroke {
    /// Stroke color.
    pub color: Color,
    /// Stroke width in pixels.
    pub width: f64,
    /// Line join style.
    pub join: LineJoin,
    /// Line cap style.
    pub cap: LineCap,
    /// Start end style.
    pub start_end: StrokeEnd,
    /// Finish end style.
    pub finish_end: StrokeEnd,
    /// Dash pattern: alternating on/off lengths. Empty = solid line.
    pub dash_pattern: Vec<f64>,
    /// Dash offset.
    pub dash_offset: f64,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 1.0,
            join: LineJoin::Miter,
            cap: LineCap::Butt,
            start_end: StrokeEnd::Butt,
            finish_end: StrokeEnd::Butt,
            dash_pattern: Vec::new(),
            dash_offset: 0.0,
        }
    }
}

impl Stroke {
    /// Create a simple solid stroke with the given color and width.
    pub fn new(color: Color, width: f64) -> Self {
        Self {
            color,
            width,
            ..Default::default()
        }
    }
}
