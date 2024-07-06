use crate::{
    compression::{decompress_tar_file_to_csv_string, CUMULATIVE_OBJECT, OBSERVATIONS_OBJECT},
    reservoir::Reservoir,
    survey::{CompressedStringRecord, CumulativeSummedStringRecord},
};
use chrono::naive::NaiveDate;
// use core::result::Result;
use csv::{ReaderBuilder, StringRecord};
use futures::future::join_all;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    hash::Hash,
    str,
};
pub const DATE_FORMAT: &str = "%Y%m%d %H%M";
pub const YEAR_FORMAT: &str = "%Y-%m-%d";
pub const CSV_ROW_LENGTH: usize = 9;

#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum ObservationError {
    HttpRequestError,
    HttpResponseParseError,
    ObservationCollectionError,
}

#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum Duration {
    Daily,
    Monthly,
}
#[derive(Debug, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum DataRecording {
    Brt,
    Art,
    Dash,
    Recording(u32),
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub station_id: String,
    pub date_observation: NaiveDate,
    pub date_recording: NaiveDate,
    pub value: DataRecording,
    pub duration: Duration,
}

impl Observation {
    pub fn get_all_records_v2() -> Vec<CumulativeSummedStringRecord> {
        let bytes_of_csv_string = decompress_tar_file_to_csv_string(CUMULATIVE_OBJECT);
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(bytes_of_csv_string.as_slice())
            .records()
            .map(|x| {
                let a = x.expect("failed record parse");
                CumulativeSummedStringRecord(a)
            })
            .collect::<Vec<_>>()
    }

    pub fn get_all_records_from_bytes(bytes: &[u8]) -> Vec<CompressedStringRecord> {
        let bytes_of_csv_string = decompress_tar_file_to_csv_string(bytes);
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(bytes_of_csv_string.as_slice())
            .records()
            .map(|x| {
                let a = x.expect("failed record parse");
                CompressedStringRecord(a)
            })
            .collect::<Vec<CompressedStringRecord>>()
    }

    pub fn get_all_records() -> Vec<CompressedStringRecord> {
        Self::get_all_records_from_bytes(OBSERVATIONS_OBJECT)
    }

    pub async fn get_all_reservoirs_data_by_dates(
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<BTreeMap<NaiveDate, u32>, ObservationError> {
        let reservoirs = Reservoir::get_reservoir_vector();
        let mut date_water_btree: BTreeMap<NaiveDate, u32> = BTreeMap::new();
        let client = Client::new();
        let all_reservoir_observations = join_all(reservoirs.iter().map(|reservoir| {
            let client_ref = &client;
            let start_date_ref = start_date;
            let end_date_ref = end_date;
            async move {
                Observation::get_observations(
                    client_ref,
                    reservoir.station_id.as_str(),
                    start_date_ref,
                    end_date_ref,
                )
                .await
            }
        }))
        .await;
        for reservoir_observations in all_reservoir_observations {
            let observations = reservoir_observations.unwrap();
            for observation in observations {
                let k = {
                    if let DataRecording::Recording(v) = observation.value {
                        v
                    } else {
                        0u32
                    }
                };
                date_water_btree
                    .entry(observation.date_observation)
                    .and_modify(|e| *e += k)
                    .or_insert(k);
            }
        }
        Ok(date_water_btree)
    }

    pub async fn get_observations(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<Vec<Observation>, ObservationError> {
        let mut observations: Vec<Observation> = Vec::new();
        let request_body_daily =
            Observation::http_request_body(client, reservoir_id, start_date, end_date, "D").await;
        let _request_body_monthly =
            Observation::http_request_body(client, reservoir_id, start_date, end_date, "M").await;
        if let Ok(body) = request_body_daily {
            if let Ok(mut obs) = Observation::request_to_observations(body) {
                observations.append(obs.as_mut());
            } else {
                return Err(ObservationError::HttpResponseParseError);
            }
        } else {
            return Err(ObservationError::HttpRequestError);
        }
        Ok(observations)
    }

    pub async fn get_string_records(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<Vec<StringRecord>, ObservationError> {
        let request_body =
            Observation::http_request_body(client, reservoir_id, start_date, end_date, "D").await;
        if let Ok(body) = request_body {
            if let Ok(records) = Observation::request_to_string_records(body) {
                Ok(records)
            } else {
                Err(ObservationError::HttpResponseParseError)
            }
        } else {
            Err(ObservationError::HttpRequestError)
        }
    }
    async fn http_request_body(
        client: &Client,
        reservoir_id: &str,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        rate: &str,
    ) -> Result<String, reqwest::Error> {
        let url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", reservoir_id, rate, start_date.format(YEAR_FORMAT), end_date.format(YEAR_FORMAT));
        let response = client.get(url).send().await?;
        response.text().await
    }
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
    fn request_to_observations(request_body: String) -> Result<Vec<Observation>, ObservationError> {
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
    fn request_to_string_records(
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
    pub fn vector_to_hashmap(
        vec_observations: Vec<Observation>,
    ) -> HashMap<String, Vec<Observation>> {
        let mut result = HashMap::new();
        // for (station_id, group) in &vec_observations.iter().group_by(|obs| &obs.station_id) {
        //     result.insert(station_id.clone(), group.cloned().collect());
        // }
        for (station_id, group) in &vec_observations.iter().chunk_by(|obs| &obs.station_id) {
            result.insert(station_id.clone(), group.cloned().collect());
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
            // _ => Err(()),
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
    use chrono::NaiveDate;
    use reqwest::Client;
    use std::assert_ne;

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
        // ID , DAM , LAKE          , STREAM        , CAPACITY (AF), YEAR FILL
        // VIL, Vail, Vail Reservoir, Temecula Creek, 51000,
        // https://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations=VIL&SensorNums=15&dur_code=D&Start=2022-02-15&End=2022-02-28
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
        // ID , DAM , LAKE          , STREAM        , CAPACITY (AF), YEAR FILL
        // VIL, Vail, Vail Reservoir, Temecula Creek, 51000,
        // https://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations=VIL&SensorNums=15&dur_code=D&Start=2022-02-15&End=2022-02-28
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
        // ID , DAM , LAKE          , STREAM        , CAPACITY (AF), YEAR FILL
        // VIL, Vail, Vail Reservoir, Temecula Creek, 51000,
        // https://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations=VIL&SensorNums=15&dur_code=D&Start=2022-02-15&End=2022-02-28
        let string_result = String::from(STR_RESULT);
        let observations = Observation::request_to_observations(string_result).unwrap();
        assert_eq!(observations[0].value, DataRecording::Recording(9593));
    }
}
