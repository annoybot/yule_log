#![allow(dead_code)]

use std::mem::size_of;

use byteorder::{ByteOrder, LittleEndian};

use crate::errors::ULogError;

/// `MessageBuf` wraps a vector of bytes and allows the user to
/// successively take values from it without manually calculating
/// index offsets. Each `take_*` method retrieves the next value
/// of a specific type and advances the internal index accordingly.
/// Little endian byte order is assumed.
///
/// # Example
///
/// ```rust
/// use byteorder::LittleEndian;
/// use ulog_rs::message_buf::MessageBuf;
///
/// // Create a buffer with mixed types:
/// // - 0xDEADBEEF (u32)
/// // - 0x7F (i8)
/// // - 0xBEEF (u16)
/// let buf: Vec<u8> = vec![
///     0xEF, 0xBE, 0xAD, 0xDE, // u32: 0xDEADBEEF
///     0x7F,                   // i8: 127
///     0xEF, 0xBE,             // u16: 0xBEEF
/// ];
///
/// // Initialize the MessageBuf
/// let mut message_buf = MessageBuf::from_vec(buf);
///
/// // Take values successively without needing to track offsets:
/// let val_u32 = message_buf.take_u32().unwrap(); // 0xDEADBEEF
/// let val_i8 = message_buf.take_i8().unwrap();   // 127
/// let val_u16 = message_buf.take_u16().unwrap(); // 0xBEEF
///
/// assert_eq!(val_u32, 0xDEADBEEF);
/// assert_eq!(val_i8, 127);
/// assert_eq!(val_u16, 0xBEEF);
/// ```
pub struct MessageBuf {
    /// The raw byte vector from which values will be read.
    buf: Vec<u8>,

    /// The current position in the byte vector, starting at zero.
    current_index: usize,
}

impl MessageBuf {
    /// Creates a new `MessageBuf` from the provided byte vector.
    ///
    /// # Arguments
    ///
    /// * `buf` - A `Vec<u8>` containing the raw bytes.
    ///
    /// # Returns
    ///
    /// A new `MessageBuf` instance initialized with the provided byte vector and the current index set to zero.
    pub fn new(buf: Vec<u8>) -> Self {
        Self {
            buf,
            current_index: 0,
        }
    }

    /// Creates a new `MessageBuf` from a `Vec<u8>`.
    ///
    /// # Arguments
    ///
    /// * `buf` - A `Vec<u8>` containing the raw bytes.
    ///
    /// # Returns
    ///
    /// A new `MessageBuf` instance initialized with the provided byte vector and the current index set to zero.
    pub fn from_vec(buf: Vec<u8>) -> Self {
        Self {
            buf,
            current_index: 0,
        }
    }

    /// Returns the number of remaining bytes in the buffer.
    ///
    /// This method calculates how many bytes are left to be taken
    /// from the buffer, based on the current index.
    ///
    /// # Returns
    ///
    /// The number of remaining bytes in the buffer.
    pub fn len(&self) -> usize {
        self.buf.len().saturating_sub(self.current_index)
    }

    /// Checks if the buffer has no remaining bytes.
    ///
    /// This method returns `true` if there are no more bytes to be taken
    /// from the buffer, i.e., the current index has reached the end of the buffer.
    ///
    /// # Returns
    ///
    /// `true` if the buffer is empty, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Consumes the `MessageBuf` and returns the remaining bytes starting from the current index.
    ///
    /// After calling this method, the `MessageBuf` is invalidated and can no longer be used.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the remaining bytes from the current position to the end of the buffer.
    pub fn into_remaining_bytes(self) -> Vec<u8> {
        self.buf[self.current_index..].to_vec()
    }

    /// Takes the next `u8` value from the buffer and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `u8` value or an error message if
    /// the buffer is exhausted.
    pub fn take_u8(&mut self) -> Result<u8, ULogError> {
        self.advance(size_of::<u8>()).map(|bytes| bytes[0])
    }

