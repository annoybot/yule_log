use std::collections::HashMap;
use std::io::Write;
use std::ops::{Deref, DerefMut};

use csv::Writer;

pub struct CsvExporter {
    columns: Vec<Column>,
}

// Key: A subscription message name. Example: "vehicle_angular_acceleration".
//      Multiplicity is indicated with two digits: "telemetry_status.00"
// Value: Column
pub struct ColumnMap(pub HashMap<String, Column>);

impl ColumnMap {
    pub fn new() -> Self {
        ColumnMap(HashMap::new())
    }
}

// Implement Deref and DerefMut to delegate to HashMap's methods.
impl Deref for ColumnMap {
    type Target = HashMap<String, Column>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ColumnMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct Column {
    name: String,
    data: Vec<Point>,
    current_index: usize,
}

impl Deref for Column {
    type Target = Vec<Point>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Column {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}


impl Column {
    pub fn new(name: String) -> Self {
        Column {
            name,
            ..Default::default()
        }
    }
}

impl Default for Column {
    fn default() -> Self {
        Column {
            name: String::new(),
            data: Vec::new(),
            current_index: 0,
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub t: f64,  // Timestamp
    pub x: f64,  // Data value
}

impl Column {
    fn value_at_time(&mut self, t: f64) -> Option<f64> {
        match &self.current() {
            None => { None}
            Some(current_point) => {
                if (current_point.t - t).abs() < f64::EPSILON {
                    self.current_index += 1;
                    Some(current_point.x)
                } else {
                    None
                }
            }
        }
    }

    fn current(&self) -> Option<Point> {
        if self.current_index >= self.data.len() {
            None// Column exhausted
        } else {
            Some(self.data[self.current_index])
        }
    }
}

impl CsvExporter {
    fn min_timestamp(&self) -> Option<f64> {
        let mut min_time = f64::MAX;

        for s in &self.columns {
            if let Some(current_point) = s.data.get(s.current_index) {
                if current_point.t < min_time {
                    min_time = current_point.t;
                }
            }
        }

        if min_time == f64::MAX {
            None
        } else {
            Some(min_time)
        }
    }

    pub fn from_columns(mut columns: Vec<Column>) -> Self
    {
        columns.sort_by_key(|s| s.name.clone());

        CsvExporter {
            columns,
        }
    }

    pub fn to_csv(&mut self, writer: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
        let mut csv_writer = Writer::from_writer(writer);

        let column_count = self.columns.len();

        // The length of a row is the number of columns + 1 to account for the `__time` column.
        let mut csv_record: Vec<String> = Vec::with_capacity(column_count + 1);

        // 1. Write header.
        csv_record.push("__time".to_string());

        for column in &self.columns {
            csv_record.push(column.name.clone());
        }

        csv_writer.write_record(&csv_record)?;
        csv_record.clear(); // Clear the vec to make it ready for reuse.

        // 2. Write rows.
        let empty_str = String::new();

        while let Some(min_timestamp) = self.min_timestamp() {
            csv_record.push(format!("{min_timestamp:.6}"));

            for column in self.columns.iter_mut() {
                csv_record.push(match column.value_at_time(min_timestamp) {
                    None => { empty_str.clone() }
                    Some(x) => { format!("{x:.9}") }
                });
            }

            csv_writer.write_record(&csv_record)?;
            csv_record.clear();
        }

        csv_writer.flush()?;
        Ok(())
    }
}




