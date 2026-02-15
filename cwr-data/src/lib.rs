//! Data processing and interpolation for water/snow observations.
//!
//! This crate handles transforming raw observation data into forms
//! suitable for charting and analysis.

/// Linear interpolation for filling gaps in observation data.
pub mod interpolation {
    use chrono::NaiveDate;

    /// A single data point for interpolation
    #[derive(Debug, Clone)]
    pub struct DataPoint {
        pub date: NaiveDate,
        pub value: f64,
    }

    /// Linearly interpolate between two data points, filling in daily values.
    ///
    /// Returns a Vec of DataPoints for each day between start and end (inclusive).
    /// If start and end are the same day, returns just that point.
    pub fn interpolate_pair(start: &DataPoint, end: &DataPoint) -> Vec<DataPoint> {
        let days = (end.date - start.date).num_days();
        if days <= 0 {
            return vec![start.clone()];
        }

        let slope = (end.value - start.value) / days as f64;
        let mut result = Vec::with_capacity((days + 1) as usize);

        for i in 0..=days {
            let date = start.date + chrono::Duration::days(i);
            let value = (start.value + slope * i as f64).round();
            result.push(DataPoint { date, value });
        }

        result
    }

    /// Fill gaps in a sorted series of data points using linear interpolation.
    ///
    /// Input must be sorted by date. Gaps larger than 1 day are filled.
    /// Non-numeric values (gaps) are represented by missing entries.
    pub fn fill_gaps(points: &[DataPoint]) -> Vec<DataPoint> {
        if points.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();

        for window in points.windows(2) {
            let start = &window[0];
            let end = &window[1];
            let days_between = (end.date - start.date).num_days();

            if days_between <= 1 {
                result.push(start.clone());
            } else {
                // Interpolate the gap (excluding the end point, which will be
                // the start of the next window)
                let interpolated = interpolate_pair(start, end);
                // Add all except the last (which is the end point)
                for point in &interpolated[..interpolated.len() - 1] {
                    result.push(point.clone());
                }
            }
        }

        // Add the last point
        if let Some(last) = points.last() {
            result.push(last.clone());
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::NaiveDate;

        #[test]
        fn test_interpolate_pair_basic() {
            let start = DataPoint {
                date: NaiveDate::from_ymd_opt(2022, 11, 12).unwrap(),
                value: 7.0,
            };
            let end = DataPoint {
                date: NaiveDate::from_ymd_opt(2022, 11, 17).unwrap(),
                value: 16.0,
            };
            let result = interpolate_pair(&start, &end);
            assert_eq!(result.len(), 6);
            assert_eq!(result[0].value, 7.0);
            assert_eq!(result[1].value, 9.0); // 7 + 1.8 rounded
            assert_eq!(result[5].value, 16.0);
        }

        #[test]
        fn test_interpolate_pair_same_day() {
            let point = DataPoint {
                date: NaiveDate::from_ymd_opt(2022, 11, 12).unwrap(),
                value: 100.0,
            };
            let result = interpolate_pair(&point, &point);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].value, 100.0);
        }

        #[test]
        fn test_fill_gaps() {
            let points = vec![
                DataPoint {
                    date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
                    value: 100.0,
                },
                DataPoint {
                    date: NaiveDate::from_ymd_opt(2022, 1, 2).unwrap(),
                    value: 110.0,
                },
                // Gap: Jan 3-4 missing
                DataPoint {
                    date: NaiveDate::from_ymd_opt(2022, 1, 5).unwrap(),
                    value: 140.0,
                },
            ];
            let filled = fill_gaps(&points);
            assert_eq!(filled.len(), 5);
            assert_eq!(filled[0].value, 100.0);
            assert_eq!(filled[1].value, 110.0);
            assert_eq!(filled[4].value, 140.0);
        }
    }
}

/// Water level observation processing
pub mod water_level {
    /// Scale factor for Colorado River allocation (California's 27% share)
    pub const COLORADO_RIVER_CA_SHARE: f64 = 0.27;

    /// Station IDs for Lake Mead and Lake Powell
    pub const LAKE_MEAD_ID: &str = "MED";
    pub const LAKE_POWELL_ID: &str = "PWL";

    /// Check if a station is on the Colorado River (Mead or Powell)
    pub fn is_colorado_river_station(station_id: &str) -> bool {
        station_id == LAKE_MEAD_ID || station_id == LAKE_POWELL_ID
    }

    /// Scale a value for California's share of Colorado River water
    pub fn scale_colorado_share(value: f64) -> f64 {
        value * COLORADO_RIVER_CA_SHARE
    }
}
