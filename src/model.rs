pub(crate) const MAGIC: [u8; 7] = [b'U', b'L', b'o', b'g', 0x01, 0x12, 0x35];

pub mod msg {
    use crate::errors::ULogError;
    use crate::model::{def, inst};
    use crate::model::MAGIC;

    #[derive(Debug)]
    pub enum UlogMessage {
        Header(FileHeader),
        FlagBits(FlagBits),
        FormatDefinition(def::Format),
        LoggedData(LoggedData),
        AddSubscription(Subscription),
        Info(Info),
        MultiInfo(MultiInfo),
        Parameter(Parameter),
        DefaultParameter(DefaultParameter),
        LoggedString(LoggedString),
        TaggedLoggedString(LoggedString),
        DropoutMark(Dropout),
        Unhandled {
            msg_type: u8,
            message_contents: Vec<u8>,
        },
        Ignored {
            msg_type: u8,
            message_contents: Vec<u8>,
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct FileHeader {
        pub version: u8,
        pub timestamp: u64,
    }

    impl FileHeader {
        pub fn to_bytes(&self) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(16);

            bytes.extend_from_slice(&MAGIC);
            bytes.push(self.version);
            bytes.extend_from_slice(&self.timestamp.to_le_bytes());

            bytes
        }
    }

    #[derive(Debug)]
    pub struct FlagBits {
        pub compat_flags: [u8;8],
        pub incompat_flags: [u8;8],
        pub appended_data_offsets: [u64;3],
    }

    impl FlagBits {
        const ULOG_COMPAT_FLAG0_DEFAULT_PARAMETERS_MASK: u8 = 0b0000_0001; // Bit 0 indicates presence of DEFAULT_PARAMETERS
        const ULOG_INCOMPAT_FLAG0_DATA_APPENDED_MASK: u8 = 0b0000_0001; // Bit 0 indicates presence of DATA_APPENDED

        // If true, the log contains default parameters message
        // FIXME: Handle default parameters.
        pub fn has_default_parameters(&self)-> bool {
            self.compat_flags[0] & Self::ULOG_COMPAT_FLAG0_DEFAULT_PARAMETERS_MASK != 0

        }

        // If true, the log contains appended data and at least one of the appended_offsets is non-zero.
        pub fn has_data_appended(&self) ->  bool {
            self.incompat_flags[0] & Self::ULOG_INCOMPAT_FLAG0_DATA_APPENDED_MASK != 0
        }
    }

    #[derive(Debug)]
    // Represents both Logged Messages and Tagged Logged Messages
    pub struct LoggedString {
        pub level: LogLevel,
        pub tag: Option<u16>,
        pub timestamp: u64,
        pub msg: String,
    }

    #[derive(Debug, PartialEq, Copy, Clone)]
    #[repr(u8)]
    pub enum LogLevel {
        Emerg = b'0',
        Alert = b'1',
        Crit = b'2',
        Err = b'3',
        Warning = b'4',
        Notice = b'5',
        Info = b'6',
        Debug = b'7',
    }

    #[derive(Debug, Clone)]
    pub struct Subscription {
        pub multi_id: u8,
        pub msg_id: u16,
        pub message_name: String,
    }

    #[derive(Debug, Clone)]
    pub struct Info {
        pub key: String,
        pub r#type: def::TypeExpr,
        pub value: inst::FieldValue,
    }

    #[derive(Debug, Clone)]
    pub struct MultiInfo {
        pub is_continued: bool,
        pub key: String,
	    pub r#type: def::TypeExpr,
        pub value: inst::FieldValue,
    }

    #[derive(Debug)]
    pub struct Parameter {
        pub key: String,
        pub r#type: def::TypeExpr,
        pub value: inst::ParameterValue,
    }

    #[derive(Debug)]
    pub struct DefaultParameter {
        pub key: String,
        pub default_types: u8,
        pub r#type: def::TypeExpr,
        pub value: inst::ParameterValue
    }

    #[derive(Debug)]
    pub enum DefaultType {
        SystemWide,
        Configuration,
        Both, // when both bits are set
    }

    impl DefaultParameter {
        pub fn get_default_type(&self) -> DefaultType {
            match self.default_types {
                0b01 => DefaultType::SystemWide,
                0b10 => DefaultType::Configuration,
                0b11 => DefaultType::Both,
                _ => panic!("Invalid default types bitfield"), // You can handle this more gracefully if needed
            }
        }
    }

    impl DefaultType {
        pub fn from_bits(bits: u8) -> Self {
            match bits {
                0b01 => DefaultType::SystemWide,
                0b10 => DefaultType::Configuration,
                0b11 => DefaultType::Both,
                _ => panic!("Invalid default types bitfield"), // You can handle this more gracefully if needed
            }
        }
    }
    
    #[derive(Debug, Clone)]
    pub struct LoggedData {
        pub timestamp: u64,
        pub msg_id: u16,
        pub data: inst::Format,
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Dropout {
        pub(crate) duration: u16,
    }

    impl TryFrom<u8> for LogLevel {
        type Error = ULogError;

