//! Incremental query - only fetch data newer than what's already in the CSV.
//!
//! This dramatically reduces CI/CD time by avoiding re-querying 100 years
//! of data on every build.

use chrono::{Local, NaiveDate};
use log::{info, warn};
use std::collections::HashMap;
use std::io::Write;

/// Find the most recent date for each station in an existing CSV.
///
/// Returns a HashMap of station_id -> most_recent_date.
fn find_max_dates(csv_path: &str) -> anyhow::Result<HashMap<String, NaiveDate>> {
    let mut max_dates: HashMap<String, NaiveDate> = HashMap::new();

    if !std::path::Path::new(csv_path).exists() {
        return Ok(max_dates);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(csv_path)?;

    for result in rdr.records() {
        let record = result?;
        if let (Some(station_id), Some(date_str)) = (record.get(0), record.get(2)) {
            let station = station_id.trim().to_string();
            if let Ok(date) = NaiveDate::parse_from_str(date_str.trim(), "%Y%m%d") {
                let current_max = max_dates.entry(station).or_insert(date);
                if date > *current_max {
                    *current_max = date;
                }
            }
        }
    }

    Ok(max_dates)
}

/// Run incremental update: only fetch data newer than what's in the existing CSV.
///
/// Cumulative totals are no longer pre-computed here; they are derived
/// on-the-fly via SQL in the chart applications.
pub async fn run_incremental(
    reservoirs_csv: &str,
    california_only: bool,
) -> anyhow::Result<()> {
    let max_dates = find_max_dates(reservoirs_csv)?;
    let end_date = Local::now().naive_local().date();

    let reservoirs = if california_only {
        cwr_cdec::reservoir::Reservoir::get_reservoir_vector_no_colorado()
    } else {
        cwr_cdec::reservoir::Reservoir::get_reservoir_vector()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    for reservoir in &reservoirs {
        let start_date = match max_dates.get(&reservoir.station_id) {
            Some(last_date) => {
                // Start from the day after the last known date
                *last_date + chrono::Duration::days(1)
            }
            None => {
                // No existing data, fetch from beginning
                NaiveDate::from_ymd_opt(1924, 1, 1).unwrap()
            }
        };

        if start_date >= end_date {
            info!("Station {} is up to date", reservoir.station_id);
            continue;
        }

        info!(
            "Fetching {} from {} to {}",
            reservoir.station_id, start_date, end_date
        );

        let start_str = start_date.format("%Y-%m-%d");
        let end_str = end_date.format("%Y-%m-%d");
        let url = format!(
            "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code=D&Start={}&End={}",
            reservoir.station_id, start_str, end_str
        );

        // Retry logic: 3 attempts with exponential backoff
        let max_tries = 3;
        let mut sleep_millis: u64 = 1000;
        let mut body: Option<String> = None;

        for attempt in 1..=max_tries {
            match client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!(
                            "Attempt {}/{}: Bad response for {}: {}",
                            attempt, max_tries, reservoir.station_id, response.status()
                        );
                    } else {
                        match response.text().await {
                            Ok(text) => {
                                if text.len() <= 2 {
                                    warn!(
                                        "Attempt {}/{}: Empty response for {}",
                                        attempt, max_tries, reservoir.station_id
                                    );
                                } else {
                                    body = Some(text);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Attempt {}/{}: Failed to read body for {}: {}",
                                    attempt, max_tries, reservoir.station_id, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Attempt {}/{}: Request failed for {}: {}",
                        attempt, max_tries, reservoir.station_id, e
                    );
                }
            }

            if attempt < max_tries {
                info!(
                    "Sleeping {}ms before retry for {}",
                    sleep_millis, reservoir.station_id
                );
                tokio::time::sleep(std::time::Duration::from_millis(sleep_millis)).await;
                sleep_millis *= 2;
            }
        }

        let body = match body {
            Some(b) => b,
            None => {
                warn!("All attempts failed for {}", reservoir.station_id);
                continue;
            }
        };

        // Parse the CDEC CSV response
        // Headers: STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(body.as_bytes());

        let mut new_rows: Vec<String> = Vec::new();

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };

            // OBS DATE is at index 5
            let obs_date_raw = match record.get(5) {
                Some(d) => d.trim().to_string(),
                None => continue,
            };

            // Convert date from "YYYY-MM-DD HH:MM" or similar to "YYYYMMDD"
            let date_yyyymmdd = if obs_date_raw.contains('-') {
                obs_date_raw
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .replace('-', "")
            } else {
                obs_date_raw.clone()
            };

            if date_yyyymmdd.len() < 8 {
                continue;
            }

            // VALUE is at index 6, skip non-numeric values
            let value: f64 = match record.get(6).and_then(|s| s.trim().parse().ok()) {
                Some(v) => v,
                None => continue,
            };

            // Format: station_id,D,YYYYMMDD,value
            new_rows.push(format!(
                "{},D,{},{:.2}",
                reservoir.station_id, date_yyyymmdd, value
            ));
        }

        if !new_rows.is_empty() {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(reservoirs_csv)?;

            for row in &new_rows {
                writeln!(file, "{}", row)?;
            }

            info!(
                "Appended {} rows for {}",
                new_rows.len(),
                reservoir.station_id
            );
        } else {
            info!("No new data for {}", reservoir.station_id);
        }

        // Be polite to the CDEC server
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    info!(
        "Incremental update complete. Output: {}",
        reservoirs_csv
    );
    Ok(())
}
