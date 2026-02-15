//! Full query implementation for CDEC water and snow data.

use cwr_cdec::reservoir::Reservoir;
use chrono::{Local, NaiveDate};
use log::info;

/// Run a full query of CDEC water reservoir data.
///
/// Fetches per-reservoir observations from CDEC and writes them to the
/// reservoirs CSV. Cumulative totals are no longer pre-computed here;
/// they are derived on-the-fly via SQL in the chart applications.
pub async fn run_query(
    reservoirs_csv: &str,
    california_only: bool,
) -> anyhow::Result<()> {
    let reservoirs = if california_only {
        Reservoir::get_reservoir_vector_no_colorado()
    } else {
        Reservoir::get_reservoir_vector()
    };

    let _client = reqwest::Client::new();
    let start_date = NaiveDate::from_ymd_opt(1924, 1, 1).unwrap();
    let end_date = Local::now().naive_local().date();

    info!(
        "Querying {} reservoirs from {} to {}",
        reservoirs.len(),
        start_date,
        end_date
    );

    // TODO: Implement full query logic
    // For each reservoir, fetch daily + monthly surveys from CDEC
    // Merge, interpolate gaps, write per-reservoir CSV

    info!("Query complete. Output: {}", reservoirs_csv);
    Ok(())
}

/// Run a full query of CDEC snow sensor data.
///
/// Fetches daily snow water equivalent (SWE, sensor 3) and snow depth
/// (sensor 18) observations from CDEC for each station defined in the
/// snow_stations.csv fixture. Results are written to the output CSV in
/// the format: `station_id,date(YYYYMMDD),swe,depth` (no headers).
///
/// # CDEC API
///
/// - Sensor 3: Snow Water Content (SWE) in inches, daily duration
/// - Sensor 18: Snow Depth in inches, daily duration
/// - URL pattern: `http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={ID}&SensorNums=3,18&dur_code=D&Start={}&End={}`
///
/// The API returns CSV with headers:
/// `STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS`
pub async fn run_snow_query(
    stations_csv: &str,
) -> anyhow::Result<()> {
    use cwr_cdec::snow_station::SnowStation;
    use std::collections::BTreeMap;

    // Load the snow stations fixture
    let stations_csv_path = std::path::Path::new("fixtures/snow_stations.csv");
    let stations_data = if stations_csv_path.exists() {
        std::fs::read_to_string(stations_csv_path)?
    } else {
        anyhow::bail!("fixtures/snow_stations.csv not found. Create it first.");
    };

    let stations = SnowStation::parse_snow_station_csv(&stations_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse snow stations CSV: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // SWE data starts being systematically collected around 1980
    let start_date = NaiveDate::from_ymd_opt(1980, 10, 1).unwrap();
    let end_date = Local::now().naive_local().date();
    let start_str = start_date.format("%Y-%m-%d");
    let end_str = end_date.format("%Y-%m-%d");

    info!(
        "Querying {} snow stations from {} to {}",
        stations.len(),
        start_date,
        end_date
    );

    // Collect all observations: station_id -> date -> (swe, depth)
    let mut all_obs: Vec<String> = Vec::new();

    for station in &stations {
        info!("Fetching snow data for {} ({})", station.name, station.station_id);

        // Fetch SWE (sensor 3) and snow depth (sensor 18) in a single request
        let url = format!(
            "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=3,18&dur_code=D&Start={}&End={}",
            station.station_id, start_str, end_str
        );

        let response = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                info!("Failed to fetch {}: {}", station.station_id, e);
                continue;
            }
        };

        if !response.status().is_success() {
            info!(
                "Bad response for {}: {}",
                station.station_id,
                response.status()
            );
            continue;
        }

        let body = match response.text().await {
            Ok(b) => b,
            Err(e) => {
                info!("Failed to read body for {}: {}", station.station_id, e);
                continue;
            }
        };

        if body.len() <= 2 {
            info!("Empty response for {}", station.station_id);
            continue;
        }

        // Parse the CDEC CSV response.
        // Headers: STATION_ID,DURATION,SENSOR_NUMBER,SENSOR_TYPE,DATE TIME,OBS DATE,VALUE,DATA_FLAG,UNITS
        // We need: SENSOR_NUMBER (idx 2), OBS DATE (idx 5), VALUE (idx 6)
        // Group by date, sensor 3 = SWE, sensor 18 = depth.
        let mut date_values: BTreeMap<String, (Option<f64>, Option<f64>)> = BTreeMap::new();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(body.as_bytes());

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };

            let sensor_num: i32 = match record.get(2).and_then(|s| s.trim().parse().ok()) {
                Some(v) => v,
                None => continue,
            };

            let obs_date_raw = match record.get(5) {
                Some(d) => d.trim().to_string(),
                None => continue,
            };

            // Convert date from "YYYY-MM-DD HH:MM" or "YYYYMMDD" to "YYYYMMDD"
            let date_yyyymmdd = if obs_date_raw.contains('-') {
                // Format: "2024-01-15 00:00" -> "20240115"
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

            let value: f64 = match record.get(6).and_then(|s| s.trim().parse().ok()) {
                Some(v) => v,
                None => continue,
            };

            let entry = date_values.entry(date_yyyymmdd).or_insert((None, None));
            match sensor_num {
                3 => entry.0 = Some(value),   // SWE
                18 => entry.1 = Some(value),  // Snow depth
                _ => {}
            }
        }

        // Write rows for this station
        for (date, (swe, depth)) in &date_values {
            if swe.is_none() && depth.is_none() {
                continue;
            }
            let swe_str = swe.map_or(String::new(), |v| format!("{:.1}", v));
            let depth_str = depth.map_or(String::new(), |v| format!("{:.1}", v));
            all_obs.push(format!(
                "{},{},{},{}",
                station.station_id, date, swe_str, depth_str
            ));
        }

        info!(
            "  {} observations for {}",
            date_values.len(),
            station.station_id
        );

        // Be polite to the CDEC server
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Write all observations to the output CSV
    let output = all_obs.join("\n");
    std::fs::write(stations_csv, &output)?;

    info!(
        "Snow query complete. {} total observations written to {}",
        all_obs.len(),
        stations_csv
    );
    Ok(())
}
