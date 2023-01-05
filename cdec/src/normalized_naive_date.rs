use chrono::{DateTime, Datelike, Duration, IsoWeek, Local, NaiveDate, Weekday};
use core::{mem::replace, ops::Add};
use plotters::prelude::*;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub struct NormalizedNaiveDate {
    year: i32,
    month: u32,
    day: u32,
}

impl NormalizedNaiveDate {
    pub fn from_md_opt(month: u32, day: u32) -> Option<NormalizedNaiveDate> {
        let normalized_year = NormalizedNaiveDate::derive_normalized_year(month);
        NaiveDate::from_ymd_opt(normalized_year, month, day).map(|_| NormalizedNaiveDate {
            year: normalized_year,
            month,
            day,
        })
    }
    pub fn get_normalized_ranged_date() -> RangedDate<NormalizedNaiveDate> {
        // California’s water year runs from October 1 to September 30 and is the official 12-month timeframe
        let start = NormalizedNaiveDate::from_md_opt(10, 1).unwrap();
        let end = NormalizedNaiveDate::from_md_opt(9, 30).unwrap();
        let date_range = Range { start, end };
        let ranged_date: RangedDate<NormalizedNaiveDate> = date_range.clone().into();
        ranged_date
    }

    pub fn normalized_year(&self) -> i32 {
        Self::derive_normalized_year(self.month)
    }

    pub fn as_naive_date(&self) -> NaiveDate {
        let day = self.day;
        let month = self.month;
        let year = self.normalized_year();
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn derive_normalized_year(month: u32) -> i32 {
        let dt: DateTime<Local> = Local::now();
        let first_year = dt.naive_local().date().year() - 1;
        let second_year = first_year + 1;
        match month {
            10..=12 => first_year,
            _ => second_year,
        }
    }

    fn map_to_option_self(sub_result: Option<NaiveDate>) -> Option<Self> {
        if let Some(naive_date) = sub_result {
            let inner_result: NormalizedNaiveDate = naive_date.into();
            return Some(inner_result);
        }
        None
    }
}

impl Datelike for NormalizedNaiveDate {
    fn year(&self) -> i32 {
        let naive = self.as_naive_date();
        naive.year()
    }
    fn month(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.month()
    }
    fn month0(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.month0()
    }
    fn day(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.day()
    }
    fn day0(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.day0()
    }
    fn ordinal(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.ordinal()
    }
    fn ordinal0(&self) -> u32 {
        let naive = self.as_naive_date();
        naive.ordinal0()
    }
    fn weekday(&self) -> Weekday {
        let naive = self.as_naive_date();
        naive.weekday()
    }
    fn iso_week(&self) -> IsoWeek {
        let naive = self.as_naive_date();
        naive.iso_week()
    }
    fn with_year(&self, year: i32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_year(year))
    }
    fn with_month(&self, month: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_month(month))
    }
    fn with_month0(&self, month0: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_month0(month0))
    }
    fn with_day(&self, day: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_day(day))
    }
    fn with_day0(&self, day0: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_day0(day0))
    }
    fn with_ordinal(&self, ordinal: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_ordinal(ordinal))
    }
    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self> {
        let naive = self.as_naive_date();
        NormalizedNaiveDate::map_to_option_self(naive.with_ordinal0(ordinal0))
    }
    fn year_ce(&self) -> (bool, u32) {
        let naive = self.as_naive_date();
        naive.year_ce()
    }
    fn num_days_from_ce(&self) -> i32 {
        let naive = self.as_naive_date();
        naive.num_days_from_ce()
    }
}

impl From<NaiveDate> for NormalizedNaiveDate {
    fn from(value: NaiveDate) -> Self {
        let day = value.day();
        let month = value.month();
        NormalizedNaiveDate::from_md_opt(month, day).unwrap()
    }
}

impl From<NormalizedNaiveDate> for NaiveDate {
    fn from(value: NormalizedNaiveDate) -> Self {
        let day = value.day();
        let month = value.month();
        let year = value.year();
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }
}

impl Add<Duration> for NormalizedNaiveDate {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        let naive_date = self.as_naive_date();
        let operation = naive_date + rhs;
        let result: NormalizedNaiveDate = operation.into();
        result
    }
}

#[derive(Clone, Eq, PartialEq, Copy, Debug)]
pub struct NormalizedDateRange(pub NormalizedNaiveDate, pub NormalizedNaiveDate);

impl Iterator for NormalizedDateRange {
    type Item = NormalizedNaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 <= self.1 {
            let next = self.0 + Duration::days(1);
            Some(replace(&mut self.0, next))
        } else {
            None
        }
    }
}
