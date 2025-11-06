use crate::errors::ULogError;
use crate::message_buf::MessageBuf;
use crate::model::def;
use crate::tokenizer::Token;
use crate::tokenizer::TokenList;

pub(crate) fn parse_format(message_buf: MessageBuf) -> Result<def::Format, ULogError> {
    let str_format = String::from_utf8(message_buf.into_remaining_bytes())?;

    let mut token_list = TokenList::from_str(&str_format);
    log::trace!("token_list: {token_list:?}");

    let name = match token_list.consume_two()? {
        (Token::Identifier(str), Token::Colon) => str,
        (token1, token2) => {
            return Err(ULogError::ParseError(format!(
                "Invalid format name. Expected [Identifier, Colon], got: {token1:?}, {token2:?}"
            )));
        }
    };

    let mut fields: Vec<def::Field> = vec![];

    while !token_list.is_empty() {
        fields.push(parse_field(&mut token_list)?);

        match token_list.consume_one()? {
            Token::Semicolon => {}
            token => {
                Err(ULogError::ParseError(format!(
                    "Invalid format definition. Expected a Semicolon, got: {token:?}"
                )))?;
            }
        }
    }

    Ok(def::Format {
        name: name.to_string(),
        fields,
        padding: 0,
    })
}

pub(crate) fn parse_field(token_list: &mut TokenList) -> Result<def::Field, ULogError> {
    log::trace!("token_list: {token_list:?}");

    let base_type = match token_list.consume_one()? {
        Token::Identifier(type_name) => def::BaseType::from_string(type_name),

        token => Err(ULogError::ParseError(format!(
            "Invalid field definition. Expected an Identifier, got: {token:?}"
        )))?,
    };

    let mut array_size: Option<usize> = None;

    if token_list.peek() == Some(&Token::LBrace) {
        array_size = match token_list.consume_three()? {
            (Token::LBrace, Token::Number(size), Token::RBrace) => { Some(size) }
            (token1, token2, token3) => { Err(ULogError::ParseError(format!(
                "Invalid array definition. Expected [LBrace, Number, RBrace], got: [{token1:?}, {token2:?}, {token3:?}]"
            )))? }
        }
    }

    let field_name = match token_list.consume_one()? {
        Token::Identifier(str) => str,
        token => Err(ULogError::ParseError(format!(
            "Invalid field definition. Expected an Identifier, got: {token:?}"
        )))?,
    };

    Ok(def::Field {
        name: field_name.to_string(),
        r#type: def::TypeExpr {
            base_type,
            array_size,
        },
    })
}

impl def::BaseType {
    pub fn from_string(string: &str) -> def::BaseType {
        match string {
            "int8_t" => def::BaseType::INT8,
            "int16_t" => def::BaseType::INT16,
            "int32_t" => def::BaseType::INT32,
            "int64_t" => def::BaseType::INT64,
            "uint8_t" => def::BaseType::UINT8,
            "uint16_t" => def::BaseType::UINT16,
            "uint32_t" => def::BaseType::UINT32,
            "uint64_t" => def::BaseType::UINT64,
            "double" => def::BaseType::DOUBLE,
            "float" => def::BaseType::FLOAT,
            "bool" => def::BaseType::BOOL,
            "char" => def::BaseType::CHAR,
            _ => def::BaseType::OTHER(string.to_string()),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_other(&self) -> bool {
        matches!(self, def::BaseType::OTHER(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Encode;

    #[test]
    fn test_parse_format() {
        let input = b"my_format:uint64_t timestamp; bool is_happy; uint8_t[8] pet_ids;";
        let message_buf = MessageBuf::from_vec(input.to_vec());

        // Call the parse_format method
        let result = parse_format(message_buf);

        // Assert the result is Ok and has the expected structure
        let expected_format = def::Format {
            name: "my_format".to_string(),
            fields: vec![
                def::Field {
                    name: "timestamp".to_string(),
                    r#type: def::TypeExpr {
                        base_type: def::BaseType::UINT64,
                        array_size: None,
                    },
                },
                def::Field {
                    name: "is_happy".to_string(),
                    r#type: def::TypeExpr {
                        base_type: def::BaseType::BOOL,
                        array_size: None,
                    },
                },
                def::Field {
                    name: "pet_ids".to_string(),
                    r#type: def::TypeExpr {
                        base_type: def::BaseType::UINT8,
                        array_size: Some(8),
                    },
                },
            ],
            padding: 0,
        };

        // Assert that the result matches the expected format
        assert_eq!(result.unwrap(), expected_format);
    }

    #[test]
    fn test_round_trip_format() {
        let input = b"my_format:uint64_t timestamp;custom_type custom_field;bool is_happy;custom_type2[4] custom_field;uint8_t[8] pet_ids;";
        let message_buf = MessageBuf::from_vec(input.to_vec());

        let parsed_format = parse_format(message_buf).unwrap();

        let mut re_emitted_bytes = Vec::new();
        parsed_format.encode(&mut re_emitted_bytes).unwrap();

        println!(
            "re_emitted_bytes: {:?}",
            str::from_utf8(&re_emitted_bytes).unwrap()
        );

        assert_eq!(re_emitted_bytes, input);
    }
}
