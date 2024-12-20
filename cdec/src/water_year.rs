use crate::observation::Observation;
use crate::reservoir::Reservoir;
use crate::survey::{CompressedStringRecord, VectorCompressedStringRecord};
use crate::{
    normalized_naive_date::NormalizedNaiveDate, observable::ObservableRange, survey::Survey,
};
use chrono::{DateTime, Datelike, Local, NaiveDate};
use easy_cast::Cast;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::HashMap;
pub const NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT: usize = 20;

/// California’s water year runs from October 1 to September 30 and is the official 12-month timeframe used by water managers to compile and compare hydrologic records.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaterYear(pub Vec<Survey>);

#[derive(Debug, Serialize, Deserialize)]
pub struct WaterYearStatistics {
    pub year: i32,
    pub date_lowest: NaiveDate,
    pub date_highest: NaiveDate,
    pub highest_value: f64,
    pub lowest_value: f64,
}
pub trait NormalizeCalendarYear {
    fn normalize_calendar_years(&mut self);
}
#[derive(Debug)]
pub enum WaterYearErrors {
    InsufficientWaterYears,
}

pub trait NormalizeWaterYears {
    fn normalize_dates(&mut self);
    fn get_largest_acrefeet_over_n_years(&self, len: usize) -> Result<f64, WaterYearErrors>;
    fn get_complete_normalized_water_years(&self) -> Self;
    fn sort_by_lowest_recorded_years(&mut self);
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
            // California’s water year runs from October 1 to September 30 and is the official 12-month timeframe
            for survey in &mut water_year.0 {
                let tap = survey.tap();
                tap.date_recording = tap.date_observation;
                // California’s water year runs from October 1 to September 30 and is the official 12-month timeframe
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
            let mut y_max: f64 = ((largest_acrefeet
                .iter()
                .max_by(|a, b| a.total_cmp(b))
                .unwrap()
                + 0.0) as i64)
                .cast();
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
            // California’s water year runs from October 1 to September 30 and is the official 12-month timeframe
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
            !matches!((month, day), (2, 29)) // Note the ! to invert the condition
        });
    }
}

impl WaterYear {
    pub fn init_reservoirs_from_lzma_without_interpolation() -> HashMap<String, Vec<Self>> {
        let records: Vec<CompressedStringRecord> = Observation::get_all_records();
        let mut observations = records.records_to_surveys();
        let mut hash_map: HashMap<String, Vec<Self>> = HashMap::new();
        let reservoirs = Reservoir::get_reservoir_vector();
        for reservoir in reservoirs {
            let station_id = reservoir.station_id;
            let (mut surveys, remaining): (Vec<_>, Vec<_>) =
                observations.into_iter().partition(|survey| {
                    let tap = survey.get_tap();
                    let tap_station_id = tap.station_id.clone();
                    tap_station_id == station_id
                });
            surveys.sort();
            observations = remaining;
            if surveys.is_empty() {
                continue;
            }
            let surveys_len = surveys.len();
            let start_date = surveys[0].get_tap().date_observation;
            let end_date = surveys[surveys_len - 1].get_tap().date_observation;
            let min_year = start_date.year() - 1;
            let max_year = end_date.year();
            let mut water_years: Vec<WaterYear> = Vec::new();
            // build vecs of water years
            for year in min_year..=max_year {
                let start_of_year = NaiveDate::from_ymd_opt(year, 10, 1).unwrap();
                let end_of_year = NaiveDate::from_ymd_opt(year + 1, 9, 30).unwrap();
                let (water_year_of_surveys, remaining): (Vec<_>, Vec<_>) =
                    surveys.into_iter().partition(|survey| {
                        let tap = survey.get_tap();
                        let obs_date = tap.date_observation;
                        start_of_year <= obs_date && obs_date <= end_of_year
                    });
                surveys = remaining;
                water_years.push(WaterYear(water_year_of_surveys));
            }
            if water_years.len() >= NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT {
                hash_map.insert(station_id, water_years);
            }
        }
        hash_map
    }

    pub fn calendar_year_from_normalized_water_year(&self) -> (NaiveDate, NaiveDate) {
        // in a normalized water year - the date_recording has the original date_observation
        // let mut surveys = self.0.clone();
        let first_survey = self.0.first().unwrap();
        let last_survey = self.0.last().unwrap();
        let first_date = first_survey.get_tap().date_recording;
        let last_date = last_survey.get_tap().date_recording;
        (first_date, last_date)
    }
    pub fn calendar_year_change(&mut self) -> f64 {
        let _ = &self.0.sort();
        let first_day = self.0.first().unwrap();
        let last_day = self.0.last().unwrap();
        (last_day.get_value() - first_day.get_value()).round()
    }
    // pub fn water_years_from_observable_range(water_observations: &ObservableRange) -> Vec<Self> {
    //     let min_year = water_observations.start_date.year() - 1;
    //     let max_year = water_observations.end_date.year();
    //     let mut hm: HashMap<i32, WaterYear> = HashMap::new();
    //     for year in min_year..=max_year {
    //         let start_of_year = NaiveDate::from_ymd_opt(year, 10, 1).unwrap();
    //         let end_of_year = NaiveDate::from_ymd_opt(year + 1, 9, 30).unwrap();
    //         let water_calendar_year_of_observations = water_observations
    //             .observations
    //             .iter()
    //             .filter_map(|survey| {
    //                 let tap = survey.get_tap();
    //                 let obs_date = tap.date_observation;
    //                 if start_of_year <= obs_date && obs_date <= end_of_year {
    //                     Some(survey.clone())
    //                 } else {
    //                     None
    //                 }
    //             })
    //             .collect::<Vec<_>>();
    //         if !water_calendar_year_of_observations.is_empty() {
    //             hm.insert(year, WaterYear(water_calendar_year_of_observations));
    //         }
    //     }
    //     hm.into_values().collect::<Vec<_>>()
    // }

    // src/water_year.rs
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

                    // evaluate this boolean logic
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
        // surveys should be sorted by date
        let surveys = value.clone();
        surveys.into()
    }
}

// let mut floats = [5f64, 4.0, 1.0, 3.0, 2.0];
// floats.sort_by(|a, b| a.partial_cmp(b).unwrap());
// assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0]);
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
    use super::WaterYear;
    use crate::date_range::DateRange;
    use crate::observable::MonthDatum;
    use crate::observable::ObservableRange;
    use crate::observation::DataRecording;
    use crate::survey::{Survey, Tap};
    use crate::water_year::NormalizeCalendarYear;
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
        // for three years 1924 to 1926:
        // make basic surveys
        // convert to water years and normalize
        // expect date observations years to be
        // this year and last year
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
        // for three years 1924 to 1926:
        // make basic surveys
        // convert to water years and normalize
        // expect date observations years to be
        // this year and last year
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
        // 2024 was a leap year and breaks the test
        for water_year in &mut expected_water_years {
            water_year.normalize_calendar_years();
        }
        // Note that expected_water_years may have a record that looks like
        // Daily(Tap { station_id: "", date_observation: 2024-09-30, date_recording: 2024-09-30, value: Recording(3) })
        // while  the actual is
        // Daily(Tap { station_id: "", date_observation: 2023-10-01, date_recording: 1924-10-01, value: Recording(3) })
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
        // assert_eq!(actual_water_years, expected_water_years);
    }
}
