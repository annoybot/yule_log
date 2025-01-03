use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use env_logger::Builder;
use log::LevelFilter;

use ulog_rs::csv::{Column, ColumnMap, CsvExporter, Point};
use ulog_rs::model::{inst, msg};
use ulog_rs::parser::ULogParser;

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
    let mut column_map = ColumnMap::new();

    let file = File::open(path).map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;

    let reader = BufReader::new(file);

    let parser = ULogParser::new(reader).map_err(|e| {
        eprintln!("Failed to create ULogParser: {:?}", e);
        e
    })?;

    for result in parser {
        let ulog_message = result?;

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                let field_list:Vec<(String, inst::BaseType)> = logged_data.data.flatten();

                for (index, (path, value)) in field_list.iter().enumerate() {
                    let column = column_map.entry(path.clone()).
                        or_insert_with(|| Column::new(path.to_owned()));

                    let value = match value {
                        inst::BaseType::UINT8(v) => *v as f64,
                        inst::BaseType::UINT16(v) => *v as f64,
                        inst::BaseType::UINT32(v) => *v as f64,
                        inst::BaseType::UINT64(v) => *v as f64,
                        inst::BaseType::INT8(v) => *v as f64,
                        inst::BaseType::INT16(v) => *v as f64,
                        inst::BaseType::INT32(v) => *v as f64,
                        inst::BaseType::INT64(v) => *v as f64,
                        inst::BaseType::FLOAT(v) => *v as f64,
                        inst::BaseType::DOUBLE(v) => *v,
                        inst::BaseType::BOOL(v) => if *v { 1.0 } else { 0.0 },
                        inst::BaseType::CHAR(v) => *v as u8 as f64,
                        inst::BaseType::OTHER(_) => unreachable!(), // Placeholder, since OTHER is not expected here
                    };

                    let timestamp_seconds = (logged_data.timestamp as f64) * 0.000001;

                    column.push(Point {
                        t: timestamp_seconds,
                        x: value
                    });
                }
            }
            _ => {
                // Ignore other message types
            }
        }
    }
    
    log::info!("Done parsing {:?}", path);

    // Save file as <orig_filename>_exported.csv
    let csv_filename = path.with_file_name(format!("{}_export", path.file_stem().unwrap().to_str().unwrap())).with_extension("csv");
    let file = File::create(csv_filename)?;
    let mut writer = BufWriter::new(file);

    let columns:Vec<Column> = column_map.0.into_values().collect();

    let mut csv_exporter = CsvExporter::from_columns(columns);

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