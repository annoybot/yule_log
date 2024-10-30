use std::fs;
use std::fs::File;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use ulog_rs::csv::CsvExporter;
use ulog_rs::datastream::DataStream;
use ulog_rs::parser::ULogParser;
use ulog_rs::timeseries::TimeseriesMap;

fn main() -> Result<(), Box<dyn std::error::Error>>  {
    Builder::new()
        .filter(None, LevelFilter::Debug)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    let mut timeseries_map: TimeseriesMap = TimeseriesMap::new();

    let file = File::open("data/trig_stats.ulg").map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;

    let data_stream = DataStream::new(file);

    let _parser = ULogParser::new(data_stream, &mut timeseries_map).map_err(|e| {
        eprintln!("Failed to create ULogParser: {:?}", e);
        e
    })?;

    let csv_exporter = CsvExporter::from_timeseries_map(timeseries_map);

    let csv_data = csv_exporter.to_csv_string()?;

    fs::write("data/trig_stats_export.csv", csv_data)?;

    Ok(())
}