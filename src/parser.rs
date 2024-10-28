#![allow(non_camel_case_types)]

use byteorder::{ByteOrder, LittleEndian};
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::datastream::DataStream;

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
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    OTHER
}

#[derive(Debug, Clone)]
pub struct Field {
    type_: FormatType,
    pub field_name: String,
    pub other_type_id: String,
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

#[derive(Debug, Clone)]
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
    //read_buffer: Vec<u8>,
    //data_section_start: usize,
    //read_until_file_position: i64,
    overridden_params: HashSet<String>,
    formats: HashMap<String, Format>,
    info: HashMap<String, String>,
    subscriptions: HashMap<u16, Subscription>,
    //timeseries: HashMap<String, Timeseries>,
    message_name_with_multi_id: HashSet<String>,
    message_logs: Vec<MessageLog>,
    _phantom: PhantomData<R>,

}

impl <R: Read>ULogParser <R> {
    pub fn new(mut datastream: DataStream<R>, timeseries_map: &mut HashMap<String, Timeseries>) -> Result<ULogParser<R>, ULogError> {
        log::trace!("Entering {}", "ULogParser::new" );
        let mut parser = ULogParser {
            file_start_time: 0,
            parameters: Vec::new(),
            //read_buffer: Vec::new(),
            //data_section_start: 0,
            //read_until_file_position: 1 << 60,
            overridden_params: HashSet::new(),
            formats: HashMap::new(),
            info: HashMap::new(),
            subscriptions: HashMap::new(),
            //timeseries: HashMap::new(),
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

            // Read the next message header.
            message_header = parser.read_message_header(&mut datastream)?;
        }

        log::trace!("Exiting {}", "ULogParser::new" );
        Ok(parser)
    }

    fn read_message_header(&mut self, datastream: &mut DataStream<R>) -> Result<ULogMessageHeader, ULogError> {
        log::trace!("Entering {}", "read_message_header" );

        let msg_size = datastream.read_u16()?;
        let msg_type = ULogMessageType::from(datastream.read_u8()?);
        log::trace!("MSG HEADER: {} {:?}", msg_size, msg_type);

        log::trace!("Exiting {}", "read_message_header" );
        Ok(ULogMessageHeader { msg_size, msg_type })
    }

    fn parse_data_message(&self, sub: &Subscription, message: &[u8], timeseries_map: &mut HashMap<String, Timeseries>) {
        log::trace!("Entering {}", "parse_data_message" );
        let message = message.to_vec();
        let mut other_fields_count = 0;
        let mut ts_name = sub.message_name.clone();

        if let Some(format) = &sub.format {
            for field in &format.fields {
                if field.type_ == FormatType::OTHER {
                    other_fields_count += 1;
                }
            }
        }

        if self.message_name_with_multi_id.contains(&ts_name) {
            let buff = format!(".{:02}", sub.multi_id);
            ts_name.push_str(&buff);
        }

        let timeseries = timeseries_map.entry(ts_name.clone())
            .or_insert_with(|| self.create_timeseries(sub.format.as_ref().unwrap()));

        let time_val = LittleEndian::read_u64(&message[0..8]);
        timeseries.timestamps.push(time_val);
        let mut index = 0;
        self.parse_simple_data_message(timeseries, sub.format.as_ref().unwrap(), &message[8..], &mut index);
        log::trace!("Exiting {}", "parse_data_message" );
    }

    fn parse_simple_data_message<'a>(&'a self, timeseries: &mut Timeseries, format: &Format, mut message: &'a [u8], index: &mut usize) -> &'a [u8] {
        log::trace!("Entering {}", "parse_simple_data_message" );

        for field in &format.fields {
            if field.field_name.starts_with("_padding") {
                message = &message[field.array_size..];
                continue;
            }

            for _ in 0..field.array_size {
                let value = match field.type_ {
                    FormatType::UINT8 => message[0] as f64,
                    FormatType::INT8 => message[0] as f64,
                    FormatType::UINT16 => LittleEndian::read_u16(message) as f64,
                    FormatType::INT16 => LittleEndian::read_i16(message) as f64,
                    FormatType::UINT32 => LittleEndian::read_u32(message) as f64,
                    FormatType::INT32 => LittleEndian::read_i32(message) as f64,
                    FormatType::UINT64 => LittleEndian::read_u64(message) as f64,
                    FormatType::INT64 => LittleEndian::read_i64(message) as f64,
                    FormatType::FLOAT => LittleEndian::read_f32(message) as f64,
                    FormatType::DOUBLE => LittleEndian::read_f64(message),
                    FormatType::CHAR => message[0] as f64,
                    FormatType::BOOL => if message[0] != 0 { 1.0 } else { 0.0 },
                    FormatType::OTHER => {
                        let child_format = self.formats.get(&field.other_type_id).unwrap();
                        message = &message[8..]; // skip timestamp
                        message = self.parse_simple_data_message(timeseries, child_format, message, index);
                        continue;
                    }
                };

                if field.type_ != FormatType::OTHER {
                    timeseries.data[*index].1.push(value);
                    *index += 1;
                }

                message = &message[1..]; // Advance the message pointer
            }
        }

        log::trace!("Exiting {}", "parse_simple_data_message" );
        message
    }

