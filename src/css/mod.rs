//! CSS subset: tokenizer, recursive-descent parser, and stylesheet AST.
//!
//! Supported features:
//! - Selectors: universal (`*`), type (`div`), class (`.card`), ID (`#hero`),
//!   compound combinations (`div#hero.card`), comma-separated selector lists,
//!   and descendant combinators (`main .card p`)
//! - Declarations: `property: value;` (single or multi-token shorthand values)
//! - Values: keywords, `px` lengths (plus unitless `0`), `#rgb` / `#rrggbb`
//!   hex colors, and CSS named colors

pub mod parser;
pub mod tokenizer;

pub use crate::util::Color;
pub use parser::{parse, ParseError};

use std::fmt;

/// A parsed CSS stylesheet containing rules in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// A CSS style rule pairing one or more selectors with declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// A selector chain ordered from outermost ancestor to the matched subject
/// element (e.g. `main .card p` has three [`SimpleSelector`] parts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<SimpleSelector>,
}

/// A compound selector matching a single element without combinators.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

/// Standard CSS selector specificity `(ids, classes, types)`.
///
/// Field declaration order gives lexicographic comparison matching CSS rules:
/// `types < classes < ids`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity {
    pub ids: u32,
    pub classes: u32,
    pub types: u32,
}

/// A single `property: value` pair inside a rule block.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

/// Supported CSS property values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    Color(Color),
    Number(f32),
    /// Space-separated value list (used by shorthands such as `margin: 8px 16px`).
    Multiple(Vec<Value>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Px,
}

impl Selector {
    pub fn simple(simple: SimpleSelector) -> Self {
        Self {
            parts: vec![simple],
        }
    }

    /// The rightmost compound selector that must match the target element itself.
    pub fn subject(&self) -> Option<&SimpleSelector> {
        self.parts.last()
    }

    /// Required ancestor compound selectors from outermost to innermost.
    pub fn ancestors(&self) -> &[SimpleSelector] {
        match self.parts.len() {
            0 => &[],
            n => &self.parts[..n - 1],
        }
    }

    /// Compute total `(ids, classes, types)` specificity across the selector chain.
    pub fn specificity(&self) -> Specificity {
        let mut spec = Specificity::default();
        for part in &self.parts {
            if part.id.is_some() {
                spec.ids += 1;
            }
            spec.classes += part.classes.len() as u32;
            if part.tag_name.is_some() {
                spec.types += 1;
            }
        }
        spec
    }
}

impl Value {
    /// Convert a length (or unitless `0`) to CSS pixels.
    pub fn to_px(&self) -> Option<f32> {
        match *self {
            Self::Length(px, Unit::Px) => Some(px),
            Self::Number(0.0) => Some(0.0),
            _ => None,
        }
    }

    /// View a single value as a 1-element slice or `Multiple` as its inner slice.
    pub fn as_slice(&self) -> &[Value] {
        match self {
            Self::Multiple(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        }
    }
}

impl fmt::Display for SimpleSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tag) = &self.tag_name {
            write!(f, "{tag}")?;
        } else if self.id.is_none() && self.classes.is_empty() {
            write!(f, "*")?;
        }
        if let Some(id) = &self.id {
            write!(f, "#{id}")?;
        }
        for class in &self.classes {
            write!(f, ".{class}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyword(kw) => write!(f, "{kw}"),
            Self::Length(v, Unit::Px) => write!(f, "{v}px"),
            Self::Color(c) => write!(f, "{c}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::Multiple(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{item}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Stylesheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, rule) in self.rules.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            for (j, sel) in rule.selectors.iter().enumerate() {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{sel}")?;
            }
            writeln!(f, " {{")?;
            for decl in &rule.declarations {
                writeln!(f, "  {}: {};", decl.name, decl.value)?;
            }
            writeln!(f, "}}")?;
        }
        Ok(())
    }
}
