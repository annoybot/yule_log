#![cfg(feature = "macros")]

use std::fs::File;
use std::io::BufReader;
use yule_log_macros::yule_log_prelude;

#[yule_log_prelude]
pub mod ulog {
    use yule_log::model::msg::UlogMessage;
    use yule_log_macros::{ULogData, ULogMessages};

    #[derive(ULogMessages)]
    pub enum LoggedMessages {
        PositionSetpointTriplet(PositionSetpointTriplet),
        TelemetryStatus(TelemetryStatus),
        #[yule_log(forward_other)]
        Other(UlogMessage),
    }

    #[derive(ULogData, Debug)]
    //https://docs.px4.io/main/en/msg_docs/PositionSetpointTriplet.html
    pub struct PositionSetpointTriplet {
        previous: PositionSetpoint,
        current: PositionSetpoint,
        next: PositionSetpoint,
    }

    //https://docs.px4.io/main/en/msg_docs/PositionSetpoint.html
    #[derive(ULogData, Debug)]
    pub struct PositionSetpoint {
        timestamp: u64,
        vx: f32,    // local velocity setpoint in m/s in NED
        vy: f32,    // local velocity setpoint in m/s in NED
        vz: f32,    // local velocity setpoint in m/s in NED

        lat: f64,   // latitude, in deg
        lon: f64,   // longitude, in deg
        alt: f32,   // altitude AMSL, in m
    }

    // Heartbeat uint64_t timestamp; uint8_t system_id; uint8_t component_id; uint8_t type; uint8_t state; uint8_t[4] _padding0

    #[derive(ULogData, Debug)]
    #[yule_log(multi_id = 1)]
    pub struct TelemetryStatus {
        //timestamp: u64,
        heartbeats: Vec<TelemetryHeartbeat>,
    }

    #[derive(ULogData, Debug)]
    pub struct TelemetryHeartbeat {
        timestamp: u64,
        system_id: u8,
        component_id: u8,
        state: u8,
    }
}

#[test]
fn integration_macros_nested() -> Result<(), Box<dyn std::error::Error>> {
    use ulog::LoggedMessages;
    
    // Integration tests run from the workspace root, so prefix with `core/`
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = LoggedMessages::stream(reader)?;

    //FIXME: Test 'integration_macros_nested' does not really test the output.

    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::PositionSetpointTriplet(v) => {
                println!("{:?}", v);
                //assert!(v.timestamp > 0);
            }
            LoggedMessages::TelemetryStatus(t) => {
                println!("{:?}", t);
            }
            LoggedMessages::Other(_) => {
                // Fine, ignore
            }
        }
    }

    Ok(())
}