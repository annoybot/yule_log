use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ULogError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 Decoding Error: {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Unknown Parameter Type")]
    UnknownParameterType(String),

    #[error("Invalid magic bits. Not a ULOG file.")]
    InvalidMagicBits,

    #[error("Invalid Header")]
    InvalidHeader,

    #[error("Invalid Definitions")]
    InvalidDefinitions,

    #[error("Unexpected End of File")]
    UnexpectedEndOfFile,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Undefined format {0}")]
    UndefinedFormat(String),

    #[error("Cound not find subscription for msg_id: {0}")]
    UndefinedSubscription(u16),

    #[error("Unknown Incompat Bits")]
    UnknownIncompatBits,

    #[error("Missing timestamp for legged data message.")]
    MissingTimestamp,

    #[error("Invalid MultiInfo message. {0}")]
    InvalindMultiInfo(String),

    #[error("Invalid Default Parameter Type")]
    InvalidDefaultParameterType,

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("Invalid LoggedData field name: {0}")]
    InvalidFieldName(String),

    #[error("Invalid parser configuration: {0}")]
    InvalidConfiguration(String),
}
