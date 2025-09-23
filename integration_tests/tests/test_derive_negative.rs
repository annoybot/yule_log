use std::fs::File;
use std::io::BufReader;
use yule_log::{ULogData, ULogMessages};
use yule_log::errors::ULogError;
use yule_log::model::msg::UlogMessage;

#[test]
#[allow(clippy::float_cmp)]
fn test_nonexistent_field_error()  {
    
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