    fn create_timeseries(&self, format: &Format) -> Timeseries {
        log::trace!("Entering {}", "create_timeseries" );
        let mut timeseries = Timeseries {
            timestamps: Vec::new(),
            data: Vec::new(),
        };

        fn append_vector(format: &Format, prefix: &str, timeseries: &mut Timeseries, formats: &HashMap<String, Format>) {
            for field in &format.fields {
                if field.field_name.starts_with("_padding") {
                    continue;
                }

                let new_prefix = format!("{} / {}", prefix, field.field_name);
                for i in 0..field.array_size {
                    let array_suffix = if field.array_size > 1 {
                        format!(".{:02}", i)
                    } else {
                        String::new()
                    };

                    if field.type_ != FormatType::OTHER {
                        timeseries.data.push((format!("{}{}", new_prefix, array_suffix), Vec::new()));
                    } else {
                        let child_format = formats.get(&field.other_type_id).unwrap();
                        append_vector(child_format, &format!("{}{}", new_prefix, array_suffix), timeseries, formats);
                    }
                }
            }
        }

        append_vector(format, "", &mut timeseries, &self.formats);

        log::trace!("Exiting {}", "create_timeseries" );
        timeseries
    }

    fn read_file_header(&mut self, datastream: &mut DataStream<R>) -> Result<bool, ULogError> {
        log::trace!("Entering {}", "read_file_header" );
        let mut msg_header = [0; 16];
        datastream.read_exact(&mut msg_header)?;

        self.file_start_time = LittleEndian::read_u64(&msg_header[8..16]);

        // Verify it's a ULog file
        const MAGIC: [u8; 7] = [b'U', b'L', b'o', b'g', 0x01, 0x12, 0x35];
        if &msg_header[0..7] != MAGIC {
            return Ok(false);
        }

        log::trace!("Exiting {}", "read_file_header" );
        Ok(true)
    }

    fn read_file_definitions(&mut self, datastream: &mut DataStream<R>) -> Result<ULogMessageHeader, ULogError> {
        log::trace!("Entering {}", "read_file_definitions" );

        loop {
            let message_header = self.read_message_header(datastream)?;

            match message_header.msg_type {
                ULogMessageType::FLAG_BITS => {
                    self.read_flag_bits(datastream, message_header.msg_size)?;
                }
                ULogMessageType::FORMAT => {
                    self.read_format(datastream, message_header.msg_size)?;
                }
                ULogMessageType::PARAMETER => {
                    self.read_parameter(datastream, message_header.msg_size)?;
                }
                ULogMessageType::ADD_LOGGED_MSG => {
                    // Return the message header, of the first logged message.
                    log::trace!("Exiting {}", "read_file_definitions" );
                    return Ok(message_header);
                }
                ULogMessageType::INFO => {
                    self.read_info(datastream, message_header.msg_size)?;
                }
                ULogMessageType::INFO_MULTIPLE | ULogMessageType::PARAMETER_DEFAULT => {
                    datastream.skip(message_header.msg_size as usize);
                }
                ULogMessageType::UNKNOWN => {
                    //log::debug!("Warning: Unknown ULogMessageType. Skipping.");
                    //continue;
                    //return Err(ULogError::FormatError);
                    panic!("Warning: Unknown ULogMessageType.");
                },
                _ => {
                    log::trace!("Exiting {}", "read_file_definitions" );
                    return Err(ULogError::FormatError);
                }
            }
        }
    }

