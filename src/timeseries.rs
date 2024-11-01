use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
// Key: A subscription message name. Example: "vehicle_angular_acceleration".
//      Multiplicity is indicated with two digits: "telemetry_status.00"
// Value: Timeseries
pub struct TimeseriesMap(HashMap<String, Timeseries>);

#[derive(Debug)]
pub struct Timeseries {
    // Array of timestamps for all data in this time series.
    // Invariant: timestamps.len() == data.1.len()
    pub timestamps: Vec<u64>,

    // Key: Absolute path of field names. Example: ""/streams"
    // Path may have multiple components and multiplicity: "/heartbeats.00/system_id"
    pub data: Vec<(String, Vec<f64>)>,
}

impl TimeseriesMap {
    pub fn new() -> Self {
        TimeseriesMap(HashMap::new())
    }
}

impl Deref for TimeseriesMap {
    type Target = HashMap<String, Timeseries>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement DerefMut to delegate to HashMap for mutable methods
impl DerefMut for TimeseriesMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}