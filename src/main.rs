//! Sayanox Core — Minimal browser engine demo.
//!
//! Parses a small HTML + CSS page, runs style cascade and block layout, prints
//! pipeline diagnostics, rasterizes an initial frame, and opens a `winit` +
//! `softbuffer` window to display the rendered result (pass `--headless` to
//! skip opening a native window in headless environments).

use sayanox_core::paint::{self, WindowConfig};
use sayanox_core::util::Color;
use sayanox_core::{css, html, layout, style};

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = r#"
        <!-- Sayanox Core example page -->
        <main id="page">
            <header id="topbar" class="banner">
                <div class="brand-pill"></div>
            </header>

            <section class="hero card">
                <div class="accent"></div>
                <div class="headline-bar"></div>
                <div class="subtitle-bar"></div>
            </section>

            <section class="card">
                <div class="accent warm"></div>
                <div class="content-block"></div>
                <div class="content-block muted"></div>
            </section>

            <footer id="footer"></footer>
            <div class="hidden"></div>
        </main>
    "#;

    let css = r#"
        main#page {
            width: 640px;
            margin: 28px auto;
            padding: 24px;
            background-color: #0f172a;
            border: 2px solid #334155;
            color: white;
            font-size: 16px;
        }

        #topbar.banner {
            padding: 16px 20px;
            margin-bottom: 20px;
            background-color: #1d4ed8;
            border: 1px solid #3b82f6;
        }

        #topbar .brand-pill {
            width: 140px;
            height: 18px;
            background-color: #93c5fd;
        }

        .card {
            padding: 20px;
            margin-bottom: 20px;
            background-color: #1e293b;
            border: 1px solid #475569;
        }

        .hero {
            background-color: #172554;
            border-color: #2563eb;
        }

        .card .accent {
            width: 96px;
            height: 8px;
            margin-bottom: 14px;
            background-color: #38bdf8;
        }

        .card .accent.warm {
            background-color: #f59e0b;
        }

        .hero .headline-bar {
            width: 360px;
            height: 24px;
            margin-bottom: 10px;
            background-color: #f8fafc;
        }

        .hero .subtitle-bar {
            width: 480px;
            height: 14px;
            background-color: #94a3b8;
        }

        .card .content-block {
            height: 36px;
            margin-bottom: 12px;
            background-color: #334155;
        }

        .card .content-block.muted {
            width: 420px;
            height: 24px;
            margin-bottom: 0;
            background-color: #475569;
        }

        #footer {
            height: 28px;
            background-color: #1e293b;
            border: 1px solid #334155;
        }

        .hidden {
            display: none;
            height: 500px;
            background-color: red;
        }
    "#;

    // 1. HTML -> DOM
    let document = html::parse(html)?;

    // 2. CSS -> Stylesheet
    let stylesheet = css::parse(css)?;
    println!("=== Stylesheet ({} rules) ===", stylesheet.rules.len());
    println!("{stylesheet}");

    // 3. DOM + Stylesheet -> Styled DOM (Cascade + Specificity + Inheritance)
    let styled = style::style_document(&document, &stylesheet);
    println!("=== Computed Styles ===");
    println!("{styled}");

    // 4. Styled DOM -> Block Layout Tree
    let layout_root =
        layout::layout_document(&styled, INITIAL_WIDTH as f32, INITIAL_HEIGHT as f32);
    println!("=== Layout Tree ({INITIAL_WIDTH}x{INITIAL_HEIGHT}) ===");
    println!("{layout_root}");

    // 5. Layout Tree -> DisplayList -> Software Rasterizer
    let display_list = paint::build_display_list(&layout_root);
    let viewport_bg = Color::rgb(0x02, 0x06, 0x17);
    let canvas = paint::paint(
        &layout_root,
        INITIAL_WIDTH as usize,
        INITIAL_HEIGHT as usize,
        viewport_bg,
    );
    println!(
        "=== Paint Summary: {} display commands rasterized onto {}x{} canvas ===",
        display_list.len(),
        canvas.width,
        canvas.height,
    );

    if std::env::args().any(|arg| arg == "--headless") {
        println!("Headless mode enabled; skipping native window creation.");
        return Ok(());
    }

    // 6. Open window via winit + softbuffer and render interactively
    paint::open_window(
        &styled,
        WindowConfig {
            title: "Sayanox Core — Minimal Browser Engine".to_owned(),
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
            background: viewport_bg,
        },
    )?;

    Ok(())
}
