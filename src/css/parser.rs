//! Recursive-descent parser for our CSS subset.
//!
//! Grammar overview:
//! - `stylesheet := rule*`
//! - `rule       := selector (',' selector)* '{' declaration* '}'`
//! - `selector   := simple_selector (WHITESPACE simple_selector)*`
//! - `simple     := (IDENT | '*')? ('.' IDENT | '#' IDENT)*` (at least one part)
//! - `decl       := IDENT ':' value+ ';'?`

use super::tokenizer::{Token, TokenizeError, Tokenizer};
use super::{Color, Declaration, Rule, Selector, SimpleSelector, Stylesheet, Unit, Value};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Tokenize(TokenizeError),
    UnexpectedEof(&'static str),
    UnexpectedToken { expected: &'static str, found: String },
    InvalidColor(String),
    UnsupportedUnit(String),
    InvalidSelector(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tokenize(err) => write!(f, "{err}"),
            Self::UnexpectedEof(expected) => {
                write!(f, "unexpected end of CSS input while expecting {expected}")
            }
            Self::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            Self::InvalidColor(raw) => write!(f, "invalid CSS color literal #{raw}"),
            Self::UnsupportedUnit(unit) => write!(f, "unsupported CSS length unit {unit:?}"),
            Self::InvalidSelector(msg) => write!(f, "invalid CSS selector: {msg}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Tokenize(err) => Some(err),
            _ => None,
        }
    }
}

impl From<TokenizeError> for ParseError {
    fn from(err: TokenizeError) -> Self {
        Self::Tokenize(err)
    }
}

/// Parse a CSS stylesheet string into rules, selectors, and declarations.
pub fn parse(input: &str) -> Result<Stylesheet, ParseError> {
    let tokens = Tokenizer::new(input).tokenize()?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_stylesheet()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse_stylesheet(&mut self) -> Result<Stylesheet, ParseError> {
        let mut rules = Vec::new();
        self.skip_whitespace();
        while self.peek().is_some() {
            rules.push(self.parse_rule()?);
            self.skip_whitespace();
        }
        Ok(Stylesheet { rules })
    }

    fn parse_rule(&mut self) -> Result<Rule, ParseError> {
        let selectors = self.parse_selector_list()?;
        self.expect_lbrace()?;
        let declarations = self.parse_declarations()?;
        Ok(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selector_list(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();
        loop {
            self.skip_whitespace();
            selectors.push(self.parse_selector()?);
            self.skip_whitespace();
            match self.peek() {
                Some(Token::Comma) => {
                    self.pos += 1;
                }
                Some(Token::LBrace) => break,
                Some(other) => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or '{'",
                        found: format!("{other:?}"),
                    });
                }
                None => return Err(ParseError::UnexpectedEof("',' or '{'")),
            }
        }
        Ok(selectors)
    }

    /// Parse one selector, which may be a single `SimpleSelector` or a chain of
    /// descendant-combined `SimpleSelector`s separated by whitespace.
    fn parse_selector(&mut self) -> Result<Selector, ParseError> {
        let mut parts = vec![self.parse_simple_selector()?];
        while matches!(self.peek(), Some(Token::Whitespace)) {
            self.skip_whitespace();
            if matches!(self.peek(), Some(Token::Comma | Token::LBrace) | None) {
                break;
            }
            parts.push(self.parse_simple_selector()?);
        }
        Ok(Selector { parts })
    }

    fn parse_simple_selector(&mut self) -> Result<SimpleSelector, ParseError> {
        let mut selector = SimpleSelector::default();
        let mut matched_any = false;

        // Optional leading type selector (`div`) or universal selector (`*`).
        match self.peek() {
            Some(Token::Ident(_)) => {
                if let Some(Token::Ident(name)) = self.next() {
                    selector.tag_name = Some(name.to_ascii_lowercase());
                    matched_any = true;
                }
            }
            Some(Token::Star) => {
                self.pos += 1;
                matched_any = true;
            }
            _ => {}
        }

        // Zero or more `#id` and `.class` qualifiers attached with no whitespace.
        loop {
            match self.peek() {
                Some(Token::Hash(_)) => {
                    let Some(Token::Hash(id)) = self.next() else {
                        unreachable!()
                    };
                    if selector.id.is_some() {
                        return Err(ParseError::InvalidSelector(
                            "multiple ID selectors on the same compound selector".into(),
                        ));
                    }
                    selector.id = Some(id);
                    matched_any = true;
                }
                Some(Token::Dot) => {
                    self.pos += 1;
                    match self.next() {
                        Some(Token::Ident(class_name)) => {
                            selector.classes.push(class_name);
                            matched_any = true;
                        }
                        Some(other) => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "class name after '.'",
                                found: format!("{other:?}"),
                            });
                        }
                        None => return Err(ParseError::UnexpectedEof("class name after '.'")),
                    }
                }
                _ => break,
            }
        }

        if !matched_any {
            return match self.peek() {
                Some(other) => Err(ParseError::UnexpectedToken {
                    expected: "selector (tag, .class, #id, or *)",
                    found: format!("{other:?}"),
                }),
                None => Err(ParseError::UnexpectedEof("selector")),
            };
        }

        Ok(selector)
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(Token::RBrace) => {
                    self.pos += 1;
                    break;
                }
                Some(Token::Semicolon) => {
                    // Tolerate extra semicolons inside declaration blocks.
                    self.pos += 1;
                }
                Some(_) => {
                    declarations.push(self.parse_declaration()?);
                }
                None => return Err(ParseError::UnexpectedEof("'}' to close declaration block")),
            }
        }
        Ok(declarations)
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        let name = match self.next() {
            Some(Token::Ident(name)) => name.to_ascii_lowercase(),
            Some(other) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "CSS property name",
                    found: format!("{other:?}"),
                });
            }
            None => return Err(ParseError::UnexpectedEof("CSS property name")),
        };

        self.skip_whitespace();
        match self.next() {
            Some(Token::Colon) => {}
            Some(other) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "':' after property name",
                    found: format!("{other:?}"),
                });
            }
            None => return Err(ParseError::UnexpectedEof("':' after property name")),
        }

        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(Token::Semicolon) => {
                    self.pos += 1;
                    break;
                }
                Some(Token::RBrace) => {
                    // Allow omitting the final semicolon before '}'.
                    break;
                }
                None => return Err(ParseError::UnexpectedEof("';' or '}' after property value")),
                Some(_) => values.push(self.parse_single_value()?),
            }
        }

        if values.is_empty() {
            return Err(ParseError::UnexpectedToken {
                expected: "property value",
                found: "empty declaration value".into(),
            });
        }

        let value = if values.len() == 1 {
            values.pop().unwrap()
        } else {
            Value::Multiple(values)
        };

        Ok(Declaration { name, value })
    }

    fn parse_single_value(&mut self) -> Result<Value, ParseError> {
        match self.next() {
            Some(Token::Dimension { value, unit }) => match unit.as_str() {
                "px" => Ok(Value::Length(value, Unit::Px)),
                other => Err(ParseError::UnsupportedUnit(other.to_owned())),
            },
            Some(Token::Number(n)) => {
                if n == 0.0 {
                    Ok(Value::Length(0.0, Unit::Px))
                } else {
                    Ok(Value::Number(n))
                }
            }
            Some(Token::Hash(hex)) => Color::from_hex(&hex)
                .map(Value::Color)
                .ok_or(ParseError::InvalidColor(hex)),
            Some(Token::Ident(ident)) => {
                let lower = ident.to_ascii_lowercase();
                if let Some(color) = Color::from_name(&lower) {
                    Ok(Value::Color(color))
                } else {
                    Ok(Value::Keyword(lower))
                }
            }
            Some(other) => Err(ParseError::UnexpectedToken {
                expected: "CSS value (keyword, px length, or color)",
                found: format!("{other:?}"),
            }),
            None => Err(ParseError::UnexpectedEof("CSS value")),
        }
    }

    fn expect_lbrace(&mut self) -> Result<(), ParseError> {
        self.skip_whitespace();
        match self.next() {
            Some(Token::LBrace) => Ok(()),
            Some(other) => Err(ParseError::UnexpectedToken {
                expected: "'{'",
                found: format!("{other:?}"),
            }),
            None => Err(ParseError::UnexpectedEof("'{'")),
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(Token::Whitespace)) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_type_class_id_and_descendant_selectors() {
        let css = r#"
            main#welcome {
                background-color: #1e293b;
                padding: 16px 24px;
            }
            main .card p, h1.hero {
                color: white;
                font-size: 18px;
                margin: 0
            }
        "#;

        let sheet = parse(css).unwrap();
        assert_eq!(sheet.rules.len(), 2);

        let first = &sheet.rules[0];
        assert_eq!(
            first.selectors,
            vec![Selector {
                parts: vec![SimpleSelector {
                    tag_name: Some("main".into()),
                    id: Some("welcome".into()),
                    classes: vec![],
                }],
            }]
        );
        assert_eq!(
            first.declarations,
            vec![
                Declaration {
                    name: "background-color".into(),
                    value: Value::Color(Color::rgb(0x1e, 0x29, 0x3b)),
                },
                Declaration {
                    name: "padding".into(),
                    value: Value::Multiple(vec![
                        Value::Length(16.0, Unit::Px),
                        Value::Length(24.0, Unit::Px),
                    ]),
                },
            ]
        );

        let second = &sheet.rules[1];
        assert_eq!(second.selectors.len(), 2);
        assert_eq!(second.selectors[0].parts.len(), 3);
        assert_eq!(second.selectors[0].parts[0].tag_name.as_deref(), Some("main"));
        assert_eq!(second.selectors[0].parts[1].classes, vec!["card"]);
        assert_eq!(second.selectors[0].parts[2].tag_name.as_deref(), Some("p"));

        assert_eq!(
            second.declarations,
            vec![
                Declaration {
                    name: "color".into(),
                    value: Value::Color(Color::WHITE),
                },
                Declaration {
                    name: "font-size".into(),
                    value: Value::Length(18.0, Unit::Px),
                },
                Declaration {
                    name: "margin".into(),
                    value: Value::Length(0.0, Unit::Px),
                },
            ]
        );
    }

    #[test]
    fn parses_short_hex_named_colors_and_keywords() {
        let sheet = parse(".box { display: block; color: #f80; background-color: navy; }").unwrap();
        let decls = &sheet.rules[0].declarations;
        assert_eq!(decls[0].value, Value::Keyword("block".into()));
        assert_eq!(decls[1].value, Value::Color(Color::rgb(0xff, 0x88, 0x00)));
        assert_eq!(decls[2].value, Value::Color(Color::rgb(0, 0, 128)));
    }

    #[test]
    fn computes_specificity_ordering() {
        let sheet = parse("p {} .card {} #hero {} main .card p {}").unwrap();
        let specs: Vec<_> = sheet.rules.iter().map(|r| r.selectors[0].specificity()).collect();
        assert!(specs[0] < specs[1]); // type < class
        assert!(specs[1] < specs[2]); // class < id
        assert_eq!(specs[3].ids, 0);
        assert_eq!(specs[3].classes, 1);
        assert_eq!(specs[3].types, 2);
    }

    #[test]
    fn rejects_invalid_css() {
        for bad in ["div { color: #12; }", "div { width: 10em; }", "div { color: ; }", ". { }"] {
            assert!(parse(bad).is_err(), "expected error for: {bad}");
        }
    }
}
