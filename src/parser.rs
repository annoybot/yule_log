#![allow(non_camel_case_types)]

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::marker::PhantomData;

use byteorder::{ByteOrder, LittleEndian};

use crate::datastream::DataStream;
use crate::errors::ULogError;
use crate::errors::ULogError::{InternalError, UndefinedFormat, UndefinedSubscription, UnknownParameterType};
use crate::formats::{parse_field, parse_format};
use crate::message_buf::MessageBuf;
use crate::model::{def, inst, msg};
use crate::model::def::Format;
use crate::model::MAGIC;
use crate::model::msg::{Dropout, FileHeader, FlagBits, LoggedData, LogLevel, MultiInfo, UlogMessage};
use crate::tokenizer::TokenList;

pub struct ULogParser<R: Read> {
    state: State,
    file_header: Option<FileHeader>,
    overridden_params: HashSet<String>,
    pub formats: HashMap<String, def::Format>,
    subscriptions: HashMap<u16, msg::Subscription>,
    message_name_with_multi_id: HashSet<String>,
    allowed_subscription_names: Option<HashSet<String>>,
    datastream: DataStream<R>,
    max_bytes_to_read: Option<usize>,
    pub include_header: bool,
    pub include_timestamp: bool,
    pub include_padding: bool,
    _phantom: PhantomData<R>,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum State {
    HEADER = 0,
    DEFINITIONS = 1,
    DATA = 2,
    EOF = 3,
    ERROR = 10,
}

impl<R: Read> Iterator for ULogParser<R> {
    type Item = Result<msg::UlogMessage, ULogError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_sub() {
            Ok(Some(data)) => Some(Ok(data)),
            Ok(None) => None, // Iterator exhausted.
            Err(e) => Some(Err(e)),
        }
    }
}

impl<R: Read> ULogParser<R> {
    pub fn new(reader: R,) -> Result<ULogParser<R>, ULogError> {
        Ok(ULogParser {
            state: State::HEADER.into(),
            file_header: None,
            overridden_params: HashSet::new(),
            formats: HashMap::new(),
            subscriptions: HashMap::new(),
            message_name_with_multi_id: HashSet::new(),
            allowed_subscription_names: None,
            datastream: DataStream::new(reader),
            max_bytes_to_read: None,
            include_header: false,
            include_timestamp: false,
            include_padding: false,
            _phantom: PhantomData,
        })
    }
    
    pub fn set_subscription_allow_list(&mut self, set: HashSet<String>) {
        self.allowed_subscription_names = Some(set);
    }

    pub fn get_format(&self, message_name: &str) -> Result<def::Format, ULogError> {
        match self.formats.get(message_name) {
            None => Err(UndefinedFormat(message_name.to_owned()) ),
            Some(format) => Ok( format.clone() ),
        }
    }

    pub fn get_subscription(&self, msg_id: u16) -> Result<msg::Subscription, ULogError> {
        match self.subscriptions.get(&msg_id) {
            None => Err(UndefinedSubscription(msg_id) ),
            Some(sub) => Ok( sub.clone() ),
        }
    }

    pub(crate)  fn read_message(&mut self, msg_size: usize) -> Result<MessageBuf, ULogError> {
        let mut message: Vec<u8> = vec![0; msg_size];
        self.datastream.read_exact(&mut message)?;
        Ok(MessageBuf::from_vec(message))
    }

