#![allow(non_camel_case_types)]

use crate::field_helpers::parse_primitive_array;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::rc::Rc;
use byteorder::{ByteOrder, LittleEndian};

use crate::datastream::DataStream;
use crate::errors::ULogError;
use crate::errors::ULogError::{UndefinedFormat, UndefinedSubscription};
use crate::field_helpers::{parse_array, parse_data_field};
use crate::formats::{parse_field, parse_format};
use crate::message_buf::MessageBuf;
use crate::model::def::BaseType;
use crate::model::msg::{
    Dropout, FileHeader, FlagBits, LogLevel, LoggedData, MultiInfo, Subscription, UlogMessage,
};
use crate::model::{def, inst, msg, MAGIC};
use crate::tokenizer::TokenList;

pub struct ULogParser<R: Read> {
    state: State,
    file_header: Option<FileHeader>,
    overridden_params: HashSet<String>,
    pub formats: HashMap<String, Rc<def::Format>>,
    subscriptions: HashMap<u16, msg::Subscription>,
    message_name_with_multi_id: HashSet<String>,
    subscription_filter: SubscriptionFilter,
    datastream: DataStream<R>,
    max_bytes_to_read: Option<usize>,
    pub(crate) include_header: bool,
    pub(crate) include_timestamp: bool,
    pub(crate) include_padding: bool,
}

pub struct SubscriptionFilter {
    allowed_subscription_names: Option<HashSet<String>>,
    allowed_subscription_ids: Option<HashSet<u16>>,
}

impl Default for SubscriptionFilter {
    fn default() -> Self {
        Self {
            allowed_subscription_names: None,
            allowed_subscription_ids: None,
        }
    }
}

impl SubscriptionFilter {
    pub fn new(subscr_names: impl IntoIterator<Item = String>) -> Self {
        let names: HashSet<String> = subscr_names.into_iter().collect::<HashSet<_>>();
        Self {
            allowed_subscription_names: Some(names),
            allowed_subscription_ids: Some(HashSet::new()),
        }
    }

    fn update_ids(&mut self, sub: &Subscription) {
        // Because msg_ids are not known ahead of time the API specifies allowed subscriptions by name.
        // Once the AddSubscription messages come in, then we can convert the strings names to msg_ids
        // to more efficiently filter the subscriptions.
        if let Some(allowed_subscription_names) = &self.allowed_subscription_names {
            if allowed_subscription_names.contains(&sub.message_name) {
                // Unwrap is safe here because of the initialisation code in set_allowed_subscription_names().
                self.allowed_subscription_ids
                    .as_mut()
                    .unwrap()
                    .insert(sub.msg_id);
            }
        }
    }

    fn is_allowed(&self, msg_id: u16) -> bool {
        match &self.allowed_subscription_ids {
            None => true,
            Some(set) => set.contains(&msg_id),
        }
    }
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
    pub fn new(reader: R) -> Result<ULogParser<R>, ULogError> {
        Ok(ULogParser {
            state: State::HEADER,
            file_header: None,
            overridden_params: HashSet::new(),
            formats: HashMap::new(),
            subscriptions: HashMap::new(),
            message_name_with_multi_id: HashSet::new(),
            subscription_filter: SubscriptionFilter::default(),
            datastream: DataStream::new(reader),
            max_bytes_to_read: None,
            include_header: false,
            include_timestamp: false,
            include_padding: false,
        })
    }

    pub(crate) fn set_allowed_subscription_names(
        &mut self,
        subscr_names: impl IntoIterator<Item = String>,
    ) {
        self.subscription_filter = SubscriptionFilter::new(subscr_names);
    }

    /// Deprecated. Use `ULogParserBuilder::set_subscription_allow_list()` instead.
    /// This will be removed or made private in a future release.
    #[deprecated]
    pub fn set_subscription_allow_list(&mut self, set: HashSet<String>) {
        self.subscription_filter = SubscriptionFilter::new(set);
    }

    pub fn get_format(&self, message_name: &str) -> Result<Rc<def::Format>, ULogError> {
        match self.formats.get(message_name) {
            None => Err(UndefinedFormat(message_name.to_owned())),
            Some(format) => Ok(format.clone()),
        }
    }

