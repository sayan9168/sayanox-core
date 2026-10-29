//! End-to-end integration tests for the Sayanox Core rendering pipeline:
//! HTML -> DOM -> CSS -> Style Cascade -> Block Layout -> DisplayList -> Rasterizer.

use sayanox_core::util::Color;
use sayanox_core::{css, html, layout, paint, style};

#[test]
fn full_pipeline_renders_centered_blocks_and_respects_cascade() {
    let html = r#"
        <main id="shell" class="box">
            <div class="header"></div>
            <div class="panel">
                <div class="swatch"></div>
            </div>
            <div class="invisible"></div>
        </main>
    "#;

    let css = r#"
        .box {
            background-color: #111111;
            width: 300px;
        }
        #shell {
            width: 200px;
            margin: 20px auto;
            padding: 10px;
            background-color: #223344;
            border: 2px solid #ffffff;
        }
        .header {
            height: 20px;
            margin-bottom: 10px;
            background-color: red;
        }
        #shell .panel {
            padding: 5px;
            background-color: #445566;
        }
        #shell .panel .swatch {
            width: 40px;
            height: 15px;
            background-color: #0f0;
        }
        .invisible {
            display: none;
            height: 200px;
            background-color: yellow;
        }
    "#;

    let doc = html::parse(html).unwrap();
    let sheet = css::parse(css).unwrap();
    let styled = style::style_document(&doc, &sheet);

    // Viewport 400x200:
    // #shell border_box width = 200 + 20 (padding) + 4 (border) = 224.
    // Centered horizontal margins = (400 - 224) / 2 = 88 -> border_box x = 88, y = 20.
    let layout_root = layout::layout_document(&styled, 400.0, 200.0);
    let canvas = paint::paint(&layout_root, 400, 200, Color::BLACK);

    // Viewport margin area stays black
    assert_eq!(canvas.pixel(10, 10), Some(Color::BLACK));

    // Top-left corner of #shell border at (88, 20) is white
    assert_eq!(canvas.pixel(88, 20), Some(Color::WHITE));

    // Inside #shell top padding at (95, 25) is #223344 (ID selector beat .box)
    assert_eq!(canvas.pixel(95, 25), Some(Color::rgb(0x22, 0x33, 0x44)));

    // .header starts at content origin (88 + 2 + 10 = 100, 20 + 2 + 10 = 32)
    assert_eq!(canvas.pixel(105, 40), Some(Color::rgb(255, 0, 0)));

    // .panel starts at y = 32 + 20 + 10 = 62; .swatch starts at (100 + 5, 62 + 5) = (105, 67)
    assert_eq!(canvas.pixel(110, 70), Some(Color::rgb(0, 255, 0)));
    // Right of .swatch inside .panel is #445566
    assert_eq!(canvas.pixel(160, 70), Some(Color::rgb(0x44, 0x55, 0x66)));

    // Packed softbuffer output matches expected 0x00RRGGBB values
    let packed = canvas.to_0rgb_vec();
    assert_eq!(packed.len(), 400 * 200);
    assert_eq!(packed[70 * 400 + 110], 0x0000_ff00);
}