    #[allow(clippy::single_match_else)]
    fn next_sub(&mut self) -> Result<Option<msg::UlogMessage>, ULogError> {
        if self.state == State::HEADER  {
            match self.read_file_header() {
                Ok(header) => {
                    self.file_header = Some(header);
                    self.state = State::DEFINITIONS;

                    #[allow(clippy::redundant_else)]
                    if self.include_header {
                        return Ok(Some(UlogMessage::Header(header)))
                    } else {
                        //Fallthrough.
                    }
                },
                Err(_) => {
                    self.state = State::ERROR;
                    return Err(ULogError::InvalidHeader);
                }
            }
        };
        
        if self.state == State::EOF {
            return Ok(None);
        }

        // ⚠️ ULOG files can contain binary crash dumps at offsets determined by the FLAG_BITS message.
        // In such cases self.max_bytes_to_read will contain the offset in the stream where the crash dump begins.
        // We must return EOF when we reach this limit to avoid attempting to parse invalid ULOG data.
        let max_bytes_to_read = self.max_bytes_to_read;

        if let Some(max_bytes_to_read) = max_bytes_to_read {
                if self.datastream.num_bytes_read >= max_bytes_to_read {
                self.state = State::EOF;
                return Ok(None);
            }
        }

        let (message_type, message_buf) = match self.read_message_header()? {
            None => {
                self.state = State::EOF;
                return Ok(None);
            }
            Some(header) => (header.msg_type, self.read_message(header.msg_size as usize)?),
        };

        match self.state {
            State::DEFINITIONS => {
                let msg = self.parse_definition(message_type, message_buf)?;

                match msg {
                    UlogMessage::FormatDefinition(ref format) => {
                        self.formats.insert(format.name.clone(), format.clone());
                    }
                    UlogMessage::AddSubscription(ref sub) => {
                        self.subscriptions.insert(sub.msg_id, sub.clone());

                        if sub.multi_id > 0 {
                            self.message_name_with_multi_id.insert(sub.message_name.clone());
                        }

                        // Now that we've seen the first subscription message we can advance to state 'DATA.'
                        self.state = State::DATA;
                    }
                    _ => (),
                }

                return Ok(Some(msg));
            }
            State::DATA => {
                let mut msg = self.parse_data(message_type, message_buf)?;

                match msg {
                    UlogMessage::AddSubscription(ref sub) => {
                        self.subscriptions.insert(sub.msg_id, sub.clone());

                        if sub.multi_id > 0 {
                            self.message_name_with_multi_id.insert(sub.message_name.clone());
                        }
                    }
                    UlogMessage::LoggedData(ref mut logged_data) => {
                        logged_data.filter_fields(
                            self.include_timestamp,
                            self.include_padding);
                    }
                    _ =>  {}
                };


                return Ok(Some(msg));
            }
            _ => {
                return Err(ULogError::ParseError(format!(
                    "Parser is in an invalid state: {:?}",
                    self.state
                )));
            }
        }
    }
    
    pub fn parse_data(&mut self, message_type: ULogMessageType, mut message_buf: MessageBuf) -> Result<UlogMessage, ULogError> {
        match message_type {
            ULogMessageType::ADD_SUBSCRIPTION => {
                let sub = self.parse_subscription(message_buf)?;
                Ok( msg::UlogMessage::AddSubscription(sub) )
            }
            ULogMessageType::REMOVE_SUBSCRIPTION => {
                let msg_id = message_buf.take_u16()?;
                self.subscriptions.remove(&msg_id);
                Ok(UlogMessage::Unhandled { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() })
            }
            ULogMessageType::DATA => {
                let msg_id = message_buf.take_u16()?;
                if let Ok(sub) = self.get_subscription(msg_id) {
                    let allowed = match &self.allowed_subscription_names {

                        None => true,
                        Some(set) => set.contains(&sub.message_name),
                    };

                    if allowed {
                        let logged_data = self.parse_data_message(sub.clone(), message_buf)?;

                        return Ok( msg::UlogMessage::LoggedData(logged_data.clone()));
                    } else {
                        return Ok(UlogMessage::Ignored { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() });
                    }
                } else {
                    return Err(ULogError::ParseError(format!(
                        "Received logged data with an unknown msg_id {msg_id}.  Could not find a subscription for this data. Ignoring."
                    )));
                }
            }
            ULogMessageType::LOGGING => {
                Ok( msg::UlogMessage::LoggedString( msg::LoggedString {
                    level: LogLevel::try_from(message_buf.take_u8()?)?,
                    tag: None,
                    timestamp: message_buf.take_u64()?,
                    msg: String::from_utf8(message_buf.into_remaining_bytes())?,
                }))
            }
            ULogMessageType::LOGGING_TAGGED => {
                Ok( msg::UlogMessage::LoggedString( msg::LoggedString {
                    level: LogLevel::try_from(message_buf.take_u8()?)?,
                    tag: Some(message_buf.take_u16()?,),
                    timestamp: message_buf.take_u64()?,
                    msg: String::from_utf8(message_buf.into_remaining_bytes())?,

                }))
            }
            ULogMessageType::DROPOUT => {
                Ok( msg::UlogMessage::DropoutMark( Dropout {
                    duration: message_buf.take_u16()?,
                }))
            },
            // FIXME: Implement SYNC
            //ULogMessageType::SYNC => {}
            ULogMessageType::PARAMETER => {
                let param = self.parse_parameter(message_buf)?;
                Ok(msg::UlogMessage::Parameter(param))
            }
            ULogMessageType::PARAMETER_DEFAULT => {
                let param = self.parse_default_parameter(message_buf)?;
                Ok(msg::UlogMessage::DefaultParameter(param))
            }
            ULogMessageType::INFO => {
                let info = self.parse_info(message_buf)?;
                Ok(msg::UlogMessage::Info(info))
            }
            ULogMessageType::INFO_MULTIPLE => {
                let multi_info = self.parse_multi_info(message_buf)?;
                Ok(msg::UlogMessage::MultiInfo(multi_info))
            }
            _ => {
                log::debug!("Received unhandled message type {message_type:?}. Ignoring.");
                Ok(UlogMessage::Unhandled { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() })
            }
        }
    }

