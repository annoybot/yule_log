#![allow(non_camel_case_types)]

use byteorder::{ByteOrder, LittleEndian};
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::datastream::DataStream;
use crate::formats::parse_format;
use core::mem::size_of;

#[derive(Error, Debug)]
pub enum ULogError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 Decoding Error: {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Format Error")]
    FormatError,

    #[error("Unknown Parameter Type")]
    UnknownParameterType,

    #[error("Invalid Header")]
    InvalidHeader,

    #[error("Invalid Definitions")]
    InvalidDefinitions,

    #[error("Unexpected End of File")]
    UnexpectedEndOfFile,

    #[error("Parse error Error: {0}")]
    ParseError(String)
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FormatType {
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

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub field_name: String,
    pub(crate) type_: FormatType,
    pub array_size: usize,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub value: ParameterValue,
    pub val_type: FormatType,
}

#[derive(Debug)]
pub enum ParameterValue { Int(i32), Real(f32) }

#[derive(Debug, Clone, PartialEq)]
pub struct Format {
    pub name: String,
    pub fields: Vec<Field>,
    pub padding: usize,
}

#[derive(Debug)]
pub struct MessageLog {
    pub level: char,
    pub timestamp: u64,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub msg_id: u16,
    pub multi_id: u8,
    pub message_name: String,
    pub format: Option<Format>,
}

#[derive(Debug)]
pub struct Timeseries {
    pub timestamps: Vec<u64>,
    pub data: Vec<(String, Vec<f64>)>,
}

pub struct ULogParser<R:Read> {
    file_start_time: u64,
    parameters: Vec<Parameter>,
    overridden_params: HashSet<String>,
    pub(crate) formats: HashMap<String, Format>,
    info: HashMap<String, String>,
    subscriptions: HashMap<u16, Subscription>,
    message_name_with_multi_id: HashSet<String>,
    message_logs: Vec<MessageLog>,
    _phantom: PhantomData<R>,

}

impl <R: Read>ULogParser <R> {
    pub fn new(mut datastream: DataStream<R>, timeseries_map: &mut HashMap<String, Timeseries>) -> Result<ULogParser<R>, ULogError> {
        let mut parser = ULogParser {
            file_start_time: 0,
            parameters: Vec::new(),
            overridden_params: HashSet::new(),
            formats: HashMap::new(),
            info: HashMap::new(),
            subscriptions: HashMap::new(),
            message_name_with_multi_id: HashSet::new(),
            message_logs: Vec::new(),
            _phantom: PhantomData,
        };

        if !parser.read_file_header(&mut datastream)? {
            return Err(ULogError::InvalidHeader);
        }

        // read_file_definitions will return the message header of the first logged message.
        let mut message_header = parser.read_file_definitions(&mut datastream)?;

        while datastream.should_read() {

            // Allocate a buffer for the message using the message size specified in the header.
            let mut message:Vec<u8> = vec![0; message_header.msg_size as usize];
            datastream.read_exact(&mut message)?;

            //message[message_header.msg_size as usize] = 0;

            match message_header.msg_type {
                ULogMessageType::ADD_LOGGED_MSG => {
                    let message_name = String::from_utf8(message[3..message_header.msg_size as usize].to_owned())?;

                    let format = match parser.formats.get(&message_name) {
                        None => { None }
                        Some(format) => { Some((*format).clone()) }
                    };

                    let sub = Subscription {
                        msg_id: LittleEndian::read_u16(&message[1..3]),
                        multi_id: message[0],
                        message_name,
                        format,
                    };

                    parser.subscriptions.insert(sub.msg_id, sub.clone());

                    if sub.multi_id > 0 {
                        parser.message_name_with_multi_id.insert(sub.message_name.clone());
                    }
                }
                ULogMessageType::REMOVE_LOGGED_MSG => {
                    let msg_id = LittleEndian::read_u16(&message[0..2]);
                    parser.subscriptions.remove(&msg_id);
                }
                ULogMessageType::DATA => {
                    let msg_id = LittleEndian::read_u16(&message[0..2]);
                    if let Some(sub) = parser.subscriptions.get(&msg_id) {
                        parser.parse_data_message(sub, &message[2..], timeseries_map);
                    }
                }
                ULogMessageType::LOGGING => {
                    let mut msg = MessageLog {
                        level: message[0] as char,
                        timestamp: LittleEndian::read_u64(&message[1..9]),
                        msg: String::new(),
                    };
                    msg.msg = String::from_utf8(message[9..message_header.msg_size as usize].to_vec())?;
                    parser.message_logs.push(msg);
                }
                _ => {}
            }

            // Read the next message header.  Receiving None indicates EOF.
            message_header = match parser.read_message_header(&mut datastream)? {
                None => break,
                Some(header) => header
            }
        }

        log::debug!("Timeseries {:?}", timeseries_map);

        Ok(parser)
    }

