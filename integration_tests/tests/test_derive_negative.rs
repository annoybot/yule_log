use std::fs::File;
use std::io::BufReader;
use yule_log::errors::ULogError;
use yule_log::model::msg::UlogMessage;
use yule_log::{ULogData, ULogMessages};

#[test]
#[allow(clippy::float_cmp)]
fn test_nonexistent_field_error() {
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

        // Test error handling for a field which is not present in the ULOG file.
        not_there: u64,
    }

    fn read_messages() -> Result<(), ULogError> {
        let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);
        let stream = LoggedMessages::stream(reader)?;

        for msg_res in stream {
            let msg = msg_res?;
            match msg {
                _ => {}
            }
        }

        Ok(())
    }

    let result = read_messages();
    println!("{:?}", result);
    assert!(matches!(result, Err(ULogError::InvalidFieldName(_))));
}

#[test]
fn test_not_added_subscription_absent() {
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

    let reader = BufReader::new(
        File::open("../core/test_data/input/sample_log_small.ulg").expect("Unable to open file"),
    );

    let stream = LoggedMessages::stream(reader).unwrap();

    // "vehicle_gps_position" should never appear because we haven't added it using `add_subscription()`.
    for msg_res in stream {
        if let Ok(LoggedMessages::Other(UlogMessage::LoggedData(data))) = msg_res {
            assert_ne!(
                data.data.name.to_string(), "vehicle_gps_position",
                "vehicle_gps_position should not appear."
            );
        }
    }
}

#[test]
fn test_non_existent_variant_add_subscription() {
    #[derive(ULogMessages)]
    #[allow(dead_code)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
    }

    #[derive(ULogData, Debug, PartialEq, Clone)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    let reader = BufReader::new(
        File::open("../core/test_data/input/sample_log_small.ulg").expect("Unable to open file"),
    );

    const EXTRA_SUBSCR_NAME: &'static str = "vehicle_gps_position";

    let result = LoggedMessages::builder(reader).add_subscription(EXTRA_SUBSCR_NAME);

    println!("{:?}", result);
    assert!(matches!(result, Err(ULogError::InvalidConfiguration(_))));
}

#[test]
fn test_non_existent_variant_extend_subscriptions() {
    #[derive(ULogMessages)]
    #[allow(dead_code)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
    }

    #[derive(ULogData, Debug, PartialEq, Clone)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    let reader = BufReader::new(
        File::open("../core/test_data/input/sample_log_small.ulg").expect("Unable to open file"),
    );

    let result = LoggedMessages::builder(reader)
        .extend_subscriptions(["vehicle_gps_position", "vehicle_attitude"]);

    println!("{:?}", result);
    assert!(matches!(result, Err(ULogError::InvalidConfiguration(_))));
}

#[test]
fn test_nonexistent_variant_fwd_subscription() {
    #[derive(ULogMessages)]
    #[allow(dead_code)]
    pub enum LoggedMessages {
        VehicleLocalPosition(VehicleLocalPosition),
    }

    #[derive(ULogData, Debug, PartialEq, Clone)]
    pub struct VehicleLocalPosition {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    let reader = BufReader::new(
        File::open("../core/test_data/input/sample_log_small.ulg").expect("Unable to open file"),
    );
    let result = LoggedMessages::builder(reader).forward_subscriptions(true);

    println!("{:?}", result);
    assert!(matches!(result, Err(ULogError::InvalidConfiguration(_))));
}