    fn parse_subscription(&self, mut message_buf: MessageBuf) -> Result<msg::Subscription, ULogError> {
        let multi_id = message_buf.take_u8()?;
        let msg_id = message_buf.take_u16()?;

        let message_name = String::from_utf8(message_buf.into_remaining_bytes())?;

        // Force a lookup of the format and return an error if not found.
        let _format = self.get_format(&message_name)?;

        let allowed = if let Some(allowed_subscription_names) = &self.allowed_subscription_names {
            allowed_subscription_names.contains(&message_name)
        } else {
            true
        };

        Ok ( msg::Subscription {
            multi_id,
            msg_id,
            message_name: message_name.clone(),
        })
    }

    fn read_message_header(&mut self) -> Result<Option<ULogMessageHeader>, ULogError> {
        let msg_size = self.datastream.read_u16()?;

        // ⚠️This is the only place where we check for EOF when calling a datastream read method.
        // If we encounter EOF anywhere else, it counts as a true 'Unexpected EOF' and is treated as an error.
        if self.datastream.eof {
            return Ok(None);
        }

        let msg_type = ULogMessageType::from(self.datastream.read_u8()?);
        log::trace!("MSG HEADER: {} {:?}", msg_size, msg_type);

        Ok(Some(ULogMessageHeader { msg_size, msg_type }))
    }



    fn parse_data_message(&self, sub: msg::Subscription, mut message_buf: MessageBuf) -> Result<msg::LoggedData, ULogError> {
        let format = self.get_format(&sub.message_name)?;

        if !format.fields.iter().any(|f| f.name == "timestamp") {
            return Err(ULogError::MissingTimestamp);
        }
        // Read the timestamp from the logged data message.
        // This seemed to assume that the assert above is correct, and that the timestamp field will be the first field in sub.format.
        //let time_val = message_buf.take_u64()?;

        let mut data_format = self.parse_data_message_sub(&format, &mut message_buf)?;

        if self.message_name_with_multi_id.contains(&sub.message_name) {
            data_format.multi_id_index = Some(sub.multi_id);
        }

        // ⚠️ The timestamp for this message is the value of the `timestamp` field from the top-level `data_format` 
        // returned by `parse_data_message_sub()`.  We now remove the field from `data_format` to avoid returning redundant timestamps.
        // See the comment in `parse_data_message_sub()` for more information.
        let timestamp = data_format.timestamp.ok_or(ULogError::MissingTimestamp)?;

        // FXIME: ⚠️We must not filter out the timestamps if we want to round trip the data.
        //data_format.fields.retain(|f| f.name != "timestamp");

        if !message_buf.is_empty() {
            log::warn!("Leftover bytes in messagebuf after parsing LOGGED_DATA message! Possible data corruption.");
        }

        Ok( msg::LoggedData {
                timestamp,
                msg_id: sub.msg_id,
                data: data_format,
            })
    }

