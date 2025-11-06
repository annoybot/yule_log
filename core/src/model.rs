pub(crate) const MAGIC: [u8; 7] = [b'U', b'L', b'o', b'g', 0x01, 0x12, 0x35];

pub mod msg {
    use crate::errors::ULogError;
    use crate::model::MAGIC;
    use crate::model::{def, inst};

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
        },
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
        pub compat_flags: [u8; 8],
        pub incompat_flags: [u8; 8],
        pub appended_data_offsets: [u64; 3],
    }

    impl FlagBits {
        const ULOG_COMPAT_FLAG0_DEFAULT_PARAMETERS_MASK: u8 = 0b0000_0001; // Bit 0 indicates presence of DEFAULT_PARAMETERS
        const ULOG_INCOMPAT_FLAG0_DATA_APPENDED_MASK: u8 = 0b0000_0001; // Bit 0 indicates presence of DATA_APPENDED

        // If true, the log contains default parameters message
        // FIXME: Handle default parameters.
        pub fn has_default_parameters(&self) -> bool {
            self.compat_flags[0] & Self::ULOG_COMPAT_FLAG0_DEFAULT_PARAMETERS_MASK != 0
        }

        // If true, the log contains appended data and at least one of the appended_offsets is non-zero.
        pub fn has_data_appended(&self) -> bool {
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
        pub value: inst::ParameterValue,
    }

    #[derive(Debug)]
    pub struct DefaultType {
        pub system_wide: bool,
        pub configuration: bool,
    }

    const DEFAULT_TYPE_SYSTEM_WIDE: u8 = 0b01;
    const DEFAULT_TYPE_CONFIGURATION: u8 = 0b10;

    impl DefaultParameter {
        pub fn get_default_type(&self) -> DefaultType {
            DefaultType {
                system_wide: self.default_types & DEFAULT_TYPE_SYSTEM_WIDE != 0,
                configuration: self.default_types & DEFAULT_TYPE_CONFIGURATION != 0,
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
                byte => Err(ULogError::ParseError(format!(
                    "Invalid LogLevel value: {byte:02X}"
                ))),
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
    #[derive(Debug, Clone, PartialEq)]
    pub struct Format {
        pub name: String,
        pub fields: Vec<Field>,
        pub padding: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Field {
        pub name: String,
        pub r#type: TypeExpr,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TypeExpr {
        pub base_type: BaseType,
        pub array_size: Option<usize>,
    }

    #[allow(clippy::upper_case_acronyms)]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    use std::rc::Rc;
    use crate::model::def::TypeExpr;
    use crate::model::{def, inst, CChar};

    #[derive(Debug, Clone, PartialEq)]
    pub struct Format {
        pub timestamp: Option<u64>,
        pub name: String,
        pub fields: Vec<Field>,
        pub multi_id_index: Option<u8>,
        pub def_format: Rc<def::Format>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Field {
        pub name: String,
        pub r#type: TypeExpr,
        pub value: FieldValue,
    }

    #[derive(Debug, Clone)]
    pub enum ParameterValue {
        INT32(i32),
        FLOAT(f32),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum FieldValue {
        // Typed scalars
        ScalarU8(u8),
        ScalarU16(u16),
        ScalarU32(u32),
        ScalarU64(u64),
        ScalarI8(i8),
        ScalarI16(i16),
        ScalarI32(i32),
        ScalarI64(i64),
        ScalarF32(f32),
        ScalarF64(f64),
        ScalarBool(bool),
        ScalarChar(CChar),
        ScalarOther(Rc<inst::Format>),

        // Typed arrays
        ArrayU8(Vec<u8>),
        ArrayU16(Vec<u16>),
        ArrayU32(Vec<u32>),
        ArrayU64(Vec<u64>),
        ArrayI8(Vec<i8>),
        ArrayI16(Vec<i16>),
        ArrayI32(Vec<i32>),
        ArrayI64(Vec<i64>),
        ArrayF32(Vec<f32>),
        ArrayF64(Vec<f64>),
        ArrayBool(Vec<bool>),
        ArrayChar(Vec<CChar>),
        ArrayOther(Vec<inst::Format>),
    }
}

impl inst::FieldValue {
    pub fn to_scalars(&self) -> Option<Vec<inst::FieldValue>> {
        use inst::FieldValue::*;
        match self {
            ArrayU8(v) => Some(v.iter().map(|&x| ScalarU8(x)).collect()),
            ArrayU16(v) => Some(v.iter().map(|&x| ScalarU16(x)).collect()),
            ArrayU32(v) => Some(v.iter().map(|&x| ScalarU32(x)).collect()),
            ArrayU64(v) => Some(v.iter().map(|&x| ScalarU64(x)).collect()),
            ArrayI8(v) => Some(v.iter().map(|&x| ScalarI8(x)).collect()),
            ArrayI16(v) => Some(v.iter().map(|&x| ScalarI16(x)).collect()),
            ArrayI32(v) => Some(v.iter().map(|&x| ScalarI32(x)).collect()),
            ArrayI64(v) => Some(v.iter().map(|&x| ScalarI64(x)).collect()),
            ArrayF32(v) => Some(v.iter().map(|&x| ScalarF32(x)).collect()),
            ArrayF64(v) => Some(v.iter().map(|&x| ScalarF64(x)).collect()),
            ArrayBool(v) => Some(v.iter().map(|&x| ScalarBool(x)).collect()),
            ArrayChar(v) => Some(v.iter().map(|&x| ScalarChar(x)).collect()),
            ArrayOther(v) => Some(v.iter().map(|x| ScalarOther(x.clone().into())).collect()),
            _ => None, // not an array
        }
    }
}

impl inst::Format {
    #[deprecated]
    pub fn flatten(&self) -> Vec<(String, inst::FieldValue)> {
        let prefix: String = self.to_string();

        self.flatten_sub(&prefix)
    }

    fn flatten_sub(&self, path: &str) -> Vec<(String, inst::FieldValue)> {
        let mut flattened = Vec::new();

        for field in &self.fields {
            let current_path = format!("{}/{}", path, field.name);
            if field.r#type.is_scalar() {
                flattened.extend(self.flatten_data_type(current_path, &field.value));
            } else {
                let vec_of_scalars = field.value.to_scalars().unwrap();

                for (index, value) in vec_of_scalars.into_iter().enumerate() {
                    let array_path = format!("{current_path}.{index:02}");
                    flattened.extend(self.flatten_data_type(array_path, &value));
                }
            }
        }

        flattened
    }

    // Helper method to flatten an inst::DataType, handling recursion if the type is OTHER
    #[allow(clippy::unused_self)]
    fn flatten_data_type(
        &self,
        path: String,
        value: &inst::FieldValue,
    ) -> Vec<(String, inst::FieldValue)> {
        use inst::FieldValue::*;
        match value {
            ScalarOther(nested_format) => {
                // Recursively flatten nested DataFormat
                nested_format.flatten_sub(&path)
            }
            _ => {
                // For any other scalar data type, just append the path and the data type
                vec![(path, value.clone())]
            }
        }
    }
}

impl def::TypeExpr {
    pub fn is_scalar(&self) -> bool {
        self.array_size.is_none()
    }

    pub fn is_array(&self) -> bool {
        self.array_size.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
/// A newtype wrapper for u8 when it represents character data.
pub struct CChar(pub u8);

impl From<u8> for CChar {
    fn from(byte: u8) -> Self {
        CChar(byte)
    }
}

impl From<CChar> for u8 {
    fn from(c: CChar) -> Self {
        c.0
    }
}

impl std::fmt::Display for CChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display ASCII directly, fallback for non-ASCII
        if self.0.is_ascii() {
            write!(f, "{}", self.0 as char)
        } else {
            write!(f, "\\x{:02X}", self.0)
        }
    }
}

pub trait CCharSlice {
    fn to_string_lossy(&self) -> String;
}

impl CCharSlice for [CChar] {
    fn to_string_lossy(&self) -> String {
        // Safe because CChar is repr(transparent) over u8 and slice is valid.
        // ⚠️ According to the ULOG spec, strings are not NULL terminated, so we just take the whole slice.
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(self.as_ptr() as *const u8, self.len()) };
        String::from_utf8_lossy(bytes).into_owned()
    }
}

