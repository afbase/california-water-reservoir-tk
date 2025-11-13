/// Observation data structures and fetching logic for CDEC reservoir observations
use crate::{
    compression::{
        decompress_tar_file_to_csv_string, CUMULATIVE_OBJECT, CUMULATIVE_OBJECT_V2,
        OBSERVATIONS_OBJECT,
    },
    error::{CdecError, Result},
    reservoir::Reservoir,
    survey::{CompressedStringRecord, CumulativeSummedStringRecord},
};
use chrono::naive::NaiveDate;
use csv::{ReaderBuilder, StringRecord};
use futures::future::join_all;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

/// Date format used by CDEC API for observations
pub const DATE_FORMAT: &str = "%Y%m%d %H%M";

/// Date format for API requests
pub const YEAR_FORMAT: &str = "%Y-%m-%d";

/// Expected number of CSV columns in observation records
pub const CSV_ROW_LENGTH: usize = 9;

/// Deprecated error type - use `CdecError` instead
#[deprecated(since = "1.2.0", note = "Use CdecError instead")]
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum ObservationError {
    HttpRequestError,
    HttpResponseParseError,
    ObservationCollectionError,
}

/// Duration/frequency of observations
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum Duration {
    /// Daily observations
    Daily,
    /// Monthly observations
    Monthly,
}

/// Represents a recorded data value or special status
#[derive(Debug, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum DataRecording {
    /// Below Reporting Threshold
    Brt,
    /// Above Reporting Threshold
    Art,
    /// Data not available (shown as "---")
    Dash,
    /// Actual recorded value in acre-feet
    Recording(u32),
}

/// A single reservoir observation from CDEC
#[derive(Debug, Clone)]
pub struct Observation {
    /// Station identifier (e.g., "SHA" for Shasta)
    pub station_id: String,
    /// Date the observation was made
    pub date_observation: NaiveDate,
    /// Date the observation was recorded/reported
    pub date_recording: NaiveDate,
    /// The recorded value or status
    pub value: DataRecording,
    /// Frequency of the observation
    pub duration: Duration,
}

impl Observation {
    /// Returns all cumulative statewide observations (version 2 - updated)
    ///
    /// Decompresses and parses the embedded cumulative_v2.tar.lzma file
    pub fn get_all_records_v2() -> Result<Vec<CumulativeSummedStringRecord>> {
        Self::get_cumulative_records(CUMULATIVE_OBJECT)
    }

    /// Returns all cumulative statewide observations (version 3 - latest)
    ///
    /// Decompresses and parses the embedded cumulative_v2.tar.lzma file
    pub fn get_all_records_v3() -> Result<Vec<CumulativeSummedStringRecord>> {
        Self::get_cumulative_records(CUMULATIVE_OBJECT_V2)
    }

    /// Helper to parse cumulative records from compressed data
    fn get_cumulative_records(bytes: &[u8]) -> Result<Vec<CumulativeSummedStringRecord>> {
        let csv_bytes = decompress_tar_file_to_csv_string(bytes)?;
        ReaderBuilder::new()
            .has_headers(false)
            .from_reader(csv_bytes.as_slice())
            .records()
            .map(|r| r.map(CumulativeSummedStringRecord).map_err(CdecError::from))
            .collect()
    }

    /// Returns all per-reservoir observations from compressed data
    ///
    /// # Arguments
    ///
    /// * `bytes` - Compressed tar.lzma archive containing CSV data
    pub fn get_all_records_from_bytes(bytes: &[u8]) -> Result<Vec<CompressedStringRecord>> {
        let csv_bytes = decompress_tar_file_to_csv_string(bytes)?;
        ReaderBuilder::new()
            .has_headers(false)
            .from_reader(csv_bytes.as_slice())
            .records()
            .map(|r| r.map(CompressedStringRecord).map_err(CdecError::from))
            .collect()
    }

    /// Returns all per-reservoir observations from embedded data
    pub fn get_all_records() -> Result<Vec<CompressedStringRecord>> {
        Self::get_all_records_from_bytes(OBSERVATIONS_OBJECT)
    }

