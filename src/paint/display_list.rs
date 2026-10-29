//! Display list construction from a positioned [`LayoutBox`] tree.
//!
//! A [`DisplayList`] decouples layout geometry from pixel rasterization: layout
//! produces positioned boxes, this module lowers them into ordered drawing
//! primitives, and [`super::rasterizer::Canvas`] executes those primitives.

use crate::layout::{LayoutBox, Rect};
use crate::util::Color;
use std::fmt;

/// A single drawing operation in painter's order (back-to-front).
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    /// Fill an axis-aligned rectangle (in CSS pixels) with a solid color.
    SolidRect { color: Color, rect: Rect },
}

/// Ordered sequence of [`DisplayCommand`]s ready for rasterization.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DisplayCommand> {
        self.commands.iter()
    }
}

impl fmt::Display for DisplayList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, cmd) in self.commands.iter().enumerate() {
            match cmd {
                DisplayCommand::SolidRect { color, rect } => {
                    writeln!(f, "  [{idx:02}] SolidRect rect={rect} color={color}")?;
                }
            }
        }
        Ok(())
    }
}

/// Traverse `layout_root` in pre-order to build a back-to-front [`DisplayList`].
pub fn build_display_list(layout_root: &LayoutBox<'_>) -> DisplayList {
    let mut list = DisplayList::new();
    emit_layout_box(&mut list, layout_root);
    list
}

fn emit_layout_box(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    emit_background(list, layout_box);
    emit_borders(list, layout_box);

    for child in &layout_box.children {
        emit_layout_box(list, child);
    }
}

fn emit_background(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    let Some(style) = layout_box.style() else {
        return;
    };
    if style.background_color.a == 0 {
        return;
    }

    let rect = layout_box.dimensions.border_box();
    if rect.width > 0.0 && rect.height > 0.0 {
        list.push(DisplayCommand::SolidRect {
            color: style.background_color,
            rect,
        });
    }
}

fn emit_borders(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    let Some(style) = layout_box.style() else {
        return;
    };
    let color = style.border_color;
    if color.a == 0 {
        return;
    }

    let d = &layout_box.dimensions;
    let b = d.border;
    if b.is_zero() {
        return;
    }

    let bbox = d.border_box();
    // Top border strip
    if b.top > 0.0 {
        list.push(DisplayCommand::SolidRect {
            color,
            rect: Rect::new(bbox.x, bbox.y, bbox.width, b.top),
        });
    }
    // Bottom border strip
    if b.bottom > 0.0 {
        list.push(DisplayCommand::SolidRect {
            color,
            rect: Rect::new(bbox.x, bbox.y + bbox.height - b.bottom, bbox.width, b.bottom),
        });
    }
    // Left and right strips span between top and bottom borders so corners are
    // not double-composited when using translucent border colors.
    let side_y = bbox.y + b.top;
    let side_h = (bbox.height - b.top - b.bottom).max(0.0);
    if side_h > 0.0 {
        if b.left > 0.0 {
            list.push(DisplayCommand::SolidRect {
                color,
                rect: Rect::new(bbox.x, side_y, b.left, side_h),
            });
        }
        if b.right > 0.0 {
            list.push(DisplayCommand::SolidRect {
                color,
                rect: Rect::new(bbox.x + bbox.width - b.right, side_y, b.right, side_h),
            });
        }
    }
}
