use crate::{
    normalized_naive_date::NormalizedNaiveDate, observable::ObservableRange, survey::Survey,
};
use chrono::{DateTime, Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::HashMap;

/// Default number of water year charts to display.
pub const NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT: usize = 20;

/// California's water year runs from October 1 to September 30 and is the official
/// 12-month timeframe used by water managers to compile and compare hydrologic records.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaterYear(pub Vec<Survey>);

/// Statistics computed for a single water year: highest/lowest values and their dates.
#[derive(Debug, Serialize, Deserialize)]
pub struct WaterYearStatistics {
    pub year: i32,
    pub date_lowest: NaiveDate,
    pub date_highest: NaiveDate,
    pub highest_value: f64,
    pub lowest_value: f64,
}

impl WaterYearStatistics {
    /// Returns true if this is the driest year (lowest minimum) in a collection.
    pub fn is_driest_in(&self, all_stats: &[WaterYearStatistics]) -> bool {
        all_stats
            .iter()
            .all(|other| self.lowest_value <= other.lowest_value)
    }

    /// Returns true if this is the wettest year (highest maximum) in a collection.
    pub fn is_wettest_in(&self, all_stats: &[WaterYearStatistics]) -> bool {
        all_stats
            .iter()
            .all(|other| self.highest_value >= other.highest_value)
    }
}

/// Trait for normalizing calendar years within a single water year.
pub trait NormalizeCalendarYear {
    fn normalize_calendar_years(&mut self);
}

/// Errors related to water year operations.
#[derive(Debug)]
pub enum WaterYearErrors {
    InsufficientWaterYears,
}

/// Trait for normalizing, sorting, and analyzing collections of water years.
pub trait NormalizeWaterYears {
    fn normalize_dates(&mut self);
    fn get_largest_acrefeet_over_n_years(&self, len: usize) -> Result<f64, WaterYearErrors>;
    fn get_complete_normalized_water_years(&self) -> Self;
    fn sort_by_lowest_recorded_years(&mut self);
    fn sort_by_wettest_years(&mut self);
    fn sort_by_most_recent(&mut self);
    fn sort_surveys(&mut self);
}

impl NormalizeWaterYears for Vec<WaterYear> {
    fn normalize_dates(&mut self) {
        self.retain(|water_year| {
            // keep the water year if it has at least ~12 months of data
            water_year.0.len() >= 364
        });
        for water_year in self.iter_mut() {
            // get rid of feb_29
            water_year.0.retain(|survey| {
                let obs_date = survey.date_observation();
                let month = obs_date.month();
                let day = obs_date.day();
                !matches!((month, day), (2, 29))
            });
            // turn date_recording into date_observation of the original date
            // California's water year runs from October 1 to September 30
            for survey in &mut water_year.0 {
                let tap = survey.tap();
                tap.date_recording = tap.date_observation;
                let month = tap.date_observation.month();
                let day = tap.date_observation.day();
                let year = {
                    let dt: DateTime<Local> = Local::now();
                    let (first_year, second_year) = {
                        let this = &dt.naive_local().date();
                        let year = this.year();
                        (year - 1, year)
                    };
                    match month {
                        10..=12 => first_year,
                        _ => second_year,
                    }
                };
                match NaiveDate::from_ymd_opt(year, month, day) {
                    Some(d) => {
                        tap.date_observation = d;
                    }
                    None => {
                        panic!("Normalize Date Failed: {year}/{month}/{day}");
                    }
                }
                tap.date_observation = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            }
        }
    }

    fn get_largest_acrefeet_over_n_years(&self, len: usize) -> Result<f64, WaterYearErrors> {
        let number_of_charts = self.len().min(len);
        if number_of_charts > 0 {
            let largest_acrefeet = self[0..number_of_charts]
                .to_vec()
                .iter()
                .map(|water_year| {
                    let water_stat: WaterYearStatistics = water_year.into();
                    water_stat.highest_value
                })
                .collect::<Vec<_>>();
            let mut y_max: f64 = *largest_acrefeet
                .iter()
                .max_by(|a, b| a.total_cmp(b))
                .unwrap();
            if y_max > 500000.0 {
                y_max += 500000.0;
            } else {
                y_max += y_max / 5.0;
            }
            Ok(y_max)
        } else {
            Err(WaterYearErrors::InsufficientWaterYears)
        }
    }

    fn get_complete_normalized_water_years(&self) -> Self {
        let mut vector_clone = self.clone();
        vector_clone.retain(|water_year| {
            // keep the water year if it has at least ~12 months of data
            water_year.0.len() >= 364
        });
        for water_year in &mut vector_clone {
            water_year.normalize_calendar_years();
        }
        vector_clone
    }

    fn sort_by_lowest_recorded_years(&mut self) {
        self.sort_by(|a, b| {
            let a_surveys = &a.0;
            let b_surveys = &b.0;
            let a_min = {
                let mut val = f64::MAX;
                let mut other;
                for survey in a_surveys {
                    other = survey.get_value();
                    val = val.min(other)
                }
                val
            };
            let b_min = {
                let mut val = f64::MAX;
                let mut other;
                for survey in b_surveys {
                    other = survey.get_value();
                    val = val.min(other)
                }
                val
            };
            a_min.partial_cmp(&b_min).unwrap()
        });
    }

    /// Sort water years by their wettest peak (highest maximum value), descending.
    fn sort_by_wettest_years(&mut self) {
        self.sort_by(|a, b| {
            let a_max = a.0.iter().map(|s| s.get_value()).fold(f64::MIN, f64::max);
            let b_max = b.0.iter().map(|s| s.get_value()).fold(f64::MIN, f64::max);
            b_max
                .partial_cmp(&a_max)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn sort_by_most_recent(&mut self) {
        // use date recording
        self.sort_by(|a, b| {
            let a_surveys = &a.0;
            let b_surveys = &b.0;
            let a_survey = a_surveys.first().unwrap();
            let b_survey = b_surveys.first().unwrap();
            let a_year = a_survey.get_tap().date_recording.year();
            let b_year = b_survey.get_tap().date_recording.year();
            a_year.partial_cmp(&b_year).unwrap()
        });
        self.reverse();
    }

    fn sort_surveys(&mut self) {
        for water_year in self {
            water_year.0.sort_by(|a, b| {
                let a_tap = a.get_tap();
                let b_tap = b.get_tap();
                let a_date_recording = a_tap.date_recording;
                let b_date_recording = b_tap.date_recording;
                a_date_recording.partial_cmp(&b_date_recording).unwrap()
            });
        }
    }
}

/// Trait for retrieving clean (complete + normalized) water year data for a reservoir.
pub trait CleanReservoirData {
    fn get_clean_reservoir_water_years(&self, key: String) -> Option<Vec<WaterYear>>;
}

impl CleanReservoirData for HashMap<String, Vec<WaterYear>> {
    fn get_clean_reservoir_water_years(&self, key: String) -> Option<Vec<WaterYear>> {
        let test = self.get(&key);
        if test.is_none() {
            panic!("something is going on here");
        }
        test.map(|selected_reservoir_data| {
            selected_reservoir_data.get_complete_normalized_water_years()
        })
    }
}

impl NormalizeCalendarYear for WaterYear {
    fn normalize_calendar_years(&mut self) {
        if !self.0.iter().is_sorted() {
            self.0.sort();
        }
        for survey in &mut self.0 {
            // turn date_recording into date_observation of the original date
            let tap = survey.tap();
            tap.date_recording = tap.date_observation;
            // California's water year runs from October 1 to September 30
            let month = tap.date_observation.month();
            let day = tap.date_observation.day();
            let normalized_year = NormalizedNaiveDate::derive_normalized_year(month);
            let normalized_date =
                NaiveDate::from_ymd_opt(normalized_year, month, day).map(|_| NormalizedNaiveDate {
                    year: normalized_year,
                    month,
                    day,
                });
            if normalized_date.is_none() {
                continue;
            }
            let normalized_naive_date: NaiveDate = normalized_date.unwrap().into();
            tap.date_observation = normalized_naive_date;
        }
        // get rid of feb_29
        self.0.retain(|survey| {
            let obs_date = survey.date_observation();
            let month = obs_date.month();
            let day = obs_date.day();
            !matches!((month, day), (2, 29))
        });
    }
}

impl WaterYear {
    /// Get the original calendar year date range from a normalized water year.
    /// In a normalized water year, date_recording holds the original date_observation.
    pub fn calendar_year_from_normalized_water_year(&self) -> (NaiveDate, NaiveDate) {
        let first_survey = self.0.first().unwrap();
        let last_survey = self.0.last().unwrap();
        let first_date = first_survey.get_tap().date_recording;
        let last_date = last_survey.get_tap().date_recording;
        (first_date, last_date)
    }

    /// Compute the net change in water level over the year (last day - first day).
    pub fn calendar_year_change(&mut self) -> f64 {
        let _ = &self.0.sort();
        let first_day = self.0.first().unwrap();
        let last_day = self.0.last().unwrap();
        (last_day.get_value() - first_day.get_value()).round()
    }

    /// Create water years from an ObservableRange by partitioning surveys
    /// into October 1 - September 30 periods.
    pub fn water_years_from_observable_range(
        water_observations: &ObservableRange,
    ) -> Vec<WaterYear> {
        let min_year = water_observations.start_date.year() - 1;
        let max_year = water_observations.end_date.year();
        let mut water_years = Vec::new();

        for year in min_year..=max_year {
            let start_of_year = NaiveDate::from_ymd_opt(year, 10, 1).unwrap();
            let end_of_year = NaiveDate::from_ymd_opt(year + 1, 9, 30).unwrap();

            let water_calendar_year_of_observations: Vec<_> = water_observations
                .observations
                .iter()
                .filter(|survey| {
                    let tap = survey.get_tap();
                    let obs_date = tap.date_observation;
                    // cannot have Feb 29
                    let month = obs_date.month();
                    let day = obs_date.day();
                    let not_feb_29 = (2, 29) != (month, day);

                    start_of_year <= obs_date && obs_date <= end_of_year && not_feb_29
                })
                .cloned()
                .collect();

            if !water_calendar_year_of_observations.is_empty() {
                water_years.push(WaterYear(water_calendar_year_of_observations));
            }
        }

        water_years
    }
}

impl From<WaterYear> for WaterYearStatistics {
    fn from(value: WaterYear) -> Self {
        // surveys should be sorted by date
        let mut surveys = value.0;
        let year = {
            match surveys.first() {
                Some(survey) => {
                    let date_observation = survey.get_tap().date_observation;
                    let date_observation_year = date_observation.year();
                    // if date precedes water calendar year, then it is year minus 1
                    let start_of_year =
                        NaiveDate::from_ymd_opt(date_observation_year, 10, 1).unwrap();
                    if date_observation < start_of_year {
                        date_observation_year - 1
                    } else {
                        date_observation_year
                    }
                }
                None => 0,
            }
        };
        sort_by_values_ascending(&mut surveys);
        surveys.reverse();
        let vec_len = surveys.len();
        let lowest = surveys[vec_len - 1].clone();
        let lowest_tap = lowest.get_tap();
        let highest = surveys[0].clone();
        let highest_tap = highest.get_tap();
        WaterYearStatistics {
            year,
            date_lowest: lowest_tap.date_observation,
            date_highest: highest_tap.date_observation,
            highest_value: highest.get_value(),
            lowest_value: lowest.get_value(),
        }
    }
}

impl From<&WaterYear> for WaterYearStatistics {
    fn from(value: &WaterYear) -> Self {
        let surveys = value.clone();
        surveys.into()
    }
}

fn sort_by_values_ascending(surveys: &mut [Survey]) {
    surveys.sort_by(|survey_a, survey_b| {
        let a = survey_a.get_value();
        let b = survey_b.get_value();
        a.partial_cmp(&b).unwrap()
    });
}

impl PartialOrd for WaterYearStatistics {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for WaterYearStatistics {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year
            && self.date_lowest == other.date_lowest
            && self.date_highest == other.date_highest
            && self.highest_value == other.highest_value
            && self.lowest_value == other.lowest_value
    }
}

impl Ord for WaterYearStatistics {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.lowest_value < other.lowest_value {
            Less
        } else if self.lowest_value == other.lowest_value {
            Equal
        } else {
            Greater
        }
    }
}

impl Eq for WaterYearStatistics {}

#[cfg(test)]
mod tests {
    use super::{WaterYear, WaterYearStatistics};
    use crate::date_range::DateRange;
    use crate::observable::MonthDatum;
    use crate::observable::ObservableRange;
    use crate::observation::DataRecording;
    use crate::survey::{Survey, Tap};
    use crate::water_year::{NormalizeCalendarYear, NormalizeWaterYears};
    use chrono::{DateTime, Datelike, Local, NaiveDate};
    use std::collections::HashSet;

    #[test]
    fn test_water_years_from_surveys() {
        let a = MonthDatum::new(1, 1);
        let b = HashSet::from([a]);
        let d_1 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let surveys = vec![
            Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d,
                date_recording: d,
                value: DataRecording::Recording(3),
            }),
            Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d_1,
                date_recording: d_1,
                value: DataRecording::Recording(3),
            }),
        ];
        let obs = ObservableRange {
            observations: surveys,
            start_date: d_1,
            end_date: d,
            month_datum: b,
        };

        let actual: HashSet<WaterYear> =
            HashSet::from_iter(WaterYear::water_years_from_observable_range(&obs));
        let expected: HashSet<WaterYear> = HashSet::from_iter(vec![
            WaterYear(vec![Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d_1,
                date_recording: d_1,
                value: DataRecording::Recording(3),
            })]),
            WaterYear(vec![Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d,
                date_recording: d,
                value: DataRecording::Recording(3),
            })]),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_normalization() {
        let start_day = 29;
        let start_month = 12;
        let end_day = 3;
        let end_month = 1;
        let actual_start_year = NaiveDate::from_ymd_opt(1924, start_month, start_day).unwrap();
        let actual_end_year = NaiveDate::from_ymd_opt(1925, end_month, end_day).unwrap();
        let actual_date_range = DateRange(actual_start_year, actual_end_year);
        let mut surveys: Vec<Survey> = Vec::new();
        let mut survey;
        for day in actual_date_range {
            survey = Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: day,
                date_recording: day,
                value: DataRecording::Recording(3),
            });
            surveys.push(survey);
        }
        let actual_observable_range: ObservableRange = surveys.into();
        let mut actual_water_years =
            WaterYear::water_years_from_observable_range(&actual_observable_range);
        for water_year in &mut actual_water_years {
            water_year.normalize_calendar_years();
        }
        // make expected
        let dt: DateTime<Local> = Local::now();
        let first_year = dt.naive_local().date().year() - 1;
        let last_year = first_year + 1;
        let first_date = NaiveDate::from_ymd_opt(first_year, start_month, start_day).unwrap();
        let last_date = NaiveDate::from_ymd_opt(last_year, end_month, end_day).unwrap();
        let expected_date_range = DateRange(first_date, last_date);
        surveys = Vec::new();
        for day in expected_date_range {
            survey = Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: day,
                date_recording: day,
                value: DataRecording::Recording(3),
            });
            surveys.push(survey);
        }
        let expected_observable_range: ObservableRange = surveys.into();
        let expected_water_years =
            WaterYear::water_years_from_observable_range(&expected_observable_range);
        assert_eq!(actual_water_years, expected_water_years);
    }

    #[test]
    fn test_normalization_2() {
        let start_day = 1;
        let start_month = 10;
        let end_day = 30;
        let end_month = 9;
        let actual_start_year = NaiveDate::from_ymd_opt(1924, start_month, start_day).unwrap();
        let actual_end_year = NaiveDate::from_ymd_opt(1925, end_month, end_day).unwrap();
        let actual_date_range = DateRange(actual_start_year, actual_end_year);
        let mut surveys: Vec<Survey> = Vec::new();
        let mut survey;
        for day in actual_date_range {
            survey = Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: day,
                date_recording: day,
                value: DataRecording::Recording(3),
            });
            surveys.push(survey);
        }
        let actual_observable_range: ObservableRange = surveys.into();
        let mut actual_water_years =
            WaterYear::water_years_from_observable_range(&actual_observable_range);
        for water_year in &mut actual_water_years {
            water_year.normalize_calendar_years();
        }
        // make expected
        let dt: DateTime<Local> = Local::now();
        let first_year = dt.naive_local().date().year() - 1;
        let last_year = first_year + 1;
        let first_date = NaiveDate::from_ymd_opt(first_year, start_month, start_day).unwrap();
        let last_date = NaiveDate::from_ymd_opt(last_year, end_month, end_day).unwrap();
        let expected_date_range = DateRange(first_date, last_date);
        surveys = Vec::new();
        for day in expected_date_range {
            survey = Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: day,
                date_recording: day,
                value: DataRecording::Recording(3),
            });
            surveys.push(survey);
        }
        let expected_observable_range: ObservableRange = surveys.into();
        let mut expected_water_years =
            WaterYear::water_years_from_observable_range(&expected_observable_range);
        for water_year in &mut expected_water_years {
            water_year.normalize_calendar_years();
        }
        let it = actual_water_years.iter().zip(expected_water_years.iter());
        for (actual_water_year, expected_water_year) in it {
            let surveys_it = actual_water_year.0.iter().zip(expected_water_year.0.iter());
            for (actual_survey, expected_survey) in surveys_it {
                assert_eq!(
                    actual_survey.get_tap().station_id,
                    expected_survey.get_tap().station_id
                );
                assert_eq!(
                    actual_survey.date_observation(),
                    expected_survey.date_observation()
                );
                assert_eq!(
                    actual_survey.get_tap().value,
                    expected_survey.get_tap().value
                );
            }
        }
    }

    #[test]
    fn test_sort_by_wettest_years() {
        let d1 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();

        let wy_low = WaterYear(vec![Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: d1,
            date_recording: d1,
            value: DataRecording::Recording(100),
        })]);
        let wy_mid = WaterYear(vec![Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: d2,
            date_recording: d2,
            value: DataRecording::Recording(500),
        })]);
        let wy_high = WaterYear(vec![Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: d3,
            date_recording: d3,
            value: DataRecording::Recording(1000),
        })]);

        let mut water_years = vec![wy_low.clone(), wy_high.clone(), wy_mid.clone()];
        water_years.sort_by_wettest_years();

        // Should be sorted descending by max value: 1000, 500, 100
        assert_eq!(water_years[0], wy_high);
        assert_eq!(water_years[1], wy_mid);
        assert_eq!(water_years[2], wy_low);
    }

    #[test]
    fn test_water_year_statistics_driest_wettest() {
        let stats = vec![
            WaterYearStatistics {
                year: 2020,
                date_lowest: NaiveDate::from_ymd_opt(2020, 9, 1).unwrap(),
                date_highest: NaiveDate::from_ymd_opt(2021, 3, 1).unwrap(),
                highest_value: 50000.0,
                lowest_value: 10000.0,
            },
            WaterYearStatistics {
                year: 2021,
                date_lowest: NaiveDate::from_ymd_opt(2021, 9, 1).unwrap(),
                date_highest: NaiveDate::from_ymd_opt(2022, 3, 1).unwrap(),
                highest_value: 80000.0,
                lowest_value: 5000.0,
            },
            WaterYearStatistics {
                year: 2022,
                date_lowest: NaiveDate::from_ymd_opt(2022, 9, 1).unwrap(),
                date_highest: NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                highest_value: 60000.0,
                lowest_value: 20000.0,
            },
        ];

        // 2021 has the lowest minimum (5000) => driest
        assert!(!stats[0].is_driest_in(&stats));
        assert!(stats[1].is_driest_in(&stats));
        assert!(!stats[2].is_driest_in(&stats));

        // 2021 has the highest maximum (80000) => wettest
        assert!(!stats[0].is_wettest_in(&stats));
        assert!(stats[1].is_wettest_in(&stats));
        assert!(!stats[2].is_wettest_in(&stats));
    }
}
