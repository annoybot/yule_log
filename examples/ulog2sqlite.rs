use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use env_logger::Builder;
use lazy_static::lazy_static;
use log::LevelFilter;
use regex::Regex;
use rusqlite::Connection;

use yule_log::errors::ULogError;
use yule_log::model::{inst, msg};
use yule_log::parser::ULogParser;

// For using write! macro on String.

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

fn process_file(ulog_path: &Path) -> Result<(), Box<dyn Error>> {
    let db_path = ulog_path.with_file_name(format!("{}_export", ulog_path.file_stem().unwrap().to_str().unwrap())).with_extension("db");
    
    let create_statement = create_schema(ulog_path)?;

    let conn = Connection::open(db_path)?;

    // Execute the CREATE TABLE statement.
    conn.execute(&create_statement, [])?;

    println!("Table created successfully!");

    let ulog_file = File::open(ulog_path)?;
    let reader = BufReader::new(ulog_file);
    let mut parser = ULogParser::new(reader)?;
    
    parser.set_subscription_allow_list(ALLOWED_SUBSCRIPTION_NAMES.clone());
    
    for result in parser {
        let ulog_message = result?;

        log::debug!("callback: {:?}", ulog_message);

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                let field_list: Vec<(String, inst::BaseType)> = logged_data.data.flatten();
                let timestamp = logged_data.timestamp as i64;

                insert_or_update_data(&conn, "my_table", timestamp, field_list)?;
            }
            _ => { },
        }
    }

    log::info!("Done processing data for {:?}", ulog_path);

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

fn create_schema(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut column_defs = BTreeSet::<ColumnDef>::new();

    let file = File::open(path).map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;

    let reader = BufReader::new(file);

    let mut parser = ULogParser::new(reader)?;

    parser.set_subscription_allow_list(ALLOWED_SUBSCRIPTION_NAMES.clone());


    for result in parser {
        let ulog_message = result?;

        log::debug!("callback: {:?}", ulog_message);

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                // Flatten the data and extract the columns
                let field_list: Vec<(String, inst::BaseType)> = logged_data.data.flatten();

                for (path, data_type) in field_list {
                    let column_type = map_datatype_to_sql(&data_type);

                    let column_def = ColumnDef {
                        name: sanitize_column_name(&path),
                        column_type,
                    };

                    // Insert into the BTreeSet, will ignore duplicates by `name`
                    column_defs.insert(column_def);
                }
            }
            _ => {
            }
        }
    }

    log::info!("Done parsing {:?}", path);

    let create_statement = generate_create_table_sql(&column_defs);

    log::debug!("Create statement {:?}", create_statement);

    Ok(create_statement)
}



fn generate_create_table_sql(column_defs: &BTreeSet<ColumnDef>) -> String {
    let mut create_stmt = String::from("CREATE TABLE IF NOT EXISTS my_table (\n    __time INTEGER PRIMARY KEY");

    for column in column_defs {
        create_stmt.push_str(&format!(",\n    `{}` {}", column.name, column.column_type));
    }

    create_stmt.push_str("\n);");
    create_stmt
}

fn insert_or_update_data(conn: &Connection, table_name: &str, timestamp: i64, field_list: Vec<(String, inst::BaseType)>) -> Result<(), ULogError> {
    let mut stmt = String::new();

    // Start the SQL statement with the basic insert structure.
    stmt.push_str(&format!("INSERT OR REPLACE INTO {} (__time", table_name));

    // Append column names (fields)
    let columns: Vec<String> = field_list.iter().map(|(name, _)| sanitize_column_name(name)).collect();
    stmt.push_str(&columns.iter().map(|col| format!(", `{}`", col)).collect::<String>());

    stmt.push_str(") VALUES (?, "); // This placeholder is for the tiemstamp.

    // Append placeholders for the values
    let values = vec!["?"; columns.len()];
    stmt.push_str(&values.join(", "));
    stmt.push(')');

    // Add an ON CONFLICT clause to use COALESCE for each column.
    stmt.push_str(" ON CONFLICT(__time) DO UPDATE SET ");

    // Update each column using COALESCE to avoid overwriting existing non-NULL values with a NULL.
    let updates: Vec<String> = columns.iter()
        .map(|col| format!("`{}` = COALESCE(excluded.`{}`, `{}`)", col, col, col))
        .collect();
    stmt.push_str(&updates.join(", "));

    log::debug!("QUERY {:?}", stmt);

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(timestamp)];  // The timestamp is always the first parameter

    // Map values to correct types and add to params
    for (_, value) in field_list {
        let param: Box<dyn rusqlite::ToSql> = match value {
            inst::BaseType::UINT8(v) => Box::new(v as i64),
            inst::BaseType::UINT16(v) => Box::new(v as i64),
            inst::BaseType::UINT32(v) => Box::new(v as i64),
            inst::BaseType::UINT64(v) => Box::new(v as i64),
            inst::BaseType::INT8(v) => Box::new(v as i64),
            inst::BaseType::INT16(v) => Box::new(v as i64),
            inst::BaseType::INT32(v) => Box::new(v as i64),
            inst::BaseType::INT64(v) => Box::new(v),
            inst::BaseType::FLOAT(v) => Box::new(v as f64),
            inst::BaseType::DOUBLE(v) => Box::new(v),
            inst::BaseType::BOOL(v) => Box::new(if v { 1 } else { 0 }),
            inst::BaseType::CHAR(v) => Box::new(v.to_string()),  // Store CHAR as TEXT (String)
            inst::BaseType::OTHER(_) => return Err(ULogError::DatabaseError(rusqlite::Error::InvalidQuery.to_string())), // We don't handle `OTHER` for now
        };
        params.push(param);
    }

    // Convert the params into a slice of `&dyn ToSql` for rusqlite's execute
    let param_slice: Vec<&dyn rusqlite::ToSql> = params.iter().map(|x| &**x).collect();
    conn.execute(&stmt, &*param_slice)?;

    Ok(())
}




fn map_datatype_to_sql(datatype: &inst::BaseType) -> String {
    match datatype {
        inst::BaseType::UINT8(_)
        | inst::BaseType::UINT16(_)
        | inst::BaseType::UINT32(_)
        | inst::BaseType::UINT64(_)
        | inst::BaseType::INT8(_)
        | inst::BaseType::INT16(_)
        | inst::BaseType::INT32(_)
        | inst::BaseType::INT64(_)
        | inst::BaseType::BOOL(_) => "INTEGER".to_owned(),
        inst::BaseType::FLOAT(_) | inst::BaseType::DOUBLE(_) => "REAL".to_owned(),
        inst::BaseType::CHAR(_) => "TEXT".to_owned(), // Could be INTEGER too, but TEXT for readability.
        inst::BaseType::OTHER(_) => unreachable!("Nested DataFormat should have been flattened."),
    }
}

lazy_static! {
    static ref INVALID_CHAR_REGEX: Regex = Regex::new(r"[^a-zA-Z0-9_]").unwrap();
}

fn sanitize_column_name(name: &str) -> String {
    // Replace invalid characters with underscores.
    INVALID_CHAR_REGEX.replace_all(name, "_").to_string()
}

// ColumnDef.  Used for creating a uniqe list of column sqlite column definitions.
#[derive(Debug, Clone)]
struct ColumnDef {
    name: String,
    column_type: String, // Store SQLite column type as a string
}

// Implement ordering and equality checks for the column by name only.
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

impl Eq for ColumnDef {}

// You can implement `Hash` if needed
impl std::hash::Hash for ColumnDef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
