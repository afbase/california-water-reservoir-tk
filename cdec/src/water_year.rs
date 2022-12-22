use crate::observable::ObservableRange;
use crate::survey::Survey;
use chrono::{Datelike, NaiveDate};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::HashMap;
/// California’s water year runs from October 1 to September 30 and is the official 12-month timeframe used by water managers to compile and compare hydrologic records.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterYear(pub Vec<Survey>);

pub struct WaterYearStatistics {
    pub year: i32,
    pub date_lowest: NaiveDate,
    pub date_highest: NaiveDate,
    pub highest_value: f64,
    pub lowest_value: f64,
}

impl WaterYear {
    pub fn calendar_year_change(&mut self) -> f64 {
        let _ = &self.0.sort();
        let first_day = self.0.first().unwrap();
        let last_day = self.0.last().unwrap();
        (last_day.get_value() - first_day.get_value()).round()
    }
    pub fn water_years_from_observable_range(water_observations: &ObservableRange) -> Vec<Self> {
        let min_year = water_observations.start_date.year() - 1;
        let max_year = water_observations.end_date.year();
        let mut hm: HashMap<i32, WaterYear> = HashMap::new();
        for year in min_year..=max_year {
            let start_of_year = NaiveDate::from_ymd_opt(year, 10, 1).unwrap();
            let end_of_year = NaiveDate::from_ymd_opt(year + 1, 9, 30).unwrap();
            let water_calendar_year_of_observations = water_observations
                .observations
                .iter()
                .filter_map(|survey| {
                    let tap = survey.get_tap();
                    let obs_date = tap.date_observation;
                    if start_of_year <= obs_date && obs_date <= end_of_year {
                        Some(survey.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if !water_calendar_year_of_observations.is_empty() {
                hm.insert(year, WaterYear(water_calendar_year_of_observations));
            }
        }
        hm.into_values().collect::<Vec<_>>()
    }
}

impl From<WaterYear> for WaterYearStatistics {
    fn from(value: WaterYear) -> Self {
        // surveys should be sorted by date
        let mut surveys = value.0;
        let year = {
            let survey_clone = surveys[0].clone();
            let tap = survey_clone.get_tap();
            let date_observation = tap.date_observation;
            let date_observation_year = date_observation.year();
            // if date precedes water calendar year, then it is year minus 1
            let start_of_year = NaiveDate::from_ymd_opt(date_observation_year, 10, 1).unwrap();
            if date_observation < start_of_year {
                date_observation_year - 1
            } else {
                date_observation_year
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

/// let mut floats = [5f64, 4.0, 1.0, 3.0, 2.0];
/// floats.sort_by(|a, b| a.partial_cmp(b).unwrap());
/// assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0]);
fn sort_by_values_ascending(surveys: &mut [Survey]) {
    surveys.sort_by(|survey_a, survey_b| {
        let a = survey_a.get_value();
        let b = survey_b.get_value();
        a.partial_cmp(&b).unwrap()
    });
}

impl PartialOrd for WaterYearStatistics {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.lowest_value.partial_cmp(&other.lowest_value)
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
    use chrono::NaiveDate;

    use super::WaterYear;
    use crate::observable::MonthDatum;
    use crate::observable::ObservableRange;
    use crate::observation::DataRecording;
    use crate::survey::{Survey, Tap};
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
                date_recording: d.clone(),
                value: DataRecording::Recording(3),
            }),
            Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d_1,
                date_recording: d_1.clone(),
                value: DataRecording::Recording(3),
            }),
        ];
        let obs = ObservableRange {
            observations: surveys,
            start_date: d_1.clone(),
            end_date: d.clone(),
            month_datum: b,
        };

        let actual = WaterYear::water_years_from_observable_range(&obs);
        let expected = vec![
            WaterYear(vec![Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d_1,
                date_recording: d_1.clone(),
                value: DataRecording::Recording(3),
            })]),
            WaterYear(vec![Survey::Daily(Tap {
                station_id: String::new(),
                date_observation: d,
                date_recording: d.clone(),
                value: DataRecording::Recording(3),
            })]),
        ];
        assert_eq!(actual, expected);
    }
}
