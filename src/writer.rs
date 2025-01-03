use crate::model::{def, inst, msg};
use crate::model::msg::{LoggedData, UlogMessage};
use crate::parser::ULogMessageType;

// msg
impl UlogMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        struct Message {
            msg_size: u16,
            msg_type: u8,
            content_bytes: Vec<u8>,
        }

        impl Message {
            fn new(msg_type: u8, content_bytes: Vec<u8>) -> Self {
                Message {
                    msg_size: content_bytes.len() as u16,
                    msg_type,
                    content_bytes,
                }
            }

            // Convert the message to bytes
            fn to_bytes(&self) -> Vec<u8> {
                let mut result = Vec::new();
                result.extend_from_slice(&self.msg_size.to_le_bytes()); // msg_size as u16 (little-endian)
                result.push(self.msg_type); // msg_type as u8
                result.extend_from_slice(&self.content_bytes); // The message content
                result
            }
        }

        let message = match self {
            UlogMessage::Header(header) => {
                // The header is not really a ULog message.
                // Return the raw byte representation only.
                return header.to_bytes();
            }
            UlogMessage::FlagBits(flag_bits) => {
                let content_bytes = flag_bits.to_bytes();
                Message::new(ULogMessageType::FLAG_BITS.into(), content_bytes)
            }
            UlogMessage::FormatDefinition(format) => {
                let content_bytes = format.to_bytes();
                Message::new(ULogMessageType::FORMAT.into(), content_bytes)
            }
            UlogMessage::LoggedData(logged_data) => {
                let content_bytes = logged_data.to_bytes();
                Message::new(ULogMessageType::DATA.into(), content_bytes)
            }
            UlogMessage::AddSubscription(sub) => {
                let content_bytes = sub.to_bytes();
                Message::new(ULogMessageType::ADD_SUBSCRIPTION.into(), content_bytes)
            }
            UlogMessage::Info(info) => {
                let content_bytes = info.to_bytes();
                Message::new(ULogMessageType::INFO.into(), content_bytes)
            }
            UlogMessage::MultiInfo(info) => {
                let content_bytes = info.to_bytes();
                Message::new(ULogMessageType::INFO_MULTIPLE.into(), content_bytes)
            }
            UlogMessage::Parameter(param) => {
                let content_bytes = param.to_bytes();
                Message::new(ULogMessageType::PARAMETER.into(), content_bytes)
            }
            UlogMessage::DefaultParameter(param) => {
                let content_bytes = param.to_bytes();
                Message::new(ULogMessageType::PARAMETER_DEFAULT.into(), content_bytes)
            }
            UlogMessage::LoggedString(logged_string) => {
                let content_bytes = logged_string.to_bytes();
                Message::new(ULogMessageType::LOGGING.into(), content_bytes)
            }
            UlogMessage::TaggedLoggedString(logged_string) => {
                let content_bytes = logged_string.to_bytes();
                Message::new(ULogMessageType::LOGGING_TAGGED.into(), content_bytes)
            }
            UlogMessage::DropoutMark(dropout) => {
                let content_bytes = dropout.to_bytes();
                Message::new(ULogMessageType::DROPOUT.into(), content_bytes)
            }
            UlogMessage::Unhandled { msg_type, message_contents } => {
                Message::new(*msg_type, message_contents.clone())
            }
            UlogMessage::Ignored { msg_type, message_contents } => {
                Message::new(*msg_type, message_contents.clone())
            }
        };

        message.to_bytes()
    }
}


impl msg::FlagBits {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 * 8 + 3 * 8); // compat_flags + incompat_flags + appended_data_offsets

        bytes.extend_from_slice(&self.compat_flags);          // 8 bytes for compat_flags
        bytes.extend_from_slice(&self.incompat_flags);       // 8 bytes for incompat_flags
        for &offset in &self.appended_data_offsets {         // 3 * 8 bytes for appended_data_offsets
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        bytes
    }
}


impl msg::Subscription {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(3 + self.message_name.len());

        // Emit the subscription data
        result.push(self.multi_id);
        result.extend(&self.msg_id.to_le_bytes());
        result.extend(self.message_name.as_bytes());

        result
    }
}

impl LoggedData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(self.msg_id).to_le_bytes());
        bytes.extend_from_slice(&self.data.to_bytes());

        bytes
    }
}

impl msg::Info {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let mut key_bytes = self.r#type.to_bytes();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());


        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&self.value.to_bytes());

        bytes
    }
}

impl msg::MultiInfo {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(u8::from(self.is_continued));

        let mut key_bytes = self.r#type.to_bytes();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&self.value.to_bytes());

