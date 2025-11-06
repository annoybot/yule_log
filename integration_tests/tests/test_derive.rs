use std::fs::File;
use std::io::BufReader;
use yule_log::model::msg::UlogMessage;
use yule_log::{ULogData, ULogMessages};

#[derive(ULogMessages)]
pub enum LoggedMessages {
    VehicleLocalPosition(VehicleLocalPosition),
    ActuatorOutputs(ActuatorOutputs),
    #[yule_log(forward_other)]
    Other(UlogMessage),
}

#[derive(ULogData, Debug, PartialEq, Clone)]
pub struct VehicleLocalPosition {
    timestamp: u64,
    x: f32,
    y: f32,

    // Test handling of an Optional field which is present in the ULOG file.
    z: Option<f32>,

    // Test handling of an optional field which is not present in the ULOG file.
    not_there: Option<u64>,
}

#[derive(ULogData, Debug, PartialEq, Clone)]
#[yule_log(multi_id = 0)]
pub struct ActuatorOutputs {
    timestamp: u64,
    output: Vec<f32>,
}

#[test]
#[allow(clippy::float_cmp)]
fn test_derive() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);
    let stream = LoggedMessages::stream(reader)?;

    let mut vehicle_positions = Vec::<VehicleLocalPosition>::new();
    let mut actuator_outputs = Vec::<ActuatorOutputs>::new();

    const MAX_VEHICLE_POS: usize = 3;
    const MAX_ACTUATOR: usize = 2;

    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::VehicleLocalPosition(v)
                if vehicle_positions.len() < MAX_VEHICLE_POS =>
            {
                println!("Vehicle Local Position: {v:?}");
                vehicle_positions.push(v);
            }
            LoggedMessages::ActuatorOutputs(a) if actuator_outputs.len() < MAX_ACTUATOR => {
                println!("Actuator Output s: {a:?}");
                actuator_outputs.push(a);
            }
            LoggedMessages::Other(_) => {}
            _ => {}
        }

        if vehicle_positions.len() >= MAX_VEHICLE_POS && actuator_outputs.len() >= MAX_ACTUATOR {
            break;
        }
    }

    // Expected values
    let expected_positions = vec![
        VehicleLocalPosition {
            timestamp: 20321827,
            x: 0.0,
            y: 0.0,
            z: Some(-1.167656),
            not_there: None,
        },
        VehicleLocalPosition {
            timestamp: 20483131,
            x: 0.0,
            y: 0.0,
            z: Some(-1.1788101),
            not_there: None,
        },
        VehicleLocalPosition {
            timestamp: 20492907,
            x: 0.0,
            y: 0.0,
            z: Some(-1.1792563),
            not_there: None,
        },
    ];

    let expected_actuator_outputs = vec![
        ActuatorOutputs {
            timestamp: 20305329,
            output: vec![
                1596.0, 1491.0, 950.0, 1508.0, 1031.0, 1002.0, 1000.0, 1023.0,
            ],
        },
        ActuatorOutputs {
            timestamp: 20483329,
            output: vec![
                1597.0, 1491.0, 950.0, 1508.0, 1031.0, 1002.0, 1000.0, 1025.0,
            ],
        },
    ];

    let actual_positions: Vec<VehicleLocalPosition> = vehicle_positions.clone();
    assert_eq!(actual_positions, expected_positions);

    let actual_actuator_outputs: Vec<ActuatorOutputs> = actuator_outputs
        .iter()
        .map(|a| ActuatorOutputs {
            timestamp: a.timestamp,
            output: a.output[..8].to_vec(),
        })
        .collect();

    assert_eq!(actual_actuator_outputs, expected_actuator_outputs);

    Ok(())
}

#[test]
fn test_add_subscription() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(ULogMessages)]
    #[allow(dead_code)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData, Debug, PartialEq, Clone)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    let reader =
        BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg").unwrap());

    const EXTRA_SUBSCR_NAME: &'static str = "vehicle_gps_position";

    let stream = LoggedMessages::builder(reader)
        .add_subscription(EXTRA_SUBSCR_NAME)?
        .stream()
        .unwrap();

    let mut gps_message_present: bool = false;
    let mut vehicle_position_present: bool = false;

    // Confirm that messages like "vehicle_gps_position" never appear
    for msg_res in stream {
        match msg_res {
            Ok(LoggedMessages::Other(UlogMessage::LoggedData(data)))
                if data.data.name == EXTRA_SUBSCR_NAME =>
            {
                gps_message_present = true;
                println!("{data:?}");

                if vehicle_position_present {
                    break;
                }
            }
            Ok(LoggedMessages::VehicleLocalPosition(_v)) => {
                vehicle_position_present = true;
                if gps_message_present {
                    break;
                }
            }

            _ => {}
        }
    }

    assert!(
        gps_message_present,
        "Expected at least one `vehicle_gps_position` logged message."
    );
    assert!(
        vehicle_position_present,
        "Expected to receive at least one`vehicle_position` message."
    );
    Ok(())
}

