use std::error::Error;
use std::fs;
use std::fs::File;
use std::env;
use env_logger::Builder;
use log::LevelFilter;
use std::io::{BufWriter, Write};
use ulog_rs::csv::CsvExporter;
use ulog_rs::datastream::DataStream;
use ulog_rs::parser::ULogParser;
use ulog_rs::timeseries::TimeseriesMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    if path.is_dir() {
        process_directory(path)?;
    } else if path.is_file() {
        process_file(path)?;
    } else {
        eprintln!("Error: The specified path is neither a file nor a directory.");
        std::process::exit(1);
    }

    Ok(())
}

fn process_file(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut timeseries_map: TimeseriesMap = TimeseriesMap::new();
    let file = File::open(&path).map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;
    
    let mut data_stream = DataStream::new(file);

    let mut parser = ULogParser::new().map_err(|e| {
        eprintln!("Failed to create ULogParser: {:?}", e);
        e
    })?;

    //FIXME: This filtering feature might not be working yet.  I've been comparing the full output filterd by xsv and it doesn't match?
    // parser.filter_by_paths(&vec!["vehicle_local_position/x", "vehicle_local_position/y", "vehicle_local_position/z"]);

    parser.parse(&mut data_stream, &mut timeseries_map)?;

    log::info!("Done parsing {:?}", path);

    // Save file as <orig_filename>_exported.csv
    let csv_filename = path.with_file_name(format!("{}_export", path.file_stem().unwrap().to_str().unwrap())).with_extension("csv");
    let file = File::create(csv_filename)?;
    let mut writer = BufWriter::new(file);

    let mut csv_exporter = CsvExporter::from_timeseries_map(timeseries_map);
    csv_exporter.to_csv(&mut writer)?;
    writer.flush()?;

    log::info!("Exported {} to CSV", path.display());

    Ok(())
}

fn process_directory(dir: &Path) -> Result<(), Box<dyn Error>> {
    // Find all ULOG files in the specified directory
    let ulg_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok) // Handle possible errors
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        process_file(&path)?;
    }
    Ok(())
}