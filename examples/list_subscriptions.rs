use std::{env, fmt, io};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use env_logger::Builder;
use log::LevelFilter;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeStruct};
use serde_yaml;

use yule_log::model::{def, inst, msg};
use yule_log::parser::ULogParser;

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

    let mut subscriptions: BTreeMap<String, MyFormat> = BTreeMap::new();

    let path = Path::new(&args[1]);

    if path.is_dir() {
        process_directory(path, &mut subscriptions)?;
    } else  {
        process_file(path.into(), &mut subscriptions)?;
    }

    /*
    let yaml = serde_yaml::to_string(&subscriptions)?; 

    let mut stdout = io::stdout();
    write!(stdout, "{}", yaml)?;
    */

    let json = serde_json::to_string_pretty(&subscriptions)?;

    let mut stdout = io::stdout();
    write!(stdout, "{}", json)?;

    Ok(())
}


fn process_file(ulog_path: Box<Path>, subscriptions: &mut BTreeMap<String, MyFormat> ) -> Result<(), Box<dyn Error>> {
    let mut parser = ULogParser::new(BufReader::new(File::open(ulog_path.clone())?))?;

    while let Some(result) = parser.next() {
        let ulog_message = result?;

        match ulog_message {
            msg::UlogMessage::LoggedData(logged_data) => {
                let subscription = &parser.get_subscription(logged_data.msg_id)?;

                subscriptions.insert(subscription.message_name.clone(), MyFormat(logged_data.data));
            }

            _ => ()
        }
    }

    Ok(())
}

fn process_directory(dir: &Path, subscriptions: &mut BTreeMap<String, MyFormat>) -> Result<(), Box<dyn Error>> {
    let ulg_files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "ulg"))
        .map(|entry| entry.path())
        .collect();

    for path in ulg_files {
        process_file(Box::from(path), subscriptions)?;
    }

    Ok(())
}

struct MyFormat(inst::Format);

impl Serialize for MyFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let format = &self.0;

        // Start serializing the struct "Format"
        let mut state = serializer.serialize_struct("Format", 3)?;
        state.serialize_field("name", &format.name)?;

        // Manually serialize the `fields` as a BTreeMap of field names to type names
        let mut fields_map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        for field in &format.fields {
            fields_map.insert(field.name.clone(), FieldValueSerializer(&field.value).to_string());
        }
        state.serialize_field("fields", &fields_map)?;


        // Serialize the optional multi_id_index
        if let Some(multi_id_index) = &format.multi_id_index {
            state.serialize_field("multi_id_index", multi_id_index)?;
        } else {
            state.skip_field("multi_id_index")?;
        }

        state.end()
    }
}


// Custom serializer for Vec<Field>
struct FieldsSerializer<'a>(&'a Vec<inst::Field>);

impl<'a> Serialize for FieldsSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        // Start serializing the sequence (i.e., the vector of fields)
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for field in self.0 {
            seq.serialize_element(&FieldSerializer(field))?;
        }
        seq.end()
    }
}

// Custom serializer for a single Field
struct FieldSerializer<'a>(&'a inst::Field);

impl<'a> Serialize for FieldSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        // Serialize the struct "Field"
        let mut state = serializer.serialize_struct("Field", 2)?;
        state.serialize_field("name", &self.0.name)?;
        state.serialize_field("value", &FieldValueSerializer(&self.0.value))?;
        state.end()
    }
}

// Custom serializer for FieldValue
struct FieldValueSerializer<'a>(&'a inst::FieldValue);

impl<'a> Serialize for FieldValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match self.0 {
            inst::FieldValue::SCALAR(base_type) => {
                // Serialize scalar type (e.g., "uint8")
                BaseTypeSerializer(base_type).serialize(serializer)
            }
            inst::FieldValue::ARRAY(array) => {
                // Serialize array type with size (e.g., "uint8[4]")
                let array_size = array.len();
                let mut serialized_str = BaseTypeSerializer(&array[0]).to_string();
                serialized_str.push_str(&format!("[{}]", array_size));
                serializer.serialize_str(&serialized_str)
            }
        }
    }
}

impl<'a> fmt::Display for FieldValueSerializer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            inst::FieldValue::SCALAR(base_type) => {
                write!(f, "{}", BaseTypeSerializer(base_type).to_string())
            }
            inst::FieldValue::ARRAY(array) => {
                let array_size = array.len();
                let mut serialized_str = BaseTypeSerializer(&array[0]).to_string();
                write!(f, "{}[{}]", serialized_str, array_size)
            }
        }
    }
}

// Custom serializer for BaseType
struct BaseTypeSerializer<'a>(&'a inst::BaseType);

impl<'a> BaseTypeSerializer<'a> {
    fn to_string(&self) -> String {
        match self.0 {
            inst::BaseType::UINT8(_) => "uint8".to_string(),
            inst::BaseType::UINT16(_) => "uint16".to_string(),
            inst::BaseType::UINT32(_) => "uint32".to_string(),
            inst::BaseType::UINT64(_) => "uint64".to_string(),
            inst::BaseType::INT8(_) => "int8".to_string(),
            inst::BaseType::INT16(_) => "int16".to_string(),
            inst::BaseType::INT32(_) => "int32".to_string(),
            inst::BaseType::INT64(_) => "int64".to_string(),
            inst::BaseType::FLOAT(_) => "float".to_string(),
            inst::BaseType::DOUBLE(_) => "double".to_string(),
            inst::BaseType::BOOL(_) => "bool".to_string(),
            inst::BaseType::CHAR(_) => "char".to_string(),
            inst::BaseType::OTHER(_) => "other".to_string(), // or customize as needed
        }
    }
}

impl<'a> Serialize for BaseTypeSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}