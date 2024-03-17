use crate::{
    observation::DataRecording,
    reservoir::Reservoir,
    survey::CompressedStringRecord,
    survey::{Interpolate, Survey, Tap},
};
use chrono::{Datelike, NaiveDate, TimeDelta};
use csv::{StringRecord, Writer};
use easy_cast::Cast;
use log::info;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;

const LAKE_MEAD: &str = "MEA";
const LAKE_POWELL: &str = "PWL";
// to group survey and observable types
pub trait Observable: Clone {
    fn into_survey(self) -> Survey;
}

#[derive(Debug, Clone)]
pub struct MonthDatum(u32, u32);

impl MonthDatum {
    pub fn from(date: NaiveDate) -> MonthDatum {
        let year = date.year() as u32;
        let month = date.month();
        MonthDatum(year, month)
    }
    pub fn new(year: u32, month: u32) -> MonthDatum {
        MonthDatum(year, month)
    }

    pub fn year(&self) -> u32 {
        self.0
    }

    pub fn month(&self) -> u32 {
        self.1
    }
}
pub trait CompressedSurveyBuilder {
    fn new(start_date: NaiveDate, end_date: NaiveDate) -> Self;
    fn update(&mut self, item: impl Observable);
    fn retain(&mut self);
    fn finalize(&mut self);
    fn pad_end(&mut self);
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservableRange {
    pub observations: Vec<Survey>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub month_datum: HashSet<MonthDatum>,
}

pub trait ObservableRangeRunner {
    fn run_csv(&self) -> String;
    fn run_csv_v2(&self) -> String;
}

impl ObservableRangeRunner for Vec<ObservableRange> {
    fn run_csv(&self) -> String {
        info!("ran all surveys!");
        let mut observations_downloaded = self.clone();
        let option_of_compressed_string_records = observations_downloaded
            .iter_mut()
            .map(|surveys| {
                surveys.observations.sort();
                let earliest_date = {
                    if let Some(survey_first) = surveys.observations.first() {
                        let tap = survey_first.get_tap();
                        tap.date_observation
                    } else {
                        return None;
                    }
                };
                let last_survey = surveys.observations.last().unwrap();
                let last_tap = last_survey.get_tap();
                let most_recent_date = last_tap.date_observation;
                let month_datum: HashSet<MonthDatum> = HashSet::new();
                let mut observable_range = ObservableRange {
                    observations: surveys.observations.clone(),
                    start_date: earliest_date,
                    end_date: most_recent_date,
                    month_datum,
                };
                observable_range.retain();
                let records: Vec<CompressedStringRecord> = observable_range
                    .observations
                    .into_iter()
                    .map(|survey| {
                        let record: CompressedStringRecord = survey.into();
                        record
                    })
                    .collect::<Vec<CompressedStringRecord>>();
                Some(records)
            })
            .collect::<Vec<_>>();
        //compressedstringrecords from hear on out
        let mut writer = Writer::from_writer(vec![]);
        let flattened_records = option_of_compressed_string_records.into_iter().flatten();
        for reservoir_records in flattened_records {
            for reservoir_record in reservoir_records {
                if writer
                    .write_byte_record(reservoir_record.0.as_byte_record())
                    .is_err()
                {
                    panic!("Error: writing record failed");
                }
            }
        }
        String::from_utf8(writer.into_inner().unwrap()).unwrap()
    }
    fn run_csv_v2(&self) -> String {
        let reservoirs: HashMap<String, Reservoir> = Reservoir::get_reservoir_vector()
            .iter()
            .map(|res| {
                let station = res.station_id.clone();
                let res_copy = res.clone();
                (station, res_copy)
            })
            .collect();
        info!("Surveyed Reseroirs: {}", self.len());
        info!("Observations Downloaded");
        let mut observations_downloaded = self.clone();
        observations_downloaded.interpolate_reservoir_observations();
        info!("Interpolated Reseroirs: {}", self.len());
        info!("Observations Interpolated and Sorted");
        let mut california_water_level_observations: BTreeMap<NaiveDate, f64> = BTreeMap::new();
        for observable_range in observations_downloaded {
            for survey in observable_range.observations {
                let tap = survey.get_tap();
                let date_observation = tap.date_observation;
                let station_id = tap.station_id.clone();
                let station_id_str = station_id.as_str();
                let mut recording = survey.get_value();
                let reservoir = reservoirs.get(&station_id).unwrap();
                let reservoir_capacity: f64 = reservoir.capacity.cast();
                let observed_value = {
                    // Need to scale Lake Powell and Mead to 27% of recorded data
                    // https://www.ppic.org/wp-content/uploads/californias-water-the-colorado-river-november-2018.pdf
                    if station_id_str == LAKE_MEAD || station_id_str == LAKE_POWELL {
                        recording *= 0.27;
                        recording = recording.round();
                    }
                    recording.min(reservoir_capacity)
                };
                california_water_level_observations
                    .entry(date_observation)
                    .and_modify(|e| *e += observed_value)
                    .or_insert(observed_value);
            }
        }
        info!("Observations Accumulated");
        let mut writer = Writer::from_writer(vec![]);
        for (date, observation) in california_water_level_observations {
            let date_string = date.format("%Y%m%d").to_string();
            let date_str = date_string.as_str();
            let observation_string = observation.to_string();
            let observation_str = observation_string.as_str();
            let string_record = StringRecord::from(vec![date_str, observation_str]);
            if writer
                .write_byte_record(string_record.as_byte_record())
                .is_err()
            {
                panic!("Error: writing record failed");
            }
        }
        String::from_utf8(writer.into_inner().unwrap()).unwrap()
    }
}

impl From<Vec<Survey>> for ObservableRange {
    fn from(value: Vec<Survey>) -> Self {
        let mut working_vector = value.clone();
        working_vector.sort();
        let earliest_tap = working_vector[0].get_tap();
        let vec_len = working_vector.len();
        let most_recent_tap = working_vector[vec_len - 1].get_tap();
        let earliest_date = earliest_tap.date_observation;
        let most_recent_date = most_recent_tap.date_observation;
        let mut hash_set = HashSet::new();
        for survey in working_vector {
            match survey {
                Survey::Daily(tap) => {
                    let month = tap.date_observation.month();
                    let year = tap.date_observation.year() as u32;
                    let _ = hash_set.insert(MonthDatum::new(year, month));
                }
                Survey::Monthly(tap) => {
                    let month = tap.date_observation.month();
                    let year = tap.date_observation.year() as u32;
                    let _ = hash_set.insert(MonthDatum::new(year, month));
                }
            }
        }
        ObservableRange {
            observations: value,
            start_date: earliest_date,
            end_date: most_recent_date,
            month_datum: hash_set,
        }
    }
}

impl CompressedSurveyBuilder for ObservableRange {
    fn new(start_date: NaiveDate, end_date: NaiveDate) -> Self {
        if end_date < start_date {
            panic!("CompressedSurveyBuilder<> Error: End Date precedes Start Date");
        }
        let capacity = ((end_date - start_date).num_days() + 1) as usize;
        let v: Vec<Survey> = Vec::with_capacity(capacity);
        let m: HashSet<MonthDatum> = HashSet::new();
        ObservableRange {
            observations: v,
            start_date,
            end_date,
            month_datum: m,
        }
    }

