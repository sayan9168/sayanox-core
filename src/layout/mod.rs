//! Block layout engine.
//!
//! Transforms a [`StyledDocument`] / [`StyledNode`] tree into a tree of
//! positioned [`LayoutBox`]es with resolved content, padding, border, and margin
//! rectangles.
//!
//! Only block formatting contexts (`display: block`) are implemented in this
//! stage; elements with `display: none` are omitted, and inline/text layout is
//! deferred to a future stage.

pub mod block;
pub mod box_model;

pub use box_model::{Dimensions, EdgeSizes, Rect};

use crate::style::{ComputedStyle, Display, StyledDocument, StyledNode};
use block::fmt_edges;
use std::fmt;

/// Classification of a [`LayoutBox`].
#[derive(Debug, Clone, PartialEq)]
pub enum BoxType<'a> {
    /// Synthetic root box spanning the viewport for full-document layout.
    Viewport,
    /// A block-level DOM element box.
    BlockNode(&'a StyledNode<'a>),
    /// Placeholder for anonymous block wrappers around inline content.
    AnonymousBlock,
}

/// A node in the layout box tree holding computed box-model geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox<'a> {
    pub box_type: BoxType<'a>,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox<'a>>,
}

impl<'a> LayoutBox<'a> {
    pub fn new(box_type: BoxType<'a>) -> Self {
        Self {
            box_type,
            dimensions: Dimensions::default(),
            children: Vec::new(),
        }
    }

    /// Return the underlying [`StyledNode`] if this box corresponds to an element.
    pub fn styled_node(&self) -> Option<&'a StyledNode<'a>> {
        match self.box_type {
            BoxType::BlockNode(node) => Some(node),
            BoxType::Viewport | BoxType::AnonymousBlock => None,
        }
    }

    /// Return the [`ComputedStyle`] governing this box, if any.
    pub fn style(&self) -> Option<&'a ComputedStyle> {
        self.styled_node().map(|n| &n.style)
    }

    /// Format this layout subtree as a human-readable multi-line string.
    pub fn format_tree(&self) -> String {
        let mut out = String::new();
        let _ = self.fmt_indented(&mut out, 0);
        out
    }

    fn fmt_indented(&self, f: &mut dyn fmt::Write, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        let label = match self.box_type {
            BoxType::Viewport => "<viewport>".to_owned(),
            BoxType::AnonymousBlock => "<anonymous-block>".to_owned(),
            BoxType::BlockNode(node) => match node.as_element() {
                Some(el) => {
                    let mut tag = format!("<{}", el.tag_name);
                    if let Some(id) = el.id() {
                        tag.push('#');
                        tag.push_str(id);
                    }
                    for cls in el.classes() {
                        tag.push('.');
                        tag.push_str(cls);
                    }
                    tag.push('>');
                    tag
                }
                None => "<text>".to_owned(),
            },
        };

        let d = &self.dimensions;
        write!(
            f,
            "{indent}Block {label} content={} border_box={}",
            d.content,
            d.border_box()
        )?;
        if !d.padding.is_zero() {
            write!(f, " padding={}", fmt_edges(d.padding))?;
        }
        if !d.border.is_zero() {
            write!(f, " border={}", fmt_edges(d.border))?;
        }
        if !d.margin.is_zero() {
            write!(f, " margin={}", fmt_edges(d.margin))?;
        }
        writeln!(f)?;

        for child in &self.children {
            child.fmt_indented(f, depth + 1)?;
        }
        Ok(())
    }
}

impl fmt::Display for LayoutBox<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indented(f, 0)
    }
}

/// Build and lay out a single root [`StyledNode`] inside `containing_block`.
///
/// Returns `None` if `root` has `display: none` or is not a block element.
pub fn layout_tree<'a>(
    root: &'a StyledNode<'a>,
    mut containing_block: Dimensions,
) -> Option<LayoutBox<'a>> {
    // Saved viewport height is not part of initial vertical flow accumulation.
    containing_block.content.height = 0.0;
    let mut root_box = build_layout_box(root)?;
    root_box.layout(containing_block);
    Some(root_box)
}