        bytes
    }
}

impl msg::Parameter {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Parameter values are always scalars, hence: is_array == false.
        let mut key_bytes = self.r#type.to_bytes();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&self.value.to_bytes());

        bytes
    }
}

impl msg::DefaultParameter {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.default_types);

        let mut key_bytes = self.r#type.to_bytes();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);

        bytes.extend_from_slice(&self.value.to_bytes());

        bytes
    }
}


impl msg::LoggedString {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.level as u8);

        if let Some(tag) = self.tag {
            bytes.extend_from_slice(&tag.to_le_bytes());
        }

        // Serialize timestamp as a little-endian u64
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(self.msg.as_bytes());

        bytes
    }
}

impl msg::Dropout {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.duration.to_le_bytes());

        bytes
    }
}

impl msg::LogLevel {
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

// def

impl def::Format {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Emit the message name (String) as bytes
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(b':'); // Emit the colon separator after the message name

        // Emit the bytes for each field with a trailing semicolon
        for  field in &self.fields {
            // Convert the field to bytes
            bytes.extend_from_slice(&field.to_bytes());
            bytes.push(b';'); // Trailing semicolon after each field
        }

        bytes
    }
}

impl def::Field {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Emit the base type (ex. uint8_t)
        bytes.extend_from_slice(&self.r#type.base_type.to_bytes());

        match self.r#type.array_size {
            None => (),
            Some(array_size) => {
                bytes.push(b'[');
                bytes.extend_from_slice(array_size.to_string().as_bytes());
                bytes.push(b']');
            }
        }
        
        bytes.push(b' ');

        // Append the field name
        bytes.extend_from_slice(self.name.as_bytes());

        bytes
    }
}

impl def::TypeExpr {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Emit the base type (ex. uint8_t)
        bytes.extend_from_slice(&self.base_type.to_bytes());

        match self.array_size {
            None => (),
            Some(array_size) => {
                bytes.push(b'[');
                bytes.extend_from_slice(array_size.to_string().as_bytes());
                bytes.push(b']');
            }
        }

        bytes
    }
}


impl def::BaseType {
    pub fn to_bytes(&self) -> Vec<u8> {
        let type_string = self.to_string();
        type_string.into_bytes()
    }
}

// inst

impl inst::Format {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes:Vec<u8> = Vec::new();

        // Serialise fields only
        for field in &self.fields {
            bytes.extend(field.to_bytes());
        }

        bytes
    }
}

impl inst::Field {
    pub fn to_bytes(&self) -> Vec<u8> {
        // Serilalise value only
        self.value.to_bytes()
    }
}

impl inst::FieldValue {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            inst::FieldValue::SCALAR(base_type) => base_type.to_bytes(),
            inst::FieldValue::ARRAY(arr) => {
                let mut bytes = Vec::new();
                for value in arr {
                    bytes.extend(value.to_bytes());
                }
                bytes
            }
        }
    }
}

impl inst::BaseType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            inst::BaseType::UINT8(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::UINT16(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::UINT32(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::UINT64(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::INT8(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::INT16(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::INT32(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::INT64(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::FLOAT(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::DOUBLE(v) => v.to_le_bytes().to_vec(),
            inst::BaseType::BOOL(v) => if *v { vec![ 1u8 ] } else { vec![0u8] },
            inst::BaseType::CHAR(v) => vec![*v as u8],
            inst::BaseType::OTHER(format) => format.to_bytes(),
        }
    }
}

impl inst::ParameterValue {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            inst::ParameterValue::INT32(val) => val.to_le_bytes().to_vec(),
            inst::ParameterValue::FLOAT(val) => val.to_le_bytes().to_vec(),
        }
    }
}



#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use crate::formats::parse_format;
    use crate::message_buf::MessageBuf;
    use crate::model::def;
    use crate::parser::ULogParser;

    impl<R: std::io::Read> ULogParser<R> {
        pub fn insert_format(&mut self, message_name: &str, format: def::Format) {
            self.formats.insert(message_name.to_string(), format);
        }
    }
    
    #[test]
    fn test_round_trip_format() {
        let input = b"my_format:uint64_t timestamp;custom_type custom_field;bool is_happy;custom_type2[4] custom_field;uint8_t[8] pet_ids;";
        let message_buf = MessageBuf::from_vec(input.to_vec());

        let parsed_format = parse_format(message_buf).unwrap();

        let re_emitted_bytes = parsed_format.to_bytes();

        println!("re_emitted_bytes: {:?}", String::from_utf8(re_emitted_bytes.clone()).unwrap());

        assert_eq!(re_emitted_bytes, input);
    }
}