    fn update(&mut self, item: impl Observable) {
        let item_clone = item.clone();
        // let observations_clone = self.observations.clone();
        let t = item.into_survey();
        match t {
            Survey::Daily(tap) => {
                let has_record =
                    <std::vec::Vec<Survey> as std::convert::AsRef<Vec<Survey>>>::as_ref(
                        &self.observations,
                    )
                    .iter()
                    .any(|obs| {
                        let survey = obs.clone().into_survey();
                        let survey_tap = survey.get_tap();
                        tap.date_observation == survey_tap.date_observation
                    });
                if !has_record {
                    self.observations.push(item_clone.into_survey());
                }
                let month_datum_test = MonthDatum::from(tap.date_observation);
                let _result = self.month_datum.insert(month_datum_test);
            }
            Survey::Monthly(tap) => {
                // needs to see if there is a daily observation for that month,
                // if there is, then do not insert
                // otherwise insert
                let month_datum_test = MonthDatum::from(tap.date_observation);
                if self.month_datum.insert(month_datum_test) {
                    // if there are no observations for that month
                    // then insert the observation
                    self.observations.push(item_clone.into_survey());
                }
            }
        }
    }

    fn retain(&mut self) {
        // remove anything that isn't a recording
        self.observations.retain(|observable| {
            let survey = observable.clone().into_survey();
            let tap = survey.get_tap();
            matches!(tap.value, DataRecording::Recording(_))
        });
    }

