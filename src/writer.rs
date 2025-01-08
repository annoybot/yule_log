use crate::model::{def, inst, msg};
use crate::model::def::Format;
use crate::model::msg::{LoggedData, UlogMessage};
use crate::parser::ULogMessageType;

// msg
impl From<UlogMessage> for Vec<u8> {
    fn from(ulog_msg: UlogMessage) -> Self {
        (&ulog_msg).into()
    }
}

impl From<&UlogMessage> for Vec<u8>  {
    fn from(ulog_msg: &UlogMessage) -> Self {
        struct Message {
            msg_size: u16,
            msg_type: u8,
            content_bytes: Vec<u8>,
        }

        impl Message {
            fn new<T: Into<Vec<u8>>>(msg_type: u8, content_bytes: T) -> Self {
                let content_bytes = content_bytes.into();
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

        let message = match ulog_msg {
            UlogMessage::Header(header) => {
                // The header is not really a ULog message.
                // Return the raw byte representation only.
                return header.to_bytes();
            }
            UlogMessage::FlagBits(flag_bits) => {
                Message::new(ULogMessageType::FLAG_BITS.into(), flag_bits)
            }
            UlogMessage::FormatDefinition(format) => {
                Message::new(ULogMessageType::FORMAT.into(), format)
            }
            UlogMessage::LoggedData(logged_data) => {
                Message::new(ULogMessageType::DATA.into(), logged_data)
            }
            UlogMessage::AddSubscription(sub) => {
                Message::new(ULogMessageType::ADD_SUBSCRIPTION.into(), sub)
            }
            UlogMessage::Info(info) => {
                Message::new(ULogMessageType::INFO.into(), info)
            }
            UlogMessage::MultiInfo(info) => {
                Message::new(ULogMessageType::INFO_MULTIPLE.into(), info)
            }
            UlogMessage::Parameter(param) => {
                Message::new(ULogMessageType::PARAMETER.into(), param)
            }
            UlogMessage::DefaultParameter(param) => {
                Message::new(ULogMessageType::PARAMETER_DEFAULT.into(), param)
            }
            UlogMessage::LoggedString(logged_string) => {
                Message::new(ULogMessageType::LOGGING.into(), logged_string)
            }
            UlogMessage::TaggedLoggedString(logged_string) => {
                Message::new(ULogMessageType::LOGGING_TAGGED.into(), logged_string)
            }
            UlogMessage::DropoutMark(dropout) => {
                Message::new(ULogMessageType::DROPOUT.into(), dropout)
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

impl From<msg::FlagBits> for Vec<u8> {
    fn from(flag_bits: msg::FlagBits) -> Self {
        (&flag_bits).into()
    }
}

impl From<&msg::FlagBits> for Vec<u8> {
    fn from(flag_bits: &msg::FlagBits) -> Self {
        let mut bytes = Vec::with_capacity(8 * 8 + 3 * 8); // compat_flags + incompat_flags + appended_data_offsets

        bytes.extend_from_slice(&flag_bits.compat_flags);         // 8 bytes for compat_flags
        bytes.extend_from_slice(&flag_bits.incompat_flags);       // 8 bytes for incompat_flags
        for &offset in &flag_bits.appended_data_offsets {   // 3 * 8 bytes for appended_data_offsets
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        bytes
    }
}

impl From<msg::Subscription> for Vec<u8> {
    fn from(subscription: msg::Subscription) -> Self {
        (&subscription).into()
    }
}

impl From<&msg::Subscription> for Vec<u8> {
    fn from(subscription: &msg::Subscription) -> Self {
        let mut bytes = Vec::with_capacity(3 + subscription.message_name.len());

        // Emit the subscription data
        bytes.push(subscription.multi_id);
        bytes.extend(&subscription.msg_id.to_le_bytes());
        bytes.extend(subscription.message_name.as_bytes());

        bytes
    }
}


impl From<LoggedData> for Vec<u8> {
    fn from(logged_data: LoggedData) -> Self {
        logged_data.into()
    }
}

impl From<&LoggedData> for Vec<u8>  {
    fn from(logged_data: &LoggedData) -> Self {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(logged_data.msg_id).to_le_bytes());
        bytes.extend_from_slice(&Vec::from(&logged_data.data));

        bytes
    }
}

impl From<msg::Info> for Vec<u8> {
    fn from(info: msg::Info) -> Self {
        (&info).into()
    }
}

impl From<&msg::Info> for Vec<u8>  {
    fn from(info: &msg::Info) -> Self {
        let mut bytes = Vec::new();

        let mut key_bytes:Vec<u8> = (&info.r#type).into();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(info.key.as_bytes());


        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&Vec::from(&info.value));

        bytes
    }
}

impl From<msg::MultiInfo> for Vec<u8>  {
    fn from(multi_info: msg::MultiInfo) -> Self {
        (&multi_info).into()
    }
}

impl From<&msg::MultiInfo> for Vec<u8>  {
    fn from(multi_info: &msg::MultiInfo) -> Self {
        let mut bytes = Vec::new();

        bytes.push(u8::from(multi_info.is_continued));

        let mut key_bytes:Vec<u8> = (&multi_info.r#type).into();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(multi_info.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&Vec::from(&multi_info.value));

        bytes
    }
}

impl From<msg::Parameter> for Vec<u8> {
    fn from(param: msg::Parameter) -> Self {
        (&param).into()
    }
}

impl From<&msg::Parameter> for Vec<u8>  {
    fn from(param: &msg::Parameter) -> Self {
        let mut bytes = Vec::new();

        // Parameter values are always scalars, hence: is_array == false.
        let mut key_bytes:Vec<u8> = (&param.r#type).into();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(param.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);
        bytes.extend_from_slice(&Vec::from(&param.value));

        bytes
    }
}

impl From<msg::DefaultParameter> for Vec<u8> {
    fn from(default_parameter: msg::DefaultParameter) -> Self {
        (&default_parameter).into()
    }
}

impl From<&msg::DefaultParameter> for Vec<u8>  {
    fn from(default_parameter: &msg::DefaultParameter) -> Self {
        let mut bytes = Vec::new();

        bytes.push(default_parameter.default_types);

        let mut key_bytes:Vec<u8> = (&default_parameter.r#type).into();
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(default_parameter.key.as_bytes());

        bytes.push(key_bytes.len() as u8);
        bytes.extend(key_bytes);

        bytes.extend_from_slice(&Vec::from(&default_parameter.value));

        bytes
    }
}

impl From<msg::LoggedString> for Vec<u8> {
    fn from(logged_string: msg::LoggedString) -> Self {
        (&logged_string).into()
    }
}

impl From<&msg::LoggedString> for Vec<u8>  {
    fn from(logged_string: &msg::LoggedString) -> Self {
        let mut bytes = Vec::new();

        bytes.push(logged_string.level as u8);

        if let Some(tag) = logged_string.tag {
            bytes.extend_from_slice(&tag.to_le_bytes());
        }

        // Serialize timestamp as a little-endian u64
        bytes.extend_from_slice(&logged_string.timestamp.to_le_bytes());
        bytes.extend_from_slice(logged_string.msg.as_bytes());

        bytes
    }
}


impl From<msg::Dropout> for Vec<u8> {
    fn from(dropout: msg::Dropout) -> Self {
        (&dropout).into()
    }
}

impl From<&msg::Dropout> for Vec<u8>  {
    fn from(dropout: &msg::Dropout) -> Self {
        let mut bytes = Vec::new();

        bytes.extend(dropout.duration.to_le_bytes());

        bytes
    }
}

impl From<msg::LogLevel> for Vec<u8> {
    fn from(log_level: msg::LogLevel) -> Self {
        (&log_level).into()
    }
}

impl From<&msg::LogLevel> for Vec<u8>  {
    fn from(log_level: &msg::LogLevel) -> Self {
        vec![*log_level as u8]
    }
}

// def

impl From<def::Format> for Vec<u8> {
    fn from(format: Format) -> Self {
        (&format).into()
    }
}

impl From<&def::Format> for Vec<u8>  {
    fn from(format: &def::Format) -> Self {
        let mut bytes = Vec::new();

        // Emit the message name (String) as bytes
        bytes.extend_from_slice(format.name.as_bytes());
        bytes.push(b':'); // Emit the colon separator after the message name

        // Emit the bytes for each field with a trailing semicolon
        for  field in &format.fields {
            // Convert the field to bytes
            bytes.extend_from_slice(&Vec::from(field));
            bytes.push(b';'); // Trailing semicolon after each field
        }

        bytes
    }
}

impl From<def::Field> for Vec<u8> {
    fn from(field: def::Field) -> Self {
        (&field).into()
    }
}

impl From<&def::Field> for Vec<u8>  {
    fn from(field: &def::Field) -> Self {
        let mut bytes = Vec::new();

        // Emit the base type (ex. uint8_t)
        bytes.extend_from_slice(&Vec::from(&field.r#type.base_type));

        match field.r#type.array_size {
            None => (),
            Some(array_size) => {
                bytes.push(b'[');
                bytes.extend_from_slice(array_size.to_string().as_bytes());
                bytes.push(b']');
            }
        }
        
        bytes.push(b' ');

        // Append the field name
        bytes.extend_from_slice(field.name.as_bytes());

        bytes
    }
}

impl From<def::TypeExpr> for Vec<u8> {
    fn from(type_expr: def::TypeExpr) -> Self {
        (&type_expr).into()
    }
}

impl From<&def::TypeExpr> for Vec<u8>  {
    fn from(type_expr: &def::TypeExpr) -> Self {
        let mut bytes = Vec::new();

        // Emit the base type (ex. uint8_t)
        bytes.extend_from_slice(&Vec::from(&type_expr.base_type));

        match type_expr.array_size {
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

impl From<def::BaseType> for Vec<u8> {
    fn from(base_type: def::BaseType) -> Self {
        (&base_type).into()
    }
}

impl From<&def::BaseType> for Vec<u8>  {
    fn from(base_type: &def::BaseType) -> Self {
        let type_string = base_type.to_string();
        type_string.into_bytes()
    }
}

// inst

impl From<inst::Format> for Vec<u8> {
    fn from(format: inst::Format) -> Self {
        (&format).into()
    }
}

impl From<&inst::Format> for Vec<u8>  {
    fn from(format: &inst::Format) -> Self {
        let mut bytes:Vec<u8> = Vec::new();

        // Serialise fields only
        for field in &format.fields {
            bytes.extend(Vec::from(field));
        }

        bytes
    }
}

impl From<inst::Field> for Vec<u8> {
    fn from(field: inst::Field) -> Self {
        (&field).into()
    }
}

impl From<&inst::Field> for Vec<u8>  {
    fn from(field: &inst::Field) -> Self {
        // Serilalise value only
        Vec::from(&field.value)
    }
}

impl From<inst::FieldValue> for Vec<u8> {
    fn from(field_value: inst::FieldValue) -> Self {
        (&field_value).into()
    }
}

impl From<&inst::FieldValue> for Vec<u8>  {
    fn from(field_value: &inst::FieldValue) -> Self {
        match field_value {
            inst::FieldValue::SCALAR(base_type) => Vec::from(base_type),
            inst::FieldValue::ARRAY(arr) => {
                let mut bytes = Vec::new();
                for value in arr {
                    bytes.extend(Vec::from(value));
                }
                bytes
            }
        }
    }
}

impl From<inst::BaseType> for Vec<u8> {
    fn from(base_type: inst::BaseType) -> Self {
        (&base_type).into()
    }
}

impl From<&inst::BaseType> for Vec<u8>  {
    fn from(base_type: &inst::BaseType) -> Self {
        match base_type {
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
            inst::BaseType::OTHER(format) => format.into(),
        }
    }
}

impl From<inst::ParameterValue> for Vec<u8> {
    fn from(parameter_value: inst::ParameterValue) -> Self {
        (&parameter_value).into()
    }
}

impl From<&inst::ParameterValue> for Vec<u8>  {
    fn from(parameter_value: &inst::ParameterValue) -> Self {
        match parameter_value {
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

        let re_emitted_bytes:Vec<u8> = parsed_format.into();

        println!("re_emitted_bytes: {:?}", String::from_utf8(re_emitted_bytes.clone()).unwrap());

        assert_eq!(re_emitted_bytes, input);
    }
}