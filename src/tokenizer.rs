use lazy_static::lazy_static;
use regex::Regex;
use crate::parser::ULogError;

lazy_static! {
    static ref TOKEN_REGEXP: Regex = Regex::new(r"(?P<identifier>[a-zA-Z_][a-zA-Z0-9_]*)|(?P<number>[0-9]+)|(?P<colon>:)|(?P<semicolon>;)|(?P<lbrace>\[)|(?P<rbrace>\])|(?P<whitespace>\s+)|(?P<unknown>.)").unwrap();
}
#[derive(Debug)]
pub struct TokenList(Vec<Token>);

impl TokenList {
    pub fn new(tokens: Vec<Token>) -> Self {
        TokenList(tokens)
    }

    pub(crate) fn from_str(str: &String) -> Self {
        TokenList(tokenize(str))

    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    
    pub fn remaining(&self) -> usize {
        self.0.len() // Access the inner Vec using index
    }

    pub fn peek(&self) -> Option<&Token> {
        self.0.get(0) // Always peek at the first token
    }

    pub fn consume_one(&mut self) -> Result<Token, ULogError> {
        if self.remaining() < 1 {
            return Err(ULogError::UnexpectedEndOfFile);
        }
        let token = self.0.remove(0); // Remove and return the first token
        Ok(token)
    }

    pub fn consume_two(&mut self) -> Result<(Token, Token), ULogError> {
        if self.remaining() < 2 {
            return Err(ULogError::UnexpectedEndOfFile);
        }
        let token1 = self.0.remove(0); // Remove and return the first token
        let token2 = self.0.remove(0); // Remove and return the next token
        Ok((token1, token2))
    }

    pub fn consume_three(&mut self) -> Result<(Token, Token, Token), ULogError> {
        if self.remaining() < 3 {
            return Err(ULogError::UnexpectedEndOfFile);
        }
        let token1 = self.0.remove(0); // Remove and return the first token
        let token2 = self.0.remove(0); // Remove and return the second token
        let token3 = self.0.remove(0); // Remove and return the third token
        Ok((token1, token2, token3))
    }
}


#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Identifier(String),
    Number(usize),
    Colon,
    Semicolon,
    LBrace,
    RBrace,
    Unknown(char),
    // We deliberately do not define a whitespace variant so that
    // whitespace will be matched, but skipped by the tokenizer.
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    for caps in TOKEN_REGEXP.captures_iter(input) {
        if let Some(identifier) = caps.name("identifier") {
            tokens.push(Token::Identifier(identifier.as_str().to_string()));
        } else if let Some(number) = caps.name("number") {
            tokens.push(Token::Number(number.as_str().parse::<usize>().unwrap()));
        } else if let Some(colon) = caps.name("colon") {
            tokens.push(Token::Colon);
        } else if let Some(colon) = caps.name("semicolon") {
            tokens.push(Token::Semicolon);
        } else if let Some(colon) = caps.name("lbrace") {
            tokens.push(Token::LBrace);
        } else if let Some(colon) = caps.name("rbrace") {
            tokens.push(Token::RBrace);
        } else if let Some(unknown) = caps.name("unknown") {
            tokens.push(Token::Unknown(unknown.as_str().parse::<char>().unwrap()));
        }
    }

    tokens
}


fn remove_prefix(s: &mut String, prefix: &str) {
    if s.starts_with(prefix) {
        s.drain(0..prefix.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let input = "message1: int field0; float[5] field1;";
        let expected_tokens = vec![
            Token::Identifier("message1".to_string()),
            Token::Colon,
            Token::Identifier("int".to_string()),
            Token::Identifier("field0".to_string()),
            Token::Semicolon,
            Token::Identifier("float".to_string()),
            Token::LBrace,
            Token::Number(5),
            Token::RBrace,
            Token::Identifier("field1".to_string()),
            Token::Semicolon,
        ];

        let tokens = tokenize(input);
        assert_eq!(expected_tokens, tokens);
    }

    #[test]
    fn test_tokenize_unknown() {
        let input = "message1: ? int field0;";
        let expected_tokens = vec![
            Token::Identifier("message1".to_string()),
            Token::Colon,
            Token::Unknown('?'),
            Token::Identifier("int".to_string()),
            Token::Identifier("field0".to_string()),
            Token::Semicolon,
        ];

        let tokens = tokenize(input);
        assert_eq!(expected_tokens, tokens);
    }
}


