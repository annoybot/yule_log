use std::collections::HashMap;
use std::fs::File;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use ulog_rs::datastream::DataStream;
use ulog_rs::parser::{Timeseries, ULogParser};


fn main() -> Result<(), Box<dyn std::error::Error>>  {
    Builder::new()
        .filter(None, LevelFilter::Trace)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    let mut timeseries_map: HashMap<String, Timeseries> = HashMap::new();

    let file = File::open("data/powers.ulg").map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;

    let data_stream = DataStream::new(file);

    let _parser = ULogParser::new(data_stream, &mut timeseries_map).map_err(|e| {
        eprintln!("Failed to create ULogParser: {:?}", e);
        e
    })?;

    Ok(())
}