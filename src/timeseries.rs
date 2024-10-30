use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct TimeseriesMap(HashMap<String, Timeseries>);

#[derive(Debug)]
pub struct Timeseries {
    pub timestamps: Vec<u64>,
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