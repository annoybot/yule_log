use std::{env, fs};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use aws_config::BehaviorVersion;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_timestreamwrite::Client;
use aws_sdk_timestreamwrite::types::{Dimension, MeasureValue, MeasureValueType, Record};
use env_logger::Builder;
use log::LevelFilter;
use rusqlite::Connection;
use tokio::task;
use tokio::time;
use tracing_subscriber;
use ulog_rs::datastream::DataStream;
use ulog_rs::errors::ULogError;
use ulog_rs::parser::ULogParser;
use std::io::{BufReader, Write};
use aws_sdk_timestreamwrite::config::http::HttpResponse;
use aws_sdk_timestreamwrite::error::SdkError;
use aws_sdk_timestreamwrite::operation::write_records::WriteRecordsError;
use chrono::{NaiveDateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use ulog_rs::model::inst;
use ulog_rs::model::msg::UlogMessage;

const DATABASE_NAME: &str = "dummy_database";
const TABLE_NAME: &str = "ulog_rs_20240530T132652_4";
const REPORT_INTERVAL_SECONDS: u64 = 5;

lazy_static! {
        static ref ALLOWED_SUBSCRIPTION_NAMES: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("vehicle_local_position".to_string());
        set.insert("vehicle_gps_position".to_string());
        set.insert("vehicle_angular_velocity".to_string());
        set.insert("vehicle_attitude".to_string());
        set.insert("vehicle_angular_velocity".to_string());
        set.insert("actuator_outputs".to_string());
        set
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init(); // Initialize tracing subscriber for logging

    let region_provider = RegionProviderChain::default_provider().or_else("us-west-2");
    let config = aws_config::defaults(BehaviorVersion::v2024_03_28()).region(region_provider).load().await;

    // You MUST call `with_endpoint_discovery_enabled` to produce a working client for this service.
    let client = match aws_sdk_timestreamwrite::Client::new(&config).with_endpoint_discovery_enabled().await {
        Ok(client) => client.0,
        Err(err) => {
            println!("Error creating client: {:?}", err);
            std::process::exit(1);
        }
    };

    // Initialize logging
    /*
    Conflicts with the tracing_subscriber above
    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();
    */

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    let record_count = Arc::new(AtomicUsize::new(0));
    let data_size = Arc::new(AtomicUsize::new(0));

    // Start a background task to report the metrics
    let record_count_clone = Arc::clone(&record_count);
    let data_size_clone = Arc::clone(&data_size);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(REPORT_INTERVAL_SECONDS));
        let mut last_record_count = 0;
        let mut last_data_size = 0;
        loop {
            interval.tick().await;
            let current_record_count = record_count_clone.load(Ordering::Relaxed);
            let current_data_size = data_size_clone.load(Ordering::Relaxed);

            let records_per_second = (current_record_count - last_record_count) as f64 / 10.0;
            let data_size_per_second = (current_data_size - last_data_size) as f64 / 10.0 / 1024.0;

            println!("Records inserted per second: {:.2}", records_per_second);
            println!("Data size inserted per second: {:.2} KB", data_size_per_second);

            last_record_count = current_record_count;
            last_data_size = current_data_size;
        }
    });

    if path.is_dir() {
        process_directory(&client, path, record_count, data_size).await?;
    } else if path.is_file() {
        process_file(&client, path, record_count, data_size).await?;
    } else {
        eprintln!("Error: The specified path is neither a file nor a directory.");
        std::process::exit(1);
    }

    Ok(())
}