    fn parse_data_message_sub(&self, format: &def::Format, message_buf: &mut MessageBuf) -> Result<inst::Format, ULogError> {
        let mut fields: Vec<inst::Field> = vec![];
        let mut timestamp:Option<u64> = None;

        for field in &format.fields {
            if field.name.starts_with("_padding") {

                match field.r#type.array_size {
                    Some(array_size) => {
                        match array_size.cmp(&message_buf.len()) {
                            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                                log::debug!("Encountered padding, and padding <= message.len(). Ok.");

                                if self.include_padding {
                                    let array =
                                        message_buf.advance(array_size)?
                                            .iter()
                                            .map(|e| inst::BaseType::UINT8(*e) )
                                            .collect();
                                    fields.push(inst::Field {
                                        name: field.name.clone(),
                                        r#type: field.r#type.clone(),
                                        value: inst::FieldValue::ARRAY(array),
                                    });
                                } else {
                                    //Skip over the padding bytes.
                                    message_buf.skip(array_size)?;
                                }
                            }
                            std::cmp::Ordering::Greater => match message_buf.len() {
                                0 => log::debug!("Encountered padding, and message.len() == 0. Ignoring as per ULOG spec."),
                                _ => log::error!("Encountered padding, and padding > message.len(). Ignoring and hoping for the best"),
                            },
                        }
                    }
                    None => {
                        log::warn!("Encountered padding, and type is scalar. Ignoring.");
                    }
                }

                continue;
            }

            match field.r#type.array_size {
                None => {
                    let value:inst::FieldValue = inst::FieldValue::SCALAR(
                        match &field.r#type.base_type {
                            def::BaseType::OTHER(type_name) => {
                                let child_format = &self.get_format(type_name)?;

                                inst::BaseType::OTHER(
                                    self.parse_data_message_sub(child_format, message_buf)?
                                )
                            },
                            _ => self.parse_data_field(field, message_buf)?
                        }
                    );

                    // ⚠️ Extract the timestamp field if present.
                    // According to the ULOG spec, the timestamp for a LOGGED_DATA message is the value of
                    // the timestamp field from the Format linked to the Subscription.
                    // This function will extract all such fields, regardless of location in the Format hierarchy.
                    // When this function returns, the top-level timestamp will then be extracted and assigned
                    // to msg::LoggedData.timestamp. See: `parse_data_message()`
                    if let inst::FieldValue::SCALAR(inst::BaseType::UINT64(value)) = value  {
                        if field.name == "timestamp"  {
                            timestamp = Some(value);
                        }
                    }

