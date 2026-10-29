//! Block formatting context calculations (CSS 2.1 §10.3.3 & §10.6.3 subset).
//!
//! Vertical margins currently stack additively (`margin_box().height`); sibling
//! and parent-child margin collapsing is intentionally omitted to keep the
//! initial block layout algorithm transparent and predictable.

use super::box_model::Dimensions;
use super::{BoxType, LayoutBox};
use crate::style::{ComputedStyle, Dimension, EdgeSizes, MarginValue};

impl<'a> LayoutBox<'a> {
    /// Compute this box's geometry and recursively lay out its children inside
    /// `containing_block`.
    pub fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BoxType::Viewport => self.layout_viewport(containing_block),
            BoxType::BlockNode(_) | BoxType::AnonymousBlock => {
                self.layout_block(containing_block);
            }
        }
    }

    fn layout_viewport(&mut self, containing_block: Dimensions) {
        self.dimensions = Dimensions::viewport(containing_block.content.width);
        self.dimensions.content.x = containing_block.content.x;
        self.dimensions.content.y = containing_block.content.y;
        self.layout_block_children();
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // Width depends on the parent containing block's width, whereas height
        // depends on the laid-out children, so width is resolved top-down before
        // recursing and height is finalized bottom-up afterward.
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        self.layout_block_children();
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let default_style = ComputedStyle::default();
        let style = self.style().unwrap_or(&default_style);

        let padding = style.padding;
        let border = style.border_width;
        let container_width = containing_block.content.width;

        let width_px = match style.width {
            Dimension::Auto => 0.0,
            Dimension::Px(w) => w.max(0.0),
        };
        let ml_px = style.margin.left.px_or_zero();
        let mr_px = style.margin.right.px_or_zero();

        let total = padding.left
            + padding.right
            + border.left
            + border.right
            + width_px
            + ml_px
            + mr_px;

        let underflow = container_width - total;

        let (used_width, used_ml, used_mr) = match (
            style.width,
            style.margin.left,
            style.margin.right,
        ) {
            // Width is `auto`: any `auto` horizontal margins become 0 and the
            // content box expands to fill remaining horizontal space.
            (Dimension::Auto, ml, mr) => {
                let left = ml.px_or_zero();
                let right = mr.px_or_zero();
                if underflow >= 0.0 {
                    (underflow, left, right)
                } else {
                    (0.0, left, right + underflow)
                }
            }

            // Fixed width with both horizontal margins `auto`: center horizontally
            // when the box fits inside its container.
            (Dimension::Px(w), MarginValue::Auto, MarginValue::Auto) => {
                let w = w.max(0.0);
                if underflow >= 0.0 {
                    (w, underflow / 2.0, underflow / 2.0)
                } else {
                    (w, 0.0, underflow)
                }
            }

            // Fixed width with one `auto` margin: absorb remaining space on that side.
            (Dimension::Px(w), MarginValue::Auto, MarginValue::Px(mr)) => {
                (w.max(0.0), underflow, mr)
            }
            (Dimension::Px(w), MarginValue::Px(ml), MarginValue::Auto) => {
                (w.max(0.0), ml, underflow)
            }

            // Overconstrained LTR: adjust `margin-right` to satisfy the equation.
            (Dimension::Px(w), MarginValue::Px(ml), MarginValue::Px(mr)) => {
                (w.max(0.0), ml, mr + underflow)
            }
        };

        let d = &mut self.dimensions;
        d.content.width = used_width;
        d.padding = padding;
        d.border = border;
        d.margin.left = used_ml;
        d.margin.right = used_mr;
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let default_style = ComputedStyle::default();
        let style = self.style().unwrap_or(&default_style);
        let d = &mut self.dimensions;

        // Per CSS 2.1 §10.6.3, `margin-top: auto` and `margin-bottom: auto`
        // resolve to `0` in normal block flow.
        d.margin.top = style.margin.top.px_or_zero();
        d.margin.bottom = style.margin.bottom.px_or_zero();

        d.content.x =
            containing_block.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing_block.content.y
            + containing_block.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        for child in &mut self.children {
            child.layout(self.dimensions);
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        if let Some(style) = self.style() {
            if let Dimension::Px(h) = style.height {
                self.dimensions.content.height = h.max(0.0);
            }
        }
    }
}

pub(crate) fn fmt_edges(e: EdgeSizes) -> String {
    use super::box_model::fmt_num;
    format!(
        "({}, {}, {}, {})",
        fmt_num(e.top),
        fmt_num(e.right),
        fmt_num(e.bottom),
        fmt_num(e.left)
    )
}
