//! Shared utility functions for CWR crates.

/// Date utility functions
pub mod dates {
    use chrono::NaiveDate;

    /// Format a NaiveDate as "YYYY-MM-DD"
    pub fn format_date(date: &NaiveDate) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    /// Parse a date string in "YYYY-MM-DD" format
    pub fn parse_date(s: &str) -> anyhow::Result<NaiveDate> {
        Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)
    }

    /// Parse a date string in "YYYYMMDD" format (CDEC compact format)
    pub fn parse_date_compact(s: &str) -> anyhow::Result<NaiveDate> {
        Ok(NaiveDate::parse_from_str(s, "%Y%m%d")?)
    }

    /// Get the water year for a given date.
    /// Water year runs Oct 1 to Sep 30.
    /// e.g., Oct 1 2022 -> water year 2022, Sep 30 2023 -> water year 2022
    pub fn water_year_for_date(date: &NaiveDate) -> i32 {
        use chrono::Datelike;
        let month = date.month();
        let year = date.year();
        if month >= 10 {
            year
        } else {
            year - 1
        }
    }

    /// Get the day-of-water-year (0-364) for a given date.
    /// Oct 1 = day 0, Sep 30 = day 364.
    pub fn day_of_water_year(date: &NaiveDate) -> i32 {
        use chrono::Datelike;
        let month = date.month();
        let day = date.day();
        let wy = water_year_for_date(date);
        let oct1 = NaiveDate::from_ymd_opt(wy, 10, 1).unwrap();
        let diff = (*date - oct1).num_days() as i32;
        // Handle Feb 29: skip it to keep 365 days
        if diff < 0 {
            // Shouldn't happen if water_year_for_date is correct
            return 0;
        }
        // Adjust for leap year: if the date is after Feb 28 in a leap year,
        // subtract 1 to skip Feb 29
        let next_year = wy + 1;
        let is_leap = NaiveDate::from_ymd_opt(next_year, 2, 29).is_some();
        if is_leap && month >= 3 {
            // After Feb in the second half of the water year
            let feb29 = NaiveDate::from_ymd_opt(next_year, 2, 29).unwrap();
            if *date > feb29 {
                return diff - 1;
            }
        }
        // Skip Feb 29 itself
        if month == 2 && day == 29 {
            return -1; // Signal to skip this day
        }
        diff
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::NaiveDate;

        #[test]
        fn test_water_year_for_date() {
            let oct1 = NaiveDate::from_ymd_opt(2022, 10, 1).unwrap();
            assert_eq!(water_year_for_date(&oct1), 2022);

            let sep30 = NaiveDate::from_ymd_opt(2023, 9, 30).unwrap();
            assert_eq!(water_year_for_date(&sep30), 2022);

            let jan1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
            assert_eq!(water_year_for_date(&jan1), 2022);
        }

        #[test]
        fn test_day_of_water_year() {
            let oct1 = NaiveDate::from_ymd_opt(2022, 10, 1).unwrap();
            assert_eq!(day_of_water_year(&oct1), 0);

            let oct2 = NaiveDate::from_ymd_opt(2022, 10, 2).unwrap();
            assert_eq!(day_of_water_year(&oct2), 1);

            let jan1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
            assert_eq!(day_of_water_year(&jan1), 92); // Oct has 31, Nov has 30, Dec has 31 = 92

            let feb29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
            assert_eq!(day_of_water_year(&feb29), -1); // Should be skipped
        }

        #[test]
        fn test_format_and_parse() {
            let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
            let formatted = format_date(&date);
            assert_eq!(formatted, "2023-06-15");
            let parsed = parse_date(&formatted).unwrap();
            assert_eq!(parsed, date);
        }
    }
}

/// Error types
pub mod error {
    use std::fmt;

    #[derive(Debug)]
    pub struct DateError(pub String);

    impl fmt::Display for DateError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Date error: {}", self.0)
        }
    }

    impl std::error::Error for DateError {}
}
