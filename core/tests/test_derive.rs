#![cfg(feature = "macros")]

use std::fs::File;
use std::io::BufReader;
use yule_log_macros::{ULogData, ULogMessages};
use yule_log::model::msg::UlogMessage;

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
    z: f32,
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
            LoggedMessages::VehicleLocalPosition(v) if vehicle_positions.len() < MAX_VEHICLE_POS => {
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
            z: -1.167656,
        },
        VehicleLocalPosition {
            timestamp: 20483131,
            x: 0.0,
            y: 0.0,
            z: -1.1788101,
        },
        VehicleLocalPosition {
            timestamp: 20492907,
            x: 0.0,
            y: 0.0,
            z: -1.1792563,
        },
    ];

    let expected_actuator_outputs = vec![
        ActuatorOutputs {
            timestamp: 20305329,
            output: vec![1596.0, 1491.0, 950.0, 1508.0, 1031.0, 1002.0, 1000.0, 1023.0],
        },
        ActuatorOutputs {
            timestamp: 20483329,
            output: vec![1597.0, 1491.0, 950.0, 1508.0, 1031.0, 1002.0, 1000.0, 1025.0],
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
