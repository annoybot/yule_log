use std::io::{self, Write};

use crate::model::msg::{LoggedData, UlogMessage};
use crate::model::{def, inst, msg};
use crate::parser::ULogMessageType;

// Define Encode trait
pub trait Encode {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;
}

// ------------------------ UlogMessage ------------------------

impl Encode for UlogMessage {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            UlogMessage::Header(header) => {
                // Header is special, just write its raw bytes
                writer.write_all(&header.to_bytes())?;
                Ok(())
            }
            other => {
                // Wrap in Message struct with size and type prefix
                let mut content_buf = Vec::new();
                other.encode_content(&mut content_buf)?;

                let msg_size = content_buf.len() as u16;
                writer.write_all(&msg_size.to_le_bytes())?;
                writer.write_all(&[u8::from(other.message_type())])?;
                writer.write_all(&content_buf)?;
                Ok(())
            }
        }
    }
}

impl UlogMessage {
    // Return the message type code as u8
    fn message_type(&self) -> ULogMessageType {
        match self {
            UlogMessage::FlagBits(_) => ULogMessageType::FLAG_BITS,
            UlogMessage::FormatDefinition(_) => ULogMessageType::FORMAT,
            UlogMessage::LoggedData(_) => ULogMessageType::DATA,
            UlogMessage::AddSubscription(_) => ULogMessageType::ADD_SUBSCRIPTION,
            UlogMessage::Info(_) => ULogMessageType::INFO,
            UlogMessage::MultiInfo(_) => ULogMessageType::INFO_MULTIPLE,
            UlogMessage::Parameter(_) => ULogMessageType::PARAMETER,
            UlogMessage::DefaultParameter(_) => ULogMessageType::PARAMETER_DEFAULT,
            UlogMessage::LoggedString(_) => ULogMessageType::LOGGING,
            UlogMessage::TaggedLoggedString(_) => ULogMessageType::LOGGING_TAGGED,
            UlogMessage::DropoutMark(_) => ULogMessageType::DROPOUT,
            UlogMessage::Unhandled { msg_type, .. } | UlogMessage::Ignored { msg_type, .. } => {
                ULogMessageType::from(*msg_type)
            }
            UlogMessage::Header(_) => unreachable!("Handled separately"),
        }
    }

    // Encode the inner content bytes without prefix (size/type)
    fn encode_content<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            UlogMessage::FlagBits(flag_bits) => flag_bits.encode(writer),
            UlogMessage::FormatDefinition(format) => format.encode(writer),
            UlogMessage::LoggedData(logged_data) => logged_data.encode(writer),
            UlogMessage::AddSubscription(sub) => sub.encode(writer),
            UlogMessage::Info(info) => info.encode(writer),
            UlogMessage::MultiInfo(info) => info.encode(writer),
            UlogMessage::Parameter(param) => param.encode(writer),
            UlogMessage::DefaultParameter(param) => param.encode(writer),
            UlogMessage::LoggedString(logged_string)
            | UlogMessage::TaggedLoggedString(logged_string) => logged_string.encode(writer),
            UlogMessage::DropoutMark(dropout) => dropout.encode(writer),
            UlogMessage::Unhandled {
                message_contents, ..
            } => writer.write_all(message_contents),
            UlogMessage::Ignored { msg_type: _ } => { Ok(())},
            UlogMessage::Header(_) => unreachable!("Handled separately"),
        }
    }
}

// ------------------------ msg::FlagBits ------------------------

impl Encode for msg::FlagBits {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.compat_flags)?;
        writer.write_all(&self.incompat_flags)?;
        for offset in &self.appended_data_offsets {
            writer.write_all(&offset.to_le_bytes())?;
        }
        Ok(())
    }
}

// ------------------------ msg::Subscription ------------------------

impl Encode for msg::Subscription {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[self.multi_id])?;
        writer.write_all(&self.msg_id.to_le_bytes())?;
        writer.write_all(self.message_name.as_bytes())?;
        Ok(())
    }
}

// ------------------------ LoggedData ------------------------

impl Encode for LoggedData {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.msg_id.to_le_bytes())?;
        self.data.encode(writer)
    }
}

// ------------------------ msg::Info ------------------------

impl Encode for msg::Info {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut key_bytes: Vec<u8> = self.r#type.encode_to_vec()?;
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        writer.write_all(&[key_bytes.len() as u8])?;
        writer.write_all(&key_bytes)?;
        self.value.encode(writer)?;
        Ok(())
    }
}

// ------------------------ msg::MultiInfo ------------------------

impl Encode for msg::MultiInfo {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[self.is_continued as u8])?;

        let mut key_bytes: Vec<u8> = self.r#type.encode_to_vec()?;
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        writer.write_all(&[key_bytes.len() as u8])?;
        writer.write_all(&key_bytes)?;
        self.value.encode(writer)?;
        Ok(())
    }
}

// ------------------------ msg::Parameter ------------------------

impl Encode for msg::Parameter {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut key_bytes: Vec<u8> = self.r#type.encode_to_vec()?;
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        writer.write_all(&[key_bytes.len() as u8])?;
        writer.write_all(&key_bytes)?;
        self.value.encode(writer)?;
        Ok(())
    }
}

// ------------------------ msg::DefaultParameter ------------------------

impl Encode for msg::DefaultParameter {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[self.default_types])?;

        let mut key_bytes: Vec<u8> = self.r#type.encode_to_vec()?;
        key_bytes.push(b' ');
        key_bytes.extend_from_slice(self.key.as_bytes());

        writer.write_all(&[key_bytes.len() as u8])?;
        writer.write_all(&key_bytes)?;
        self.value.encode(writer)?;
        Ok(())
    }
}

