//! Recursive-descent parser over a stream of owned tokens.

use super::dom::{Document, Element, Node};
use super::tokenizer::{Token, TokenizeError, Tokenizer};
use std::error::Error;
use std::fmt;
use std::vec::IntoIter;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Tokenize(TokenizeError),
    UnexpectedEndTag(String),
    MismatchedEndTag { expected: String, found: String },
    UnclosedTag(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tokenize(err) => write!(f, "{err}"),
            Self::UnexpectedEndTag(tag) => write!(f, "unexpected closing tag </{tag}>"),
            Self::MismatchedEndTag { expected, found } => write!(f, "expected </{expected}>, found </{found}>"),
            Self::UnclosedTag(tag) => write!(f, "unclosed tag <{tag}>"),
        }
    }
}
impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self { Self::Tokenize(err) => Some(err), _ => None }
    }
}
impl From<TokenizeError> for ParseError {
    fn from(err: TokenizeError) -> Self { Self::Tokenize(err) }
}

/// Parse a fragment or document. All top-level nodes are kept in order.
pub fn parse(input: &str) -> Result<Document, ParseError> {
    let tokens = Tokenizer::new(input).tokenize()?;
    let mut parser = Parser { tokens: tokens.into_iter() };
    Ok(Document { children: parser.children(None)? })
}

struct Parser { tokens: IntoIter<Token> }

impl Parser {
    fn children(&mut self, parent: Option<&str>) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        while let Some(token) = self.tokens.next() {
            match token {
                Token::Text(text) => nodes.push(Node::Text(text)),
                Token::Comment(comment) => nodes.push(Node::Comment(comment)),
                Token::EndTag { name } => match parent {
                    None => return Err(ParseError::UnexpectedEndTag(name)),
                    Some(expected) if name == expected => return Ok(nodes),
                    Some(expected) => return Err(ParseError::MismatchedEndTag { expected: expected.into(), found: name }),
                },
                Token::StartTag { name, attributes, self_closing } => {
                    let children = if self_closing || is_void(&name) { Vec::new() } else { self.children(Some(&name))? };
                    nodes.push(Node::Element(Element { tag_name: name, attributes, children }));
                }
            }
        }
        if let Some(tag) = parent { return Err(ParseError::UnclosedTag(tag.into())); }
        Ok(nodes)
    }
}

fn is_void(name: &str) -> bool {
    matches!(name, "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::dom::Attribute;

    #[test]
    fn builds_nested_tree_and_preserves_siblings() {
        let doc = parse("<main id=app>Hello <b>world</b><!--note--><br>!</main>").unwrap();
        assert_eq!(doc.children, vec![Node::Element(Element {
            tag_name: "main".into(), attributes: vec![Attribute { name: "id".into(), value: "app".into() }],
            children: vec![Node::Text("Hello ".into()), Node::Element(Element {
                tag_name: "b".into(), attributes: vec![], children: vec![Node::Text("world".into())],
            }), Node::Comment("note".into()), Node::Element(Element {
                tag_name: "br".into(), attributes: vec![], children: vec![],
            }), Node::Text("!".into())],
        })]);
    }

    #[test]
    fn reports_bad_nesting() {
        assert_eq!(parse("<a><b></a>"), Err(ParseError::MismatchedEndTag { expected: "b".into(), found: "a".into() }));
        assert_eq!(parse("<a>"), Err(ParseError::UnclosedTag("a".into())));
        assert_eq!(parse("</a>"), Err(ParseError::UnexpectedEndTag("a".into())));
    }

    #[test]
    fn parses_empty_and_multiple_roots() {
        assert!(parse("").unwrap().children.is_empty());
        assert_eq!(parse("<x/><y/>").unwrap().children.len(), 2);
    }
}
