
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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open("core/test_data/input/sample_log_small.ulg")?);

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
