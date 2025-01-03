use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use env_logger::Builder;
use lazy_static::lazy_static;
use log::LevelFilter;
use r2d2::{Pool, PooledConnection};
use r2d2_mysql::{
    mysql::{Opts, OptsBuilder, prelude::*},
    MySqlConnectionManager, r2d2,
};
use regex::Regex;

use ulog_rs::errors::ULogError;
use ulog_rs::model::{inst, msg};
use ulog_rs::parser::ULogParser;

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

const DB_USER: &str = "ulog";
const DB_PASSWORD: &str = "password";
const DB_NAME: &str = "ulog";

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

    let pool = create_pool(&format!("mysql://{}:{}@localhost/{}", DB_USER, DB_PASSWORD, DB_NAME));

    if path.is_dir() {
        process_directory(path, &pool)?;
    } else if path.is_file() {
        process_file(path.into(), &pool)?;
    } else {
        eprintln!("Error: The specified path is neither a file nor a directory.");
        std::process::exit(1);
    }

    Ok(())
}

fn create_pool(database_url: &str) -> Pool<MySqlConnectionManager> {
    let opts = Opts::from_url(database_url).expect("Invalid database URL");
    let builder = OptsBuilder::from_opts(opts);
    let manager = MySqlConnectionManager::new(builder);
    Pool::new(manager).expect("Failed to create pool")
}

fn get_conn(pool: &Pool<MySqlConnectionManager>) -> PooledConnection<MySqlConnectionManager> {
    pool.get().expect("Failed to get a connection from the pool")
}

fn process_file(ulog_path: Box<Path>, pool: &Pool<MySqlConnectionManager>) -> Result<(), Box<dyn Error>> {
    let create_statement = create_schema(&*ulog_path)?;
    let mut conn = get_conn(pool);

    log::info!("Create statement: {}", create_statement);
    
    conn.query_drop(create_statement)?;
    
    println!("Table created successfully!");

    let ulog_file = File::open(ulog_path.clone())?;
    let reader = BufReader::new(ulog_file);
    let mut parser = ULogParser::new(reader)?;

    parser.set_subscription_allow_list(ALLOWED_SUBSCRIPTION_NAMES.clone());

    let ulog_path_clone = ulog_path.clone();
    
    for result in parser {
        let ulog_message = result?;
        log::debug!("callback: {:?}", ulog_message);

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                let field_list: Vec<(String, inst::BaseType)> = logged_data.data.flatten();
                let timestamp = logged_data.timestamp as i64;
                let file_name = ulog_path_clone.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();

                insert_or_update_data(&mut conn, "ulog_data", timestamp, file_name, field_list)?;
            }
            _ => (),
        }
    }
    
    log::info!("Done parsing and processing data for {:?}", ulog_path);

    Ok(())
}

fn process_directory(dir: &Path, pool: &Pool<MySqlConnectionManager>) -> Result<(), Box<dyn Error>> {
    let ulg_files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        process_file(Box::from(path), pool)?;
    }

    Ok(())
}

fn create_schema(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut column_defs = BTreeSet::<ColumnDef>::new();
    let file = File::open(path)?;

    let reader = BufReader::new(file);
    let mut parser = ULogParser::new(reader)?;

    parser.set_subscription_allow_list(ALLOWED_SUBSCRIPTION_NAMES.clone());

    for result in parser {
        let ulog_message = result?;
        log::debug!("callback: {:?}", ulog_message);

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                let field_list: Vec<(String, inst::BaseType)> = logged_data.data.flatten();

                for (path, data_type) in field_list {
                    let column_type = map_datatype_to_sql(&data_type);

                    let column_def = ColumnDef {
                        name: sanitize_column_name(&path),
                        column_type,
                    };

                    column_defs.insert(column_def);
                }
            }
            _ => ()
        }
    }

    // Manually add file_name column
    column_defs.insert(ColumnDef {
        name: "file_name".to_owned(),
        column_type: "VARCHAR(255)".to_owned(),  // Adjust type as needed
    });

    let create_statement = generate_create_table_sql(&column_defs);

    Ok(create_statement)
}

fn generate_create_table_sql(column_defs: &BTreeSet<ColumnDef>) -> String {
    let mut create_stmt = String::from("CREATE TABLE IF NOT EXISTS ulog_data (\n    __time BIGINT NOT NULL");

    for column in column_defs {
        create_stmt.push_str(&format!(",\n    `{}` {}", column.name, column.column_type));
    }

    create_stmt.push_str("\n, PRIMARY KEY(__time, file_name));");
    create_stmt
}

