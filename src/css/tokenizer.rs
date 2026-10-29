//! Lexical analysis for our CSS subset.
//!
//! Whitespace is emitted as a dedicated [`Token::Whitespace`] token (with runs
//! of whitespace collapsed) because CSS selector grammar is whitespace-sensitive:
//! `div.card` is a single compound selector while `div .card` is a descendant
//! selector. Comments (`/* ... */`) are stripped during tokenization.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Hash(String),
    Number(f32),
    Dimension { value: f32, unit: String },
    Whitespace,
    Dot,
    Star,
    Comma,
    Colon,
    Semicolon,
    LBrace,
    RBrace,
}

/// Byte offset and message for a CSS lexical error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CSS tokenization error at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl Error for TokenizeError {}

pub struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, TokenizeError> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            if self.rest().starts_with("/*") {
                self.pos += 2;
                let end = self
                    .rest()
                    .find("*/")
                    .ok_or_else(|| self.error("unterminated CSS comment"))?;
                self.pos += end + 2;
            } else if ch.is_ascii_whitespace() {
                self.skip_whitespace();
                if !matches!(tokens.last(), Some(Token::Whitespace)) {
                    tokens.push(Token::Whitespace);
                }
            } else if ch == '{' {
                self.pos += 1;
                tokens.push(Token::LBrace);
            } else if ch == '}' {
                self.pos += 1;
                tokens.push(Token::RBrace);
            } else if ch == ':' {
                self.pos += 1;
                tokens.push(Token::Colon);
            } else if ch == ';' {
                self.pos += 1;
                tokens.push(Token::Semicolon);
            } else if ch == ',' {
                self.pos += 1;
                tokens.push(Token::Comma);
            } else if ch == '*' {
                self.pos += 1;
                tokens.push(Token::Star);
            } else if ch == '#' {
                self.pos += 1;
                let name = self.consume_hash_chars()?;
                tokens.push(Token::Hash(name));
            } else if self.starts_number() {
                tokens.push(self.consume_numeric()?);
            } else if ch == '.' {
                self.pos += 1;
                tokens.push(Token::Dot);
            } else if is_ident_start(ch) || (ch == '-' && self.starts_ident()) {
                let ident = self.consume_ident()?;
                tokens.push(Token::Ident(ident));
            } else {
                return Err(self.error(&format!("unexpected character {ch:?}")));
            }
        }
        Ok(tokens)
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn error(&self, message: &str) -> TokenizeError {
        TokenizeError {
            offset: self.pos,
            message: message.to_owned(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            self.pos += ch.len_utf8();
        }
    }

    fn starts_number(&self) -> bool {
        match self.peek() {
            Some(c) if c.is_ascii_digit() => true,
            Some('.') => matches!(self.peek_nth(1), Some(d) if d.is_ascii_digit()),
            Some('+' | '-') => match self.peek_nth(1) {
                Some(d) if d.is_ascii_digit() => true,
                Some('.') => matches!(self.peek_nth(2), Some(d) if d.is_ascii_digit()),
                _ => false,
            },
            _ => false,
        }
    }

    fn starts_ident(&self) -> bool {
        match self.peek() {
            Some(c) if is_ident_start(c) => true,
            Some('-') => matches!(self.peek_nth(1), Some(c) if is_ident_start(c) || c == '-'),
            _ => false,
        }
    }

    fn consume_numeric(&mut self) -> Result<Token, TokenizeError> {
        let start = self.pos;
        if matches!(self.peek(), Some('+' | '-')) {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some('.') && matches!(self.peek_nth(1), Some(d) if d.is_ascii_digit()) {
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let raw = &self.input[start..self.pos];
        let value: f32 = raw
            .parse()
            .map_err(|_| self.error(&format!("invalid number {raw:?}")))?;

        if self.starts_ident() {
            let unit = self.consume_ident()?.to_ascii_lowercase();
            Ok(Token::Dimension { value, unit })
        } else {
            Ok(Token::Number(value))
        }
    }

    fn consume_ident(&mut self) -> Result<String, TokenizeError> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if !is_ident_char(ch) {
                break;
            }
            self.pos += ch.len_utf8();
        }
        if self.pos == start {
            return Err(self.error("expected CSS identifier"));
        }
        Ok(self.input[start..self.pos].to_owned())
    }

    fn consume_hash_chars(&mut self) -> Result<String, TokenizeError> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if !is_ident_char(ch) {
                break;
            }
            self.pos += ch.len_utf8();
        }
        if self.pos == start {
            return Err(self.error("expected identifier or hex digits after '#'"));
        }
        Ok(self.input[start..self.pos].to_owned())
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || !ch.is_ascii()
}

fn is_ident_char(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit() || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_selector_whitespace_and_strips_comments() {
        let css = "main.hero /* comment */ #title { font-size: 24px; margin: -8px 0; }";
        let tokens = Tokenizer::new(css).tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("main".into()),
                Token::Dot,
                Token::Ident("hero".into()),
                Token::Whitespace,
                Token::Hash("title".into()),
                Token::Whitespace,
                Token::LBrace,
                Token::Whitespace,
                Token::Ident("font-size".into()),
                Token::Colon,
                Token::Whitespace,
                Token::Dimension {
                    value: 24.0,
                    unit: "px".into()
                },
                Token::Semicolon,
                Token::Whitespace,
                Token::Ident("margin".into()),
                Token::Colon,
                Token::Whitespace,
                Token::Dimension {
                    value: -8.0,
                    unit: "px".into()
                },
                Token::Whitespace,
                Token::Number(0.0),
                Token::Semicolon,
                Token::Whitespace,
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn rejects_unterminated_comment_and_bare_hash() {
        assert!(Tokenizer::new("/* oops").tokenize().is_err());
        assert!(Tokenizer::new("# {}").tokenize().is_err());
    }
}
