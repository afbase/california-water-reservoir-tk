//! Full (from-scratch) queries of CDEC water and snow data.

use crate::cdec::{self, SENSOR_SNOW_DEPTH, SENSOR_SNOW_WATER};
use anyhow::Context;
use chrono::{Local, NaiveDate};
use cwr_cdec::snow_station::SnowStation;
use log::info;
use std::collections::BTreeMap;

const SNOW_STATIONS_FIXTURE: &str = "fixtures/snow_stations.csv";

/// Run a full query of CDEC water reservoir data.
///
/// Rebuilds `reservoirs_csv` from scratch: any existing file is discarded and
/// every reservoir is fetched from the start of the record. This is the same
/// fetch as [`crate::incremental::run_incremental`] with nothing to resume from.
pub async fn run_query(reservoirs_csv: &str, california_only: bool) -> anyhow::Result<()> {
    std::fs::write(reservoirs_csv, "")
        .with_context(|| format!("failed to create {reservoirs_csv}"))?;
    crate::incremental::run_incremental(reservoirs_csv, california_only, None, 1, 0).await
}

/// Run a full query of CDEC snow sensor data.
///
/// Fetches daily snow water equivalent and snow depth for each station in the
/// `fixtures/snow_stations.csv` fixture and writes
/// `station_id,date(YYYYMMDD),swe,depth` rows (no header) to `stations_csv`.
pub async fn run_snow_query(stations_csv: &str) -> anyhow::Result<()> {
    let stations_data = std::fs::read_to_string(SNOW_STATIONS_FIXTURE)
        .with_context(|| format!("{SNOW_STATIONS_FIXTURE} not found. Create it first."))?;
    let stations = SnowStation::parse_snow_station_csv(&stations_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse snow stations CSV: {e}"))?;

    let client = cdec::client()?;

    // SWE data starts being systematically collected around 1980
    let start_date = NaiveDate::from_ymd_opt(1980, 10, 1).unwrap();
    let end_date = Local::now().date_naive();

    info!(
        "Querying {} snow stations from {start_date} to {end_date}",
        stations.len()
    );

    let mut rows: Vec<String> = Vec::new();

    for station in &stations {
        let id = &station.station_id;
        info!("Fetching snow data for {} ({id})", station.name);

        let sensors = [SENSOR_SNOW_WATER, SENSOR_SNOW_DEPTH];
        let Some(readings) = cdec::fetch_daily(&client, id, &sensors, start_date, end_date).await
        else {
            continue;
        };

        // date -> (swe, depth)
        let mut by_date: BTreeMap<String, (Option<f64>, Option<f64>)> = BTreeMap::new();
        for reading in readings {
            let entry = by_date.entry(reading.date).or_default();
            match reading.sensor {
                SENSOR_SNOW_WATER => entry.0 = Some(reading.value),
                SENSOR_SNOW_DEPTH => entry.1 = Some(reading.value),
                _ => {}
            }
        }

        let fmt = |v: Option<f64>| v.map_or(String::new(), |v| format!("{v:.1}"));
        info!("  {} observations for {id}", by_date.len());
        rows.extend(
            by_date
                .iter()
                .map(|(date, (swe, depth))| format!("{id},{date},{},{}", fmt(*swe), fmt(*depth))),
        );

        tokio::time::sleep(cdec::POLITE_DELAY).await;
    }

    // An empty result means CDEC was unreachable; keep whatever is on disk.
    anyhow::ensure!(
        !rows.is_empty(),
        "no snow observations fetched; leaving {stations_csv} untouched"
    );

    std::fs::write(stations_csv, rows.join("\n"))?;
    info!(
        "Snow query complete. {} total observations written to {stations_csv}",
        rows.len()
    );
    Ok(())
}