// ------------------------ msg::LoggedString ------------------------

impl Encode for msg::LoggedString {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[self.level as u8])?;

        if let Some(tag) = self.tag {
            writer.write_all(&tag.to_le_bytes())?;
        }

        writer.write_all(&self.timestamp.to_le_bytes())?;
        writer.write_all(self.msg.as_bytes())?;
        Ok(())
    }
}

// ------------------------ msg::Dropout ------------------------

impl Encode for msg::Dropout {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.duration.to_le_bytes())
    }
}

// ------------------------ msg::LogLevel ------------------------

impl Encode for msg::LogLevel {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[*self as u8])
    }
}

// ------------------------ def::Format ------------------------

impl Encode for def::Format {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.name.as_bytes())?;
        writer.write_all(b":")?;

        for field in &self.fields {
            field.encode(writer)?;
            writer.write_all(b";")?;
        }
        Ok(())
    }
}

// ------------------------ def::Field ------------------------

impl Encode for def::Field {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.r#type.encode(writer)?;
        writer.write_all(b" ")?;
        writer.write_all(self.name.as_bytes())?;
        Ok(())
    }
}

// ------------------------ def::TypeExpr ------------------------

impl Encode for def::TypeExpr {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.base_type.encode(writer)?;

        if let Some(array_size) = self.array_size {
            writer.write_all(b"[")?;
            writer.write_all(array_size.to_string().as_bytes())?;
            writer.write_all(b"]")?;
        }

        Ok(())
    }
}

// ------------------------ def::BaseType ------------------------

impl Encode for def::BaseType {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let type_string = self.to_string();
        writer.write_all(type_string.as_bytes())
    }
}

// ------------------------ inst::Format ------------------------

impl Encode for inst::Format {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for field in &self.fields {
            field.encode(writer)?;
        }
        Ok(())
    }
}

// ------------------------ inst::Field ------------------------

impl Encode for inst::Field {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.value.encode(writer)
    }
}

// ------------------------ inst::FieldValue ------------------------

impl Encode for inst::FieldValue {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use inst::FieldValue::*;

        match self {
            ScalarU8(v) => writer.write_all(&[*v]),
            ScalarU16(v) => writer.write_all(&v.to_le_bytes()),
            ScalarU32(v) => writer.write_all(&v.to_le_bytes()),
            ScalarU64(v) => writer.write_all(&v.to_le_bytes()),
            ScalarI8(v) => writer.write_all(&[*v as u8]),
            ScalarI16(v) => writer.write_all(&v.to_le_bytes()),
            ScalarI32(v) => writer.write_all(&v.to_le_bytes()),
            ScalarI64(v) => writer.write_all(&v.to_le_bytes()),
            ScalarF32(v) => writer.write_all(&v.to_le_bytes()),
            ScalarF64(v) => writer.write_all(&v.to_le_bytes()),
            ScalarBool(v) => writer.write_all(&[*v as u8]),
            ScalarChar(c) => writer.write_all(&[u8::from(*c)]),
            ScalarOther(fmt) => {
                for sub_field in &fmt.fields {
                    sub_field.encode(writer)?;
                }
                Ok(())
            }

            ArrayU8(arr) => writer.write_all(arr),
            ArrayU16(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayU32(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayU64(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayI8(arr) => {
                for v in arr {
                    writer.write_all(&[*v as u8])?;
                }
                Ok(())
            }
            ArrayI16(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayI32(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayI64(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayF32(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayF64(arr) => {
                for v in arr {
                    writer.write_all(&v.to_le_bytes())?;
                }
                Ok(())
            }
            ArrayBool(arr) => {
                for v in arr {
                    writer.write_all(&[*v as u8])?;
                }
                Ok(())
            }
            ArrayChar(arr) => {
                // Safe because CChar is  struct CChar(pub u8) with #[repr(transparent)].
                let bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(arr.as_ptr() as *const u8, arr.len())
                };
                writer.write_all(bytes)?;
                Ok(())
            }
            ArrayOther(arr) => {
                for fmt in arr {
                    for sub_field in &fmt.fields {
                        sub_field.encode(writer)?;
                    }
                }
                Ok(())
            }
        }
    }
}

// ------------------------ inst::ParameterValue ------------------------

impl Encode for inst::ParameterValue {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            inst::ParameterValue::INT32(val) => writer.write_all(&val.to_le_bytes()),
            inst::ParameterValue::FLOAT(val) => writer.write_all(&val.to_le_bytes()),
        }
    }
}

// ------------------------ Helpers ------------------------

impl<T: Encode> Encode for &T {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        (*self).encode(writer)
    }
}

// Convenience method to encode to Vec<u8>
trait EncodeToVec {
    fn encode_to_vec(&self) -> io::Result<Vec<u8>>;
}

impl<T: Encode> EncodeToVec for T {
    fn encode_to_vec(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }
}

// ------------------------ Tests ------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::parse_format;
    use crate::message_buf::MessageBuf;

    #[test]
    fn test_round_trip_format() -> io::Result<()> {
        let input = b"my_format:uint64_t timestamp;custom_type custom_field;bool is_happy;custom_type2[4] custom_field;uint8_t[8] pet_ids;";

        let message_buf = MessageBuf::from_vec(input.to_vec());

        let parsed_format = parse_format(message_buf).unwrap();

        let re_emitted_bytes = parsed_format.encode_to_vec()?;

        println!(
            "re_emitted_bytes: {:?}",
            String::from_utf8_lossy(&re_emitted_bytes)
        );

        assert_eq!(re_emitted_bytes, input);

        Ok(())
    }
}
