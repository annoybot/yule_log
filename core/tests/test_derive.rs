#![cfg(feature = "macros")]

use std::fs::File;
use std::io::BufReader;
use yule_log_macros::ulog_preamble;

#[ulog_preamble]
pub mod ulog {
    use yule_log_macros::{ulog_preamble, ULogData, ULogMessages};
    use yule_log::model::msg::UlogMessage;

    #[derive(ULogMessages)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
        ActuatorOutputs(ActuatorOutputs),
        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData, Debug)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(ULogData, Debug)]
    #[yule_log(multi_id = 0)]
    pub struct ActuatorOutputs {
        timestamp: u64,
        output: Vec<f32>,
    }
}

#[test]
fn integration_macros_stream() -> Result<(), Box<dyn std::error::Error>> {
    use ulog::LoggedMessages;
    
    // Integration tests run from the workspace root, so prefix with `core/`
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::stream(reader)?;

    //FIXME: test_derive does not really test the output.
    
    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::VehicleLocalPosition(v) => {
                println!("Vehicle Local Position: {:?}", v);
            }
            LoggedMessages::ActuatorOutputs(a) => {
                println!("Actuator Outputs: {:?}", a);
            }
            LoggedMessages::Other(_) => {
                // Fine, ignore
            }
        }
    }

    Ok(())
}