async fn process_file(client: &Client, ulog_path: &Path, record_count: Arc<AtomicUsize>, data_size: Arc<AtomicUsize>) -> Result<(), Box<dyn Error>> {
    let ulog_file = File::open(ulog_path)?;
    let reader = BufReader::new(ulog_file);
    let mut parser = ULogParser::new(reader)?;

    parser.set_subscription_allow_list(ALLOWED_SUBSCRIPTION_NAMES.clone());

    let file_timestamp = parse_timestamp_from_fielname(ulog_path.to_str().expect("path name was not valid utf8"))?;

    let records: Vec<Record> = Vec::new();
    let records_arc_mutex = Arc::new(Mutex::new(records));
    let records_arc_mutex_clone = Arc::clone(&records_arc_mutex);

    for result in parser {
        let ulog_message = result?;
        log::debug!("callback: {:?}", ulog_message);

        match ulog_message {
            UlogMessage::LoggedData(logged_data) => {
                let field_list: Vec<(String, inst::BaseType)> = logged_data.data.flatten();
                let timestamp = (logged_data.timestamp as i64 + file_timestamp).to_string();

                let measure_info = create_measure_values(field_list).expect("Error creating measure values");

                //FIXME: Replace with the timestamp from the ulog data record. Hardcoded for now.
                //let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("unable to get unix time").as_micros().to_string();

                let dimension = Dimension::builder()
                    .name("example_dimension")
                    .value("example_value")
                    .build()?;

                let record = Record::builder()
                    .time(timestamp)
                    .time_unit(aws_sdk_timestreamwrite::types::TimeUnit::Microseconds)
                    .measure_name(measure_info.name)
                    .measure_value_type(MeasureValueType::Multi)
                    .set_measure_values(Some(measure_info.values.clone()))
                    .dimensions(dimension)
                    .build();

                let record_size = std::mem::size_of_val(&record) + measure_info.values.iter().map(|mv| std::mem::size_of_val(mv)).sum::<usize>();

                records_arc_mutex_clone.lock().unwrap().push(record);
                record_count.fetch_add(1, Ordering::Relaxed);
                data_size.fetch_add(record_size, Ordering::Relaxed);
            }
            _ => ()
        }
    }

    let records = records_arc_mutex.lock().unwrap().clone();
    let batch_size = 100;
    let mut tasks = vec![];

    for chunk in records.chunks(batch_size) {
        let client_clone = client.clone();
        let chunk = chunk.to_vec();
        let task = task::spawn(async move {
            let write_request = client_clone.write_records()
                .database_name(DATABASE_NAME)
                .table_name(TABLE_NAME)
                .set_records(Some(chunk.clone()))
                .send()
                .await;

            match write_request {
                Ok(_) => println!("Batch written successfully"),
                Err(err) =>  {

                    if let Some(service_error) = err.as_service_error() {
                        match service_error {
                            WriteRecordsError::AccessDeniedException(_) => {}
                            WriteRecordsError::InternalServerException(_) => {}
                            WriteRecordsError::InvalidEndpointException(_) => {}
                            WriteRecordsError::RejectedRecordsException(rejected) => {

                                let rejected_records  = rejected.rejected_records.clone().unwrap();

                                for rejected_record in rejected_records.iter() {

                                    let record_index = rejected_record.record_index;

                                    log::error!("Rejected record: {:?}", chunk[record_index as usize]);
                                }
                            }
                            WriteRecordsError::ResourceNotFoundException(_) => {}
                            WriteRecordsError::ThrottlingException(_) => {}
                            WriteRecordsError::ValidationException(_) => {}
                            WriteRecordsError::Unhandled(_) => {}
                            _ => {}
                        }
                    }
                    panic!("Error writing batch: {:?}", err)
                },
            };
        });
        tasks.push(task);
    }

    // Await completion of all tasks
    for task in tasks {
        task.await.unwrap();
    }

    log::info!("Done parsing and processing data for {:?}", ulog_path);

    Ok(())
}

struct MeasureInfo {
    name: String,
    values: Vec<MeasureValue>,
}

fn create_measure_values(field_list: Vec<(String, inst::BaseType)>) -> Result<MeasureInfo, Box<dyn Error>> {

    let dimension = Dimension::builder()
        .name("example_dimension")
        .value("example_value")
        .build()?;

    let mut measure_values: Vec<MeasureValue> = Vec::with_capacity(field_list.len());

    let mut measure_name:Option<String> = None;

    for (field, datatype) in &field_list {
        
        //split the field name at the first '/' into measure name and field_name
        let split: Vec<&str> = field.splitn(2, '/').collect();
        measure_name = Some(split[0].to_string());

        if measure_name.is_none() {
            measure_name = Some(split[0].to_string());
        } else {
            if measure_name != Some(split[0].to_string()) {
                panic!("All fields in a LOGGED_DATA message should have the same measure name");
            }
        }

        //let field_name = split[1].to_string();
        let field_name = field.replace("/", "_");

        let (measure_value_type, value) = map_datatype_to_timestream(&field_name, datatype.clone());

        if !value.is_none() {
            let measure_value = MeasureValue::builder()
                .name(field_name)
                .set_type(Some(measure_value_type))
                .set_value(value)
                .build()?;

            measure_values.push( measure_value);
        }
    }

    Ok( MeasureInfo { name: measure_name.unwrap(), values: measure_values} )
}

lazy_static!{
    static ref MEASURE_TYPES: Arc<Mutex<HashMap<String, MeasureValueType>>> = Arc::new(Mutex::new(HashMap::new()));
}

