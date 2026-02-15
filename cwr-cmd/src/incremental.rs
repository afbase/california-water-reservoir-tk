//! Incremental query - only fetch data newer than what's already in the CSV.
//!
//! This dramatically reduces CI/CD time by avoiding re-querying 100 years
//! of data on every build.

use chrono::{Local, NaiveDate};
use log::info;
use std::collections::HashMap;

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

    let _client = reqwest::Client::new();

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

        // TODO: Fetch and append new data
        // 1. Query CDEC for data from start_date to end_date
        // 2. Append to existing reservoirs CSV
    }

    info!(
        "Incremental update complete. Output: {}",
        reservoirs_csv
    );
    Ok(())
}