    fn finalize(&mut self) {
        // this smooths and pads observations
        self.observations.sort();
        let observation_clone = self.observations.clone();
        let surveys_slice = observation_clone.as_slice();
        let windows = surveys_slice.windows(2);
        for survey_window in windows {
            let survey_0 = survey_window[0].clone();
            let survey_1 = survey_window[1].clone();
            let survey_tuple = (survey_0, survey_1);
            let interpolation = survey_tuple.interpolate_pair();
            if let Some(vec_survey) = interpolation {
                for item in vec_survey {
                    self.update(item);
                }
            }
        }
    }

    fn pad_end(&mut self) {
        let mut tmp_date;
        let mut tmp_survey;
        self.observations.sort();
        let observations_clone = self.observations.clone();
        let most_recent = observations_clone.last().unwrap().get_tap();
        let most_recent_date = most_recent.date_observation;
        if most_recent_date < self.end_date {
            let days = (self.end_date - most_recent_date).num_days();
            for num_of_days in 1..days {
                tmp_date = most_recent_date + TimeDelta::try_days(num_of_days).unwrap();
                tmp_survey = Survey::Daily(Tap {
                    station_id: most_recent.station_id.clone(),
                    date_observation: tmp_date,
                    date_recording: tmp_date,
                    value: most_recent.value,
                });
                self.update(tmp_survey);
            }
        }
    }
}

impl Hash for MonthDatum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}

impl PartialEq for MonthDatum {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl PartialOrd for MonthDatum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.1.partial_cmp(&other.1)
    }
}

impl Eq for MonthDatum {}

impl Ord for MonthDatum {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_year = self.0;
        let self_month = self.1;
        let other_year = other.0;
        let other_month = other.1;
        let result;
        if self_year < other_year {
            result = std::cmp::Ordering::Less;
        } else if self_year == other_year && self_month == other_month {
            result = std::cmp::Ordering::Equal;
        } else if self_year == other_year && self_month < other_month {
            result = std::cmp::Ordering::Less;
        } else {
            // the other conditions are:
            // self_year == other_year && self_month > other_month
            // self_year > other_year
            result = std::cmp::Ordering::Greater;
        };
        result
    }
}

pub trait InterpolateObservableRanges {
    fn interpolate_reservoir_observations(&mut self);
}