fn map_datatype_to_timestream(field_name: &str, value: inst::BaseType) -> (MeasureValueType, Option<String>) {
    let result = match value {
        inst::BaseType::UINT8(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::UINT16(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::UINT32(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::UINT64(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::INT8(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::INT16(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::INT32(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::INT64(v) => (MeasureValueType::Bigint, Some(v.to_string())),
        inst::BaseType::FLOAT(v) => {
            if v == f32::INFINITY || v == f32::NEG_INFINITY || v.is_nan() {
                (MeasureValueType::Double, None)
            } else {
                (MeasureValueType::Double, Some(v.to_string()))
            }
        },
        inst::BaseType::DOUBLE(v) => {
            if v == f64::INFINITY || v == f64::NEG_INFINITY || v.is_nan() {
                (MeasureValueType::Double, None)
            } else {
                (MeasureValueType::Double, Some(v.to_string()))
            }
        },
        // ⚠️ inst::BaseType::BOOL should really be treated as a MeasureValueType::Boolean, but Timestream
        //     seems to not be able to handle NULL values for boolean fields?
        //     Whatever the cause we are getting errors from Timestream:  "Each multi measure name can have only one measure value type and cannot be changed."
        //     We are noticing that the type is sometimes interpreted as a Boolean and sometimes as a bigint.
        //     ( Field usb_connected has inconsistent data types (BOOLEAN vs BIGINT)
        //inst::BaseType::BOOL(v) => (MeasureValueType::Boolean, if v { Some("True".to_string()) } else { Some("False".to_string()) }),
        inst::BaseType::BOOL(v) => (MeasureValueType::Bigint, if v { Some("1".to_string()) } else { Some("0".to_string()) }),
        inst::BaseType::CHAR(v) => (MeasureValueType::Varchar, Some(v.to_string())),
        _ => panic!("Unsupported data type"),
    };

    let mut measure_types = MEASURE_TYPES.lock().unwrap();

    if let Some(measure_type) = measure_types.get(field_name) {
        if result.0 != *measure_type {
            log::warn!("Field {} has inconsistent data types ({} vs {})", field_name, result.0, measure_type);
        }
    } else {
        measure_types.insert(field_name.to_string(), result.0.clone());
    }

    result
}

async fn process_directory(client: &Client, dir: &Path, record_count: Arc<AtomicUsize>, data_size: Arc<AtomicUsize>) -> Result<(), Box<dyn Error>> {
    // Find all ULOG files in the specified directory
    let ulg_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok) // Handle possible errors
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        process_file(client, &path, Arc::clone(&record_count), Arc::clone(&data_size)).await?;
    }
    Ok(())
}

lazy_static! {
    static ref ULG_FILENAME_REGEX: Regex = Regex::new(r"log_\d+_(\d{4})-(\d{1,2})-(\d{1,2})-(\d{1,2})-(\d{2})-(\d{2})\.ulg").unwrap();
}

fn parse_timestamp_from_fielname(ulg_path: &str) -> Result<i64, Box<dyn Error>> {
    // Get the basename of the path
    let ulg_basename = Path::new(ulg_path)
        .file_name()
        .ok_or("Invalid path")?
        .to_str()
        .ok_or("Invalid UTF-8 sequence")?;

    // Check if it starts with "log_"
    if !ulg_basename.starts_with("log_") {
        return Err(format!("\"{}\" does not start with log_", ulg_path).into());
    }

    // Check if it ends with ".ulg"
    if !ulg_basename.ends_with(".ulg") {
        return Err(format!("\"{}\" is not a valid *.ulg file", ulg_path).into());
    }

    // Use lazy_static regex pattern to capture date and time at which UAV was flown
    let caps = ULG_FILENAME_REGEX.captures(ulg_basename).ok_or(format!("Unable to extract datetime from ULF filename {ulg_basename}"))?;

    // Parse the captured groups
    let year: i32 = caps.get(1).ok_or("Failed to capture year")?.as_str().parse()?;
    let month: u32 = caps.get(2).ok_or("Failed to capture month")?.as_str().parse()?;
    let day: u32 = caps.get(3).ok_or("Failed to capture day")?.as_str().parse()?;
    let hour: u32 = caps.get(4).ok_or("Failed to capture hour")?.as_str().parse()?;
    let minute: u32 = caps.get(5).ok_or("Failed to capture minute")?.as_str().parse()?;
    let second: u32 = caps.get(6).ok_or("Failed to capture second")?.as_str().parse()?;

    // Create a NaiveDateTime from the captured values
    let dt = NaiveDateTime::parse_from_str(
        &format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second),
        "%Y-%m-%d %H:%M:%S",
    )?;

    // Convert NaiveDateTime to a Unix timestamp in microseconds
    let timestamp = Utc.from_utc_datetime(&dt).timestamp_micros();

    Ok(timestamp)
}

