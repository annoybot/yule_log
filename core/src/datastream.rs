use std::io::{ErrorKind, Read};

use byteorder::{ByteOrder, LittleEndian};

use crate::errors::ULogError;

#[derive(Debug)]
pub struct DataStream<R: Read> {
    reader: R,
    pub(crate) num_bytes_read: usize,
    pub(crate) eof: bool,
}

impl<R: Read> DataStream<R> {
    pub fn new(reader: R) -> DataStream<R> {
        DataStream {
            reader,
            num_bytes_read: 0,
            eof: false,
        }
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<usize, ULogError> {
        log::trace!(
            "datastream read from:  [{:04X}-{:04X}]",
            self.num_bytes_read,
            self.num_bytes_read + buf.len()
        );
        self.num_bytes_read += buf.len();

        match self.reader.read_exact(buf) {
            Ok(()) => Ok(buf.len()),
            Err(err) => match err.kind() {
                // Eof is not technically an error, so signal it by reporting 0 bytes read and setting eof true.
                ErrorKind::UnexpectedEof => {
                    self.eof = true;
                    Ok(0)
                }
                _ => Err(ULogError::Io(err)),
            },
        }
    }

    /// Skips the specified number of bytes in the underlying reader.
    pub fn skip(&mut self, num_bytes: usize) -> Result<usize, ULogError> {
        let mut total_skipped = 0;
        while total_skipped < num_bytes {
            // Calculate how many bytes remaining to skip
            let bytes_to_skip = num_bytes - total_skipped;

            // Attempt to read bytes without storing them
            let bytes_read = self
                .reader
                .by_ref()
                .take(bytes_to_skip as u64)
                .read_to_end(&mut vec![])
                .map_err(ULogError::Io)?;

            if bytes_read == 0 {
                break; // End of stream reached
            }
            total_skipped += bytes_read;
        }
        self.num_bytes_read += total_skipped;
        Ok(total_skipped)
    }

    pub fn read_u8(&mut self) -> Result<u8, ULogError> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_u16(&mut self) -> Result<u16, ULogError> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_u16(&buf))
    }

    pub fn read_u32(&mut self) -> Result<u32, ULogError> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_u32(&buf))
    }

    pub fn read_u64(&mut self) -> Result<u64, ULogError> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_u64(&buf))
    }

    pub fn read_i8(&mut self) -> Result<i8, ULogError> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;

        #[allow(clippy::cast_possible_wrap)]
        Ok(buf[0] as i8)
    }

    pub fn read_i16(&mut self) -> Result<i16, ULogError> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_i16(&buf))
    }

    pub fn read_i32(&mut self) -> Result<i32, ULogError> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_i32(&buf))
    }

    pub fn read_f32(&mut self) -> Result<f32, ULogError> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_f32(&buf))
    }

    pub fn read_f64(&mut self) -> Result<f64, ULogError> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(LittleEndian::read_f64(&buf))
    }

    pub fn read_bool(&mut self) -> Result<bool, ULogError> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] != 0)
    }

    pub fn read_string(&mut self, len: usize) -> Result<String, ULogError> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(ULogError::Utf8)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom, Write};
    use std::mem;

    use tempfile::tempfile;

    use super::*;

    fn write_record<W: Write>(
        writer: &mut W,
        type_name: &str,
        length: u32,
        value_bytes: &[u8],
    ) -> std::io::Result<()> {
        let mut type_name_bytes = [0u8; 4];
        type_name_bytes[..type_name.len()].copy_from_slice(type_name.as_bytes());
        writer.write_all(&type_name_bytes)?;
        writer.write_all(&length.to_le_bytes())?;
        writer.write_all(value_bytes)?;
        Ok(())
    }

    #[test]
    // Test the DataStream read functions with a temporary file.
    // The file contains a record for each supported type, with the following format:
    // - 4 bytes: type name
    // - 4 bytes: length of the data
    // - data
    // First we create a temporary file and write the records, then we read them back and verify
    // that the data matches.
    fn test_datastream_read() -> Result<(), ULogError> {
        let mut file = tempfile().unwrap();

        // Write headers and data
        write_record(&mut file, "u8", mem::size_of::<u8>() as u32, &[42u8]).unwrap();
        write_record(&mut file, "i8", mem::size_of::<i8>() as u32, &[42i8 as u8]).unwrap();
        write_record(
            &mut file,
            "u16",
            mem::size_of::<u16>() as u32,
            &42u16.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "i16",
            mem::size_of::<i16>() as u32,
            &42i16.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "u32",
            mem::size_of::<u32>() as u32,
            &42u32.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "i32",
            mem::size_of::<i32>() as u32,
            &42i32.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "u64",
            mem::size_of::<u64>() as u32,
            &42u64.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "f32",
            mem::size_of::<f32>() as u32,
            &42.0f32.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "f64",
            mem::size_of::<f64>() as u32,
            &42.0f64.to_le_bytes(),
        )
        .unwrap();
        write_record(&mut file, "bool", mem::size_of::<bool>() as u32, &[1u8]).unwrap();
        write_record(&mut file, "str", 5, b"hello").unwrap(); // String length is still hardcoded

        file.flush().unwrap();

        file.seek(SeekFrom::Start(0)).unwrap();

        let mut datastream = DataStream::new(file);

        // Read and verify data
        fn read_record<R: Read>(datastream: &mut DataStream<R>) -> Result<(), ULogError> {
            let type_name = datastream
                .read_string(4)?
                .trim_end_matches('\0')
                .to_string();
            let length = datastream.read_u32()? as usize;

            match type_name.as_str() {
                "u8" => {
                    assert_eq!(length, mem::size_of::<u8>());
                    assert_eq!(datastream.read_u8()?, 42);
                }
                "i8" => {
                    assert_eq!(length, mem::size_of::<i8>());
                    assert_eq!(datastream.read_i8()?, 42);
                }
                "u16" => {
                    assert_eq!(length, mem::size_of::<u16>());
                    assert_eq!(datastream.read_u16()?, 42);
                }
                "i16" => {
                    assert_eq!(length, mem::size_of::<i16>());
                    assert_eq!(datastream.read_i16()?, 42);
                }
                "u32" => {
                    assert_eq!(length, mem::size_of::<u32>());
                    assert_eq!(datastream.read_u32()?, 42);
                }
                "i32" => {
                    assert_eq!(length, mem::size_of::<i32>());
                    assert_eq!(datastream.read_i32()?, 42);
                }
                "u64" => {
                    assert_eq!(length, mem::size_of::<u64>());
                    assert_eq!(datastream.read_u64()?, 42);
                }
                "f32" => {
                    assert_eq!(length, mem::size_of::<f32>());
                    assert_eq!(datastream.read_f32()?, 42.0);
                }
                "f64" => {
                    assert_eq!(length, mem::size_of::<f64>());
                    assert_eq!(datastream.read_f64()?, 42.0);
                }
                "bool" => {
                    assert_eq!(length, mem::size_of::<bool>());
                    assert_eq!(datastream.read_bool()?, true);
                }
                "str" => {
                    assert_eq!(length, 5);
                    assert_eq!(datastream.read_string(length)?, "hello".to_string());
                }
                _ => panic!("Unknown type: {}", type_name),
            }

            Ok(())
        }

        for _ in 0..11 {
            read_record(&mut datastream)?;
        }

        Ok(())
    }

    #[test]
    fn test_datastream_skip() -> Result<(), ULogError> {
        let mut file = tempfile().unwrap();

        // Write some records to the file
        write_record(&mut file, "u8", mem::size_of::<u8>() as u32, &[42u8]).unwrap();
        write_record(&mut file, "i8", mem::size_of::<i8>() as u32, &[42i8 as u8]).unwrap();
        write_record(
            &mut file,
            "u16",
            mem::size_of::<u16>() as u32,
            &42u16.to_le_bytes(),
        )
        .unwrap();
        write_record(
            &mut file,
            "f32",
            mem::size_of::<f32>() as u32,
            &42.0f32.to_le_bytes(),
        )
        .unwrap();

        file.flush().unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        let mut datastream = DataStream::new(file);

        // Skip the first record (4 bytes for type name + 4 bytes for length + 1 byte for u8)
        let skipped_bytes = datastream.skip(9)?; // 4 + 4 + 1 = 9
        assert_eq!(skipped_bytes, 9);

        // Read the next record, which should be the i8 record now
        let type_name = datastream
            .read_string(4)?
            .trim_end_matches('\0')
            .to_string();
        let length = datastream.read_u32()? as usize;

        assert_eq!(type_name, "i8");
        assert_eq!(length, mem::size_of::<i8>());
        assert_eq!(datastream.read_i8()?, 42);

        // Skip the next record
        let skipped_bytes = datastream.skip(10)?; // 4 + 4 + 2 = 10 (u16 record)
        assert_eq!(skipped_bytes, 10);

        // Read the next record, which should be the f32 record now
        let type_name = datastream
            .read_string(4)?
            .trim_end_matches('\0')
            .to_string();
        let length = datastream.read_u32()? as usize;

        assert_eq!(type_name, "f32");
        assert_eq!(length, mem::size_of::<f32>());
        assert_eq!(datastream.read_f32()?, 42.0);

        Ok(())
    }
}