                    fields.push(
                        inst::Field {
                            name: field.name.clone(),
                            r#type: field.r#type.clone(),
                            value,
                        });
                }
                Some(array_size) => {
                    let mut array: Vec<inst::BaseType> = Vec::with_capacity(array_size);

                    for _ in 0..array_size {
                        let array_element = match &field.r#type.base_type {
                            def::BaseType::OTHER(type_name) => {

                                let child_format = &self.get_format(type_name)?;

                                inst::BaseType::OTHER(
                                    self.parse_data_message_sub(child_format, message_buf)?
                                )
                            },
                            _ => self.parse_data_field(field, message_buf)?
                        };

                        array.push(array_element);
                    }

                    fields.push(inst::Field {
                        name: field.name.clone(),
                        r#type: field.r#type.clone(),
                        value: inst::FieldValue::ARRAY(array),
                    });
                }
            }
        }

        Ok(inst::Format {
            name: format.name.clone(),
            timestamp,
            fields,
            // ⚠️ The proper value for this field can't be known at this point in the code.
            // It can be filled out only after we've seen all the subscriptions in the file.
            // We have omitted it here so it can be filled later on if required.
            multi_id_index: None,
        })
    }

    fn read_file_header(&mut self) -> Result<FileHeader, ULogError> {
        let mut msg_header = [0; 16];
        self.datastream.read_exact(&mut msg_header)?;

        if msg_header[0..7] != MAGIC {
            return Err( ULogError::InvalidMagicBits);
        }

        let file_version:u8 = msg_header[7];
        let file_start_time = LittleEndian::read_u64(&msg_header[8..16]);

        Ok(FileHeader {
            version: file_version,
            timestamp: file_start_time,
        })
    }

    fn parse_definition(&mut self, message_type: ULogMessageType, message_buf: MessageBuf) -> Result<msg::UlogMessage, ULogError> {
        match message_type {
            ULogMessageType::FLAG_BITS => {

                let flag_bits = self.parse_flag_bits(message_buf)?;

                if flag_bits.has_data_appended() {
                    // Stop reading from this stream at the first non-zero appended data offset in the list.
                    self.max_bytes_to_read =
                        flag_bits.appended_data_offsets.iter().find(|&&offset| offset > 0).map(|&offset| offset as usize);
                }
                
                Ok( UlogMessage::FlagBits(flag_bits))
            }
            ULogMessageType::FORMAT => {
                let format = parse_format(message_buf)?;
                Ok(msg::UlogMessage::FormatDefinition(format))
            }
            ULogMessageType::PARAMETER => {
                let param = self.parse_parameter(message_buf)?;
                Ok(msg::UlogMessage::Parameter(param))
            }
            ULogMessageType::PARAMETER_DEFAULT => {
                let param = self.parse_default_parameter(message_buf)?;
                Ok(msg::UlogMessage::DefaultParameter(param))
            }
            ULogMessageType::ADD_SUBSCRIPTION => {
                let sub = self.parse_subscription(message_buf)?;
                Ok(msg::UlogMessage::AddSubscription(sub))
            }
            ULogMessageType::INFO => {
                let info = self.parse_info(message_buf)?;
                Ok(msg::UlogMessage::Info(info))
            }
            ULogMessageType::INFO_MULTIPLE => {
                let multi_info = self.parse_multi_info(message_buf)?;
                Ok(msg::UlogMessage::MultiInfo(multi_info))
            }
            /*
            ULogMessageType::INFO_MULTIPLE | ULogMessageType::PARAMETER_DEFAULT => {
                Ok(UlogMessage::Unhandled { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() })
            }
             */
            ULogMessageType::UNKNOWN(byte) => {
                log::warn!("Unknown message type: 0x{:02X}", byte);
                Ok(UlogMessage::Unhandled { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() })
            }
            _ => {
                // FIXME: Handle other variants in definitions section. 
                Ok( UlogMessage::Unhandled { msg_type: message_type.into(), message_contents: message_buf.into_remaining_bytes() } )
            }
        }
    }
    

    #[allow(clippy::unused_self)]
    fn parse_flag_bits(&self, mut message_buf: MessageBuf) -> Result<FlagBits, ULogError> {
        if message_buf.len() != 40 {
            log::warn!("Length of flag bits >40bytes (Contained {len} extra bytes).  Ignoring.", len = message_buf.len());
        }

        // Unwrap is ok because of the len of the array returned by advance is guaranteed to be 8.
        let compat_flags: [u8; 8] = message_buf.advance(8)?.try_into().unwrap();

        // Unwrap is ok because of the len of the array returned by advance is guaranteed to be 8.
        let incompat_flags:[u8; 8] = message_buf.advance(8)?.try_into().unwrap();

        // Check for any unknown bits in incompat_flags
        let has_unknown_incompat_bits = incompat_flags.iter().skip(1).any(|&f| f != 0);
        
        if has_unknown_incompat_bits {
            return Err(ULogError::UnknownIncompatBits);
        }

        let mut appended_data_offsets:[u64;3] = [0, 0, 0];
        for i in 0..3 {
            appended_data_offsets[i] = message_buf.take_u64()?;
        }

        Ok( FlagBits {
            compat_flags,
            incompat_flags,
            appended_data_offsets,
        })
    }
    
    pub(crate) fn parse_info(&self, mut message_buf: MessageBuf) -> Result<msg::Info, ULogError> {
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;

        let value:inst::FieldValue = match field.r#type.array_size {
            None => inst::FieldValue::SCALAR(  self.parse_data_field(&field, &mut message_buf)? ),
            Some(array_size) => {
                let mut array: Vec<inst::BaseType> = vec![];

                for _ in 0..array_size {
                    array.push(self.parse_data_field(&field, &mut message_buf)?);
                }

                inst::FieldValue::ARRAY(array)
            }
        };

        log::debug!("INFO {:?} {}:\t{}", field.r#type, &field.name, value);

        Ok(msg::Info { key: field.name, r#type: field.r#type, value })
    }

    pub(crate) fn parse_multi_info(&mut self, mut message_buf: MessageBuf) -> Result<msg::MultiInfo, ULogError> {
        let is_continued = message_buf.take_u8()? != 0;
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;

        let value:inst::FieldValue = match field.r#type.array_size {
            None => inst::FieldValue::SCALAR(  self.parse_data_field(&field, &mut message_buf)? ),
            Some(array_size) => {
                let mut array: Vec<inst::BaseType> = vec![];

                for _ in 0..array_size {
                    array.push(self.parse_data_field(&field, &mut message_buf)?);
                }

                inst::FieldValue::ARRAY(array)
            }
        };

        log::debug!("MULTI_INFO {:?} {}:\t{}", field.r#type, &field.name, value);
        log::debug!("is_continued = {}", is_continued);

        let result: MultiInfo =
            MultiInfo {
                key: field.name,
                r#type: field.r#type,
                value,
                is_continued,
            };

        Ok(result)
    }


    fn parse_parameter(&self, mut message_buf: MessageBuf) -> Result<msg::Parameter, ULogError> {
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;


        let value:inst::ParameterValue = match field.r#type.array_size {
            None => {
                let scalar_value = self.parse_data_field(&field, &mut message_buf)?;

                match scalar_value {
                    inst::BaseType::FLOAT(v) => inst::ParameterValue::FLOAT(v),
                    inst::BaseType::INT32(v) => inst::ParameterValue::INT32(v),
                    _ => return Err(ULogError::UnknownParameterType(format!("Received parameter message with unsupported type ({raw_key}->{scalar_value}). Ignoring.").to_owned()))
                }
            }
            Some(_) => return Err(UnknownParameterType(format!("Received parameter message with type ARRAY ({raw_key}). Ignoring.").to_owned())),
        };
        
        log::debug!("INFO {:?} {}:\t{:?}", field.r#type, &field.name, value);

        Ok( msg::Parameter{
            key: field.name,
            r#type: field.r#type,
            value,
            })
    }

    fn parse_default_parameter(&self, mut message_buf: MessageBuf) -> Result<msg::DefaultParameter, ULogError> {
        let default_types = message_buf.take_u8()?; // read the default_types bitfield
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;

        let value: inst::ParameterValue = match field.r#type.array_size {
            None => {
                let scalar_value = self.parse_data_field(&field, &mut message_buf)?;

                match scalar_value {
                    inst::BaseType::FLOAT(v) => inst::ParameterValue::FLOAT(v),
                    inst::BaseType::INT32(v) => inst::ParameterValue::INT32(v),
                    _ => return Err(ULogError::UnknownParameterType(format!("Received default parameter message with unsupported type ({raw_key}->{scalar_value}). Ignoring.").to_owned()))
                }
            }
            Some(_) => return Err(ULogError::UnknownParameterType(format!("Received default parameter message with type ARRAY ({raw_key}). Ignoring.").to_owned())),
        };

        log::debug!("INFO {:?} {}:\t{:?}", field.r#type, &field.name, value);

        Ok(msg::DefaultParameter {
            key: field.name,
            default_types,
            r#type: field.r#type,
            value,
        })
    }


    fn parse_data_field(&self, field: &def::Field, message_buf: &mut MessageBuf) -> Result<inst::BaseType, ULogError> {
        match &field.r#type.base_type {
            def::BaseType::UINT8 => Ok(inst::BaseType::UINT8(message_buf.take_u8()?)),
            def::BaseType::UINT16 => Ok(inst::BaseType::UINT16(message_buf.take_u16()?)),
            def::BaseType::UINT32 => Ok(inst::BaseType::UINT32(message_buf.take_u32()?)),
            def::BaseType::UINT64 => Ok(inst::BaseType::UINT64(message_buf.take_u64()?)),
            def::BaseType::INT8 => Ok(inst::BaseType::INT8(message_buf.take_i8()?)),
            def::BaseType::INT16 => Ok(inst::BaseType::INT16(message_buf.take_i16()?)),
            def::BaseType::INT32 => Ok(inst::BaseType::INT32(message_buf.take_i32()?)),
            def::BaseType::INT64 => Ok(inst::BaseType::INT64(message_buf.take_i64()?)),
            def::BaseType::FLOAT => Ok(inst::BaseType::FLOAT(message_buf.take_f32()?)),
            def::BaseType::DOUBLE => Ok(inst::BaseType::DOUBLE(message_buf.take_f64()?)),
            def::BaseType::BOOL => Ok(inst::BaseType::BOOL(message_buf.take_u8()? != 0)),
            def::BaseType::CHAR => Ok(inst::BaseType::CHAR(message_buf.take_u8()? as char)),
            def::BaseType::OTHER(_type_name) => {
                Err(InternalError("parse_data_field() can only be called on scalar types.".to_owned()))
            }
        }
    }
}

