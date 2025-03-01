use std::fmt;
use std::fmt::Formatter;
use std::string::FromUtf8Error;

use crate::model::{def, inst, msg};

impl fmt::Display for inst::FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            inst::FieldValue::SCALAR(data_type) => {
                // Delegate to the inst::DataType's display implementation.
                write!(f, "{data_type}")
            }
            inst::FieldValue::ARRAY(data_types) => {
                if let Some(inst::BaseType::CHAR(_)) = data_types.first() {
                    // If it's an array of CHAR, join the characters into a string
                    let chars: String = data_types.iter()
                        .map(|dt| match dt {
                            inst::BaseType::CHAR(c) => *c,
                            _ => unreachable!(), // Should never happen since we checked the type.
                        })
                        .collect();
                    write!(f, "\"{chars}\"") // Wrap the string in quotes like a normal string.
                } else {
                    // For other types, format normally
                    let formatted_elements: Vec<String> = data_types.iter().map(|dt| format!("{dt}")).collect();
                    write!(f, "[{}]", formatted_elements.join(", "))
                }
            }
        }
    }
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
        let default_type =  self.get_default_type();
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

        write!(
            f,
            "{} ({}): {}",
            self.key,
            default_type_str,
            self.value
        )
    }
}

impl fmt::Display for msg::Info {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.key)?;
        
        match &self.value {
            inst::FieldValue::SCALAR(value) => {
                match value {
                    inst::BaseType::CHAR(ch) => {
                        write!(f, "{ch}")
                    }
                    inst::BaseType::UINT32(val) => {
                        if self.key.starts_with("ver_") && self.key.ends_with("_release") {
                            write!(f, "{val:#X}") // Special formatting for ver_*_release
                        } else {
                            write!(f, "{val}")
                        }
                    }
                    inst::BaseType::UINT8(val) => write!(f, "{val}"),
                    inst::BaseType::UINT16(val) => write!(f, "{val}"),
                    inst::BaseType::UINT64(val) => write!(f, "{val}"),
                    inst::BaseType::INT8(val) => write!(f, "{val}"),
                    inst::BaseType::INT16(val) => write!(f, "{val}"),
                    inst::BaseType::INT32(val) => write!(f, "{val}"),
                    inst::BaseType::INT64(val) => write!(f, "{val}"),
                    inst::BaseType::FLOAT(val) => write!(f, "{val}"),
                    inst::BaseType::DOUBLE(val) => write!(f, "{val}"),
                    inst::BaseType::BOOL(val) => write!(f, "{val}"),
                    // ⚠️ Should not occur. Only basic types are allwed in an Info message.
                    //    Implemented anyway just in case.
                    inst::BaseType::OTHER(val) => write!(f, "{val}"),
                }
            }
            inst::FieldValue::ARRAY(array) => {
                if array.is_empty() {
                    write!(f, "[]")
                } else { 
                    let is_string = matches!(array.first().unwrap(), inst::BaseType::CHAR(_));
                    
                    if is_string {
                        write!(f, "{}",
                               String::from_utf8(array
                                   .iter()
                                   .map(|x| {
                                       match x {
                                           inst::BaseType::CHAR(ch) => *ch as u8,
                                           _ => unreachable!()
                                       }
                                   })
                                   .collect())
                                   .map_err(|_err: FromUtf8Error| std::fmt::Error)?)
                    } else {
                        let formatted_elements: Vec<String> = array.iter().map(|e| format!("{e}")).collect();
                        write!(f, "[{}]", formatted_elements.join(", "))
                    }
                    
                } 
            }
        }
    }
}

impl fmt::Display for msg::MultiInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.key)?;

        match &self.value {
            inst::FieldValue::SCALAR(value) => {
                match value {
                    inst::BaseType::CHAR(ch) => {
                        write!(f, "{ch}")
                    }
                    inst::BaseType::UINT32(val) => {
                        if self.key.starts_with("ver_") && self.key.ends_with("_release") {
                            write!(f, "{val:#X}") // Special formatting for ver_*_release
                        } else {
                            write!(f, "{val}")
                        }
                    }
                    inst::BaseType::UINT8(val) => write!(f, "{val}"),
                    inst::BaseType::UINT16(val) => write!(f, "{val}"),
                    inst::BaseType::UINT64(val) => write!(f, "{val}"),
                    inst::BaseType::INT8(val) => write!(f, "{val}"),
                    inst::BaseType::INT16(val) => write!(f, "{val}"),
                    inst::BaseType::INT32(val) => write!(f, "{val}"),
                    inst::BaseType::INT64(val) => write!(f, "{val}"),
                    inst::BaseType::FLOAT(val) => write!(f, "{val}"),
                    inst::BaseType::DOUBLE(val) => write!(f, "{val}"),
                    inst::BaseType::BOOL(val) => write!(f, "{val}"),
                    // ⚠️ Should not occur. Only basic types are allwed in an Info message.
                    //    Implemented anyway just in case.
                    inst::BaseType::OTHER(val) => write!(f, "{val}"),
                }
            }
            inst::FieldValue::ARRAY(array) => {
                if array.is_empty() {
                    write!(f, "[]")
                } else {
                    let is_string = matches!(array.first().unwrap(), inst::BaseType::CHAR(_));

                    if is_string {
                        write!(f, "{}",
                               String::from_utf8(array
                                   .iter()
                                   .map(|x| {
                                       match x {
                                           inst::BaseType::CHAR(ch) => *ch as u8,
                                           _ => unreachable!()
                                       }
                                   })
                                   .collect())
                                   .map_err(|_err: FromUtf8Error| std::fmt::Error)?)
                    } else {
                        let formatted_elements: Vec<String> = array.iter().map(|e| format!("{e}")).collect();
                        write!(f, "[{}]", formatted_elements.join(", "))
                    }

                }
            }
        }
    }
}

impl fmt::Display for msg::Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Subscription: {} msg_id: {}", self.message_name, self.msg_id)
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
            None => write!(f, "{name}", name=self.name),
            Some(index) => write!(f, "{name}.{index:02}", name=self.name, index=index),
        }
    }
}

impl fmt::Display for inst::BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            inst::BaseType::UINT8(v) => write!(f, "{v}"),
            inst::BaseType::UINT16(v) => write!(f, "{v}"),
            inst::BaseType::UINT32(v) => write!(f, "{v}"),
            inst::BaseType::UINT64(v) => write!(f, "{v}"),
            inst::BaseType::INT8(v) => write!(f, "{v}"),
            inst::BaseType::INT16(v) => write!(f, "{v}"),
            inst::BaseType::INT32(v) => write!(f, "{v}"),
            inst::BaseType::INT64(v) => write!(f, "{v}"),
            inst::BaseType::FLOAT(v) => write!(f, "{v}"),
            inst::BaseType::DOUBLE(v) => write!(f, "{v}"),
            inst::BaseType::BOOL(v) => write!(f, "{v}"),
            inst::BaseType::CHAR(v) => write!(f, "{v}"), 
            inst::BaseType::OTHER(v) => write!(f, "{v}"),
        }
    }
}

impl fmt::Display for msg::LoggedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tag {
            None => {
                write!(
                    f,
                    "{} {}: {}",
                    self.level, self.timestamp, self.msg
                )
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