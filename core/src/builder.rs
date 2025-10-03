use std::collections::HashSet;
use std::io::Read;

use crate::errors::ULogError;
use crate::parser::ULogParser;

pub struct ULogParserBuilder<R> {
    reader: R,
    include_header: bool,
    include_timestamp: bool,
    include_padding: bool,
    allowed_subscription_names: Option<HashSet<String>>,
}

impl<R: Read> ULogParserBuilder<R> {
    // Start the builder with a mandatory reader
    #[must_use]
    pub fn new(reader: R) -> Self {
        ULogParserBuilder {
            reader,
            include_header: false,
            include_timestamp: false,
            include_padding: false,
            allowed_subscription_names: None,
        }
    }

    #[must_use]
    pub fn include_header(mut self, include: bool) -> Self {
        self.include_header = include;
        self
    }

    #[must_use]
    pub fn include_timestamp(mut self, include: bool) -> Self {
        self.include_timestamp = include;
        self
    }

    #[must_use]
    pub fn include_padding(mut self, include: bool) -> Self {
        self.include_padding = include;
        self
    }

    /// Sets the list of `LoggedData` messages that the parser will return.
    ///
    /// By default, all `LoggedData` messages will be returned, which incurs extra parsing cost. 
    /// 
    /// Specifying only the required messages in this allow list can greatly improve parser performance.
    ///
    /// Any `LoggedData` messages not included in this allow list will be emitted as raw bytes in a 
    /// `UlogMessage::Ignored` variant, so no messages are lost.
    ///
    /// # Parameters
    /// - `subs`: An iterable collection of string-like items representing the names of `LoggedData` messages
    ///           to be parsed fully and returned.
    #[must_use]
    pub fn set_subscription_allow_list<I, S>(mut self, subs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let set: HashSet<String> = subs.into_iter().map(|s| s.into()).collect();
        self.allowed_subscription_names = Some(set);
        self
    }

    // Final method to build the `ULogParser`
    pub fn build(self) -> Result<ULogParser<R>, ULogError> {
        let result = ULogParser::new(self.reader);

        match  result {
            Ok(mut parser) => {
                parser.include_header = self.include_header;
                parser.include_timestamp = self.include_timestamp;
                parser.include_padding = self.include_padding;
                
                if let Some(allowed_subscr) = self.allowed_subscription_names {
                    parser.set_allowed_subscription_names(allowed_subscr);
                }
                
            Ok(parser)
            }
            Err(err) => { Err(err) }
        }
    }
}
