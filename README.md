# yule_log
A streaming [ULOG](https://docs.px4.io/main/en/dev_log/ulog_file_format.html) parser written in rust.

This parser is designed to be fast and efficient, and to be able to handle large ULOG files without loading them into
memory all at once.

## 🌟Derive API

This feature provides a serde-like experience, allowing ULOG data to be mapped directly into 
your own structs.

💡 No need to manually track subscription names, msg_ids, or field indices, the macros handle all 
of that for you automatically.  The stream-oriented nature of the underlying parser is fully preserved.

The feature can be enabled in `Cargo.toml` as follows:

```toml
yule_log = { version=<VERSION>, features = ["macros"] }
```

### User guide

#### Provide a wrapper module

Define a wrapper module and annotate it with: `#[yule_log_prelude]`.

#### Map subscriptions to structs

Inside the module, define one struct for each ULOG subscription you want to map, 
and annotate it with `#[derive(ULogData)]`.


💡 In most cases, no extra config is needed—just name your struct and fields to match the names used in the
ULOG subscription.  

⚠️ The macros infer the ULOG names by converting your struct and
field name to lower camel case. You can override the default mapping 
if needed: `#[yule_log(subscription_name = "...", multi_id = N)]`. 

#### List all subscriptions in an enum

Declare an enum where each variant wraps one of your ULogData structs.

This enum then becomes your interface to the data.

⚠️ Raw ULOG messages can be still captured by annotating a variant as follows:

```rust
  #[yule_log(forward_other)]
  Other(yule_log::model::msg::UlogMessage),
```

#### Iterate through mapped messages

The derive macro will generate a `::stream()` method on your enum, allowing the 
messages to be easily retrieved.

Example:

```rust
use yule_log::model::msg::UlogMessage;
use std::fs::File;
use std::io::BufReader;
use yule_log_macros::yule_log_prelude;

#[yule_log_prelude]
pub mod ulog {
 use yule_log_macros::{ULogData, ULogMessages};
 use yule_log::model::msg::UlogMessage;

 #[derive(ULogMessages)]
 pub enum LoggedMessages {
  VehicleLocalPosition(VehicleLocalPosition),
  ActuatorOutputs(ActuatorOutputs),
     
  #[yule_log(forward_other)]
  Other(yule_log::model::msg::UlogMessage),
 }

 #[derive(ULogData)]
 pub struct VehicleLocalPosition {
  pub timestamp: u64,
  pub x: f32,
  pub y: f32,
  pub z: f32,
 }

 #[derive(ULogData)]
 #[yule_log(multi_id = 1)]
 pub struct ActuatorOutputs {
  pub timestamp: u64,
  pub output: Vec<f32>,
 }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
 use crate::ulog::LoggedMessages;
 let reader = BufReader::new(File::open("sample.ulg")?);

 let stream = LoggedMessages::stream(reader)?;

 for msg_res in stream {
  let msg = msg_res?;

  match msg {
   LoggedMessages::VehicleLocalPosition(v) => {
    println!("VehicleLocalPosition: {}: x={} y={} z={}", v.timestamp, v.x, v.y, v.z);
   }
   LoggedMessages::ActuatorOutputs(a) => {
    println!("ActuatorOutputs: {}: {:?}", a.timestamp, a.output);
   },
   LoggedMessages::Other(msg) => {
    if let UlogMessage::Info(info) = msg {
     println!("INFO: {info}");
    }
   }
  }
 }

 Ok(())
}
```

## Low Level API

For those requiring complete control over the parsing process, the original low level API is still available.

Example:

```rust
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
        }
    }
```

This example is also available in the `examples` directory as `simple.rs`.


## License

This project is licensed under the [MIT Licence](LICENCE).

