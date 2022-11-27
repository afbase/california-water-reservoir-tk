use crate::{
    observable::Observable,
    observation::{DataRecording, Duration, Observation},
};
use chrono::NaiveDate;
use csv::StringRecord;
use easy_cast::Cast;
use std::{cmp::Ordering, convert::From};

// Survey and Tap are not great names but out of a need to have a name
// Survey originates from a google search for synonomym of Observation
// Tap is a reference to Tap water and Tap in electrical engineering
// to sample a signal. There is also this thing that i really want to
// be a meme for these types of situations:
// https://www.youtube.com/watch?v=xaILTs-_1z4

#[derive(Debug, Clone, PartialEq)]
pub struct Tap {
    pub station_id: String,
    pub date_observation: NaiveDate,
    pub date_recording: NaiveDate,
    pub value: DataRecording,
}

#[derive(Debug, Clone)]
pub enum Survey {
    Daily(Tap),
    Monthly(Tap),
}

pub struct CompressedStringRecord(pub StringRecord);

pub trait VectorCompressedStringRecord {
    fn records_to_surveys(self) -> Vec<Survey>;
}

impl VectorCompressedStringRecord for Vec<CompressedStringRecord> {
    fn records_to_surveys(self) -> Vec<Survey> {
        self.into_iter()
            .map(|compressed_string_record| {
                let survey: Survey = compressed_string_record.into();
                survey
            })
            .collect::<Vec<Survey>>()
    }
}

impl Observable for Survey {
    fn into_survey(self) -> Survey {
        self
    }
}

impl Observable for Observation {
    fn into_survey(self) -> Survey {
        let k: Survey = self.into();
        k
    }
}

pub trait Interpolate {
    fn interpolate_pair(self) -> Option<Vec<Survey>>;
}

impl From<Observation> for Survey {
    fn from(obs: Observation) -> Survey {
        match obs.duration {
            Duration::Daily => Survey::Daily(Tap {
                station_id: obs.station_id,
                date_observation: obs.date_observation,
                date_recording: obs.date_recording,
                value: obs.value,
            }),
            Duration::Monthly => Survey::Monthly(Tap {
                station_id: obs.station_id,
                date_observation: obs.date_observation,
                date_recording: obs.date_recording,
                value: obs.value,
            }),
        }
    }
}

impl std::convert::From<Survey> for Observation {
    fn from(survey: Survey) -> Observation {
        match survey {
            Survey::Daily(t) => Observation {
                station_id: t.station_id,
                date_observation: t.date_observation,
                date_recording: t.date_recording,
                value: t.value,
                duration: Duration::Daily,
            },
            Survey::Monthly(t) => Observation {
                station_id: t.station_id,
                date_observation: t.date_observation,
                date_recording: t.date_recording,
                value: t.value,
                duration: Duration::Monthly,
            },
        }
    }
}

// VIL,D,20220218,9585
impl std::convert::From<Survey> for CompressedStringRecord {
    fn from(value: Survey) -> Self {
        let tap = value.get_tap();
        let station = tap.station_id.as_str();
        let duration = match value {
            Survey::Daily(_) => "D",
            Survey::Monthly(_) => "M",
        };
        let date_observation_tmp = tap.date_observation.format("%Y%m%d").to_string();
        let date_observation = date_observation_tmp.as_str();
        let binding;
        let recording = match tap.value {
            DataRecording::Art => "ART",
            DataRecording::Brt => "BRT",
            DataRecording::Dash => "---",
            DataRecording::Recording(v) => {
                binding = v.to_string();
                binding.as_str()
            }
        };
        let a = StringRecord::from(vec![station, duration, date_observation, recording]);
        CompressedStringRecord(a)
    }
}

// VIL,D,20220218,9585
impl From<CompressedStringRecord> for Survey {
    fn from(value: CompressedStringRecord) -> Self {
        let station = value.0.get(0).unwrap();
        let duration = value.0.get(1).unwrap();
        let date_observation =
            NaiveDate::parse_from_str("%Y%m%d", value.0.get(3).unwrap()).unwrap();
        let date_recording = date_observation;
        let recording = match value.0.get(3).unwrap() {
            "ART" => DataRecording::Art,
            "BRT" => DataRecording::Brt,
            "---" => DataRecording::Dash,
            s => {
                let a = s.parse::<u32>().unwrap();
                DataRecording::Recording(a)
            }
        };
        let tap = Tap {
            station_id: String::from(station),
            date_observation,
            date_recording,
            value: recording,
        };
        match duration {
            "D" => Survey::Daily(tap),
            "M" => Survey::Monthly(tap),
            &_ => panic!("Hey is this an M or D???"),
        }
    }
}

impl std::convert::TryFrom<StringRecord> for Survey {
    type Error = ();
    fn try_from(value: StringRecord) -> Result<Self, Self::Error> {
        let observation_result: Result<Observation, _> = value.try_into();
        match observation_result {
            Ok(obs) => {
                let r: Survey = obs.into();
                Ok(r)
            }
            _ => Err(()),
        }
    }
}