#[derive(Debug)]
pub struct ULogMessageHeader {
    pub msg_size: u16,
    pub msg_type: ULogMessageType,
}

#[derive(Debug,Copy,Clone)]
#[repr(u8)]
pub enum ULogMessageType {
    FORMAT = b'F',
    DATA = b'D',
    INFO = b'I',
    INFO_MULTIPLE = b'M',
    PARAMETER = b'P',
    PARAMETER_DEFAULT = b'Q',
    ADD_SUBSCRIPTION = b'A',
    REMOVE_SUBSCRIPTION = b'R',
    SYNC = b'S',
    DROPOUT = b'O',
    LOGGING = b'L',
    LOGGING_TAGGED = b'C',
    FLAG_BITS = b'B',
    // ⚠️Header is not a real Ulog 'message' type, but we treat it as one for convenience.
    HEADER = 254,
    UNKNOWN(u8) = 255,
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
            b'A' => ULogMessageType::ADD_SUBSCRIPTION,
            b'R' => ULogMessageType::REMOVE_SUBSCRIPTION,
            b'S' => ULogMessageType::SYNC,
            b'O' => ULogMessageType::DROPOUT,
            b'L' => ULogMessageType::LOGGING,
            b'C' => ULogMessageType::LOGGING_TAGGED,
            b'B' => ULogMessageType::FLAG_BITS,
            _ => {
                ULogMessageType::UNKNOWN(byte)
            }
        }
    }
}