#[test]
fn test_fwd_subscriptions() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(ULogMessages)]
    #[allow(dead_code)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData, Debug, PartialEq, Clone)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    let reader =
        BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg").unwrap());

    let stream = LoggedMessages::builder(reader)
        .forward_subscriptions(true)?
        .stream()
        .unwrap();

    let mut add_subscription_message_present: bool = false;
    let mut vehicle_position_present: bool = false;

    // Confirm that we receive at least one AddSubscription messages.
    for msg_res in stream {
        match msg_res {
            Ok(LoggedMessages::Other(UlogMessage::AddSubscription(sub))) => {
                add_subscription_message_present = true;
                println!("{sub:?}");

                if vehicle_position_present {
                    break;
                }
            }
            Ok(LoggedMessages::VehicleLocalPosition(_v)) => {
                vehicle_position_present = true;

                if add_subscription_message_present {
                    break;
                }
            }
            _ => {}
        }
    }

    assert!(
        add_subscription_message_present,
        "Expected at least one AddSubscription message."
    );
    assert!(
        vehicle_position_present,
        "Expected to receive at least one`vehicle_position` message."
    );

    Ok(())
}

#[test]
fn test_extend_subscriptions() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(ULogMessages)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),

        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData)]
    pub struct VehicleLocalPosition {
        pub timestamp: u64,
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::builder(reader)
        .extend_subscriptions(["vehicle_gps_position", "vehicle_attitude"])?
        .stream()?;

    #[derive(Default)]
    struct Flags {
        gps_seen: bool,
        att_seen: bool,
        pos_seen: bool,
    }

    let mut flags = Flags::default();

    impl Flags {
        pub fn all_seen(&self) -> bool {
            self.gps_seen && self.att_seen && self.pos_seen
        }
    }

    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                flags.pos_seen = true;
                println!(
                    "VehicleLocalPosition: {}: x={} y={} z={}",
                    v.timestamp, v.x, v.y, v.z
                );
            }
            LoggedMessages::Other(UlogMessage::LoggedData(data)) => match data.data.name.as_str() {
                "vehicle_gps_position" => {
                    flags.gps_seen = true;
                    println!("Extra GPS LoggedData: {:?}", data);
                }
                "vehicle_attitude" => {
                    flags.att_seen = true;
                    println!("Extra Attitude LoggedData: {:?}", data);
                }
                _ => {}
            },
            _ => {}
        }

        if flags.all_seen() {
            break;
        }
    }

    assert!(
        flags.gps_seen,
        "Expected at least one 'vehicle_gps_position' message."
    );
    assert!(
        flags.att_seen,
        "Expected at least one 'vehicle_attitude' message."
    );
    assert!(
        flags.pos_seen,
        "Expected at least one VehicleLocalPosition message."
    );

    Ok(())
}

#[test]
/// Test the code which we include in the README to ensure it compiles.
fn readme_example() -> Result<(), Box<dyn std::error::Error>> {
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

    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::stream(reader)?;

    for msg_res in stream {
        let msg = msg_res?;

        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                println!(
                    "VehicleLocalPosition: {}: x={} y={} z={}",
                    v.timestamp, v.x, v.y, v.z
                );
            }
            LoggedMessages::ActuatorOutputs(a) => {
                println!("ActuatorOutputs: {}: {:?}", a.timestamp, a.output);
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

#[test]
/// Test the builder code which we include in the README to ensure it compiles.
fn readme_builder_example() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(ULogMessages)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),

        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData)]
    pub struct VehicleLocalPosition {
        pub timestamp: u64,
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::builder(reader)
        .add_subscription("vehicle_gps_position")? // Add extra subscription
        .forward_subscriptions(true)? // Forward AddSubscription messages
        .stream()?; // Create the iterator

    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                println!(
                    "VehicleLocalPosition: {}: x={} y={} z={}",
                    v.timestamp, v.x, v.y, v.z
                );
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

    Ok(())
}
