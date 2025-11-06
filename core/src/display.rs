use std::fmt;
use std::fmt::Formatter;

use crate::model::inst::FieldValue;
use crate::model::{def, inst, msg};

impl std::fmt::Display for inst::FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Scalars
            FieldValue::ScalarU8(v) => write!(f, "{v}"),
            FieldValue::ScalarU16(v) => write!(f, "{v}"),
            FieldValue::ScalarU32(v) => write!(f, "{v}"),
            FieldValue::ScalarU64(v) => write!(f, "{v}"),
            FieldValue::ScalarI8(v) => write!(f, "{v}"),
            FieldValue::ScalarI16(v) => write!(f, "{v}"),
            FieldValue::ScalarI32(v) => write!(f, "{v}"),
            FieldValue::ScalarI64(v) => write!(f, "{v}"),
            FieldValue::ScalarF32(v) => write!(f, "{v}"),
            FieldValue::ScalarF64(v) => write!(f, "{v}"),
            FieldValue::ScalarBool(v) => write!(f, "{v}"),
            FieldValue::ScalarChar(c) => write!(f, "'{c}'"),
            FieldValue::ScalarOther(fmt) => write!(f, "{{{fmt}}}"),

            // Arrays
            FieldValue::ArrayU8(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayU16(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayU32(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayU64(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayI8(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayI16(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayI32(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayI64(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayF32(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayF64(arr) => Ok(fmt_array(arr, f)?),
            FieldValue::ArrayBool(arr) => Ok(fmt_array(arr, f)?),

            FieldValue::ArrayChar(arr) => {
                let s: String = arr.iter().collect();
                write!(f, "\"{s}\"")
            }

            FieldValue::ArrayOther(arr) => {
                let formatted: Vec<String> = arr.iter().map(|f| format!("{{{f}}}")).collect();
                write!(f, "[{}]", formatted.join(", "))
            }
        }
    }
}

// helper for formatting arrays
fn fmt_array<T: fmt::Display>(arr: &[T], f: &mut Formatter<'_>) -> fmt::Result {
    let s = arr
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    write!(f, "[{s}]")
}

impl fmt::Display for msg::Parameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.value)
    }
}

impl fmt::Display for inst::ParameterValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            inst::ParameterValue::INT32(val) => write!(f, "{val}"),
            inst::ParameterValue::FLOAT(val) => write!(f, "{val}"),
        }
    }
}

impl fmt::Display for msg::DefaultParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let default_type = self.get_default_type();
        let mut default_type_str = String::with_capacity(25);

        if default_type.system_wide {
            default_type_str.push_str("SystemWide");
        }
        if default_type.configuration {
            if !default_type_str.is_empty() {
                default_type_str.push('|');
            }
            default_type_str.push_str("Configuration");
        }

        write!(f, "{} ({}): {}", self.key, default_type_str, self.value)
    }
}