    fn read_flag_bits(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        log::trace!("Entering {}", "read_flag_bits" );
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

        log::trace!("Exiting {}", "read_flag_bits" );
        Ok(true)
    }

    fn read_format(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        log::trace!("Entering {}", "read_format" );
        let mut message_buf:Vec<u8> = vec![0; msg_size as usize];
        datastream.read_exact(&mut message_buf)?;

        let str_format = String::from_utf8(message_buf)?;
        let pos = str_format.find(':').ok_or(ULogError::FormatError)?;

        let name = str_format[0..pos].to_string();
        let fields_str = &str_format[pos + 1..];

        let mut format = Format {
            name: name.clone(),
            fields: Vec::new(),
            padding: 0,
        };

        for field_section in fields_str.split(';') {
            let field_pair: Vec<&str> = field_section.split_whitespace().collect();
            if field_pair.len() != 2 {
                continue;
            }

            let field_type = field_pair[0].to_string();
            let field_name = field_pair[1].to_string();

            let mut field = Field {
                type_: FormatType::OTHER,
                field_name: field_name.clone(),
                other_type_id: String::new(),
                array_size: 1,
            };

            if field_type.starts_with("int8_t") {
                field.type_ = FormatType::INT8;
            } else if field_type.starts_with("int16_t") {
                field.type_ = FormatType::INT16;
            } else if field_type.starts_with("int32_t") {
                field.type_ = FormatType::INT32;
            } else if field_type.starts_with("int64_t") {
                field.type_ = FormatType::INT64;
            } else if field_type.starts_with("uint8_t") {
                field.type_ = FormatType::UINT8;
            } else if field_type.starts_with("uint16_t") {
                field.type_ = FormatType::UINT16;
            } else if field_type.starts_with("uint32_t") {
                field.type_ = FormatType::UINT32;
            } else if field_type.starts_with("uint64_t") {
                field.type_ = FormatType::UINT64;
            } else if field_type.starts_with("double") {
                field.type_ = FormatType::DOUBLE;
            } else if field_type.starts_with("float") {
                field.type_ = FormatType::FLOAT;
            } else if field_type.starts_with("bool") {
                field.type_ = FormatType::BOOL;
            } else if field_type.starts_with("char") {
                field.type_ = FormatType::CHAR;
            } else {
                field.type_ = FormatType::OTHER;
                if field_type.ends_with(']') {
                    let mut helper = field_type.as_str();
                    while !helper.ends_with('[') {
                        helper = &helper[0..helper.len() - 1];
                    }

                    helper = &helper[0..helper.len() - 1];
                    field.other_type_id = helper.to_string();

                    let mut field_type_chars = field_type.chars();
                    while field_type_chars.next() != Some('[') {}
                } else {
                    field.other_type_id = field_type.clone();
                }
            }

            field.array_size = 1;
            if field_type.contains('[') {
                let array_size_str = field_type.trim_start_matches(|c| c != '[').trim_start_matches('[').trim_end_matches(']');
                field.array_size = array_size_str.parse::<usize>().unwrap_or(1);
            }

            if field.type_ == FormatType::UINT64 && field_name == "timestamp" {
                // skip
            } else {
                format.fields.push(field);
            }
        }

        log::debug!("FORMAT: {} {:?}",name, format);

        self.formats.insert(name, format);

        log::trace!("Exiting {}", "read_format" );
        Ok(true)
    }

    fn read_info(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        log::trace!("Entering {}", "read_info" );
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

        log::trace!("Exiting {}", "read_info" );
        Ok(true)
    }

    fn read_parameter(&mut self, datastream: &mut DataStream<R>, msg_size: u16) -> Result<bool, ULogError> {
        log::trace!("Entering {}", "read_parameter" );
        let mut buffer:Vec<u8> = vec![0; msg_size as usize];
        datastream.read_exact(&mut buffer)?;

        let param = Parameter::from_buffer(&buffer)?;
        self.parameters.push(param);
        log::trace!("Exiting {}", "read_parameter" );
        Ok(true)
    }
}

impl Parameter {
    fn from_buffer(buffer: &[u8]) -> Result<Parameter, ULogError> {
        log::trace!("Entering {}", "Parameter::from_buffer" );
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
        log::trace!("Exiting {}", "Parameter::from_buffer" );
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