/// Build and lay out an entire [`StyledDocument`] inside a viewport of
/// `(viewport_width, viewport_height)` CSS pixels.
pub fn layout_document<'a>(
    document: &'a StyledDocument<'a>,
    viewport_width: f32,
    viewport_height: f32,
) -> LayoutBox<'a> {
    let mut viewport_box = LayoutBox::new(BoxType::Viewport);
    for node in &document.children {
        if let Some(child_box) = build_layout_box(node) {
            viewport_box.children.push(child_box);
        }
    }
    viewport_box.layout(Dimensions::viewport(viewport_width));
    if viewport_box.dimensions.content.height < viewport_height {
        viewport_box.dimensions.content.height = viewport_height;
    }
    viewport_box
}

/// Construct the unpositioned [`LayoutBox`] hierarchy for `node`.
pub fn build_layout_box<'a>(node: &'a StyledNode<'a>) -> Option<LayoutBox<'a>> {
    match node.style.display {
        Display::Block => {
            let mut layout_box = LayoutBox::new(BoxType::BlockNode(node));
            for child in &node.children {
                if let Some(child_box) = build_layout_box(child) {
                    layout_box.children.push(child_box);
                }
            }
            Some(layout_box)
        }
        // `display: none` skips the subtree; `display: inline` is omitted until
        // inline formatting context support is added.
        Display::Inline | Display::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html, style};

    #[test]
    fn stacks_blocks_and_applies_box_model_edges() {
        let doc = html::parse(
            r#"
            <main id="container">
                <div class="first"></div>
                <div class="second"></div>
                <div class="hidden"></div>
            </main>
        "#,
        )
        .unwrap();

        let sheet = css::parse(
            r#"
            #container {
                width: 500px;
                margin: 20px auto;
                padding: 16px;
                border: 2px solid #334155;
            }
            .first {
                height: 60px;
                padding: 10px;
                margin-bottom: 12px;
            }
            .second {
                height: 40px;
                padding: 8px 12px;
            }
            .hidden {
                display: none;
                height: 999px;
            }
        "#,
        )
        .unwrap();

        let styled = style::style_document(&doc, &sheet);
        let root = layout_tree(&styled.children[0], Dimensions::viewport(800.0)).unwrap();

        // Container: border_box width = 500 + 32 (padding) + 4 (border) = 536.
        // Remaining space in 800px viewport = 264 -> centered margins = 132 left, 132 right.
        assert_eq!(root.dimensions.content.width, 500.0);
        assert_eq!(root.dimensions.margin.left, 132.0);
        assert_eq!(root.dimensions.margin.right, 132.0);
        assert_eq!(root.dimensions.content.x, 132.0 + 2.0 + 16.0);
        assert_eq!(root.dimensions.content.y, 20.0 + 2.0 + 16.0);

        // `.hidden` (`display: none`) is excluded from the layout tree.
        assert_eq!(root.children.len(), 2);

        let first = &root.children[0];
        // Auto width fills container content width (500) minus horizontal padding (20) = 480.
        assert_eq!(first.dimensions.content.width, 480.0);
        assert_eq!(first.dimensions.content.height, 60.0);
        assert_eq!(first.dimensions.border_box(), Rect::new(150.0, 38.0, 500.0, 80.0));
        assert_eq!(first.dimensions.margin_box().height, 92.0);

        let second = &root.children[1];
        // Positioned directly below `first.margin_box()` (38 + 92 = 130 border_box.y -> +8 top padding = 138).
        assert_eq!(second.dimensions.content.width, 476.0);
        assert_eq!(second.dimensions.content.y, 138.0);
        assert_eq!(second.dimensions.border_box(), Rect::new(150.0, 130.0, 500.0, 56.0));

        // Container auto height = 92 + 56 = 148.
        assert_eq!(root.dimensions.content.height, 148.0);
        assert_eq!(root.dimensions.border_box(), Rect::new(132.0, 20.0, 536.0, 184.0));
    }

    #[test]
    fn formats_layout_tree_for_inspection() {
        let doc = html::parse(r#"<div id="root"><div class="child"></div></div>"#).unwrap();
        let sheet = css::parse("#root { width: 200px; padding: 10px; } .child { height: 30px; }")
            .unwrap();
        let styled = style::style_document(&doc, &sheet);
        let root = layout_tree(&styled.children[0], Dimensions::viewport(400.0)).unwrap();

        let printed = root.format_tree();
        assert!(printed.contains("Block <div#root>"));
        assert!(printed.contains("Block <div.child>"));
        assert!(printed.contains("padding=(10, 10, 10, 10)"));
    }
}
