use std::io::Write;
use crate::timeseries::TimeseriesMap;
use csv::Writer;

pub struct CsvExporter {
    columns: Vec<Column>,
}

#[derive(Debug)]
pub struct Column {
    name: String,
    data: Vec<Point>,
    current_index: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct Point {
    t: f64,
    // Timestamp
    x: f64,  // Data value
}

impl Column {
    fn value_at_time(&mut self, t: f64) -> Option<f64> {
        match &self.current() {
            None => { return None }
            Some(current_point) => {
                if (current_point.t - t).abs() < f64::EPSILON {
                    self.current_index += 1;
                    return Some(current_point.x);
                } else {
                    return None;
                }
            }
        }
    }

    fn current(&self) -> Option<Point> {
        if self.current_index >= self.data.len() {
            return None; // Column exhausted
        } else {
            return Some( self.data[self.current_index] );
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

    pub fn from_timeseries_map(timeseries_map: TimeseriesMap) -> Self {
        let mut columns: Vec<Column> = vec![];
        let mut min_msg_time = f64::MAX;

        for (topic_name, timeseries) in timeseries_map.iter() {
            for (field_name, data) in timeseries.data.iter() {
                // ⚠️ The field_names in the timeseries map will always contain a leading '/',
                // so there is no need to add one  in the format statement below.
                let mut column = Column { name: format!("{}{}", topic_name, field_name.clone()), data: vec![], current_index: 0 };

                assert_eq!(timeseries.timestamps.len(), data.len());

                for i in 0..data.len() {
                    let msg_time = (timeseries.timestamps[i].clone() as f64) * 0.000001;
		    
                    // Round to six digits.
                    let rounded_msg_time = (msg_time * 1_000_000.0).round() / 1_000_000.0;
                    min_msg_time = f64::min(min_msg_time, msg_time);

                    column.data.push(Point { t: rounded_msg_time, x: data[i] });
                }
                columns.push(column);
            }
        }

        columns.sort_by_key(|s| s.name.clone());

        CsvExporter {
            columns,
        }
    }

    pub fn to_csv(&mut self, writer: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
        let mut csv_writer = Writer::from_writer(writer);

        let column_count = self.columns.len();

        // The length of a row is the number of columns + 1 to account for the `__time` column.
        let mut csv_record:Vec<String> = Vec::with_capacity(column_count + 1);

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
            csv_record.push(format!("{:.6}",min_timestamp) );

            for column in self.columns.iter_mut() {
                csv_record.push(match column.value_at_time(min_timestamp) {
                    None => { empty_str.clone() }
                    Some(x) => { format!("{:.9}", x) }
                });
            }

            csv_writer.write_record(&csv_record)?;
            csv_record.clear();
        }

        csv_writer.flush()?;
        Ok(())
    }
}




