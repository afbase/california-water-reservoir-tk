use chrono::{NaiveDate, TimeDelta};
use std::mem::replace;

/// A date range iterator that yields each date from the start date
/// through the end date (inclusive).
#[derive(Clone, Eq, PartialEq, Copy, Debug)]
pub struct DateRange(pub NaiveDate, pub NaiveDate);

impl Iterator for DateRange {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 <= self.1 {
            let next = self.0 + TimeDelta::try_days(1).unwrap();
            Some(replace(&mut self.0, next))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DateRange;
    use chrono::NaiveDate;

    #[test]
    fn test_date_range_iteration() {
        let start = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2022, 1, 5).unwrap();
        let range = DateRange(start, end);
        let dates: Vec<NaiveDate> = range.collect();
        assert_eq!(dates.len(), 5);
        assert_eq!(dates[0], start);
        assert_eq!(dates[4], end);
    }

    #[test]
    fn test_date_range_single_day() {
        let start = NaiveDate::from_ymd_opt(2022, 3, 15).unwrap();
        let range = DateRange(start, start);
        let dates: Vec<NaiveDate> = range.collect();
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0], start);
    }

    #[test]
    fn test_date_range_empty() {
        let start = NaiveDate::from_ymd_opt(2022, 3, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2022, 3, 14).unwrap();
        let range = DateRange(start, end);
        let dates: Vec<NaiveDate> = range.collect();
        assert_eq!(dates.len(), 0);
    }
}