    pub fn get_subscription(&self, msg_id: u16) -> Result<&msg::Subscription, ULogError> {
        match self.subscriptions.get(&msg_id) {
            None => Err(UndefinedSubscription(msg_id)),
            Some(sub) => Ok(sub),
        }
    }

    pub(crate) fn read_message(&mut self, msg_size: usize) -> Result<MessageBuf, ULogError> {
        let mut message: Vec<u8> = vec![0; msg_size];
        self.datastream.read_exact(&mut message)?;
        Ok(MessageBuf::from_vec(message))
    }

    #[allow(clippy::single_match_else)]
    fn next_sub(&mut self) -> Result<Option<msg::UlogMessage>, ULogError> {
        if self.state == State::HEADER {
            match self.read_file_header() {
                Ok(header) => {
                    self.file_header = Some(header);
                    self.state = State::DEFINITIONS;

                    #[allow(clippy::redundant_else)]
                    if self.include_header {
                        return Ok(Some(UlogMessage::Header(header)));
                    } else {
                        //Fallthrough.
                    }
                }
                Err(_) => {
                    self.state = State::ERROR;
                    return Err(ULogError::InvalidHeader);
                }
            }
        }

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
            Some(header) => (
                header.msg_type,
                self.read_message(header.msg_size as usize)?,
            ),
        };