fn insert_or_update_data(conn: &mut PooledConnection<MySqlConnectionManager>, table_name: &str, timestamp: i64, file_name: String, field_list: Vec<(String, inst::BaseType)>) -> Result<(), ULogError> {
    let mut stmt = format!("INSERT INTO {} (__time, file_name", table_name);

    // Prepare column names and placeholders
    let columns: Vec<String> = field_list.iter().map(|(name, _)| sanitize_column_name(name)).collect();
    stmt.push_str(&columns.iter().map(|col| format!(", `{}`", col)).collect::<String>());

    // Start building the value placeholders for the timestamp and file name
    stmt.push_str(") VALUES (?, ?, ");

    // Placeholder for each field value
    let values = vec!["?"; columns.len()];
    stmt.push_str(&values.join(", "));
    stmt.push_str(") ON DUPLICATE KEY UPDATE ");

    // Prepare the update part of the query: make sure NULLs don't overwrite existing values
    let updates: Vec<String> = columns.iter()
        .map(|col| format!("`{}` = IFNULL(VALUES(`{}`), `{}`)", col, col, col))
        .collect();
    stmt.push_str(&updates.join(", "));

    log::debug!("QUERY: {:?}", stmt);

    // Prepare parameters for the query: the timestamp, file_name, and field values
    let mut params: Vec<Box<dyn mysql::prelude::ToValue>> = vec![Box::new(timestamp), Box::new(file_name)];

    // Convert each field's value to a Box<dyn ToValue>
    for (_, value) in field_list {
        let param: Box<dyn mysql::prelude::ToValue> = match value {
            inst::BaseType::UINT8(v) => Box::new(v as u8),
            inst::BaseType::UINT16(v) => Box::new(v as u16),
            inst::BaseType::UINT32(v) => Box::new(v as u32),
            inst::BaseType::UINT64(v) => Box::new(v as u64),
            inst::BaseType::INT8(v) => Box::new(v as i8),
            inst::BaseType::INT16(v) => Box::new(v as i16),
            inst::BaseType::INT32(v) => Box::new(v as i32),
            inst::BaseType::INT64(v) => Box::new(v),
            inst::BaseType::FLOAT(v) => {
                // Check for NaN, +inf, and -inf
                if v.is_nan() || v.is_infinite() {
                    Box::new(None::<f32>) // Insert NULL for NaN or infinity
                } else {
                    Box::new(v as f32)
                }
            },
            inst::BaseType::DOUBLE(v) => {
                // Check for NaN, +inf, and -inf
                if v.is_nan() || v.is_infinite() {
                    Box::new(None::<f64>) // Insert NULL for NaN or infinity
                } else {
                    Box::new(v)
                }
            },
            inst::BaseType::BOOL(v) => Box::new(if v { 1 } else { 0 }),
            inst::BaseType::CHAR(v) => Box::new(v.to_string()),
            inst::BaseType::OTHER(_) => return Err(ULogError::DatabaseError("Unsupported data type".into())),
        };
        params.push(param);
    }

    // Execute the query with the prepared parameters
    conn.exec_drop(
        &stmt,
        mysql::Params::from(
            params
                .iter()
                .map(|x| x.as_ref().to_value())
                .collect::<Vec<mysql::Value>>(),
        ),
    )?;


    Ok(())
}

fn map_datatype_to_sql(datatype: &inst::BaseType) -> String {
    match datatype {
        inst::BaseType::UINT8(_) => "TINYINT UNSIGNED".to_owned(),
        inst::BaseType::UINT16(_) => "SMALLINT UNSIGNED".to_owned(),
        inst::BaseType::UINT32(_) => "INT UNSIGNED".to_owned(),
        inst::BaseType::UINT64(_) => "BIGINT UNSIGNED".to_owned(),
        inst::BaseType::INT8(_) => "TINYINT".to_owned(),
        inst::BaseType::INT16(_) => "SMALLINT".to_owned(),
        inst::BaseType::INT32(_) => "INT".to_owned(),
        inst::BaseType::INT64(_) => "BIGINT".to_owned(),
        inst::BaseType::BOOL(_) => "TINYINT(1)".to_owned(), // MySQL stores booleans as TINYINT(1)
        inst::BaseType::FLOAT(_) => "FLOAT".to_owned(),
        inst::BaseType::DOUBLE(_) => "DOUBLE".to_owned(),
        inst::BaseType::CHAR(_) => "VARCHAR(255)".to_owned(), // VARCHAR for text, you can adjust length as needed
        inst::BaseType::OTHER(_) => unreachable!("Flattened data format shouldn't have other types."),
    }
}


lazy_static! {
    static ref INVALID_CHAR_REGEX: Regex = Regex::new(r"[^a-zA-Z0-9_]").unwrap();
}

fn sanitize_column_name(name: &str) -> String {
    let name = name.replace(".", "_");
    INVALID_CHAR_REGEX.replace_all(&name, "_").to_string()
}

#[derive(Debug, Eq)]
struct ColumnDef {
    name: String,
    column_type: String,
}

impl Ord for ColumnDef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for ColumnDef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ColumnDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