impl From<ULogMessageType> for u8 {
    fn from(msg_type: ULogMessageType) -> Self {
        match msg_type {
            ULogMessageType::FORMAT => b'F',
            ULogMessageType::DATA => b'D',
            ULogMessageType::INFO => b'I',
            ULogMessageType::INFO_MULTIPLE => b'M',
            ULogMessageType::PARAMETER => b'P',
            ULogMessageType::PARAMETER_DEFAULT => b'Q',
            ULogMessageType::ADD_SUBSCRIPTION => b'A',
            ULogMessageType::REMOVE_SUBSCRIPTION => b'R',
            ULogMessageType::SYNC => b'S',
            ULogMessageType::DROPOUT => b'O',
            ULogMessageType::LOGGING => b'L',
            ULogMessageType::LOGGING_TAGGED => b'C',
            ULogMessageType::FLAG_BITS => b'B',
            ULogMessageType::HEADER => 254,
            ULogMessageType::UNKNOWN(byte) => byte,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn test_round_trip_subscription() {
        //  \x0A    \x01\x00  my_message
        // multi_id  msg_id   message_name
        let input_bytes = b"\x0A\x01\x00my_message";

        let cursor = io::Cursor::new(input_bytes);

        let mut parser = ULogParser::new(cursor).expect("Unable to create ULogParser");

        parser.insert_format("my_message", def::Format{
            name: "".to_string(),
            fields: vec![],
            padding: 0,
        });

        // MessageBuf should not contain the header bytes, which is why initialise it from byte 3 onwards.
        let message_buf = MessageBuf::from_vec(input_bytes.to_vec());

        // Parse the Subscription
        let parsed_subscription = parser.parse_subscription(message_buf).expect("Unable to parse subscription");
        println!("parsed_subscription: {:?}", parsed_subscription);

        let emitted_bytes = parsed_subscription.to_bytes();
        println!("Emitted bytes: {:?}", emitted_bytes);

        assert_eq!(emitted_bytes, input_bytes);
    }
}

impl LoggedData {
    pub fn filter_fields(&mut self, include_timestamp: bool, include_padding: bool) {
        self.data.fields.retain(|field| {
            if field.name == "timestamp" {
                return include_timestamp;
            }

            if field.name.starts_with("_padding") {
                return include_padding;
            }

            true
        });
    }
}