    fn read_message_header(&mut self, datastream: &mut DataStream<R>) -> Result<Option<ULogMessageHeader>, ULogError> {

        let msg_size = datastream.read_u16()?;

        // ⚠️This is the only place where we check for EOF when calling a datastream read method.
        // If we encounter EOF anywhere else, it counts as a ture 'Unexpected EOF' and is treated as an error.
        if datastream.eof {
            return Ok(None);
        }

        let msg_type = ULogMessageType::from(datastream.read_u8()?);
        log::trace!("MSG HEADER: {} {:?}", msg_size, msg_type);

        Ok( Some(ULogMessageHeader { msg_size, msg_type } ))
    }

    fn parse_data_message(&self, sub: &Subscription, message: &[u8], timeseries_map: &mut HashMap<String, Timeseries>) {
        let message = message.to_vec();

        let mut ts_name = sub.message_name.clone();

        // Append two `.00, .01, .02` to multi id field names.
        if self.message_name_with_multi_id.contains(&ts_name) {
            ts_name.push_str(&format!(".{:02}", sub.multi_id));
        }

        let timeseries = timeseries_map.entry(ts_name.clone())
            .or_insert_with(|| self.create_timeseries(sub.format.as_ref().unwrap()));

        let time_val = LittleEndian::read_u64(&message[0..8]);
        timeseries.timestamps.push(time_val);

        let mut index = 0;
        
        // Pass the message down, skipping the first eight bytes which are taken up by the timestamp.
        self.parse_simple_data_message(timeseries, sub.format.as_ref().unwrap(), &message[8..], &mut index);
    }

