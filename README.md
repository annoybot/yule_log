# yule_log
A streaming [ULOG](https://docs.px4.io/main/en/dev_log/ulog_file_format.html) parser written in rust.

This parser is designed to be fast and efficient, and to be able to handle large ULOG files without loading them into
memory all at once.

## Usage

Example usage:

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

## Installing Rust

To use this library, you will need to have Rust installed.  You can install Rust by following the instructions 
found here: https://www.rust-lang.org/learn/get-started.

Once Rust is installed you can run the eexample as follows:

```shell
cargo run --example simple data/short_list.ulg
```

## Example Programs

The `examples/` directory contains several example programs that demonstrate how to use the ULOG parser.  These include:

- **list_subscriptions.rs**: This program lists all subscriptions found in a ULOG file.
- **simple.rs**: A minimal example demonstrating basic ULOG parsing functionality. (Shown above)
- **ulog2csv.rs**: Converts ULOG data into CSV format.  Follows PlotJuggler conventions when naming columns.
- **ulog2mysql.rs**: Imports ULOG data into a MySQL database. It inferrs the schema from the ULOG data and automatically creates the database table.
- **ulog2sqlite.rs**: Similar to `ulog2mysql.rs`, but uses SQLite for storing ULOG data.
- **ulogcat.rs**: Shows how to emit the parsed data back into a ULOG file.  This example round trips the data, parsing
 it and re-saving it to a new file.  Demonstrates the fidelity of the parser because the emitted ULOG file is typically binary identical to the input ULOG file.

## License

This project is licensed under the [MIT Licence](LICENCE).

