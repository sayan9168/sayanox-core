//! Cascade resolution and `ComputedStyle` calculation.

use super::matching::matching_rules;
use super::{ComputedStyle, Dimension, Display, EdgeSizes, MarginEdges, MarginValue};
use crate::css::{Color, Declaration, Stylesheet, Value};
use crate::html::dom::Element;

/// Compute the final [`ComputedStyle`] for `element` by starting from initial /
/// inherited defaults and applying matched stylesheet declarations in cascade order.
pub fn compute_element_style(
    element: &Element,
    ancestors: &[&Element],
    parent_style: Option<&ComputedStyle>,
    stylesheet: &Stylesheet,
) -> ComputedStyle {
    let mut style = ComputedStyle::initial_for_element(element, parent_style);
    let mut explicit_border_color = false;

    for matched in matching_rules(element, ancestors, stylesheet) {
        for decl in &matched.rule.declarations {
            apply_declaration(&mut style, decl, parent_style, &mut explicit_border_color);
        }
    }

    style
}

fn apply_declaration(
    style: &mut ComputedStyle,
    decl: &Declaration,
    parent: Option<&ComputedStyle>,
    explicit_border_color: &mut bool,
) {
    match decl.name.as_str() {
        "display" => {
            if let Value::Keyword(kw) = &decl.value {
                match kw.as_str() {
                    "block" => style.display = Display::Block,
                    "inline" => style.display = Display::Inline,
                    "none" => style.display = Display::None,
                    "inherit" => {
                        if let Some(p) = parent {
                            style.display = p.display;
                        }
                    }
                    _ => {}
                }
            }
        }

        "width" => {
            if is_inherit(&decl.value) {
                if let Some(p) = parent {
                    style.width = p.width;
                }
            } else if let Some(dim) = parse_dimension(&decl.value) {
                style.width = dim;
            }
        }

        "height" => {
            if is_inherit(&decl.value) {
                if let Some(p) = parent {
                    style.height = p.height;
                }
            } else if let Some(dim) = parse_dimension(&decl.value) {
                style.height = dim;
            }
        }

        "margin" => {
            if let Some(edges) = parse_margin_shorthand(&decl.value) {
                style.margin = edges;
            }
        }
        "margin-top" => {
            if let Some(v) = parse_margin_value(&decl.value) {
                style.margin.top = v;
            }
        }
        "margin-right" => {
            if let Some(v) = parse_margin_value(&decl.value) {
                style.margin.right = v;
            }
        }
        "margin-bottom" => {
            if let Some(v) = parse_margin_value(&decl.value) {
                style.margin.bottom = v;
            }
        }
        "margin-left" => {
            if let Some(v) = parse_margin_value(&decl.value) {
                style.margin.left = v;
            }
        }

        "padding" => {
            if let Some(edges) = parse_edge_shorthand(&decl.value) {
                style.padding = edges;
            }
        }
        "padding-top" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.padding.top = px;
            }
        }
        "padding-right" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.padding.right = px;
            }
        }
        "padding-bottom" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.padding.bottom = px;
            }
        }
        "padding-left" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.padding.left = px;
            }
        }

        "border-width" => {
            if let Some(edges) = parse_edge_shorthand(&decl.value) {
                style.border_width = edges;
            }
        }
        "border-top-width" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.border_width.top = px;
            }
        }
        "border-right-width" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.border_width.right = px;
            }
        }
        "border-bottom-width" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.border_width.bottom = px;
            }
        }
        "border-left-width" => {
            if let Some(px) = non_negative_px(&decl.value) {
                style.border_width.left = px;
            }
        }
        "border-color" => {
            if let Value::Color(color) = decl.value {
                style.border_color = color;
                *explicit_border_color = true;
            }
        }
        "border" => {
            for part in decl.value.as_slice() {
                match part {
                    Value::Keyword(kw) if kw == "none" => {
                        style.border_width = EdgeSizes::zero();
                    }
                    Value::Color(c) => {
                        style.border_color = *c;
                        *explicit_border_color = true;
                    }
                    other => {
                        if let Some(px) = non_negative_px(other) {
                            style.border_width = EdgeSizes::all(px);
                        }
                    }
                }
            }
        }

        "background-color" | "background" => {
            if is_inherit(&decl.value) {
                if let Some(p) = parent {
                    style.background_color = p.background_color;
                }
            } else if let Value::Color(color) = decl.value {
                style.background_color = color;
            }
        }

        "color" => {
            if is_inherit(&decl.value) {
                if let Some(p) = parent {
                    style.color = p.color;
                    if !*explicit_border_color {
                        style.border_color = p.color;
                    }
                }
            } else if let Value::Color(color) = decl.value {
                style.color = color;
                if !*explicit_border_color {
                    style.border_color = color;
                }
            }
        }

        "font-size" => {
            if is_inherit(&decl.value) {
                if let Some(p) = parent {
                    style.font_size = p.font_size;
                }
            } else if let Some(px) = non_negative_px(&decl.value) {
                style.font_size = px;
            }
        }

        // Unrecognized properties are ignored per CSS forward-compatibility rules.
        _ => {}
    }
}