    fn parse_simple_data_message<'a>(&'a self, timeseries: &mut Timeseries, format: &Format, mut message: &'a [u8], index: &mut usize) -> &'a [u8] {
        // Utility fn to extract a value from `message` and to advance the buffer pointer past it.
        fn extract_and_advance<F>( message: &mut &[u8], advance_by: usize, extractor: F, ) -> f64
            where  F: Fn(&[u8]) -> f64,
        {
            let value:f64 = extractor(message);

            *message = &message[advance_by..];

            return value;
        }

        for (i,field) in format.fields.iter().enumerate() {
            let is_last_field = i == format.fields.len() - 1;

            // Skip _padding messages when they appear at the end of the list of fields.
            if field.field_name.starts_with("_padding")  {
                if is_last_field {
                    continue
                } else {
                    message = &message[field.array_size..];
                }
            }

            // ⚠️This is a hack to get around the fact that the timestamp has already been read in parse_data_message()
            // The PlotJuggler code makes an unsupported assumption that the timestamp is always the first field.
            //if field.field_name == "timestamp" {
            //    continue;
            //}

            for _ in 0..field.array_size {
                let value = match &field.type_ {
                    FormatType::BOOL => { extract_and_advance(&mut message, size_of::<u8>(), |message| { if message[0] != 0 { 1.0 } else { 0.0 } }) },
                    FormatType::CHAR => { extract_and_advance(&mut message, size_of::<u8>(), |message| { message[0] as f64 }) },
                    FormatType::UINT8 => { extract_and_advance(&mut message, size_of::<u8>(), |message| { message[0] as f64 }) },
                    FormatType::INT8 => { extract_and_advance(&mut message, size_of::<u8>(), |message| { message[0] as f64 }) },
                    FormatType::UINT16 => { extract_and_advance(&mut message, size_of::<u16>(), |message| { LittleEndian::read_u16(message) as f64 }) },
                    FormatType::INT16 => { extract_and_advance(&mut message, size_of::<i16>(), |message| { LittleEndian::read_i16(message) as f64 }) },
                    FormatType::UINT32 => { extract_and_advance(&mut message, size_of::<u32>(), |message| { LittleEndian::read_u32(message) as f64 }) },
                    FormatType::INT32 => { extract_and_advance(&mut message, size_of::<i32>(), |message| { LittleEndian::read_i32(message) as f64 }) },
                    FormatType::UINT64 => { extract_and_advance(&mut message, size_of::<u64>(), |message| { LittleEndian::read_u64(message) as f64 }) },
                    FormatType::INT64 => { extract_and_advance(&mut message, size_of::<i64>(), |message| { LittleEndian::read_i64(message) as f64 }) },
                    FormatType::FLOAT => { extract_and_advance(&mut message, size_of::<f32>(), |message| { LittleEndian::read_f32(message) as f64 }) },
                    FormatType::DOUBLE => { extract_and_advance(&mut message, size_of::<f64>(), |message| { LittleEndian::read_f64(message) as f64 }) },
                    FormatType::OTHER(type_id) => {
                        let child_format = self.formats.get(type_id).unwrap();
                        // Commenting or uncommenting this line makes no difference to the data collected in the timeseries map. Why?
                        message = &message[8..]; // Skip over timestamp.
                        message = self.parse_simple_data_message(timeseries, child_format, message, index);
                        continue;
                    }
                };

                if !field.type_.is_other() {
                    timeseries.data[*index].1.push(value);
                    *index += 1;
                }
            }
        }

        message
    }

    fn create_timeseries(&self, format: &Format) -> Timeseries {
        let mut timeseries = Timeseries {
            timestamps: Vec::new(),
            data: Vec::new(),
        };

        fn append_vector(format: &Format, prefix: &str, timeseries: &mut Timeseries, formats: &HashMap<String, Format>) {
            for field in &format.fields {
                if field.field_name.starts_with("_padding") {
                    continue;
                }

                let new_prefix = format!("{}/{}", prefix, field.field_name);
                for i in 0..field.array_size {
                    let array_suffix = if field.array_size > 1 {
                        format!(".{:02}", i)
                    } else {
                        String::new()
                    };

                    match &field.type_ {
                        FormatType::OTHER(type_name) => {
                            let child_format = formats.get(type_name).unwrap();
                            append_vector(child_format, &format!("{}{}", new_prefix, array_suffix), timeseries, formats);
                        }
                        _ => {
                            timeseries.data.push((format!("{}{}", new_prefix, array_suffix), Vec::new()));
                        }
                    }
                }
            }
        }

        append_vector(format, format.name.as_str(), &mut timeseries, &self.formats);

        timeseries
    }

    fn read_file_header(&mut self, datastream: &mut DataStream<R>) -> Result<bool, ULogError> {
        let mut msg_header = [0; 16];
        datastream.read_exact(&mut msg_header)?;

        self.file_start_time = LittleEndian::read_u64(&msg_header[8..16]);

        // Verify it's a ULog file
        const MAGIC: [u8; 7] = [b'U', b'L', b'o', b'g', 0x01, 0x12, 0x35];
        if &msg_header[0..7] != MAGIC {
            return Ok(false);
        }

        Ok(true)
    }

