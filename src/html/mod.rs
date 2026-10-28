//! A small, deliberately strict HTML subset.
//!
//! This is not yet the HTML5 parsing algorithm: character references, raw-text
//! elements, implied tags, and browser-style error recovery are not supported.

pub mod dom;
pub mod parser;
pub mod tokenizer;

pub use parser::parse;