    /// Fetches and aggregates all reservoir data for a date range
    ///
    /// Makes concurrent HTTP requests to CDEC for all reservoirs and aggregates
    /// the daily observations by date.
    ///
    /// # Arguments
    ///
    /// * `start_date` - Start of date range
    /// * `end_date` - End of date range (inclusive)
    ///
    /// # Returns
    ///
    /// Map of dates to total acre-feet across all reservoirs
    pub async fn get_all_reservoirs_data_by_dates(
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<BTreeMap<NaiveDate, u32>> {
        let reservoirs = Reservoir::get_reservoir_vector()?;
        let client = Client::new();

        // Fetch all reservoirs concurrently
        let results = join_all(
            reservoirs
                .iter()
                .map(|reservoir| Self::get_observations(&client, &reservoir.station_id, start_date, end_date)),
        )
        .await;

        // Aggregate results
        let mut date_water_btree: BTreeMap<NaiveDate, u32> = BTreeMap::new();
        for result in results {
            let observations = result?;
            for observation in observations {
                if let DataRecording::Recording(v) = observation.value {
                    date_water_btree
                        .entry(observation.date_observation)
                        .and_modify(|e| *e += v)
                        .or_insert(v);
                }
            }
        }

        Ok(date_water_btree)
    }

    /// Fetches observations for a single reservoir
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client (reuse for multiple requests)
    /// * `reservoir_id` - Station ID (e.g., "SHA")
    /// * `start_date` - Start date
    /// * `end_date` - End date (inclusive)
    pub async fn get_observations(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<Vec<Observation>> {
        let body = Self::http_request_body(client, reservoir_id, start_date, end_date, "D").await?;
        Self::request_to_observations(body)
    }

    /// Fetches raw CSV records for a reservoir
    ///
    /// Returns unparsed StringRecord objects for further processing
    pub async fn get_string_records(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<Vec<StringRecord>> {
        let body = Self::http_request_body(client, reservoir_id, start_date, end_date, "D").await?;
        Self::request_to_string_records(body)
    }

    /// Makes HTTP request to CDEC API
    async fn http_request_body(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        rate: &str,
    ) -> Result<String> {
        let url = format!(
            "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}",
            reservoir_id,
            rate,
            start_date.format(YEAR_FORMAT),
            end_date.format(YEAR_FORMAT)
        );
        let response = client.get(url).send().await?;
        Ok(response.text().await?)
    }

    /// Converts CSV records to Observation objects
    pub fn records_to_observations(vec_records: Vec<StringRecord>) -> Result<Vec<Observation>> {
        vec_records
            .into_iter()
            .map(|record| record.try_into())
            .collect()
    }

    /// Parses HTTP response body into Observations
    fn request_to_observations(request_body: String) -> Result<Vec<Observation>> {
        let records = Self::request_to_string_records(request_body)?;
        records
            .into_iter()
            .map(|record| record.try_into())
            .collect()
    }

    /// Parses HTTP response body into StringRecords
    fn request_to_string_records(request_body: String) -> Result<Vec<StringRecord>> {
        ReaderBuilder::new()
            .has_headers(true)
            .from_reader(request_body.as_bytes())
            .records()
            .map(|r| r.map_err(CdecError::from))
            .collect()
    }

    /// Groups observations by station ID
    pub fn vector_to_hashmap(
        vec_observations: Vec<Observation>,
    ) -> HashMap<String, Vec<Observation>> {
        let mut result = HashMap::new();
        for (station_id, group) in &vec_observations.iter().chunk_by(|obs| &obs.station_id) {
            result.insert(station_id.clone(), group.cloned().collect());
        }
        result
    }
}

impl TryFrom<StringRecord> for Observation {
    type Error = CdecError;

    fn try_from(value: StringRecord) -> Result<Self> {
        if value.len() != CSV_ROW_LENGTH {
            return Err(CdecError::InvalidFormat(format!(
                "Expected {} columns, found {}",
                CSV_ROW_LENGTH,
                value.len()
            )));
        }

        let duration = match value.get(1).ok_or_else(|| {
            CdecError::InvalidFormat("Missing duration field".to_string())
        })? {
            "D" => Duration::Daily,
            "M" => Duration::Monthly,
            other => {
                return Err(CdecError::InvalidFormat(format!(
                    "Invalid duration: {}",
                    other
                )))
            }
        };

        let date_recording = NaiveDate::parse_from_str(
            value
                .get(4)
                .ok_or_else(|| CdecError::InvalidFormat("Missing recording date".to_string()))?,
            DATE_FORMAT,
        )
        .map_err(|e| CdecError::DateParse(e.to_string()))?;

        let date_observation = NaiveDate::parse_from_str(
            value
                .get(5)
                .ok_or_else(|| CdecError::InvalidFormat("Missing observation date".to_string()))?,
            DATE_FORMAT,
        )
        .map_err(|e| CdecError::DateParse(e.to_string()))?;

        let value_str = value
            .get(6)
            .ok_or_else(|| CdecError::InvalidFormat("Missing value field".to_string()))?;

        let data_value = match value_str {
            "BRT" => DataRecording::Brt,
            "ART" => DataRecording::Art,
            "---" => DataRecording::Dash,
            s => DataRecording::Recording(s.parse().unwrap_or(0)),
        };

        Ok(Observation {
            station_id: value
                .get(0)
                .ok_or_else(|| CdecError::InvalidFormat("Missing station_id".to_string()))?
                .to_string(),
            date_recording,
            date_observation,
            value: data_value,
            duration,
        })
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
    use super::*;

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

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_get_all_reservoirs_data_by_dates() {
        let start_date = NaiveDate::from_ymd_opt(2022, 2, 15).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2022, 2, 28).unwrap();
        let obs = Observation::get_all_reservoirs_data_by_dates(&start_date, &end_date)
            .await
            .unwrap();
        for (_, val) in obs.iter() {
            assert_ne!(*val, 0u32);
        }
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_http_request_body() {
        let reservoir_id = "VIL";
        let start_date = NaiveDate::from_ymd_opt(2022, 2, 15).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2022, 2, 28).unwrap();
        let client = Client::new();
        let observations =
            Observation::http_request_body(&client, reservoir_id, &start_date, &end_date, "D")
                .await;
        assert_eq!(
            observations.unwrap().as_str().replace("\r\n", "\n"),
            STR_RESULT
        );
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_get_observations() {
        let reservoir_id = "VIL";
        let start_date = NaiveDate::from_ymd_opt(2022, 2, 15).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2022, 2, 28).unwrap();
        let client = Client::new();
        let observations =
            Observation::get_observations(&client, reservoir_id, &start_date, &end_date).await;
        assert_eq!(observations.unwrap().len(), 14);
    }

    #[test]
    fn test_request_to_observations() {
        let string_result = String::from(STR_RESULT);
        let observations = Observation::request_to_observations(string_result).unwrap();
        assert_eq!(observations[0].value, DataRecording::Recording(9593));
    }
}
