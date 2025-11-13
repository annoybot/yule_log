use std::fs::File;
use std::io::BufReader;
use yule_log::builder::ULogParserBuilder;
use yule_log::model::msg::UlogMessage;

#[test]
fn test_sub_allow_list() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open("../core/test_data/input/sample_log_small.ulg")?);

    let stream = ULogParserBuilder::new(reader)
        .set_subscription_allow_list([
            "vehicle_local_position",
            "vehicle_gps_position",
            "vehicle_attitude",
        ])
        .build()?;

    #[derive(Default)]
    struct Flags {
        pos_seen: bool,
        gps_seen: bool,
        att_seen: bool,
        other_seen: bool,
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
            UlogMessage::LoggedData(data) => match data.data.name.as_ref() {
                "vehicle_local_position" => {
                    flags.pos_seen = true;
                }
                "vehicle_gps_position" => {
                    flags.gps_seen = true;
                }
                "vehicle_attitude" => {
                    flags.att_seen = true;
                }
                _ => {
                    flags.other_seen = true;
                }
            },
            _ => {}
        }

        if flags.all_seen() {
            break;
        }
    }

    assert!(
        flags.pos_seen,
        "Expected at least one 'vehicle_local_position' message."
    );
    assert!(
        flags.gps_seen,
        "Expected at least one 'vehicle_gps_position' message."
    );
    assert!(
        flags.att_seen,
        "Expected at least one 'vehicle_attitude' message."
    );

    assert!(!flags.other_seen, "Unexpected message received.");

    Ok(())
}
