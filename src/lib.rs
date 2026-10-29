//! Sayanox Core — minimal educational browser engine in Rust.
//!
//! Pipeline:
//! `HTML → DOM → CSS → Style Cascade → Block Layout → DisplayList → Software Rasterizer → Window`

pub mod css;
pub mod html;
pub mod layout;
pub mod paint;
pub mod style;
pub mod util;