impl fmt::Display for msg::Info {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.key)?;

        match &self.value {
            // Scalars
            FieldValue::ScalarU8(val) => write!(f, "{val}")?,
            FieldValue::ScalarU16(val) => write!(f, "{val}")?,
            FieldValue::ScalarU32(val) => {
                if self.key.starts_with("ver_") && self.key.ends_with("_release") {
                    write!(f, "{val:#X}")?;
                } else {
                    write!(f, "{val}")?;
                }
            }
            FieldValue::ScalarU64(val) => write!(f, "{val}")?,
            FieldValue::ScalarI8(val) => write!(f, "{val}")?,
            FieldValue::ScalarI16(val) => write!(f, "{val}")?,
            FieldValue::ScalarI32(val) => write!(f, "{val}")?,
            FieldValue::ScalarI64(val) => write!(f, "{val}")?,
            FieldValue::ScalarF32(val) => write!(f, "{val}")?,
            FieldValue::ScalarF64(val) => write!(f, "{val}")?,
            FieldValue::ScalarBool(val) => write!(f, "{val}")?,
            FieldValue::ScalarChar(ch) => write!(f, "{ch}")?,
            FieldValue::ScalarOther(fmt) => write!(f, "{{{fmt}}}")?,

            // Arrays
            FieldValue::ArrayU8(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU16(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI8(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI16(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayF32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayF64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayBool(arr) => fmt_array(arr, f)?,

            FieldValue::ArrayChar(arr) => {
                let s: String = arr.iter().collect();
                write!(f, "\"{s}\"")?;
            }

            FieldValue::ArrayOther(arr) => {
                let formatted: Vec<String> = arr.iter().map(|fmt| format!("{{{fmt}}}")).collect();
                write!(f, "[{}]", formatted.join(", "))?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for msg::MultiInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.key)?;

        match &self.value {
            // Scalars
            FieldValue::ScalarU8(val) => write!(f, "{val}")?,
            FieldValue::ScalarU16(val) => write!(f, "{val}")?,
            FieldValue::ScalarU32(val) => {
                if self.key.starts_with("ver_") && self.key.ends_with("_release") {
                    write!(f, "{val:#X}")?;
                } else {
                    write!(f, "{val}")?;
                }
            }
            FieldValue::ScalarU64(val) => write!(f, "{val}")?,
            FieldValue::ScalarI8(val) => write!(f, "{val}")?,
            FieldValue::ScalarI16(val) => write!(f, "{val}")?,
            FieldValue::ScalarI32(val) => write!(f, "{val}")?,
            FieldValue::ScalarI64(val) => write!(f, "{val}")?,
            FieldValue::ScalarF32(val) => write!(f, "{val}")?,
            FieldValue::ScalarF64(val) => write!(f, "{val}")?,
            FieldValue::ScalarBool(val) => write!(f, "{val}")?,
            FieldValue::ScalarChar(ch) => write!(f, "{ch}")?,
            FieldValue::ScalarOther(fmt) => write!(f, "{{{fmt}}}")?,

            // Arrays
            FieldValue::ArrayU8(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU16(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayU64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI8(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI16(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayI64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayF32(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayF64(arr) => fmt_array(arr, f)?,
            FieldValue::ArrayBool(arr) => fmt_array(arr, f)?,

            FieldValue::ArrayChar(arr) => {
                let s: String = arr.iter().collect();
                write!(f, "\"{s}\"")?;
            }

            FieldValue::ArrayOther(arr) => {
                let formatted: Vec<String> = arr.iter().map(|fmt| format!("{{{fmt}}}")).collect();
                write!(f, "[{}]", formatted.join(", "))?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for msg::Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Subscription: {} msg_id: {}",
            self.message_name, self.msg_id
        )
    }
}

impl fmt::Display for def::Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        //write!(f, "{}:", self.name)?;
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{field}")?;
        }
        Ok(())
    }
}

impl fmt::Display for def::Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.r#type, self.name)
    }
}

impl fmt::Display for def::TypeExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.array_size {
            None => {
                write!(f, "{}", self.base_type)
            }
            Some(array_size) => {
                write!(f, "{}[{}]", self.base_type, array_size)
            }
        }
    }
}

impl fmt::Display for def::BaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            def::BaseType::UINT8 => write!(f, "uint8_t"),
            def::BaseType::UINT16 => write!(f, "uint16_t"),
            def::BaseType::UINT32 => write!(f, "uint32_t"),
            def::BaseType::UINT64 => write!(f, "uint64_t"),
            def::BaseType::INT8 => write!(f, "int8_t"),
            def::BaseType::INT16 => write!(f, "int16_t"),
            def::BaseType::INT32 => write!(f, "int32_t"),
            def::BaseType::INT64 => write!(f, "int64_t"),
            def::BaseType::FLOAT => write!(f, "float"),
            def::BaseType::DOUBLE => write!(f, "double"),
            def::BaseType::BOOL => write!(f, "bool"),
            def::BaseType::CHAR => write!(f, "char"),
            def::BaseType::OTHER(s) => write!(f, "{s}"),
        }
    }
}

impl fmt::Display for inst::Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.multi_id_index {
            None => write!(f, "{name}", name = self.name),
            Some(index) => write!(f, "{name}.{index:02}", name = self.name, index = index),
        }
    }
}

impl fmt::Display for msg::LoggedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tag {
            None => {
                write!(f, "{} {}: {}", self.level, self.timestamp, self.msg)
            }
            Some(tag) => {
                write!(
                    f,
                    "{} Tag: {} {}: {}",
                    self.level, tag, self.timestamp, self.msg
                )
            }
        }
    }
}

impl fmt::Display for msg::LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match *self {
            msg::LogLevel::Emerg => "EMERG",
            msg::LogLevel::Alert => "ALERT",
            msg::LogLevel::Crit => "CRIT",
            msg::LogLevel::Err => "ERR",
            msg::LogLevel::Warning => "WARNING",
            msg::LogLevel::Notice => "NOTICE",
            msg::LogLevel::Info => "INFO",
            msg::LogLevel::Debug => "DEBUG",
        };
        write!(f, "{level_str}")
    }
}

impl fmt::Display for msg::Dropout {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.duration)
    }
}