fn is_inherit(value: &Value) -> bool {
    matches!(value, Value::Keyword(kw) if kw == "inherit")
}

fn parse_dimension(value: &Value) -> Option<Dimension> {
    match value {
        Value::Keyword(kw) if kw == "auto" => Some(Dimension::Auto),
        other => other.to_px().map(|px| Dimension::Px(px.max(0.0))),
    }
}

fn parse_margin_value(value: &Value) -> Option<MarginValue> {
    match value {
        Value::Keyword(kw) if kw == "auto" => Some(MarginValue::Auto),
        other => other.to_px().map(MarginValue::Px),
    }
}

fn non_negative_px(value: &Value) -> Option<f32> {
    value.to_px().map(|px| px.max(0.0))
}

fn parse_margin_shorthand(value: &Value) -> Option<MarginEdges> {
    let items: Vec<MarginValue> = value
        .as_slice()
        .iter()
        .map(parse_margin_value)
        .collect::<Option<Vec<_>>>()?;

    match items.as_slice() {
        [all] => Some(MarginEdges::all(*all)),
        [v, h] => Some(MarginEdges {
            top: *v,
            right: *h,
            bottom: *v,
            left: *h,
        }),
        [t, h, b] => Some(MarginEdges {
            top: *t,
            right: *h,
            bottom: *b,
            left: *h,
        }),
        [t, r, b, l] => Some(MarginEdges {
            top: *t,
            right: *r,
            bottom: *b,
            left: *l,
        }),
        _ => None,
    }
}

fn parse_edge_shorthand(value: &Value) -> Option<EdgeSizes> {
    let items: Vec<f32> = value
        .as_slice()
        .iter()
        .map(non_negative_px)
        .collect::<Option<Vec<_>>>()?;

    match items.as_slice() {
        [all] => Some(EdgeSizes::all(*all)),
        [v, h] => Some(EdgeSizes::symmetric(*v, *h)),
        [t, h, b] => Some(EdgeSizes {
            top: *t,
            right: *h,
            bottom: *b,
            left: *h,
        }),
        [t, r, b, l] => Some(EdgeSizes {
            top: *t,
            right: *r,
            bottom: *b,
            left: *l,
        }),
        _ => None,
    }
}

impl ComputedStyle {
    /// Construct initial CSS values for `element`, inheriting `color` and
    /// `font-size` from `parent` when available.
    pub fn initial_for_element(element: &Element, parent: Option<&ComputedStyle>) -> Self {
        let inherited_color = parent.map_or(Color::BLACK, |p| p.color);
        let inherited_font_size = parent.map_or(16.0, |p| p.font_size);

        Self {
            display: Display::default_for_tag(&element.tag_name),
            width: Dimension::Auto,
            height: Dimension::Auto,
            margin: MarginEdges::zero(),
            padding: EdgeSizes::zero(),
            border_width: EdgeSizes::zero(),
            border_color: inherited_color,
            background_color: Color::TRANSPARENT,
            color: inherited_color,
            font_size: inherited_font_size,
        }
    }

    /// Construct style for a DOM text node (inline flow, inheriting typography).
    pub fn for_text(parent: Option<&ComputedStyle>) -> Self {
        let inherited_color = parent.map_or(Color::BLACK, |p| p.color);
        let inherited_font_size = parent.map_or(16.0, |p| p.font_size);

        Self {
            display: Display::Inline,
            width: Dimension::Auto,
            height: Dimension::Auto,
            margin: MarginEdges::zero(),
            padding: EdgeSizes::zero(),
            border_width: EdgeSizes::zero(),
            border_color: inherited_color,
            background_color: Color::TRANSPARENT,
            color: inherited_color,
            font_size: inherited_font_size,
        }
    }
}
