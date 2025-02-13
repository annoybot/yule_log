use std::collections::BTreeSet;
use std::{env, fs};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use yule_log::builder::ULogParserBuilder;
use yule_log::model::msg::UlogMessage;

fn main()  -> Result<(), Box<dyn Error>>  {
    let mut multi_id_subscription_names:BTreeSet<String> = BTreeSet::new();
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <ulog file>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    if path.is_file() {
        process_file(path.into(), &mut multi_id_subscription_names)?;
    } else {
        process_directory(path, &mut multi_id_subscription_names)?;
    }


    for sub in multi_id_subscription_names.iter() {
        println!("{}", sub);
    }
    
    Ok(())
}

fn process_file(ulog_path: &Path, multi_id_sub_names: &mut BTreeSet<String>) -> Result<(), Box<dyn Error>> {
    let reader = BufReader::new(File::open(ulog_path)?);
    
    let parser = ULogParserBuilder::new(reader)
        .include_header(true)
        .include_timestamp(true)
        .include_padding(true)
        .build()?;

    for result in parser {
        let ulog_message = result?;

        match ulog_message {
            UlogMessage::AddSubscription(sub) => {
                if sub.multi_id > 0 {
                    multi_id_sub_names.insert(sub.message_name);
                }
            }
            _ => ()
        }
    }

    Ok(())
}

fn process_directory(dir: &Path, multi_id_sub_names: &mut BTreeSet<String>) -> Result<(), Box<dyn Error>> {
    // Find all ULOG files in the specified directory
    let ulg_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok) // Handle possible errors
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        process_file(&path, multi_id_sub_names)?;
    }
    Ok(())
}