impl InterpolateObservableRanges for Vec<ObservableRange> {
    fn interpolate_reservoir_observations(&mut self) {
        // at this point, the observable ranges are retained, sorted, and the dates are well bounded
        for reservoir_observable_range in self {
            let capacity = ((reservoir_observable_range.end_date
                - reservoir_observable_range.start_date)
                .num_days()
                + 1) as usize;
            let observation_clone = reservoir_observable_range.observations.clone();
            let mut reservoir_survey_hashset = HashSet::new();
            // interpolate
            let surveys_slice = observation_clone.as_slice();
            let windows = surveys_slice.windows(2);
            for survey_window in windows {
                let survey_0 = survey_window[0].clone();
                let survey_1 = survey_window[1].clone();
                let survey_tuple = (survey_0, survey_1);
                let interpolation: Option<Vec<Survey>> = survey_tuple.interpolate_pair();
                if let Some(vec_survey) = interpolation {
                    for survey_interpolated in vec_survey {
                        let _insert_result = reservoir_survey_hashset.insert(survey_interpolated);
                    }
                }
            }
            let reservoir_hash_set_len = reservoir_survey_hashset.len();
            let delta;
            // pad the end if need be
            if reservoir_hash_set_len < capacity {
                let mut tmp_date;
                let mut tmp_survey;
                delta = capacity - reservoir_hash_set_len;
                let mut hash_set_as_vec = reservoir_survey_hashset.into_iter().collect::<Vec<_>>();
                let most_recent = reservoir_observable_range.observations.last().unwrap();
                let most_recent_tap = most_recent.get_tap();
                let most_recent_date = most_recent_tap.date_observation;
                for i in 0..delta {
                    let num_of_days = i + 1;
                    tmp_date = most_recent_date + TimeDelta::try_days(num_of_days as i64).unwrap();
                    tmp_survey = Survey::Daily(Tap {
                        station_id: most_recent_tap.station_id.clone(),
                        date_observation: tmp_date,
                        date_recording: tmp_date,
                        value: most_recent_tap.value,
                    });
                    hash_set_as_vec.push(tmp_survey);
                }
                hash_set_as_vec.sort();
                reservoir_observable_range.observations = hash_set_as_vec;
            } else {
                reservoir_observable_range.observations =
                    reservoir_survey_hashset.into_iter().collect::<Vec<_>>();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use chrono::NaiveDate;

    use crate::{
        observation::DataRecording,
        survey::{Survey, Tap},
    };

    use super::{InterpolateObservableRanges, MonthDatum, ObservableRange};
    #[test]
    fn interpolate_reservoir_observations_test() {
        let mut observations = Vec::with_capacity(10);
        let a_0 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            value: DataRecording::Recording(1),
        });
        let a_1 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 2).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 2).unwrap(),
            value: DataRecording::Recording(2),
        });
        let a_2 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 3).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 3).unwrap(),
            value: DataRecording::Recording(3),
        });
        let a_3 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 4).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 4).unwrap(),
            value: DataRecording::Recording(4),
        });
        let a_4 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 5).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 5).unwrap(),
            value: DataRecording::Recording(5),
        });
        let a_5 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 6).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 6).unwrap(),
            value: DataRecording::Recording(6),
        });
        let a_6 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 7).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 7).unwrap(),
            value: DataRecording::Recording(6),
        });
        let a_7 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 8).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 8).unwrap(),
            value: DataRecording::Recording(6),
        });
        let a_8 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 9).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 9).unwrap(),
            value: DataRecording::Recording(6),
        });
        let a_9 = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: NaiveDate::from_ymd_opt(2022, 12, 10).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 12, 10).unwrap(),
            value: DataRecording::Recording(6),
        });
        let month_datum_actual = MonthDatum(2022, 12);
        let month_datum_expected = MonthDatum(2022, 12);
        let mut hash_set_actual = HashSet::new();
        hash_set_actual.insert(month_datum_actual);
        let mut hash_set_expected = HashSet::new();
        hash_set_expected.insert(month_datum_expected);
        let expected_observations = vec![
            a_0.clone(),
            a_1,
            a_2,
            a_3,
            a_4,
            a_5.clone(),
            a_6,
            a_7,
            a_8,
            a_9,
        ];
        observations.push(a_0);
        observations.push(a_5);
        let observable_range_actual = ObservableRange {
            observations,
            start_date: NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2022, 12, 10).unwrap(),
            month_datum: hash_set_actual,
        };
        let observable_range_expected = ObservableRange {
            observations: expected_observations,
            start_date: NaiveDate::from_ymd_opt(2022, 12, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2022, 12, 10).unwrap(),
            month_datum: hash_set_expected,
        };
        let mut actual = vec![observable_range_actual];
        actual.interpolate_reservoir_observations();
        let expected = [observable_range_expected];
        assert_eq!(actual[0], expected[0]);
    }
}
