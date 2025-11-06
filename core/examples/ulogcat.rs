use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use env_logger::Builder;
use log::LevelFilter;

use yule_log::builder::ULogParserBuilder;
use yule_log::encode::Encode;

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

    if path.is_file() {
        process_file(path.into())?;
    } else {
        eprintln!("Error: The path must specify a ulog file. Directories are not supported..");
        std::process::exit(1);
    }

    Ok(())
}

fn process_file(ulog_path: Box<Path>) -> Result<(), Box<dyn Error>> {
    let reader = BufReader::new(File::open(ulog_path.clone())?);

    let parser = ULogParserBuilder::new(reader)
        .include_header(true)
        .include_timestamp(true)
        .include_padding(true)
        .build()?;

    // Create the output file path.
    let output_path = ulog_path.with_extension(""); // Strip the `.ulg` extension
    let output_path = output_path.with_file_name(format!(
        "{}_emitted.ulg",
        output_path.file_name().unwrap().to_string_lossy()
    ));

    // Open the output file for writing.
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    for result in parser {
        let ulog_message = result?;
        ulog_message.encode(&mut writer)?;
    }

    writer.flush()?;
    Ok(())
}