        match self.state {
            State::DEFINITIONS => {
                let msg = self.parse_definition(message_type, message_buf)?;

                match msg {
                    UlogMessage::FormatDefinition(ref format) => {
                        if format.name.contains("heartbeat") {
                            println!("Heartbeat {format}");
                        }

                        self.formats.insert(format.name.clone(), Rc::new(format.clone()));
                    }
                    UlogMessage::AddSubscription(ref sub) => {
                        self.subscriptions.insert(sub.msg_id, sub.clone());
                        self.subscription_filter.update_ids(sub);

                        if sub.multi_id > 0 {
                            self.message_name_with_multi_id
                                .insert(sub.message_name.clone());
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
                        self.subscription_filter.update_ids(sub);

                        if sub.multi_id > 0 {
                            self.message_name_with_multi_id
                                .insert(sub.message_name.clone());
                        }
                    }
                    UlogMessage::LoggedData(ref mut logged_data) => {
                        logged_data.filter_fields(self.include_timestamp, self.include_padding);
                    }
                    _ => {}
                }

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

    pub fn parse_data(
        &mut self,
        message_type: ULogMessageType,
        mut message_buf: MessageBuf,
    ) -> Result<UlogMessage, ULogError> {
        match message_type {
            ULogMessageType::ADD_SUBSCRIPTION => {
                let sub = self.parse_subscription(message_buf)?;

                Ok(msg::UlogMessage::AddSubscription(sub))
            }
            ULogMessageType::REMOVE_SUBSCRIPTION => {
                let msg_id = message_buf.take_u16()?;
                self.subscriptions.remove(&msg_id);
                Ok(UlogMessage::Unhandled {
                    msg_type: message_type.into(),
                    message_contents: message_buf.into_remaining_bytes(),
                })
            }
            ULogMessageType::DATA => {
                let msg_id = message_buf.take_u16()?;
                if let Ok(sub) = self.get_subscription(msg_id) {
                    if self.subscription_filter.is_allowed(sub.msg_id) {
                        let logged_data = self.parse_data_message(sub, message_buf)?;

                        return Ok(msg::UlogMessage::LoggedData(logged_data));
                    } else {
                        return Ok(UlogMessage::Ignored {
                            msg_type: message_type.into(),
                            message_contents: message_buf.into_remaining_bytes(),
                        });
                    }
                } else {
                    return Err(ULogError::ParseError(format!(
                        "Received logged data with an unknown msg_id {msg_id}.  Could not find a subscription for this data. Ignoring."
                    )));
                }
            }
            ULogMessageType::LOGGING => Ok(msg::UlogMessage::LoggedString(msg::LoggedString {
                level: LogLevel::try_from(message_buf.take_u8()?)?,
                tag: None,
                timestamp: message_buf.take_u64()?,
                msg: String::from_utf8(message_buf.into_remaining_bytes())?,
            })),
            ULogMessageType::LOGGING_TAGGED => {
                Ok(msg::UlogMessage::LoggedString(msg::LoggedString {
                    level: LogLevel::try_from(message_buf.take_u8()?)?,
                    tag: Some(message_buf.take_u16()?),
                    timestamp: message_buf.take_u64()?,
                    msg: String::from_utf8(message_buf.into_remaining_bytes())?,
                }))
            }
            ULogMessageType::DROPOUT => Ok(msg::UlogMessage::DropoutMark(Dropout {
                duration: message_buf.take_u16()?,
            })),
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
                Ok(UlogMessage::Unhandled {
                    msg_type: message_type.into(),
                    message_contents: message_buf.into_remaining_bytes(),
                })
            }
        }
    }

    fn parse_subscription(
        &self,
        mut message_buf: MessageBuf,
    ) -> Result<msg::Subscription, ULogError> {
        let multi_id = message_buf.take_u8()?;
        let msg_id = message_buf.take_u16()?;

        let message_name = String::from_utf8(message_buf.into_remaining_bytes())?;

        // Force a lookup of the format and return an error if not found.
        let _format = self.get_format(&message_name)?;

        Ok(msg::Subscription {
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
        log::trace!("MSG HEADER: {msg_size} {msg_type:?}");

        Ok(Some(ULogMessageHeader { msg_size, msg_type }))
    }

    fn parse_data_message(
        &self,
        sub: &msg::Subscription,
        mut message_buf: MessageBuf,
    ) -> Result<msg::LoggedData, ULogError> {
        let format = self.get_format(&sub.message_name)?;
        let _message_len = message_buf.len();

        if !format.fields.iter().any(|f| f.name.as_ref() == "timestamp") {
            return Err(ULogError::MissingTimestamp);
        }

        let mut data_format = self.parse_data_message_sub(format, &mut message_buf)?;

        if self.message_name_with_multi_id.contains(&sub.message_name) {
            data_format.multi_id_index = Some(sub.multi_id);
        }

        // ⚠️ The timestamp for this message is the value of the `timestamp` field from the top-level `data_format`
        // returned by `parse_data_message_sub()`.  We now remove the field from `data_format` to avoid returning redundant timestamps.
        // See the comment in `parse_data_message_sub()` for more information.
        let timestamp = data_format.timestamp.ok_or(ULogError::MissingTimestamp)?;

        if !message_buf.is_empty() {
            log::warn!("Leftover bytes in messagebuf after parsing LOGGED_DATA message! Possible data corruption.");
        }

        Ok(msg::LoggedData {
            timestamp,
            msg_id: sub.msg_id,
            data: data_format,
        })
    }

    fn parse_data_message_sub(
        &self,
        format: Rc<def::Format>,
        message_buf: &mut MessageBuf,
    ) -> Result<inst::Format, ULogError> {
        let mut fields: Vec<inst::Field> = Vec::with_capacity(format.fields.len());
        let mut timestamp: Option<u64> = None;

        for field in &format.fields {
            // Handle padding fields.
            if field.name.starts_with("_padding") {
                if let Some(padding_field) = self.parse_padding(field, message_buf)? {
                    fields.push(padding_field);
                }
                continue;
            }

            let value: inst::FieldValue = self.parse_field_value(field, message_buf)?;

            // ⚠️ Extract the timestamp field if present.
            // According to the ULOG spec, the timestamp for a LOGGED_DATA message is the value of
            // the timestamp field from the Format linked to the Subscription.
            // This function will extract all such fields, regardless of location in the Format hierarchy.
            // When this function returns, the top-level timestamp will then be extracted and assigned
            // to msg::LoggedData.timestamp. See: `parse_data_message()`
            if let inst::FieldValue::ScalarU64(value) = value {
                if field.name.as_ref() == "timestamp" {
                    timestamp = Some(value);
                }
            }

            fields.push(inst::Field {
                name: Rc::from(field.name.clone()),
                r#type: field.r#type.clone(),
                value,
            });
        }

        Ok(inst::Format {
            name: format.name.clone(),
            timestamp,
            fields,
            // ⚠️ The proper value for this field can't be known at this point in the code.
            // It can be filled out only after we've seen all the subscriptions in the file.
            // We have omitted it here so it can be filled later on if required.
            multi_id_index: None,
            def_format: format.clone(),
        })
    }

    fn parse_padding(
        &self,
        field: &def::Field,
        message_buf: &mut MessageBuf,
    ) -> Result<Option<inst::Field>, ULogError> {
        let Some(array_size) = field.r#type.array_size else {
            log::warn!("Encountered padding, and type is scalar. Ignoring.");
            return Ok(None);
        };
        
        if array_size <= message_buf.len() {
            log::debug!("Encountered padding, and padding <= message.len(). Ok.");

            if self.include_padding {
                let array = message_buf.advance(array_size)?.to_vec();
                return Ok( Some( inst::Field {
                    name: Rc::from(field.name.clone()),
                    r#type: field.r#type.clone(),
                    value: inst::FieldValue::ArrayU8(array),
                }));
            } else {
                //Skip over the padding bytes.
                message_buf.skip(array_size)?;
            }
        } else {
            match message_buf.len() {
                0 => log::debug!("Encountered padding, and message.len() == 0. Ignoring as per ULOG spec."),
                _ => log::error!("Encountered padding, and padding > message.len(). Ignoring and hoping for the best"),
            }
        }
        Ok(None)
    }

    fn parse_field_value(
        &self,
        field: &def::Field,
        message_buf: &mut MessageBuf,
    ) -> Result<inst::FieldValue, ULogError> {
        match field.r#type.array_size {
            None => {
                // scalar
                use def::BaseType::*;
                use inst::FieldValue::*;
                Ok(match &field.r#type.base_type {
                    UINT8 => ScalarU8(parse_data_field(message_buf)?),
                    UINT16 => ScalarU16(parse_data_field(message_buf)?),
                    UINT32 => ScalarU32(parse_data_field(message_buf)?),
                    UINT64 => ScalarU64(parse_data_field(message_buf)?),
                    INT8 => ScalarI8(parse_data_field(message_buf)?),
                    INT16 => ScalarI16(parse_data_field(message_buf)?),
                    INT32 => ScalarI32(parse_data_field(message_buf)?),
                    INT64 => ScalarI64(parse_data_field(message_buf)?),
                    FLOAT => ScalarF32(parse_data_field(message_buf)?),
                    DOUBLE => ScalarF64(parse_data_field(message_buf)?),
                    BOOL => ScalarBool(parse_data_field(message_buf)?),
                    CHAR => ScalarChar(parse_data_field(message_buf)?),
                    OTHER(type_name) => {
                        let child_format = self.get_format(type_name)?;
                        ScalarOther(
                            self.parse_data_message_sub(child_format, message_buf)?
                                .into(),
                        )
                    }
                })
            }
            Some(array_size) => self.parse_array_field(field, array_size, message_buf),
        }
    }

    fn parse_array_field(
        &self,
        field: &def::Field,
        array_size: usize,
        message_buf: &mut MessageBuf,
    ) -> Result<inst::FieldValue, ULogError> {
        use def::BaseType::*;
        use inst::FieldValue::*;

        Ok(match &field.r#type.base_type {
            UINT8 => ArrayU8(parse_primitive_array(array_size, message_buf)?),
            UINT16 => ArrayU16(parse_primitive_array(array_size, message_buf)?),
            UINT32 => ArrayU32(parse_primitive_array(array_size, message_buf)?),
            UINT64 => ArrayU64(parse_primitive_array(array_size, message_buf)?),
            INT8 => ArrayI8(parse_primitive_array(array_size, message_buf)?),
            INT16 => ArrayI16(parse_primitive_array(array_size, message_buf)?),
            INT32 => ArrayI32(parse_primitive_array(array_size, message_buf)?),
            INT64 => ArrayI64(parse_primitive_array(array_size, message_buf)?),
            FLOAT => ArrayF32(parse_primitive_array(array_size, message_buf)?),
            DOUBLE => ArrayF64(parse_primitive_array(array_size, message_buf)?),
            BOOL => ArrayBool(parse_primitive_array(array_size, message_buf)?),
            CHAR => ArrayChar(parse_primitive_array(array_size, message_buf)?),
            OTHER(type_name) => {
                let child_format = &self.get_format(type_name)?;
                ArrayOther(parse_array(array_size, message_buf, |buf| {
                    self.parse_data_message_sub(child_format.clone(), buf)
                })?)
            }
        })
    }

