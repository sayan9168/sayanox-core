//! Style cascade & computed values.
//!
//! Matches CSS selectors against DOM elements, resolves declarations by
//! `(specificity, source_order)`, propagates inherited properties (`color`,
//! `font-size`), and produces a [`StyledDocument`] / [`StyledNode`] tree ready
//! for layout.

pub mod cascade;
pub mod matching;

pub use crate::util::Color;
pub use cascade::compute_element_style;
pub use matching::{matches_selector, matches_simple_selector, matching_rules};

use crate::css::Stylesheet;
use crate::html::dom::{Document, Element, Node};
use std::fmt;

/// CSS `display` modes supported by Sayanox Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    None,
}

impl Display {
    /// Default UA `display` value for common HTML tags.
    pub fn default_for_tag(tag_name: &str) -> Self {
        match tag_name {
            "head" | "style" | "script" | "meta" | "link" | "title" => Self::None,
            "span" | "a" | "b" | "strong" | "i" | "em" | "u" | "small" | "code" | "label"
            | "br" | "img" => Self::Inline,
            _ => Self::Block,
        }
    }
}

/// Width / height sizing value (`auto` or fixed `px`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Px(f32),
}

/// Top, right, bottom, left `px` lengths (used for padding and border widths,
/// and resolved margins during layout).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    pub const fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub const fn all(px: f32) -> Self {
        Self {
            top: px,
            right: px,
            bottom: px,
            left: px,
        }
    }

    pub const fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }
}

/// Margin value for a single edge (`auto` or signed `px`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginValue {
    Auto,
    Px(f32),
}

impl MarginValue {
    pub fn px_or_zero(self) -> f32 {
        match self {
            Self::Auto => 0.0,
            Self::Px(px) => px,
        }
    }
}

/// Four-edge CSS `margin` values prior to horizontal `auto` resolution in layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarginEdges {
    pub top: MarginValue,
    pub right: MarginValue,
    pub bottom: MarginValue,
    pub left: MarginValue,
}

impl MarginEdges {
    pub const fn zero() -> Self {
        Self::all(MarginValue::Px(0.0))
    }

    pub const fn all(val: MarginValue) -> Self {
        Self {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }
}

impl Default for MarginEdges {
    fn default() -> Self {
        Self::zero()
    }
}

/// Resolved CSS properties for a single DOM node.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub width: Dimension,
    pub height: Dimension,
    pub margin: MarginEdges,
    pub padding: EdgeSizes,
    pub border_width: EdgeSizes,
    pub border_color: Color,
    pub background_color: Color,
    pub color: Color,
    pub font_size: f32,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            width: Dimension::Auto,
            height: Dimension::Auto,
            margin: MarginEdges::zero(),
            padding: EdgeSizes::zero(),
            border_width: EdgeSizes::zero(),
            border_color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            color: Color::BLACK,
            font_size: 16.0,
        }
    }
}

/// A styled DOM document containing all top-level styled nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledDocument<'a> {
    pub children: Vec<StyledNode<'a>>,
}

/// Payload of a [`StyledNode`] (comments are stripped during styling).
#[derive(Debug, Clone, PartialEq)]
pub enum StyledNodeKind<'a> {
    Element(&'a Element),
    Text(&'a str),
}

/// A DOM node paired with its [`ComputedStyle`] and styled children.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode<'a> {
    pub kind: StyledNodeKind<'a>,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode<'a>>,
}

impl<'a> StyledNode<'a> {
    pub fn as_element(&self) -> Option<&'a Element> {
        match self.kind {
            StyledNodeKind::Element(el) => Some(el),
            StyledNodeKind::Text(_) => None,
        }
    }
}

/// Style an entire [`Document`] against a [`Stylesheet`].
pub fn style_document<'a>(document: &'a Document, stylesheet: &Stylesheet) -> StyledDocument<'a> {
    let mut ancestors = Vec::new();
    let children = style_nodes(&document.children, &mut ancestors, None, stylesheet);
    StyledDocument { children }
}

/// Style a single root [`Element`] against a [`Stylesheet`].
pub fn style_element<'a>(element: &'a Element, stylesheet: &Stylesheet) -> StyledNode<'a> {
    let mut ancestors = Vec::new();
    style_element_inner(element, &mut ancestors, None, stylesheet)
}

fn style_nodes<'a>(
    nodes: &'a [Node],
    ancestors: &mut Vec<&'a Element>,
    parent_style: Option<&ComputedStyle>,
    stylesheet: &Stylesheet,
) -> Vec<StyledNode<'a>> {
    let mut styled = Vec::new();
    for node in nodes {
        match node {
            Node::Element(el) => {
                styled.push(style_element_inner(el, ancestors, parent_style, stylesheet));
            }
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    styled.push(StyledNode {
                        kind: StyledNodeKind::Text(text.as_str()),
                        style: ComputedStyle::for_text(parent_style),
                        children: Vec::new(),
                    });
                }
            }
            Node::Comment(_) => {}
        }
    }
    styled
}