impl std::convert::TryFrom<Survey> for StringRecord {
    type Error = ();
    fn try_from(value: Survey) -> Result<Self, Self::Error> {
        // VIL,D,15,STORAGE,20220218 0000,20220218 0000,9585, ,AF
        let tap = value.get_tap();
        let station_id_tmp = tap.station_id.clone();
        let station = station_id_tmp.as_str();
        let sensor = "15";
        let unit = "AF";
        let other = " ";
        let storage = "STORAGE";
        let duration = match value {
            Survey::Daily(_) => "D",
            Survey::Monthly(_) => "M",
        };
        let tap = value.get_tap();
        let date_observation_tmp = tap.date_observation.format("%Y%m%d");
        let date_observation_tmp_string = date_observation_tmp.to_string();
        let formated_date_observation =
            format!("{} {}", date_observation_tmp_string.as_str(), "0000");
        let date_observation = formated_date_observation.as_str();
        let binding;
        let recording = match tap.value {
            DataRecording::Art => "ART",
            DataRecording::Brt => "BRT",
            DataRecording::Dash => "---",
            DataRecording::Recording(v) => {
                binding = v.to_string();
                binding.as_str()
            }
        };
        let record = vec![
            station,
            duration,
            sensor,
            storage,
            date_observation,
            date_observation,
            recording,
            other,
            unit,
        ];
        Ok(StringRecord::from(record))
    }
}

impl Tap {
    fn value_as_f64(self) -> f64 {
        match self.value {
            DataRecording::Recording(a) => {
                let k: f64 = a.cast();
                k
            }
            _ => 0.0f64,
        }
    }
}

impl Interpolate for (Survey, Survey) {
    fn interpolate_pair(self) -> Option<Vec<Survey>> {
        let start = self.0.clone();
        let end = self.1;
        let start_obs: Observation = start.clone().into();
        let end_obs: Observation = end.clone().into();
        // pair of surveys must have recordings
        if !start.has_recording() || !end.has_recording() {
            return None;
        }
        let days = (end_obs.date_observation - start_obs.date_observation).num_days();
        let capacity = (days + 1) as usize;
        let mut interpolated_surveys: Vec<Survey> = Vec::with_capacity(capacity);
        interpolated_surveys.push(start.clone());
        // compute linear interpolation things
        let y_n = end.get_value();
        let y_0 = start.get_value();
        let x_n: f64 = days.cast();
        let x_0: f64 = 0.0;
        let slope = (y_n - y_0) / (x_n - x_0);
        for idx in 1..=days {
            let fdx: f64 = idx.cast();
            let y_i = (slope * (fdx - x_0) + y_0).round();
            let value = y_i as u32;
            let recording = DataRecording::Recording(value);
            let date = start_obs.date_observation + chrono::Duration::days(idx);
            let survey = Survey::Daily(Tap {
                station_id: start_obs.clone().station_id,
                date_observation: date,
                date_recording: date,
                value: recording,
            });
            interpolated_surveys.push(survey);
        }
        Some(interpolated_surveys)
    }
}

impl Survey {
    pub fn get_tap(&self) -> &Tap {
        match self {
            Survey::Daily(t) => t,
            Survey::Monthly(t) => t,
        }
    }

    pub fn get_value(&self) -> f64 {
        match self {
            Survey::Daily(t) => t.clone().value_as_f64(),
            Survey::Monthly(t) => t.clone().value_as_f64(),
        }
    }

    pub fn has_recording(&self) -> bool {
        match self {
            Survey::Daily(t) => {
                matches!(t.value, DataRecording::Recording(_))
            }
            Survey::Monthly(t) => {
                matches!(t.value, DataRecording::Recording(_))
            }
        }
    }
}

impl Ord for Survey {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_date = self.get_tap().date_observation;
        let other_date = other.get_tap().date_observation;
        self_date.cmp(&other_date)
    }
}

impl Eq for Survey {}

impl PartialEq for Survey {
    fn eq(&self, other: &Self) -> bool {
        let tap = self.get_tap();
        let top = other.get_tap();
        tap.date_observation == top.date_observation && tap.station_id == top.station_id
    }
}