        fn try_from(byte: u8) -> Result<Self, Self::Error> {
            match byte {
                b'0' => Ok(LogLevel::Emerg),
                b'1' => Ok(LogLevel::Alert),
                b'2' => Ok(LogLevel::Crit),
                b'3' => Ok(LogLevel::Err),
                b'4' => Ok(LogLevel::Warning),
                b'5' => Ok(LogLevel::Notice),
                b'6' => Ok(LogLevel::Info),
                b'7' => Ok(LogLevel::Debug),
                byte => Err(ULogError::ParseError(format!("Invalid LogLevel value: {byte:02X}"))),
            }
        }
    }
}

/// This module defines structs that represent the structure or schema, without any actual data.
/// For example, a `def::Format` represents the declaration of an aggregate data type, 
/// similar to a struct, containing `def::Field` definitions.
///
/// See also the `inst` module, which defines structs that carry actual data, which are analogues
/// of the structures defined in this module.
pub mod def {
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct Format {
        pub name: String,
        pub fields: Vec<Field>,
        pub padding: usize,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct Field {
        pub name: String,
        pub r#type: TypeExpr
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct TypeExpr {
        pub base_type: BaseType,
        pub array_size: Option<usize>,
    }

    #[allow(clippy::upper_case_acronyms)]
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
    pub enum BaseType {
        UINT8,
        UINT16,
        UINT32,
        UINT64,
        INT8,
        INT16,
        INT32,
        INT64,
        FLOAT,
        DOUBLE,
        BOOL,
        CHAR,
        OTHER(String),
    }
}

/// This module defines structs that represent data instances, based on the structures
/// and schemas defined in the module `def`.
///
/// For example, `inst::Format` and `inst::Field` represent concrete data objects, which
/// are instances of the type definitions described by `def::Format` and `def::Field`.
pub mod inst {
    use crate::model::def::TypeExpr;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Format {
        pub timestamp: Option<u64>,
        pub name: String,
        pub fields: Vec<Field>,
        pub multi_id_index: Option<u8>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Field {
        pub name: String,
        pub r#type: TypeExpr,
        pub value: FieldValue,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum FieldValue {
        SCALAR(BaseType),
        ARRAY(Vec<BaseType>),
    }

    #[derive(Debug, Clone)]
    pub enum ParameterValue { INT32(i32), FLOAT(f32) }

    #[derive(Debug, Clone, PartialEq)]
    pub enum BaseType {
        UINT8(u8),
        UINT16(u16),
        UINT32(u32),
        UINT64(u64),
        INT8(i8),
        INT16(i16),
        INT32(i32),
        INT64(i64),
        FLOAT(f32),
        DOUBLE(f64),
        BOOL(bool),
        CHAR(char),
        OTHER(Format),
    }
}

impl inst::Format {
    pub fn flatten(&self) -> Vec<(String, inst::BaseType)> {
        let prefix:String = self.to_string();

        self.flatten_sub(&prefix)
    }

    fn flatten_sub(&self, path: &str) -> Vec<(String, inst::BaseType)> {
        let mut flattened = Vec::new();

        for field in &self.fields {
            let current_path = format!("{}/{}", path, field.name);

            match &field.value {
                inst::FieldValue::SCALAR(data_type) => {
                    flattened.extend(self.flatten_data_type(current_path.clone(), data_type));
                }
                inst::FieldValue::ARRAY(array) => {
                    for (index, data_type) in array.iter().enumerate() {
                        let array_path = format!("{current_path}.{index:02}");
                        flattened.extend(self.flatten_data_type(array_path, data_type));
                    }
                }
            }
        }

        flattened
    }

    // Helper method to flatten a inst::DataType, handling recursion if the type is OTHER
    #[allow(clippy::unused_self)]
    fn flatten_data_type(&self, path: String, data_type: &inst::BaseType) -> Vec<(String, inst::BaseType)> {
        match data_type {
            inst::BaseType::OTHER(nested_format) => {
                // Recursively flatten nested DataFormat
                nested_format.flatten_sub(&path)
            }
            _ => {
                // For any other scalar data type, just append the path and the data type
                vec![(path, data_type.clone())]
            }
        }
    }
}

impl From<inst::BaseType> for def::BaseType {
    fn from(base: inst::BaseType) -> Self {
        match base {
            inst::BaseType::UINT8(_) => def::BaseType::UINT8,
            inst::BaseType::UINT16(_) => def::BaseType::UINT16,
            inst::BaseType::UINT32(_) => def::BaseType::UINT32,
            inst::BaseType::UINT64(_) => def::BaseType::UINT64,
            inst::BaseType::INT8(_) => def::BaseType::INT8,
            inst::BaseType::INT16(_) => def::BaseType::INT16,
            inst::BaseType::INT32(_) => def::BaseType::INT32,
            inst::BaseType::INT64(_) => def::BaseType::INT64,
            inst::BaseType::FLOAT(_) => def::BaseType::FLOAT,
            inst::BaseType::DOUBLE(_) => def::BaseType::DOUBLE,
            inst::BaseType::BOOL(_) => def::BaseType::BOOL,
            inst::BaseType::CHAR(_) => def::BaseType::CHAR,
            inst::BaseType::OTHER(format) => def::BaseType::OTHER(format.name),
        }
    }
}