fn style_element_inner<'a>(
    element: &'a Element,
    ancestors: &mut Vec<&'a Element>,
    parent_style: Option<&ComputedStyle>,
    stylesheet: &Stylesheet,
) -> StyledNode<'a> {
    let style = compute_element_style(element, ancestors, parent_style, stylesheet);
    ancestors.push(element);
    let children = style_nodes(&element.children, ancestors, Some(&style), stylesheet);
    ancestors.pop();

    StyledNode {
        kind: StyledNodeKind::Element(element),
        style,
        children,
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Px(px) => write!(f, "{px}px"),
        }
    }
}

impl fmt::Display for MarginValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Px(px) => write!(f, "{px}px"),
        }
    }
}

impl fmt::Display for StyledDocument<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in &self.children {
            child.fmt_indented(f, 0)?;
        }
        Ok(())
    }
}

impl fmt::Display for StyledNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indented(f, 0)
    }
}

impl StyledNode<'_> {
    fn fmt_indented(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        match self.kind {
            StyledNodeKind::Element(el) => {
                write!(f, "{indent}<{}", el.tag_name)?;
                if let Some(id) = el.id() {
                    write!(f, "#{id}")?;
                }
                for cls in el.classes() {
                    write!(f, ".{cls}")?;
                }
                let s = &self.style;
                writeln!(
                    f,
                    "> display={:?} size=({}, {}) bg={} color={} font={}px",
                    s.display, s.width, s.height, s.background_color, s.color, s.font_size
                )?;
                for child in &self.children {
                    child.fmt_indented(f, depth + 1)?;
                }
                Ok(())
            }
            StyledNodeKind::Text(text) => {
                writeln!(
                    f,
                    "{indent}#text {:?} (color={}, font={}px)",
                    text.trim(),
                    self.style.color,
                    self.style.font_size
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html};

    #[test]
    fn resolves_specificity_type_less_than_class_less_than_id() {
        let doc = html::parse(r#"<div id="hero" class="card"></div>"#).unwrap();
        let sheet = css::parse(
            r#"
            #hero { background-color: #ff0000; width: 500px; }
            .card { background-color: #00ff00; width: 300px; height: 120px; }
            div   { background-color: #0000ff; width: 100px; height: 80px; padding: 12px; }
        "#,
        )
        .unwrap();

        let styled = style_document(&doc, &sheet);
        let root_style = &styled.children[0].style;

        // #hero beats .card and div regardless of source order
        assert_eq!(root_style.background_color, Color::rgb(255, 0, 0));
        assert_eq!(root_style.width, Dimension::Px(500.0));
        // .card beats div for height
        assert_eq!(root_style.height, Dimension::Px(120.0));
        // div sets padding because higher-specificity rules did not override it
        assert_eq!(root_style.padding, EdgeSizes::all(12.0));
    }

    #[test]
    fn breaks_equal_specificity_ties_by_source_order() {
        let doc = html::parse(r#"<p class="a b"></p>"#).unwrap();
        let sheet = css::parse(
            r#"
            .a { color: red; margin: 10px 20px; }
            .b { color: blue; margin-top: 30px; }
        "#,
        )
        .unwrap();

        let styled = style_document(&doc, &sheet);
        let s = &styled.children[0].style;
        assert_eq!(s.color, Color::rgb(0, 0, 255));
        assert_eq!(
            s.margin,
            MarginEdges {
                top: MarginValue::Px(30.0),
                right: MarginValue::Px(20.0),
                bottom: MarginValue::Px(10.0),
                left: MarginValue::Px(20.0),
            }
        );
    }

    #[test]
    fn matches_descendant_selectors_and_inherits_typography() {
        let doc = html::parse(
            r#"
            <main id="app">
                <section class="panel">
                    <p>Styled inside panel</p>
                </section>
                <p>Outside panel</p>
            </main>
        "#,
        )
        .unwrap();

        let sheet = css::parse(
            r#"
            #app {
                color: #e2e8f0;
                font-size: 18px;
                background-color: #0f172a;
            }
            main .panel p {
                background-color: #3b82f6;
                padding: 8px 16px;
            }
        "#,
        )
        .unwrap();

        let styled = style_document(&doc, &sheet);
        let app = &styled.children[0];
        let panel = &app.children[0];
        let inner_p = &panel.children[0];
        let outer_p = &app.children[1];

        // Inherited color & font-size propagate down to both <p> elements
        assert_eq!(inner_p.style.color, Color::rgb(0xe2, 0xe8, 0xf0));
        assert_eq!(inner_p.style.font_size, 18.0);
        assert_eq!(outer_p.style.color, Color::rgb(0xe2, 0xe8, 0xf0));
        assert_eq!(outer_p.style.font_size, 18.0);

        // Non-inherited background-color does not leak from #app, while
        // `main .panel p` matches only the inner <p>.
        assert_eq!(panel.style.background_color, Color::TRANSPARENT);
        assert_eq!(inner_p.style.background_color, Color::rgb(0x3b, 0x82, 0xf6));
        assert_eq!(inner_p.style.padding, EdgeSizes::symmetric(8.0, 16.0));
        assert_eq!(outer_p.style.background_color, Color::TRANSPARENT);
    }
}
