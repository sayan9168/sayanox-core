//! Sayanox Core — Minimal browser engine.

mod css;
mod html;
mod layout;
mod paint;
mod style;
mod util;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = r#"<main id="welcome"><h1>Hello, Sayanox!</h1><p>A tiny browser engine.<br>Written in Rust.</p><!-- More to come --></main>"#;
    let document = html::parse(html)?;
    println!("{document:#?}");
    Ok(())
}
