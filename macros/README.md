# yule_log_macros

Derive macros for the [yule_log](https://crates.io/crates/yule_log) ULOG parser.

## Features

- **#[derive(ULogData)]:** Maps ULOG subscriptions directly into your structs.

- **#[derive(ULogMessages)]:** Aggregates your structs into a single streaming interface.

- **Forward unknown messages:** Capture raw ULOG messages via #[yule_log(forward_other)].

- **Memory Efficient:** Preserves the streaming nature of the yule_log parser.

## Quickstart

For more information, see: [yule_log](https://crates.io/crates/yule_log).

### Cargo.toml

```toml
yule_log = { version = "0.4", features = ["macros"] }
```

### Define mappings

```rust
#[derive(ULogData)]
pub struct VehicleLocalPosition {
pub timestamp: u64,
pub x: f32,
pub y: f32,
pub z: f32,
}
```

### Create an enum

```rust
#[derive(ULogMessages)]
pub enum LoggedMessages {
    VehicleLocalPosition(VehicleLocalPosition),
    #[yule_log(forward_other)]
    Other(yule_log::model::msg::UlogMessage),
}
```

### Iterate over data

```rust
let stream = LoggedMessages::stream(reader)?;
for msg_res in stream {
    let msg = msg_res?;
    match msg {
        LoggedMessages::VehicleLocalPosition(v) => println!("x={} y={} z={}", v.x, v.y, v.z),
        LoggedMessages::Other(msg) => println!("Other message: {:?}", msg),
    }
}
```

## License

This project is licensed under the [MIT Licence](LICENCE).

