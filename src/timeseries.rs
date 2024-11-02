use std::collections::{BTreeSet, HashMap};
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

    // Filters the TimeseriesMap based on user-provided paths and their prefixes
    pub fn filter_by_paths(&mut self, paths: &[&str]) {
        // This has the effect of sorting the paths uniquely.
        let paths: BTreeSet<_> = paths.into_iter().collect();

        self.0.retain(|key, timeseries| {
            let mut retain_timeseries = false;
            let mut filtered_data = Vec::new();

            for path in &paths {
                // Split the path into left (subscription message) and right (field path)
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                let (left, right) = match &parts[..] {
                    [l, r] => (*l, Some(format!("/{}", r))),
                    [l] => (*l, None),
                    _ => continue,
                };

                // Check if the subscription message name (key) matches the left part of the path
                if key.eq(left) {   // or starts_with()
                    retain_timeseries = true;

                    // If there's a right part (field path), filter the data within the Timeseries
                    if let Some(field_prefix) = right {
                        timeseries.data.iter().for_each(|(data_path, values)| {
                            if data_path.eq(&field_prefix) {  // or starts_with()
                                filtered_data.push((data_path.clone(), values.clone()));
                            }
                        });
                    } else {
                        // If there's no right part, retain all data for this Timeseries
                        filtered_data = timeseries.data.clone();
                    }
                }
            }

            // Replace the timeseries data with filtered data (if any), and retain only if data exists
            timeseries.data = filtered_data;
            retain_timeseries && !timeseries.data.is_empty()
        });
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