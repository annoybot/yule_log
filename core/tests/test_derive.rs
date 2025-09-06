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
    Other(yule_log::model::msg::UlogMessage),
}

#[derive(ULogData)]
pub struct VehicleLocalPosition {
    timestamp: u64,
    x: f32,
    y: f32,
    z: f32,
}

#[derive(ULogData)]
#[yule_log(multi_id=1)]
pub struct ActuatorOutputs {
    timestamp: u64,
    output: Vec<f32>,
}

#[test]
fn integration_macros_stream() -> Result<(), Box<dyn std::error::Error>> {
    // Integration tests run from the workspace root, so prefix with `core/`
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::stream(reader)?;

    for msg_res in stream.take(5) {
        let msg = msg_res?;
        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                assert!(v.timestamp > 0);
            }
            LoggedMessages::ActuatorOutputs(a) => {
                assert!(a.output.len() >= 0);
            }
            LoggedMessages::Other(_) => {
                // Fine, ignore
            }
        }
    }

    Ok(())
}
