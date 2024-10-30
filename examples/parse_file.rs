use std::error::Error;
use std::fs;
use std::fs::File;
use env_logger::Builder;
use log::LevelFilter;
use std::io::{BufWriter, Write};
use csv::Writer;
use ulog_rs::csv::CsvExporter;
use ulog_rs::datastream::DataStream;
use ulog_rs::parser::ULogParser;
use ulog_rs::timeseries::TimeseriesMap;


fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    // Find all ULOG files in the current directory
    let ulg_files: Vec<_> = fs::read_dir("data")?
        .filter_map(Result::ok) // Handle possible errors
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        let mut timeseries_map: TimeseriesMap = TimeseriesMap::new();
        let file = File::open(&path).map_err(|e| {
            eprintln!("Failed to open file: {}", e);
            e
        })?;
        let data_stream = DataStream::new(file);

        let _parser = ULogParser::new(data_stream, &mut timeseries_map).map_err(|e| {
            eprintln!("Failed to create ULogParser: {:?}", e);
            e
        })?;

        log::info!("Done parsing {:?}", path);

        // Save file as <orig_filename>_exported.csv
        let csv_filename = path.with_file_name(format!("{}_export", path.file_stem().unwrap().to_str().unwrap())).with_extension("csv");
        let file = File::create(csv_filename)?;
        let mut writer = BufWriter::new(file);

        let csv_exporter = CsvExporter::from_timeseries_map(timeseries_map);
        csv_exporter.to_csv(&mut writer)?;
        writer.flush()?;

        log::info!("Exported {} to CSV", path.display());
    }

    Ok(())
}
