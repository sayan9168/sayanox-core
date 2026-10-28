//! Tokenization of tags, attributes, comments, and text.

use super::dom::Attribute;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    StartTag { name: String, attributes: Vec<Attribute>, self_closing: bool },
    EndTag { name: String },
    Text(String),
    Comment(String),
}

/// A byte offset into the original UTF-8 input and a description of the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTML tokenization error at byte {}: {}", self.offset, self.message)
    }
}
impl Error for TokenizeError {}

pub struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self { Self { input, pos: 0 } }

    pub fn tokenize(mut self) -> Result<Vec<Token>, TokenizeError> {
        let mut tokens = Vec::new();
        while !self.rest().is_empty() {
            if self.rest().starts_with("<!--") {
                self.pos += 4;
                let end = self.rest().find("-->").ok_or_else(|| self.error("unterminated comment"))?;
                tokens.push(Token::Comment(self.rest()[..end].to_owned()));
                self.pos += end + 3;
            } else if self.rest().starts_with("</") {
                self.pos += 2;
                let name = self.name()?;
                self.skip_whitespace();
                self.expect(">")?;
                tokens.push(Token::EndTag { name });
            } else if self.rest().starts_with('<') {
                self.pos += 1;
                let name = self.name()?;
                let mut attributes = Vec::new();
                let self_closing = loop {
                    self.skip_whitespace();
                    if self.rest().starts_with("/>") {
                        self.pos += 2;
                        break true;
                    }
                    if self.rest().starts_with('>') {
                        self.pos += 1;
                        break false;
                    }
                    if self.rest().is_empty() { return Err(self.error("unterminated start tag")); }
                    let attr_name = self.name()?;
                    self.skip_whitespace();
                    let value = if self.rest().starts_with('=') {
                        self.pos += 1;
                        self.skip_whitespace();
                        self.attribute_value()?
                    } else { String::new() };
                    attributes.push(Attribute { name: attr_name, value });
                };
                tokens.push(Token::StartTag { name, attributes, self_closing });
            } else {
                let end = self.rest().find('<').unwrap_or(self.rest().len());
                tokens.push(Token::Text(self.rest()[..end].to_owned()));
                self.pos += end;
            }
        }
        Ok(tokens)
    }

    fn rest(&self) -> &'a str { &self.input[self.pos..] }
    fn error(&self, message: &str) -> TokenizeError {
        TokenizeError { offset: self.pos, message: message.to_owned() }
    }
    fn expect(&mut self, value: &str) -> Result<(), TokenizeError> {
        if !self.rest().starts_with(value) { return Err(self.error(&format!("expected {value:?}"))); }
        self.pos += value.len();
        Ok(())
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.rest().chars().next() {
            if !c.is_ascii_whitespace() { break; }
            self.pos += c.len_utf8();
        }
    }
    fn name(&mut self) -> Result<String, TokenizeError> {
        let start = self.pos;
        while let Some(c) = self.rest().chars().next() {
            if !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.')) { break; }
            self.pos += c.len_utf8();
        }
        if self.pos == start { return Err(self.error("expected a tag or attribute name")); }
        Ok(self.input[start..self.pos].to_ascii_lowercase())
    }
    fn attribute_value(&mut self) -> Result<String, TokenizeError> {
        let first = self.rest().chars().next().ok_or_else(|| self.error("expected attribute value"))?;
        if first == '"' || first == '\'' {
            self.pos += 1;
            let end = self.rest().find(first).ok_or_else(|| self.error("unterminated attribute value"))?;
            let value = self.rest()[..end].to_owned();
            self.pos += end + 1;
            Ok(value)
        } else {
            let start = self.pos;
            while let Some(c) = self.rest().chars().next() {
                if c.is_ascii_whitespace() || c == '>' { break; }
                if matches!(c, '"' | '\'' | '<' | '=' | '`') {
                    return Err(self.error("invalid character in unquoted attribute value"));
                }
                self.pos += c.len_utf8();
            }
            if self.pos == start { return Err(self.error("expected attribute value")); }
            Ok(self.input[start..self.pos].to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_attributes_comments_and_unicode() {
        let tokens = Tokenizer::new("<DIV id='one' title=hello disabled>hé<!--ok--><br /></DIV>").tokenize().unwrap();
        assert_eq!(tokens, vec![
            Token::StartTag { name: "div".into(), attributes: vec![
                Attribute { name: "id".into(), value: "one".into() },
                Attribute { name: "title".into(), value: "hello".into() },
                Attribute { name: "disabled".into(), value: "".into() },
            ], self_closing: false },
            Token::Text("hé".into()), Token::Comment("ok".into()),
            Token::StartTag { name: "br".into(), attributes: vec![], self_closing: true },
            Token::EndTag { name: "div".into() },
        ]);
    }

    #[test]
    fn rejects_unterminated_input() {
        for input in ["<!--x", "<div", "<p a='x", "<p a=>", "</p"] {
            assert!(Tokenizer::new(input).tokenize().is_err(), "{input}");
        }
    }
}
