use std::collections::HashMap;
use std::fs::File;
use ulog_rs::datastream::DataStream;
use ulog_rs::parser::{Timeseries, ULogError, ULogParser};


fn main() -> Result<(), Box<dyn std::error::Error>>  {
    let mut timeseries_map: HashMap<String, Timeseries> = HashMap::new();
    let file = File::open("data/sample_log_small.ulg")?;

    let file = File::open("data/sample_log_small.ulg").map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;

    let data_stream = DataStream::new(file);

    let parser = ULogParser::new(data_stream, &mut timeseries_map).map_err(|e| {
        eprintln!("Failed to create ULogParser: {:?}", e);
        e
    })?;

    Ok(())
}