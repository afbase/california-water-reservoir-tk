use chrono::{Duration, NaiveDate};
use std::iter::Iterator;
use std::mem::replace;

#[derive(Clone, Eq, PartialEq, Copy, Debug)]
pub struct DateRange(pub NaiveDate, pub NaiveDate);

impl Iterator for DateRange {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 <= self.1 {
            let next = self.0 + Duration::days(1);
            Some(replace(&mut self.0, next))
        } else {
            None
        }
    }
}