    fn read_file_definitions(&mut self, datastream: &mut DataStream<R>) -> Result<ULogMessageHeader, ULogError> {
        loop {
            let message_header = match self.read_message_header(datastream)? {
                None => { return Err(ULogError::UnexpectedEndOfFile) }
                Some(value) => value
            };

            match message_header.msg_type {
                ULogMessageType::FLAG_BITS => {
                    self.read_flag_bits(datastream, message_header.msg_size)?;
                }
                ULogMessageType::FORMAT => {
                    let mut format = parse_format(datastream, message_header.msg_size)?;

                    // ⚠️ Remove the timestamp field.  We don't end up needing it in the Timeseries data structure.
                    format.fields = format.fields.into_iter()
                        .filter(|f| f.field_name != "timestamp")
                        .collect();

                    self.formats.insert(format.name.clone(), format);
                }
                ULogMessageType::PARAMETER => {
                    self.read_parameter(datastream, message_header.msg_size)?;
                }
                ULogMessageType::ADD_LOGGED_MSG => {
                    // Return the message header, of the first logged message.
                    return Ok(message_header);
                }
                ULogMessageType::INFO => {
                    self.read_info(datastream, message_header.msg_size)?;
                }
                ULogMessageType::INFO_MULTIPLE | ULogMessageType::PARAMETER_DEFAULT => {
                    datastream.skip(message_header.msg_size as usize)?;
                }
                ULogMessageType::UNKNOWN => {
                    //log::debug!("Warning: Unknown ULogMessageType. Skipping.");
                    //continue;
                    //return Err(ULogError::FormatError);
                    panic!("Warning: Unknown ULogMessageType.");
                },
                _ => {
                    return Err(ULogError::FormatError);
                }
            }
        }
    }

    fn read_flag_bits(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        if msg_size != 40 {
            return Err(ULogError::FormatError);
        }

        let mut message = vec![0; msg_size as usize];
        datastream.read_exact(&mut message)?;

        let incompat_flags = &message[8..16];
        let contains_appended_data = incompat_flags[0] & ULOG_INCOMPAT_FLAG0_DATA_APPENDED_MASK != 0;
        let has_unknown_incompat_bits = incompat_flags.iter().skip(1).any(|&f| f != 0);

        if has_unknown_incompat_bits {
            return Err(ULogError::FormatError);
        }

        if contains_appended_data {
            let appended_offsets = &message[16..40];
            let offset_0 = LittleEndian::read_u64(&appended_offsets[0..8]);
            if offset_0 > 0 {
                // The appended data is currently only used for hardfault dumps, so it's safe to ignore it.
                //self.read_until_file_position = offset_0 as i64;
                datastream.max_bytes_to_read = Some(offset_0 as usize);
            }
        }

        Ok(true)
    }



    fn read_info(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        let mut buffer:Vec<u8> = vec![0; msg_size as usize];
        datastream.read_exact(&mut buffer)?;

        let key_len = buffer[0] as usize;

        // Print the buffer as hex
        //log::trace!("Buffer in hex: {}", buffer.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<String>>().join(" "));

        let raw_key = String::from_utf8(buffer[1..1 + key_len].to_vec())?;

        let key_parts: Vec<&str> = raw_key.split_whitespace().collect();
        if key_parts.len() < 2 {
            return Ok(false);
        }

        //log::debug!("Key parts: {:?}", key_parts);

        let key = key_parts[1].to_string();
        let value = match key_parts[0] {
            _ if key_parts[0].starts_with("char[") => {
                String::from_utf8(buffer[1 + key_len..msg_size as usize].to_vec())?
            },
            "bool" => {
                let val = buffer[1 + key_len] != 0;
                val.to_string()
            },
            "uint8_t" => {
                let val = buffer[1 + key_len] as u8;
                val.to_string()
            },
            "int8_t" => {
                let val = buffer[1 + key_len] as i8;
                val.to_string()
            },
            "uint16_t" => {
                let val = LittleEndian::read_u16(&buffer[1 + key_len..1 + key_len + 2]);
                val.to_string()
            },
            "int16_t" => {
                let val = LittleEndian::read_i16(&buffer[1 + key_len..1 + key_len + 2]);
                val.to_string()
            },
            "uint32_t" => {
                let val = LittleEndian::read_u32(&buffer[1 + key_len..1 + key_len + 4]);
                if key.starts_with("ver_") && key.ends_with("_release") {
                    format!("{:#X}", val)
                } else {
                    val.to_string()
                }
            },
            "int32_t" => {
                let val = LittleEndian::read_i32(&buffer[1 + key_len..1 + key_len + 4]);
                val.to_string()
            },
            "float" => {
                let val = LittleEndian::read_f32(&buffer[1 + key_len..1 + key_len + 4]);
                val.to_string()
            },
            "double" => {
                let val = LittleEndian::read_f64(&buffer[1 + key_len..1 + key_len + 8]);
                val.to_string()
            },
            "uint64_t" => {
                let val = LittleEndian::read_u64(&buffer[1 + key_len..1 + key_len + 8]);
                val.to_string()
            },
            "int64_t" => {
                let val = LittleEndian::read_i64(&buffer[1 + key_len..1 + key_len + 8]);
                val.to_string()
            },
            _ => return Err(ULogError::FormatError),
        };


        log::debug!("INFO {} {}:\t{}", key_parts[0], key_parts[1], value);

        self.info.insert(key, value);

        Ok(true)
    }

