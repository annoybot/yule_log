use yule_log::ULogData;

#[derive(ULogData)]
pub struct VehicleLocalPosition {
    timestamp: u64,
    x: f32,
    y: f32,
    z: f32,
}

fn main() {}
