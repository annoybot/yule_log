use crate::errors::ULogError;
use crate::message_buf::MessageBuf;
use crate::model::def;

pub trait ParseFromBuf: Sized {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError>;
}

impl ParseFromBuf for u8 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_u8()
    }
}
impl ParseFromBuf for u16 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_u16()
    }
}
impl ParseFromBuf for u32 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_u32()
    }
}
impl ParseFromBuf for u64 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_u64()
    }
}
impl ParseFromBuf for i8 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_i8()
    }
}
impl ParseFromBuf for i16 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_i16()
    }
}
impl ParseFromBuf for i32 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_i32()
    }
}
impl ParseFromBuf for i64 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_i64()
    }
}
impl ParseFromBuf for f32 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_f32()
    }
}
impl ParseFromBuf for f64 {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        buf.take_f64()
    }
}
impl ParseFromBuf for bool {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        Ok(buf.take_u8()? != 0)
    }
}
impl ParseFromBuf for char {
    fn parse_from_buf(buf: &mut MessageBuf) -> Result<Self, ULogError> {
        Ok(buf.take_u8()? as char)
    }
}

pub fn parse_data_field<T: ParseFromBuf>(
    _field: &def::Field,
    message_buf: &mut MessageBuf,
) -> Result<T, ULogError> {
    T::parse_from_buf(message_buf)
}

pub fn parse_array<T, F>(
    array_size: usize,
    message_buf: &mut MessageBuf,
    mut parse_element: F,
) -> Result<Vec<T>, ULogError>
where
    F: FnMut(&mut MessageBuf) -> Result<T, ULogError>,
{
    let mut array = Vec::with_capacity(array_size);
    for _ in 0..array_size {
        array.push(parse_element(message_buf)?);
    }
    Ok(array)
}
