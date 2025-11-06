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
    let reader = BufReader::new(File::open("core/test_data/input/sample_log_small.ulg")?);

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
