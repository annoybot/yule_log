use crate::tokenizer::{tokenize, Token};
use std::io::Read;
use crate::datastream::DataStream;
use crate::parser::{Field, Format, FormatType, ULogError, ULogParser};
use crate::tokenizer::TokenList;

pub(crate) fn parse_format<R: Read>(datastream: &mut DataStream<R>, msg_size: u16) -> Result<Format, ULogError>
{
    log::trace!("Entering {}", "parse_format");
    let mut message_buf: Vec<u8> = vec![0; msg_size as usize];
    datastream.read_exact(&mut message_buf)?;
    let str_format = String::from_utf8(message_buf)?;
    log::trace!("buffer: {}", str_format);

    let mut token_list = TokenList::from_str(&str_format);
    log::trace!("token_list: {:?}", token_list);
    
    let name = match token_list.consume_two()? {
        (Token::Identifier(str), Token::Colon) => str,
        (token1, token2) => {
            return Err(ULogError::ParseError(format!(
                "Invalid format name. Expected [Identifier, Colon], got: {:?}, {:?}",
                token1, token2
            )));
        }
    };
    
    let mut fields:Vec<Field> = vec![];
    
    while !token_list.is_empty() {
        fields.push(parse_field(&mut token_list)?);
    }

    log::trace!("Exiting {}", "parse_format");
    
    Ok( Format {
        name: name,
        fields: fields,
        padding: 0,
    } )

}

pub(crate) fn parse_field(token_list: &mut TokenList) -> Result<Field, ULogError>
{
    log::trace!("Entering {}", "parse_field");
    log::trace!("token_list: {:?}", token_list);

    let type_name = match token_list.consume_one()? {
        Token::Identifier(type_name) => { parse_format_type(type_name.as_str()) }

        _ => {Err(ULogError::FormatError)?}
    };
    
    let mut array_size = 1;
    
    if token_list.peek() == Some(&Token::LBrace) {
        array_size = match token_list.consume_three()? {
            (Token::LBrace, Token::Number(size), Token::RBrace) => { size },
            _ => {Err(ULogError::FormatError)?}
        }
    }

    let field_name = match token_list.consume_two()? {
        (Token::Identifier(str), Token::Semicolon) => { str }

        _ => { Err(ULogError::FormatError)? }
    };

    log::trace!("Exiting {}", "parse_field");
    
    Ok( Field {
        field_name,
        type_: type_name,
        array_size,
    } )
}

fn parse_format_type(field_type: &str) -> FormatType {
    match field_type {
        "int8_t" => FormatType::INT8,
        "int16_t" => FormatType::INT16,
        "int32_t" => FormatType::INT32,
        "int64_t" => FormatType::INT64,
        "uint8_t" => FormatType::UINT8,
        "uint16_t" => FormatType::UINT16,
        "uint32_t" => FormatType::UINT32,
        "uint64_t" => FormatType::UINT64,
        "double" => FormatType::DOUBLE,
        "float" => FormatType::FLOAT,
        "bool" => FormatType::BOOL,
        "char" => FormatType::CHAR,
        _ => FormatType::OTHER(field_type.to_string()),
    }
}

impl FormatType {
    pub(crate) fn is_other(&self) -> bool {
        match self {
            FormatType::OTHER(_) => true,
            _ => false
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use log::LevelFilter;
    // For testing with in-memory data

    #[test]
    fn test_parse_format() {
        env_logger::builder().filter_level(LevelFilter::Trace).init();

        let input = b"my_format:uint64_t timestamp; bool is_happy; uint8_t[8] pet_ids;";
        let msg_size = input.len() as u16;

        let mut datastream = DataStream::new(Cursor::new(input) );

        // Call the parse_format method
        let result = parse_format(&mut datastream, msg_size);

        // Assert the result is Ok and has the expected structure
        let expected_format = Format {
            name: "my_format".to_string(),
            fields: vec![
                Field {
                    field_name: "timestamp".to_string(),
                    type_: FormatType::UINT64,
                    array_size: 1, // assuming non-array type for uint64_t
                },
                Field {
                    field_name: "is_happy".to_string(),
                    type_: FormatType::BOOL,
                    array_size: 1, // assuming non-array type for bool
                },
                Field {
                    field_name: "pet_ids".to_string(),
                    type_: FormatType::UINT8,
                    array_size: 8, // as defined in the input string
                },
            ],
            padding: 0, // Set according to your needs
        };

        // Assert that the result matches the expected format
        assert_eq!(result.unwrap(), expected_format);
    }
}


