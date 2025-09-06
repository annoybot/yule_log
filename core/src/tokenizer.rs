use lazy_static::lazy_static;
use regex::Regex;
use std::collections::VecDeque;

use crate::errors::ULogError;

lazy_static! {
    static ref TOKEN_REGEXP: Regex = Regex::new(
        r"(?P<identifier>[a-zA-Z_][a-zA-Z0-9_]*)|(?P<number>[0-9]+)|(?P<colon>:)|(?P<semicolon>;)|(?P<lbrace>\[)|(?P<rbrace>\])|(?P<whitespace>\s+)|(?P<unknown>.)"
    )
    .unwrap();
}

#[derive(Debug)]
pub struct TokenList<'a>(VecDeque<Token<'a>>);

impl<'a> TokenList<'a> {
    #[allow(dead_code)]
    pub fn new(tokens: VecDeque<Token<'a>>) -> Self {
        TokenList(tokens)
    }

    pub(crate) fn from_str(s: &'a str) -> Self {
        TokenList(tokenize(s))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn remaining(&self) -> usize {
        self.0.len()
    }

    pub fn peek(&self) -> Option<&Token<'a>> {
        self.0.front()
    }

    pub fn consume_one(&mut self) -> Result<Token<'a>, ULogError> {
        self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)
    }

    pub fn consume_two(&mut self) -> Result<(Token<'a>, Token<'a>), ULogError> {
        Ok((
            self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)?,
            self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)?,
        ))
    }

    pub fn consume_three(&mut self) -> Result<(Token<'a>, Token<'a>, Token<'a>), ULogError> {
        Ok((
            self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)?,
            self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)?,
            self.0.pop_front().ok_or(ULogError::UnexpectedEndOfFile)?,
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Identifier(&'a str),
    Number(usize),
    Colon,
    Semicolon,
    LBrace,
    RBrace,
    Unknown(char),
    // Whitespace is skipped by the tokenizer, not represented as a token.
}

pub fn tokenize(input: &str) -> VecDeque<Token<'_>> {
    let mut tokens = VecDeque::new();

    for caps in TOKEN_REGEXP.captures_iter(input) {
        if let Some(identifier) = caps.name("identifier") {
            tokens.push_back(Token::Identifier(identifier.as_str()));
        } else if let Some(number) = caps.name("number") {
            tokens.push_back(Token::Number(number.as_str().parse::<usize>().unwrap()));
        } else if caps.name("colon").is_some() {
            tokens.push_back(Token::Colon);
        } else if caps.name("semicolon").is_some() {
            tokens.push_back(Token::Semicolon);
        } else if caps.name("lbrace").is_some() {
            tokens.push_back(Token::LBrace);
        } else if caps.name("rbrace").is_some() {
            tokens.push_back(Token::RBrace);
        } else if let Some(unknown) = caps.name("unknown") {
            tokens.push_back(Token::Unknown(unknown.as_str().chars().next().unwrap()));
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn test_tokenize() {
        let input = "message1: int field0; float[5] field1;";
        let expected_tokens: VecDeque<Token> = [
            Token::Identifier("message1"),
            Token::Colon,
            Token::Identifier("int"),
            Token::Identifier("field0"),
            Token::Semicolon,
            Token::Identifier("float"),
            Token::LBrace,
            Token::Number(5),
            Token::RBrace,
            Token::Identifier("field1"),
            Token::Semicolon,
        ]
            .into();

        let tokens = tokenize(input);
        assert_eq!(expected_tokens, tokens);
    }

    #[test]
    fn test_tokenize_unknown() {
        let input = "message1: ? int field0;";
        let expected_tokens: VecDeque<Token> = [
            Token::Identifier("message1"),
            Token::Colon,
            Token::Unknown('?'),
            Token::Identifier("int"),
            Token::Identifier("field0"),
            Token::Semicolon,
        ]
            .into();

        let tokens = tokenize(input);
        assert_eq!(expected_tokens, tokens);
    }
}
