use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use yule_log::builder::ULogParserBuilder;
use yule_log::model::msg::UlogMessage;

fn process_file(ulog_path: Box<Path>) -> Result<(), Box<dyn Error>> {
    let reader = BufReader::new(File::open(ulog_path.clone())?);

    let parser = ULogParserBuilder::new(reader)
        .include_header(true)
        .include_timestamp(true)
        .include_padding(true)
        .build()?;

    for result in parser {
        let ulog_message = result?;

        match ulog_message {
            UlogMessage::Header(header) => println!("HEADER: {header:?}"),
            UlogMessage::FlagBits(flag_bits) => println!("FLAG_BITS: {flag_bits:?}"),
            UlogMessage::Info(info) => println!("INFO: {info}"),
            UlogMessage::MultiInfo(multi_info) => println!("MULTI INFO: {multi_info}"),
            UlogMessage::FormatDefinition(format) => println!("FORMAT_DEFINITION: {format:?}"),
            UlogMessage::Parameter(param) => println!("PARAM: {param}"),
            UlogMessage::DefaultParameter(param) => println!("PARAM DEFAULT: {param}"),
            UlogMessage::LoggedData(data) => println!("LOGGED_DATA: {data:?}"),
            UlogMessage::AddSubscription(sub) => println!("SUBSCRIPTION: {sub:?}"),
            UlogMessage::LoggedString(log) => println!("LOGGED_STRING: {log}"),
            UlogMessage::TaggedLoggedString(log) => println!("TAGGED_LOGGED_STRING: {log}"),
            UlogMessage::Unhandled { msg_type, .. } => println!("Unhandled msg type: {}", msg_type as char),
            UlogMessage::Ignored { msg_type, .. } => println!("Ignored msg type:  {}", msg_type as char),
            UlogMessage::DropoutMark(dropout) => { println!("Dropout mark: {dropout}") }
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <ulog file>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    if path.is_file() {
        process_file(path.into()).unwrap();
    } else {
        eprintln!("Error: The path must specify a ulog file. Directories are not supported..");
        std::process::exit(1);
    }
}