    fn read_file_header(&mut self) -> Result<FileHeader, ULogError> {
        let mut msg_header = [0; 16];
        self.datastream.read_exact(&mut msg_header)?;

        if msg_header[0..7] != MAGIC {
            return Err(ULogError::InvalidMagicBits);
        }

        let file_version: u8 = msg_header[7];
        let file_start_time = LittleEndian::read_u64(&msg_header[8..16]);

        Ok(FileHeader {
            version: file_version,
            timestamp: file_start_time,
        })
    }

    fn parse_definition(
        &mut self,
        message_type: ULogMessageType,
        message_buf: MessageBuf,
    ) -> Result<msg::UlogMessage, ULogError> {
        match message_type {
            ULogMessageType::FLAG_BITS => {
                let flag_bits = self.parse_flag_bits(message_buf)?;

                if flag_bits.has_data_appended() {
                    // Stop reading from this stream at the first non-zero appended data offset in the list.
                    self.max_bytes_to_read = flag_bits
                        .appended_data_offsets
                        .iter()
                        .find(|&&offset| offset > 0)
                        .map(|&offset| offset as usize);
                }

                Ok(UlogMessage::FlagBits(flag_bits))
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
                log::warn!("Unknown message type: 0x{byte:02X}");
                Ok(UlogMessage::Unhandled {
                    msg_type: message_type.into(),
                    message_contents: message_buf.into_remaining_bytes(),
                })
            }
            _ => {
                // FIXME: Handle other variants in definitions section.
                Ok(UlogMessage::Unhandled {
                    msg_type: message_type.into(),
                    message_contents: message_buf.into_remaining_bytes(),
                })
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn parse_flag_bits(&self, mut message_buf: MessageBuf) -> Result<FlagBits, ULogError> {
        if message_buf.len() != 40 {
            log::warn!(
                "Length of flag bits >40bytes (Contained {len} extra bytes).  Ignoring.",
                len = message_buf.len()
            );
        }

        // Unwrap is ok because of the len of the array returned by advance is guaranteed to be 8.
        let compat_flags: [u8; 8] = message_buf.advance(8)?.try_into().unwrap();

        // Unwrap is ok because of the len of the array returned by advance is guaranteed to be 8.
        let incompat_flags: [u8; 8] = message_buf.advance(8)?.try_into().unwrap();

        // Check for any unknown bits in incompat_flags
        let has_unknown_incompat_bits = incompat_flags.iter().skip(1).any(|&f| f != 0);

        if has_unknown_incompat_bits {
            return Err(ULogError::UnknownIncompatBits);
        }

        let appended_data_offsets = [
            message_buf.take_u64()?,
            message_buf.take_u64()?,
            message_buf.take_u64()?,
        ];

        Ok(FlagBits {
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

        let value: inst::FieldValue = self.parse_field_value(&field, &mut message_buf)?;

        log::debug!("INFO {:?} {}:\t{}", field.r#type, &field.name, value);

        Ok(msg::Info {
            key: field.name.to_string(),
            r#type: field.r#type,
            value,
        })
    }

    pub(crate) fn parse_multi_info(
        &mut self,
        mut message_buf: MessageBuf,
    ) -> Result<msg::MultiInfo, ULogError> {
        let is_continued = message_buf.take_u8()? != 0;
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;

        let value: inst::FieldValue = self.parse_field_value(&field, &mut message_buf)?;

        log::debug!("MULTI_INFO {:?} {}:\t{}", field.r#type, &field.name, value);
        log::debug!("is_continued = {is_continued}");

        let result: MultiInfo = MultiInfo {
            key: field.name.to_string(),
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

        if field.r#type.is_array() {
            return Err(ULogError::UnknownParameterType(
                format!(
                    "Received default parameter message with type ARRAY ({raw_key}). Ignoring."
                )
                .to_owned(),
            ));
        } else {
            let value: inst::ParameterValue = match field.r#type.base_type {
                BaseType::INT32 => { inst::ParameterValue::INT32(parse_data_field::<i32>(&mut message_buf)?) }
                BaseType::FLOAT => { inst::ParameterValue::FLOAT(parse_data_field::<f32>(&mut message_buf)?) }
                other => {
                    return Err(ULogError::UnknownParameterType(format!("Received default parameter message with unsupported type ({raw_key}->{other:?}). Ignoring.").to_owned()))
                }
            };

            log::debug!("INFO {:?} {}:\t{:?}", field.r#type, &field.name, value);

            Ok(msg::Parameter {
                key: field.name.to_string(),
                r#type: field.r#type,
                value,
            })
        }
    }

    fn parse_default_parameter(
        &self,
        mut message_buf: MessageBuf,
    ) -> Result<msg::DefaultParameter, ULogError> {
        let default_types = message_buf.take_u8()?; // read the default_types bitfield
        let key_len = message_buf.take_u8()? as usize;
        let raw_key = String::from_utf8(message_buf.advance(key_len)?.to_vec())?;
        let mut tokens = TokenList::from_str(&raw_key);
        let field = parse_field(&mut tokens)?;

        if field.r#type.is_array() {
            return Err(ULogError::UnknownParameterType(
                format!(
                    "Received default parameter message with type ARRAY ({raw_key}). Ignoring."
                )
                .to_owned(),
            ));
        } else {
            let value: inst::ParameterValue = match field.r#type.base_type {
                BaseType::INT32 => { inst::ParameterValue::INT32(parse_data_field::<i32>(&mut message_buf)?) }
                BaseType::FLOAT => { inst::ParameterValue::FLOAT(parse_data_field::<f32>(&mut message_buf)?) }
                other => {
                    return Err(ULogError::UnknownParameterType(format!("Received default parameter message with unsupported type ({raw_key}->{other:?}). Ignoring.").to_owned()))
                }
            };

            log::debug!("INFO {:?} {}:\t{:?}", field.r#type, &field.name, value);

            Ok(msg::DefaultParameter {
                key: field.name.to_string(),
                default_types,
                r#type: field.r#type,
                value,
            })
        }
    }
}

#[derive(Debug)]
pub struct ULogMessageHeader {
    pub msg_size: u16,
    pub msg_type: ULogMessageType,
}

#[derive(Debug, Copy, Clone)]
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
            _ => ULogMessageType::UNKNOWN(byte),
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

impl LoggedData {
    pub fn filter_fields(&mut self, include_timestamp: bool, include_padding: bool) {
        self.data.fields.retain(|field| {
            if field.name.as_ref() == "timestamp" {
                return include_timestamp;
            }

            if field.name.starts_with("_padding") {
                return include_padding;
            }

            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Encode;
    use std::io;

    impl<R: std::io::Read> ULogParser<R> {
        pub fn insert_format(&mut self, message_name: &str, format: def::Format) {
            self.formats.insert(message_name.to_string(), format.into());
        }
    }

    #[test]
    fn test_round_trip_format() {
        let input = b"my_format:uint64_t timestamp;custom_type custom_field;bool is_happy;custom_type2[4] custom_field;uint8_t[8] pet_ids;";
        let message_buf = MessageBuf::from_vec(input.to_vec());

        let parsed_format = parse_format(message_buf).unwrap();

        let mut emitted_bytes = Vec::new();
        parsed_format
            .encode(&mut emitted_bytes)
            .expect("Unable to encode format?!");

        println!(
            "re_emitted_bytes: {:?}",
            String::from_utf8(emitted_bytes.clone()).unwrap()
        );

        assert_eq!(emitted_bytes, input);
    }

    #[test]
    fn test_round_trip_subscription() {
        //  \x0A    \x01\x00  my_message
        // multi_id  msg_id   message_name
        let input_bytes = b"\x0A\x01\x00my_message";

        let cursor = io::Cursor::new(input_bytes);

        let mut parser = ULogParser::new(cursor).expect("Unable to create ULogParser");

        parser.insert_format(
            "my_message",
            def::Format {
                name: "".to_string(),
                fields: vec![],
                padding: 0,
            },
        );

        // MessageBuf should not contain the header bytes, which is why initialise it from byte 3 onwards.
        let message_buf = MessageBuf::from_vec(input_bytes.to_vec());

        // Parse the Subscription
        let parsed_subscription = parser
            .parse_subscription(message_buf)
            .expect("Unable to parse subscription");
        println!("parsed_subscription: {:?}", parsed_subscription);

        let mut emitted_bytes = Vec::new();
        parsed_subscription
            .encode(&mut emitted_bytes)
            .expect("Unable to encode subscription?!");

        println!("Emitted bytes: {:?}", emitted_bytes);

        assert_eq!(emitted_bytes, input_bytes);
    }
}

