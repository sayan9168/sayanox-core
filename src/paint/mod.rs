//! Painting and display subsystem.
//!
//! Pipeline:
//! 1. [`display_list::build_display_list`] lowers a [`crate::layout::LayoutBox`]
//!    tree into back-to-front [`DisplayCommand`] primitives.
//! 2. [`rasterizer::Canvas`] executes those commands onto a CPU RGBA framebuffer.
//! 3. [`window::open_window`] presents the framebuffer inside a native OS window
//!    via `winit` and `softbuffer`.

pub mod display_list;
pub mod rasterizer;
pub mod window;

pub use display_list::{build_display_list, DisplayCommand, DisplayList};
pub use rasterizer::{paint, Canvas};
pub use window::{open_window, WindowConfig, WindowError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{layout_document, Rect};
    use crate::util::Color;
    use crate::{css, html, style};

    #[test]
    fn builds_display_list_and_rasterizes_boxes_and_borders() {
        let doc = html::parse(
            r#"
            <main id="card">
                <div class="swatch"></div>
            </main>
        "#,
        )
        .unwrap();

        let sheet = css::parse(
            r#"
            #card {
                width: 100px;
                margin: 10px;
                padding: 10px;
                border: 2px solid #ff0000;
                background-color: #1e293b;
            }
            .swatch {
                height: 30px;
                background-color: #38bdf8;
            }
        "#,
        )
        .unwrap();

        let styled = style::style_document(&doc, &sheet);
        let layout_root = layout_document(&styled, 200.0, 150.0);
        let display_list = build_display_list(&layout_root);

        // 1 card background + 4 card border strips + 1 inner swatch background = 6 commands
        assert_eq!(display_list.len(), 6);

        let canvas = paint(&layout_root, 200, 150, Color::WHITE);
        // Outside margin (5, 5) remains canvas clear color
        assert_eq!(canvas.pixel(5, 5), Some(Color::WHITE));
        // Card top border at (15, 10) is red
        assert_eq!(canvas.pixel(15, 10), Some(Color::rgb(255, 0, 0)));
        // Card padding area at (15, 15) is #1e293b
        assert_eq!(canvas.pixel(15, 15), Some(Color::rgb(0x1e, 0x29, 0x3b)));
        // Inner swatch at (30, 30) is #38bdf8
        assert_eq!(canvas.pixel(30, 30), Some(Color::rgb(0x38, 0xbd, 0xf8)));
    }

    #[test]
    fn clips_out_of_bounds_rectangles_and_alpha_blends() {
        let mut canvas = Canvas::new(20, 20, Color::rgb(0, 0, 255));
        // Negative origin and overflow past right/bottom edges should clip cleanly
        canvas.fill_rect(Rect::new(-10.0, -5.0, 20.0, 15.0), Color::rgb(255, 0, 0));
        assert_eq!(canvas.pixel(0, 0), Some(Color::rgb(255, 0, 0)));
        assert_eq!(canvas.pixel(9, 9), Some(Color::rgb(255, 0, 0)));
        assert_eq!(canvas.pixel(10, 10), Some(Color::rgb(0, 0, 255)));

        // Translucent overlay blends over existing red pixel
        canvas.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), Color::rgba(0, 255, 0, 128));
        assert_eq!(canvas.pixel(2, 2), Some(Color::rgb(127, 128, 0)));
    }
}
