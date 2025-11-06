use std::fs::File;
use std::io::BufReader;

use yule_log::model::msg::UlogMessage;
use yule_log::{ULogData, ULogMessages};

#[derive(ULogMessages)]
pub enum LoggedMessages {
    PositionSetpointTriplet(PositionSetpointTriplet),
    TelemetryStatus(TelemetryStatus),
    #[yule_log(forward_other)]
    Other(UlogMessage),
}

#[derive(ULogData, Debug, Clone)]
// https://docs.px4.io/main/en/msg_docs/PositionSetpointTriplet.html
pub struct PositionSetpointTriplet {
    previous: PositionSetpoint,
    current: PositionSetpoint,
    next: PositionSetpoint,
}

#[derive(ULogData, Debug, Clone)]
// https://docs.px4.io/main/en/msg_docs/PositionSetpoint.html
pub struct PositionSetpoint {
    timestamp: u64,
    vx: f32, // local velocity setpoint in m/s in NED
    vy: f32, // local velocity setpoint in m/s in NED
    vz: f32, // local velocity setpoint in m/s in NED

    lat: f64, // latitude, in deg
    lon: f64, // longitude, in deg
    alt: f32, // altitude AMSL, in m
}

#[derive(ULogData, Debug, Clone)]
#[yule_log(multi_id = 1)]
pub struct TelemetryStatus {
    heartbeats: Vec<TelemetryHeartbeat>,
}

#[derive(ULogData, Debug, Clone)]
pub struct TelemetryHeartbeat {
    timestamp: u64,
    system_id: u8,
    component_id: u8,
    state: u8,
}

// We need to specially implement PartialEq because there are some NaNs in the test data.

impl PartialEq for PositionSetpoint {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.vx == other.vx
            && self.vy == other.vy
            && self.vz == other.vz
            && float_eq_nan(self.lat, other.lat)
            && float_eq_nan(self.lon, other.lon)
            && self.alt == other.alt
    }
}

impl PartialEq for PositionSetpointTriplet {
    fn eq(&self, other: &Self) -> bool {
        self.previous == other.previous && self.current == other.current && self.next == other.next
    }
}

impl PartialEq for TelemetryHeartbeat {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.system_id == other.system_id
            && self.component_id == other.component_id
            && self.state == other.state
    }
}

impl PartialEq for TelemetryStatus {
    fn eq(&self, other: &Self) -> bool {
        self.heartbeats == other.heartbeats
    }
}

// Helper function: compares f64 treating NaN == NaN
fn float_eq_nan(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a == b)
}

#[test]
fn test_derive_nested() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);
    let stream = LoggedMessages::stream(reader)?;

    let mut position_triplets = Vec::<PositionSetpointTriplet>::new();
    let mut telemetry_statuses = Vec::<TelemetryStatus>::new();

    for msg_res in stream {
        let msg = msg_res?;
        match msg {
            LoggedMessages::PositionSetpointTriplet(v) => {
                println!("PositionSetpointTriplet: {:?}", v);
                position_triplets.push(v);
            }
            LoggedMessages::TelemetryStatus(t) => {
                println!("TelemetryStatus: {:?}", t);
                telemetry_statuses.push(t);
            }
            LoggedMessages::Other(_) => {
                // Ignore other messages
            }
        }
    }

    // Expected data from your output snippet
    let expected_position_triplets = vec![PositionSetpointTriplet {
        previous: PositionSetpoint {
            timestamp: 1_425_100,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            lat: f64::NAN,
            lon: f64::NAN,
            alt: 0.0,
        },
        current: PositionSetpoint {
            timestamp: 1_425_100,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            lat: f64::NAN,
            lon: f64::NAN,
            alt: 0.0,
        },
        next: PositionSetpoint {
            timestamp: 1_425_101,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            lat: f64::NAN,
            lon: f64::NAN,
            alt: 0.0,
        },
    }];

    let expected_telemetry_statuses = vec![
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 19_465_393,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 20_465_189,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 21_465_211,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 21_465_211,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 22_477_255,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 23_465_445,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 24_477_325,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
        TelemetryStatus {
            heartbeats: vec![
                TelemetryHeartbeat {
                    timestamp: 25_477_255,
                    system_id: 255,
                    component_id: 190,
                    state: 4,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
                TelemetryHeartbeat {
                    timestamp: 0,
                    system_id: 0,
                    component_id: 0,
                    state: 0,
                },
            ],
        },
    ];

    assert_eq!(position_triplets, expected_position_triplets);
    assert_eq!(telemetry_statuses, expected_telemetry_statuses);

    Ok(())
}
