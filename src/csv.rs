use crate::timeseries::TimeseriesMap;
use csv::Writer;

pub struct CsvExporter {
    series: Vec<Series>,
}

#[derive(Debug)]
pub struct Series {
    name: String,
    data: Vec<Point>,
}

#[derive(Debug)]
pub struct Point {
    t: f64,
    // Timestamp
    x: f64,  // Data value
}


impl CsvExporter {
    pub fn from_timeseries_map(timeseries_map: TimeseriesMap) -> Self {
        let mut series: Vec<Series> = vec![];
        let mut min_msg_time = f64::MAX;

        for (topic_name, timeseries) in timeseries_map.iter() {
            for (field_name, data) in timeseries.data.iter() {
                let mut serie = Series { name: field_name.clone(), data: vec![] };

                assert_eq!(timeseries.timestamps.len(), data.len());

                for i in 0..data.len() {
                    let msg_time = (timeseries.timestamps[i].clone() as f64) * 0.000001;
                    let rounded_msg_time = (msg_time * 1_000_000.0).round() / 1_000_000.0;
                    min_msg_time = f64::min(min_msg_time, msg_time);

                    serie.data.push(Point { t: rounded_msg_time, x: data[i] });
                }
                println!("{:?}", serie);
                series.push(serie);
            }
        }

        series.sort_by_key(|s| s.name.clone());

        CsvExporter {
            series,
        }
    }

    pub fn to_csv_string(&self) -> Result<String, Box<dyn std::error::Error>> {

        let  series_count = self.series.len();

        // Initialize indices and row values
        let mut indices = vec![0; series_count];
        let mut row_values = vec![f64::NAN; series_count];

        let mut wtr = Writer::from_writer(vec![]);

        // Write headers
        let mut headers = vec!["__time".to_string()];
        for series in &self.series {
            headers.push(series.name.clone());
        }
        wtr.write_record(&headers)?;

        let mut done = false;

        while !done {
            done = true;
            let mut min_time = f64::MAX;

            for i in 0..series_count {
                let series = &self.series[i];
                row_values[i] = f64::NAN;

                if indices[i] >= series.data.len() {
                    continue;
                }

                let point = &series.data[indices[i]];


                done = false;

                if min_time > point.t {
                    min_time = point.t;  // new min_time
                    // Reset previous flags
                    row_values.iter_mut().for_each(|v| *v = f64::NAN);
                    row_values[i] = point.x;
                } else if (min_time - point.t).abs() < f64::EPSILON {
                    row_values[i] = point.x;
                }
            }

            if  done {
                break;
            }

            // Write the row to the CSV
            let mut row_record = vec![min_time.to_string()];
            for &value in &row_values {
                if !value.is_nan() {
                    row_record.push(value.to_string());
                    // Move to the next index for that series
                    let index_pos = row_values.iter().position(|&v| v == value).unwrap();
                    indices[index_pos] += 1;
                } else {
                    row_record.push("".to_string());  // Empty value for missing data
                }
            }

            wtr.write_record(&row_record)?;
        }

        // Convert the writer into a String
        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok( data )
    }
}