    /// Takes the next `i8` value from the buffer and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `i8` value or an error message if
    /// the buffer is exhausted.
    #[allow(clippy::cast_possible_wrap)]
    pub fn take_i8(&mut self) -> Result<i8, ULogError> {
        self.advance(size_of::<i8>()).map(|bytes| bytes[0] as i8)
    }

    /// Takes the next `u16` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `u16` value or an error message if
    /// the buffer is exhausted.
    pub fn take_u16(&mut self) -> Result<u16, ULogError> {
        self.advance(size_of::<u16>())
            .map(LittleEndian::read_u16)
    }

    /// Takes the next `i16` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `i16` value or an error message if
    /// the buffer is exhausted.
    pub fn take_i16(&mut self) -> Result<i16, ULogError> {
        self.advance(size_of::<i16>())
            .map(LittleEndian::read_i16)
    }

    /// Takes the next `u32` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `u32` value or an error message if
    /// the buffer is exhausted.
    pub fn take_u32(&mut self) -> Result<u32, ULogError> {
        self.advance(size_of::<u32>())
            .map(LittleEndian::read_u32)
    }

    /// Takes the next `i32` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `i32` value or an error message if
    /// the buffer is exhausted.
    pub fn take_i32(&mut self) -> Result<i32, ULogError> {
        self.advance(size_of::<i32>())
            .map(LittleEndian::read_i32)
    }

    /// Takes the next `u64` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `u64` value or an error message if
    /// the buffer is exhausted.
    pub fn take_u64(&mut self) -> Result<u64, ULogError> {
        self.advance(size_of::<u64>())
            .map(LittleEndian::read_u64)
    }

    /// Takes the next `i64` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `i64` value or an error message if
    /// the buffer is exhausted.
    pub fn take_i64(&mut self) -> Result<i64, ULogError> {
        self.advance(size_of::<i64>())
            .map(LittleEndian::read_i64)
    }

    /// Takes the next `f32` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `f32` value or an error message if
    /// the buffer is exhausted.
    pub fn take_f32(&mut self) -> Result<f32, ULogError> {
        self.advance(size_of::<f32>())
            .map(LittleEndian::read_f32)
    }

    /// Takes the next `f64` value (in little-endian format) from the buffer
    /// and advances the index.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `f64` value or an error message if
    /// the buffer is exhausted.
    pub fn take_f64(&mut self) -> Result<f64, ULogError> {
        self.advance(size_of::<f64>())
            .map(LittleEndian::read_f64)
    }

    /// Takes the next `bool` value from the buffer and advances the index.
    /// A value of `0` is considered `false`, and any other value is `true`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the next `bool` value or an error message if
    /// the buffer is exhausted.
    pub fn take_bool(&mut self) -> Result<bool, ULogError> {
        self.take_u8().map(|val| val != 0)
    }

    /// Advances the internal index by the given size and returns the
    /// corresponding byte slice from the buffer.
    ///
    /// # Arguments
    ///
    /// * `size` - The number of bytes to advance and return.
    ///
    /// # Returns
    ///
    /// A `Result` containing a reference to the next slice of bytes or an
    /// error message if there are not enough remaining bytes in the buffer.
    pub fn advance(&mut self, size: usize) -> Result<&[u8], ULogError> {
        if self.current_index + size > self.buf.len() {
            Err(ULogError::ParseError(format!(
                "MessageBuf: Out of bounds: tried to read {} bytes, but only {} remaining",
                size,
                self.buf.len() - self.current_index
            )))
        } else {
            let bytes = &self.buf[self.current_index..self.current_index + size];
            self.current_index += size;
            Ok(bytes)
        }
    }

    /// Skips the specified number of bytes in the buffer by advancing
    /// the internal index.
    ///
    /// # Arguments
    ///
    /// * `size` - The number of bytes to skip.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the operation was successful or an
    /// error message if there are not enough remaining bytes in the buffer.
    pub fn skip(&mut self, size: usize) -> Result<(), ULogError> {
        self.advance(size).map(|_| ()) // Discard the result of advance
    }
}