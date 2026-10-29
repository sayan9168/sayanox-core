//! Pure-Rust CPU software rasterizer.

use super::display_list::{build_display_list, DisplayCommand, DisplayList};
use crate::layout::{LayoutBox, Rect};
use crate::util::Color;

/// Row-major RGBA pixel buffer rendered on the CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
}

impl Canvas {
    /// Allocate a `width × height` canvas initialized to `clear_color`.
    pub fn new(width: usize, height: usize, clear_color: Color) -> Self {
        Self {
            width,
            height,
            pixels: vec![clear_color; width.saturating_mul(height)],
        }
    }

    /// Fill the entire canvas with `color`.
    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color);
    }

    /// Execute every drawing command in `display_list` in order.
    pub fn paint_display_list(&mut self, display_list: &DisplayList) {
        for command in display_list.iter() {
            self.paint_command(command);
        }
    }

    /// Execute a single [`DisplayCommand`].
    pub fn paint_command(&mut self, command: &DisplayCommand) {
        match *command {
            DisplayCommand::SolidRect { color, rect } => self.fill_rect(rect, color),
        }
    }

    /// Fill an axis-aligned rectangle clipped to canvas bounds, alpha-blending
    /// translucent colors over existing pixels.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if color.a == 0
            || self.width == 0
            || self.height == 0
            || !rect.x.is_finite()
            || !rect.y.is_finite()
            || !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width <= 0.0
            || rect.height <= 0.0
        {
            return;
        }

        let max_x = self.width as f32;
        let max_y = self.height as f32;

        let x0 = rect.x.round().clamp(0.0, max_x) as usize;
        let y0 = rect.y.round().clamp(0.0, max_y) as usize;
        let x1 = (rect.x + rect.width).round().clamp(0.0, max_x) as usize;
        let y1 = (rect.y + rect.height).round().clamp(0.0, max_y) as usize;

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        if color.a == 255 {
            for y in y0..y1 {
                let row_start = y * self.width;
                self.pixels[row_start + x0..row_start + x1].fill(color);
            }
        } else {
            for y in y0..y1 {
                let row_start = y * self.width;
                for px in &mut self.pixels[row_start + x0..row_start + x1] {
                    *px = color.blend_over(*px);
                }
            }
        }
    }

    /// Read the pixel at `(x, y)` if within bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<Color> {
        if x < self.width && y < self.height {
            Some(self.pixels[y * self.width + x])
        } else {
            None
        }
    }

    /// Copy pixels into a `0x00RRGGBB` slice suitable for `softbuffer`.
    pub fn copy_to_0rgb_buffer(&self, dst: &mut [u32]) {
        for (out, px) in dst.iter_mut().zip(self.pixels.iter()) {
            *out = px.to_0rgb_u32();
        }
    }

    /// Allocate a `Vec<u32>` of packed `0x00RRGGBB` pixels.
    pub fn to_0rgb_vec(&self) -> Vec<u32> {
        self.pixels.iter().map(|px| px.to_0rgb_u32()).collect()
    }
}

/// Convenience helper: build a [`DisplayList`] from `layout_root` and rasterize
/// it onto a `width × height` [`Canvas`] cleared to `clear_color`.
pub fn paint(
    layout_root: &LayoutBox<'_>,
    width: usize,
    height: usize,
    clear_color: Color,
) -> Canvas {
    let display_list = build_display_list(layout_root);
    let mut canvas = Canvas::new(width, height, clear_color);
    canvas.paint_display_list(&display_list);
    canvas
}
