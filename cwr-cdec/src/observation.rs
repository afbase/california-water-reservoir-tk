use chrono::naive::NaiveDate;
use csv::{ReaderBuilder, StringRecord};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::HashMap,
    hash::Hash,
};

/// Date format used for CDEC CSV responses: "YYYYMMDD HHMM"
pub const DATE_FORMAT: &str = "%Y%m%d %H%M";

/// Date format used for CDEC API query parameters: "YYYY-MM-DD"
pub const YEAR_FORMAT: &str = "%Y-%m-%d";

/// Expected number of columns in a CDEC CSV row.
pub const CSV_ROW_LENGTH: usize = 9;

/// Errors that can occur when fetching or parsing observations.
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum ObservationError {
    HttpRequestError,
    HttpResponseParseError,
    ObservationCollectionError,
}

/// The duration/frequency of a measurement: daily or monthly.
#[derive(Debug, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum Duration {
    Daily,
    Monthly,
}

/// Represents a data recording value from CDEC.
/// - `Brt` / `Art`: special status codes (Below Rating Table / Above Rating Table)
/// - `Dash`: missing or unavailable data (represented as "---" in CDEC CSV)
/// - `Recording(u32)`: an actual measurement in acre-feet
#[derive(Debug, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum DataRecording {
    Brt,
    Art,
    Dash,
    Recording(u32),
}

/// A single observation from a CDEC station.
#[derive(Debug, Clone)]
pub struct Observation {
    pub station_id: String,
    pub date_observation: NaiveDate,
    pub date_recording: NaiveDate,
    pub value: DataRecording,
    pub duration: Duration,
}

impl Observation {
    /// Convert a vector of StringRecords (from CDEC CSV) into Observations.
    pub fn records_to_observations(vec_records: Vec<StringRecord>) -> Vec<Observation> {
        vec_records
            .iter()
            .map(|x| {
                let y = x.clone();
                y.try_into()
            })
            .collect::<Result<Vec<Observation>, _>>()
            .unwrap()
    }

    /// Parse a CDEC CSV response body string into Observations.
    pub fn request_to_observations(request_body: String) -> Result<Vec<Observation>, ObservationError> {
        let string_records = Observation::request_to_string_records(request_body);
        let result = string_records
            .unwrap()
            .iter()
            .map(|x| {
                let y = x.clone();
                y.try_into()
            })
            .collect::<Result<Vec<Observation>, _>>();
        if let Ok(records) = result {
            Ok(records)
        } else {
            Err(ObservationError::ObservationCollectionError)
        }
    }

    /// Parse a CDEC CSV response body string into raw StringRecords.
    pub fn request_to_string_records(
        request_body: String,
    ) -> Result<Vec<StringRecord>, ObservationError> {
        let records = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(request_body.as_bytes())
            .records()
            .map(|x| x.expect("failed record parse"))
            .collect::<Vec<StringRecord>>();
        Ok(records)
    }

    /// Group a vector of observations by station_id.
    pub fn vector_to_hashmap(
        vec_observations: Vec<Observation>,
    ) -> HashMap<String, Vec<Observation>> {
        let mut result: HashMap<String, Vec<Observation>> = HashMap::new();
        for obs in vec_observations {
            result
                .entry(obs.station_id.clone())
                .or_default()
                .push(obs);
        }
        result
    }
}

impl TryFrom<StringRecord> for Observation {
    type Error = ();

    fn try_from(value: StringRecord) -> Result<Self, Self::Error> {
        if value.len() != CSV_ROW_LENGTH {
            return Err(());
        }
        let duration = match value.get(1).unwrap() {
            "D" => Ok(Duration::Daily),
            "M" => Ok(Duration::Monthly),
            _ => Err(()),
        };
        let date_recording_value = NaiveDate::parse_from_str(value.get(4).unwrap(), DATE_FORMAT);
        let date_observation_value = NaiveDate::parse_from_str(value.get(5).unwrap(), DATE_FORMAT);
        let data_value: Result<DataRecording, ()> = match value.get(6).unwrap() {
            "BRT" => Ok(DataRecording::Brt),
            "ART" => Ok(DataRecording::Art),
            "---" => Ok(DataRecording::Dash),
            s => match s.parse::<u32>() {
                Err(_p) => Ok(DataRecording::Recording(0u32)),
                Ok(u) => Ok(DataRecording::Recording(u)),
            },
        };
        if let Ok(duration) = duration {
            return Ok(Observation {
                station_id: value.get(0).unwrap().to_string(),
                date_recording: date_recording_value.unwrap(),
                date_observation: date_observation_value.unwrap(),
                value: data_value.unwrap(),
                duration,
            });
        }
        Err(())
    }
}

impl Hash for Observation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.station_id.hash(state);
        self.date_observation.hash(state);
        self.date_recording.hash(state);
        self.value.hash(state);
        self.duration.hash(state);
    }
}

impl Ord for Observation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date_observation.cmp(&other.date_observation)
    }
}

impl Eq for Observation {}

impl PartialEq for Observation {
    fn eq(&self, other: &Self) -> bool {
        self.date_observation == other.date_observation && self.station_id == other.station_id
    }
}

impl PartialOrd for Observation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use super::DataRecording;
    use crate::observation::Observation;

    // https://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations=VIL&SensorNums=15&dur_code=D&Start=2022-02-15&End=2022-02-28
    const STR_RESULT: &str = r#"STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS
VIL,D,15,STORAGE,20220215 0000,20220215 0000,9593, ,AF
VIL,D,15,STORAGE,20220216 0000,20220216 0000,9589, ,AF
VIL,D,15,STORAGE,20220217 0000,20220217 0000,9589, ,AF
VIL,D,15,STORAGE,20220218 0000,20220218 0000,9585, ,AF
VIL,D,15,STORAGE,20220219 0000,20220219 0000,9585, ,AF
VIL,D,15,STORAGE,20220220 0000,20220220 0000,9585, ,AF
VIL,D,15,STORAGE,20220221 0000,20220221 0000,9581, ,AF
VIL,D,15,STORAGE,20220222 0000,20220222 0000,9593, ,AF
VIL,D,15,STORAGE,20220223 0000,20220223 0000,9601, ,AF
VIL,D,15,STORAGE,20220224 0000,20220224 0000,9601, ,AF
VIL,D,15,STORAGE,20220225 0000,20220225 0000,9601, ,AF
VIL,D,15,STORAGE,20220226 0000,20220226 0000,9597, ,AF
VIL,D,15,STORAGE,20220227 0000,20220227 0000,9597, ,AF
VIL,D,15,STORAGE,20220228 0000,20220228 0000,9597, ,AF
"#;

    #[test]
    fn test_request_to_observations() {
        let string_result = String::from(STR_RESULT);
        let observations = Observation::request_to_observations(string_result).unwrap();
        assert_eq!(observations[0].value, DataRecording::Recording(9593));
    }
}
