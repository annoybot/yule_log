use std::io::Read;

use crate::errors::ULogError;
use crate::parser::ULogParser;

pub struct ULogParserBuilder<R> {
    reader: R,
    include_header: bool,
    include_timestamp: bool,
    include_padding: bool,
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

    // Final method to build the `ULogParser`
    pub fn build(self) -> Result<ULogParser<R>, ULogError> {
        let result = ULogParser::new(self.reader);

        match  result {
            Ok(mut parser) => {
                parser.include_header = self.include_header;
                parser.include_timestamp = self.include_timestamp;
                parser.include_padding = self.include_padding;

                Ok(parser)
            }
            Err(err) => { Err(err) }
        }
    }
}
