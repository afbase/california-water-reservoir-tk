//! Thin client for CDEC's `CSVDataServlet`, shared by the water and snow queries.
//!
//! CDEC answers with a CSV whose header is
//! `STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS`.

use chrono::NaiveDate;
use log::{info, warn};
use std::time::Duration;

const ENDPOINT: &str = "https://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet";

/// Reservoir storage in acre-feet.
pub const SENSOR_STORAGE: u32 = 15;
/// Snow water content (SWE) in inches.
pub const SENSOR_SNOW_WATER: u32 = 3;
/// Snow depth in inches.
pub const SENSOR_SNOW_DEPTH: u32 = 18;

/// Pause between stations, to be polite to the CDEC server.
pub const POLITE_DELAY: Duration = Duration::from_millis(500);

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// One numeric daily observation from a CDEC response.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub sensor: u32,
    /// Observation date as `YYYYMMDD`.
    pub date: String,
    pub value: f64,
}

pub fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder().timeout(REQUEST_TIMEOUT).build()
}

/// Fetch the daily observations for `station` over `start..=end`.
///
/// Retries with exponential backoff. Returns `None` (after logging) when every
/// attempt fails or CDEC returns an empty body, so one flaky station never
/// aborts a whole run.
pub async fn fetch_daily(
    client: &reqwest::Client,
    station: &str,
    sensors: &[u32],
    start: NaiveDate,
    end: NaiveDate,
) -> Option<Vec<Reading>> {
    let sensor_list = sensors
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let url = format!(
        "{ENDPOINT}?Stations={station}&SensorNums={sensor_list}&dur_code=D&Start={}&End={}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d"),
    );

    let mut backoff = INITIAL_BACKOFF;
    for attempt in 1..=MAX_ATTEMPTS {
        match fetch_body(client, &url).await {
            Ok(body) => return Some(parse_readings(&body)),
            Err(reason) => {
                warn!("Attempt {attempt}/{MAX_ATTEMPTS} for {station} failed: {reason}")
            }
        }
        if attempt < MAX_ATTEMPTS {
            info!("Retrying {station} in {backoff:?}");
            tokio::time::sleep(backoff).await;
            backoff *= 2;
        }
    }

    warn!("All attempts failed for {station}");
    None
}

async fn fetch_body(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    if body.trim().is_empty() {
        return Err("empty response".to_string());
    }
    Ok(body)
}

/// Normalize a CDEC date field to `YYYYMMDD`.
///
/// CDEC has used both `YYYYMMDD HHMM` and `YYYY-MM-DD HH:MM`; either way the
/// first eight digits are the date. Returns `None` unless they form a real date.
pub fn normalize_date(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).take(8).collect();
    NaiveDate::parse_from_str(&digits, "%Y%m%d").ok()?;
    Some(digits)
}

/// Parse a CDEC CSV body, keeping only rows with a valid date and a numeric value
/// (CDEC reports missing values as `---`).
pub fn parse_readings(body: &str) -> Vec<Reading> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(body.as_bytes());

    rdr.records()
        .flatten()
        .filter_map(|record| {
            Some(Reading {
                sensor: record.get(2)?.trim().parse().ok()?,
                date: normalize_date(record.get(5)?)?,
                value: record.get(6)?.trim().parse().ok()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "\
STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS
ORO,D,15,STORAGE,20261113 0000,20261114 0000,1433855, ,AF
ORO,D,15,STORAGE,20261114 0000,20261115 0000,1433431, ,AF
ORO,D,15,STORAGE,20261116 0000,20261116 0000,---, ,AF
";

    #[test]
    fn parses_numeric_rows_and_skips_missing_values() {
        let readings = parse_readings(BODY);
        assert_eq!(
            readings,
            vec![
                Reading {
                    sensor: 15,
                    date: "20261114".into(),
                    value: 1_433_855.0
                },
                Reading {
                    sensor: 15,
                    date: "20261115".into(),
                    value: 1_433_431.0
                },
            ]
        );
    }

    #[test]
    fn normalizes_both_cdec_date_spellings() {
        assert_eq!(normalize_date("20240115 0000").as_deref(), Some("20240115"));
        assert_eq!(
            normalize_date("2024-01-15 00:00").as_deref(),
            Some("20240115")
        );
        assert_eq!(normalize_date("20240115").as_deref(), Some("20240115"));
        assert_eq!(normalize_date("2024"), None);
        assert_eq!(normalize_date("20241345"), None);
        assert_eq!(normalize_date(""), None);
    }

    #[test]
    fn header_only_body_yields_nothing() {
        assert!(parse_readings(BODY.lines().next().unwrap()).is_empty());
    }
}