    fn read_parameter(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        let mut buffer:Vec<u8> = vec![0; msg_size as usize];
        datastream.read_exact(&mut buffer)?;

        let param = Parameter::from_buffer(&buffer)?;
        self.parameters.push(param);
        Ok(true)
    }
}

impl Parameter {
    fn from_buffer(buffer: &[u8]) -> Result<Parameter, ULogError> {
        let key_len = buffer[0] as usize;
        let key = String::from_utf8(buffer[1..1 + key_len].to_vec())?;
        let pos = key.find(' ').ok_or(ULogError::FormatError)?;

        let type_ = &key[0..pos];
        let name = key[pos + 1..].to_string();

        let value = match type_ {
            "int32_t" => ParameterValue::Int(LittleEndian::read_i32(&buffer[1 + key_len..])),
            "float" => ParameterValue::Real(LittleEndian::read_f32(&buffer[1 + key_len..])),
            _ => return Err(ULogError::UnknownParameterType),
        };

        let val_type = match type_ {
            "int32_t" => FormatType::INT32,
            "float" => FormatType::FLOAT,
            _ => return Err(ULogError::UnknownParameterType),
        };

        log::debug!("PARAM {:?} {}:\t{:?}", val_type, name, value);
        Ok(Parameter { name, value, val_type })
    }
}

#[derive(Debug)]
pub struct ULogMessageHeader {
    pub msg_size: u16,
    pub msg_type: ULogMessageType,
}

#[derive(Debug)]
#[repr(u8)]
pub enum ULogMessageType {
    FORMAT = b'F',
    DATA = b'D' ,
    INFO = b'I' ,
    INFO_MULTIPLE = b'M' ,
    PARAMETER = b'P' ,
    PARAMETER_DEFAULT = b'Q' ,
    ADD_LOGGED_MSG = b'A' ,
    REMOVE_LOGGED_MSG = b'R' ,
    SYNC = b'S' ,
    DROPOUT = b'O' ,
    LOGGING = b'L' ,
    LOGGING_TAGGED = b'C' ,
    FLAG_BITS = b'B' ,
    UNKNOWN = 255,
}

impl From<u8> for ULogMessageType {
    fn from(byte: u8) -> Self {
        match byte {
            b'F' => ULogMessageType::FORMAT,
            b'D' => ULogMessageType::DATA,
            b'I' => ULogMessageType::INFO,
            b'M' => ULogMessageType::INFO_MULTIPLE,
            b'P' => ULogMessageType::PARAMETER,
            b'Q' => ULogMessageType::PARAMETER_DEFAULT,
            b'A' => ULogMessageType::ADD_LOGGED_MSG,
            b'R' => ULogMessageType::REMOVE_LOGGED_MSG,
            b'S' => ULogMessageType::SYNC,
            b'O' => ULogMessageType::DROPOUT,
            b'L' => ULogMessageType::LOGGING,
            b'C' => ULogMessageType::LOGGING_TAGGED,
            b'B' => ULogMessageType::FLAG_BITS,
            _ => {
                log::warn!("Unknown message type: 0x{:02X}", byte);
                ULogMessageType::UNKNOWN
            }
        }
    }
}

const ULOG_MSG_HEADER_LEN: usize = 3;
const ULOG_INCOMPAT_FLAG0_DATA_APPENDED_MASK: u8 = 1 << 0;
