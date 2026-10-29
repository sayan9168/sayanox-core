//! CSS box model geometry (`Rect` and `Dimensions`).

pub use crate::style::EdgeSizes;
use std::fmt;

/// Axis-aligned rectangle in CSS pixels (`(x, y)` is top-left).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Expand this rectangle outward on all four sides by `edge`.
    pub fn expanded_by(self, edge: EdgeSizes) -> Self {
        Self {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, {}x{})",
            fmt_num(self.x),
            fmt_num(self.y),
            fmt_num(self.width),
            fmt_num(self.height)
        )
    }
}

/// Complete CSS box model geometry for a single [`super::LayoutBox`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimensions {
    /// Position and size of the content area relative to the document origin.
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    /// Initial containing block for a viewport of `width` CSS pixels.
    ///
    /// Note that `content.height` starts at `0.0` during vertical block flow so
    /// the first child is positioned at `y = 0.0` and subsequent siblings stack
    /// below as height accumulates.
    pub const fn viewport(width: f32) -> Self {
        Self {
            content: Rect::new(0.0, 0.0, width, 0.0),
            padding: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
        }
    }

    /// Content area plus padding.
    pub fn padding_box(&self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    /// Content area plus padding and border (the painted background & border area).
    pub fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }

    /// Full outer box including margin (used for vertical sibling spacing).
    pub fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

pub(crate) fn fmt_num(v: f32) -> String {
    if (v.fract()).abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}
