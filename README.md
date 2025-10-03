# yule_log

A streaming [ULOG](https://docs.px4.io/main/en/dev_log/ulog_file_format.html) parser written in Rust.

## Features

- **Memory Efficient:** A stream-oriented design allows `yule_log` handle arbitrarily large files in real time without fully loading them into memory.
- **Derive API:** Map LoggedData messages directly into your structs using the optional `macros` feature.
- **Complete coverage:** Supports all ULog message types.
- **Binary fidelity:** Can parse and re-emit a ULog file byte-for-byte identical to the original.
- **Safe and robust:** Full Rust type safety with comprehensive error handling.

## 🌟Derive API

The `macros` feature provides a serde-like experience, allowing ULOG data to be mapped directly into your own structs.

With this feature, there is no need to manually track subscription names, msg_ids, or field indices, the macros handle all 
of that for you automatically.  The stream-oriented nature of the underlying parser is fully preserved.

### User guide

#### 1. Enable the feature in Cargo.toml

```toml
yule_log = { version="0.3", features = ["macros"] }
```

#### 2. Map subscriptions to structs

Define one struct for each ULOG subscription you want to map, 
and annotate it with: `#[derive(ULogData)]`.

Example:

```rust
#[derive(ULogData)]
 pub struct VehicleLocalPosition {
  pub timestamp: u64,
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub extra_field: Option<f32>,
 }
```

In most cases, no extra config is needed—just name your struct and fields to match the names used in the
ULOG. Only include the fields you need. 

The macros will infer the ULOG names by converting your struct and field names to snake case. 

By default, struct fields will be validated
against the ULOG file, and any missing fields will cause an error.  This can be overriden by making a field an `Option<T>`, as shown above for `extra_field`.

💡Subscription and field names can also be specified using the `#[yule_log]` attribute.  For
more information refer to the [ULogData API docs](https://docs.rs/yule_log/0.3/yule_log/derive.ULogData.html).

#### 3. List all subscriptions in an enum

Declare an enum where each variant wraps one of your ULogData structs, and annotate it with:
`#[derive(ULogMessages)]`.

```rust
#[derive(ULogMessages)]
 pub enum LoggedMessages {
  VehicleLocalPosition(VehicleLocalPosition),
  ActuatorOutputs(ActuatorOutputs),
 }
```

This enum will then become your interface to the data.

💡 Raw ULOG messages can also be captured by annotating an enum variant as follows:

```rust
  #[yule_log(forward_other)]
  Other(yule_log::model::msg::UlogMessage),
```

#### 4. Iterate through mapped messages

The derive macro will generate a `::stream()` method on your enum, allowing the 
messages to be easily retrieved.

Full example:

```rust
use std::fs::File;
use std::io::BufReader;
use yule_log::model::msg::UlogMessage;

use yule_log::{ULogData, ULogMessages};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open("sample.ulg")?);

    let stream = LoggedMessages::stream(reader)?;

    for msg_res in stream {
        let msg = msg_res?;

        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                println!("VehicleLocalPosition: {}: x={} y={} z={}",
                         v.timestamp, v.x, v.y, v.z);
            }
            LoggedMessages::ActuatorOutputs(a) => {
                println!("ActuatorOutputs: {}: {:?}", 
                         a.timestamp, a.output);
            }
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

## 🔧 Builder Interface for Advanced Configuration

By default, the `::stream()` method configures the parser to only yield messages that are mapped to enum variants; 
`LoggedData` and `AddSubscription` messages are handled internally and not visible
to the caller.

To override this default behaviour, the derive macro also generates a `::builder()` API for
your `LoggedMessages` enum, allowing you to:

- Forward `LoggedData` messages not mapped to an enum variant.
- Forward `AddSubscription` messages.

💡It is recommended to only map the `LoggedData` messages you need, as this avoids 
parsing of unmapped messages which improves performance. The `add_subscription()` builder 
feature below provides this functionality and is analogous to the
`ULogParserBuilder::set_subscription_allow_list()`.

### Example Usage

```rust
let stream = LoggedMessages::builder(reader)
    .add_subscription("vehicle_gps_position")? // Add extra subscription
    .forward_subscriptions(true)?              // Forward AddSubscription messages
    .stream()?;                                // Create the iterator

for msg_res in stream {
    let msg = msg_res?;
    match msg {
        LoggedMessages::VehicleLocalPosition(v) => {
            println!("VehicleLocalPosition: {}: x={} y={} z={}", v.timestamp, v.x, v.y, v.z);
        }
        LoggedMessages::Other(UlogMessage::AddSubscription(sub)) => {
            println!("AddSubscription message: {:?}", sub);
        }
        LoggedMessages::Other(UlogMessage::LoggedData(data)) => {
            println!("Extra LoggedData message: {:?}", data);
        }
        _ => {}
    }
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