impl PartialOrd for Survey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use super::{Interpolate, Survey, Tap};
    use crate::observation::{DataRecording, Duration, Observation};
    use chrono::NaiveDate;
    use csv::StringRecord;

    #[test]
    fn convert_survey_to_string_record() {
        //VIL,D,15,STORAGE,20220218 0000,20220218 0000,9585, ,AF
        let vector_victor = vec![
            "VIL",
            "D",
            "15",
            "STORAGE",
            "20220218 0000",
            "20220218 0000",
            "9585",
            " ",
            "AF",
        ];
        let expected = StringRecord::from(vector_victor);
        let survey = Survey::Daily(Tap {
            station_id: String::from("VIL"),
            date_observation: NaiveDate::from_ymd_opt(2022, 2, 18).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 2, 18).unwrap(),
            value: DataRecording::Recording(9585),
        });
        let actual: StringRecord = survey.try_into().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_string_record_to_survey() {
        //VIL,D,15,STORAGE,20220218 0000,20220218 0000,9585, ,AF
        let vector_victor = vec![
            "VIL",
            "D",
            "15",
            "STORAGE",
            "20220218 0000",
            "20220218 0000",
            "9585",
            " ",
            "AF",
        ];
        let record = StringRecord::from(vector_victor);
        let expected = Survey::Daily(Tap {
            station_id: String::from("VIL"),
            date_observation: NaiveDate::from_ymd_opt(2022, 2, 18).unwrap(),
            date_recording: NaiveDate::from_ymd_opt(2022, 2, 18).unwrap(),
            value: DataRecording::Recording(9585),
        });
        let actual: Survey = record.try_into().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_survey_to_observation() {
        let station_id = String::new();
        let date_observation = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let date_recording = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let value = DataRecording::Recording(921);
        let survey_0 = Survey::Daily(Tap {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
        });
        let survey_1 = Survey::Monthly(Tap {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
        });
        let observation_0 = Observation {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
            duration: Duration::Daily,
        };
        let observation_1 = Observation {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
            duration: Duration::Monthly,
        };
        let actual_0: Observation = survey_0.into();
        let actual_1: Observation = survey_1.into();
        assert_eq!(actual_0, observation_0);
        assert_eq!(actual_1, observation_1);
    }

    #[test]
    fn convert_observation_to_survey() {
        let station_id = String::new();
        let date_observation = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let date_recording = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let value = DataRecording::Recording(921);
        let survey_0 = Survey::Daily(Tap {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
        });
        let survey_1 = Survey::Monthly(Tap {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
        });
        let observation_0 = Observation {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
            duration: Duration::Daily,
        };
        let observation_1 = Observation {
            station_id: station_id.clone(),
            date_observation: date_observation.clone(),
            date_recording: date_recording.clone(),
            value: value.clone(),
            duration: Duration::Monthly,
        };
        let actual_0: Survey = observation_0.into();
        let actual_1: Survey = observation_1.into();
        assert_eq!(actual_0, survey_0);
        assert_eq!(actual_1, survey_1);
    }

    #[test]
    fn interpolate_a_pair() {
        let station_id = String::new();
        let date_0 = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let date_1 = NaiveDate::from_ymd_opt(2022, 11, 17).unwrap();
        let value_0 = DataRecording::Recording(7);
        let value_1 = DataRecording::Recording(16);
        let start = Survey::Daily(Tap {
            station_id,
            date_observation: date_0,
            date_recording: date_0,
            value: value_0,
        });
        let end = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: date_1,
            date_recording: date_1,
            value: value_1,
        });
        let expected = vec![
            DataRecording::Recording(7),
            DataRecording::Recording(9),
            DataRecording::Recording(11),
            DataRecording::Recording(12),
            DataRecording::Recording(14),
            DataRecording::Recording(16),
        ];
        let actual_surveys = (start, end).interpolate_pair().unwrap();
        let actual: Vec<DataRecording> = actual_surveys
            .into_iter()
            .map(|x| {
                let k: Observation = x.into();
                k.value
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn dont_interpolate_a_pair_0() {
        let station_id = String::new();
        let date_0 = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let date_1 = NaiveDate::from_ymd_opt(2022, 11, 17).unwrap();
        let value_0 = DataRecording::Art;
        let value_1 = DataRecording::Recording(4);
        let start = Survey::Daily(Tap {
            station_id,
            date_observation: date_0,
            date_recording: date_0,
            value: value_0,
        });
        let end = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: date_1,
            date_recording: date_1,
            value: value_1,
        });
        let actual_surveys = (start, end).interpolate_pair();
        assert_eq!(actual_surveys, None);
    }

    #[test]
    fn dont_interpolate_a_pair_1() {
        let station_id = String::new();
        let date_0 = NaiveDate::from_ymd_opt(2022, 11, 12).unwrap();
        let date_1 = NaiveDate::from_ymd_opt(2022, 11, 17).unwrap();
        let value_0 = DataRecording::Recording(7);
        let value_1 = DataRecording::Brt;
        let start = Survey::Daily(Tap {
            station_id,
            date_observation: date_0,
            date_recording: date_0,
            value: value_0,
        });
        let end = Survey::Daily(Tap {
            station_id: String::new(),
            date_observation: date_1,
            date_recording: date_1,
            value: value_1,
        });
        let actual_surveys = (start, end).interpolate_pair();
        assert_eq!(actual_surveys, None);
    }
}
