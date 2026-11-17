//! Incremental query - only fetch data newer than what's already in the CSV.
//!
//! This dramatically reduces CI/CD time by avoiding re-querying 100 years
//! of data on every build.

use crate::cdec::{self, SENSOR_STORAGE};
use chrono::{Local, NaiveDate};
use log::info;
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
        .flexible(true)
        .from_path(csv_path)?;

    for result in rdr.records() {
        let record = result?;
        if let (Some(station_id), Some(date_str)) = (record.get(0), record.get(2)) {
            let station = station_id.trim().to_string();
            // Tolerate the `YYYYMMDD HHMM` spelling older runs wrote.
            let date = cdec::normalize_date(date_str)
                .and_then(|d| NaiveDate::parse_from_str(&d, "%Y%m%d").ok());
            if let Some(date) = date {
                let current_max = max_dates.entry(station).or_insert(date);
                if date > *current_max {
                    *current_max = date;
                }
            }
        }
    }

    Ok(max_dates)
}

/// Indices of `0..total` assigned to `shard` under an `index % shards == shard`
/// partition. Mirrors the filter applied in the fetch loop; split out so the
/// partition math can be unit-tested without I/O.
#[cfg(test)]
fn shard_indices(total: usize, shards: usize, shard: usize) -> Vec<usize> {
    (0..total).filter(|i| i % shards == shard).collect()
}

/// Run incremental update: only fetch data newer than what's in the existing CSV.
///
/// `baseline` is the CSV from which per-station max-dates (resume points) are
/// read; when `None` it defaults to `reservoirs_csv`, preserving the original
/// single-file behavior. Newly-fetched rows are always appended to
/// `reservoirs_csv`. Only reservoirs whose position in the vector satisfies
/// `index % shards == shard` are processed, allowing the run to be sharded
/// across parallel jobs.
///
/// Cumulative totals are no longer pre-computed here; they are derived
/// on-the-fly via SQL in the chart applications.
pub async fn run_incremental(
    reservoirs_csv: &str,
    california_only: bool,
    baseline: Option<&str>,
    shards: usize,
    shard: usize,
) -> anyhow::Result<()> {
    if shards == 0 {
        anyhow::bail!("--shards must be >= 1 (got 0)");
    }
    if shard >= shards {
        anyhow::bail!("--shard ({shard}) must be < --shards ({shards})");
    }

    let baseline_path = baseline.unwrap_or(reservoirs_csv);
    let max_dates = find_max_dates(baseline_path)?;
    let end_date = Local::now().date_naive();

    let reservoirs = if california_only {
        cwr_cdec::reservoir::Reservoir::get_reservoir_vector_no_colorado()
    } else {
        cwr_cdec::reservoir::Reservoir::get_reservoir_vector()
    };

    let client = cdec::client()?;

    for (index, reservoir) in reservoirs.iter().enumerate() {
        // Only process reservoirs belonging to this shard.
        if index % shards != shard {
            continue;
        }
        let station = &reservoir.station_id;

        // Resume the day after the last known observation, or from the start
        // of the record when the station has no data yet.
        let start_date = match max_dates.get(station) {
            Some(last_date) => *last_date + chrono::Duration::days(1),
            None => NaiveDate::from_ymd_opt(1924, 1, 1).unwrap(),
        };

        if start_date >= end_date {
            info!("Station {station} is up to date");
            continue;
        }

        info!("Fetching {station} from {start_date} to {end_date}");

        let Some(readings) =
            cdec::fetch_daily(&client, station, &[SENSOR_STORAGE], start_date, end_date).await
        else {
            continue;
        };

        if readings.is_empty() {
            info!("No new data for {station}");
        } else {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(reservoirs_csv)?;

            // Format: station_id,D,YYYYMMDD,value
            for reading in &readings {
                writeln!(file, "{station},D,{},{:.2}", reading.date, reading.value)?;
            }

            info!("Appended {} rows for {station}", readings.len());
        }

        tokio::time::sleep(cdec::POLITE_DELAY).await;
    }

    info!("Incremental update complete. Output: {}", reservoirs_csv);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn max_dates_reads_both_date_spellings() {
        let path = std::env::temp_dir().join(format!("cwr-max-dates-{}.csv", std::process::id()));
        std::fs::write(
            &path,
            "ORO,D,20240101,10.00\nORO,D,20240105 0000,11.00\nSHA,M,19991231,5.00\n",
        )
        .unwrap();
        let max_dates = find_max_dates(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(
            max_dates["ORO"],
            NaiveDate::from_ymd_opt(2024, 1, 5).unwrap()
        );
        assert_eq!(
            max_dates["SHA"],
            NaiveDate::from_ymd_opt(1999, 12, 31).unwrap()
        );
    }

    #[test]
    fn shards_form_complete_nonoverlapping_partition() {
        let total = cwr_cdec::reservoir::Reservoir::get_reservoir_vector().len();
        let shards = 12;

        let mut union: HashSet<usize> = HashSet::new();
        let mut count = 0usize;
        for shard in 0..shards {
            for idx in shard_indices(total, shards, shard) {
                // No index appears in two shards.
                assert!(union.insert(idx), "index {idx} assigned to multiple shards");
                count += 1;
            }
        }

        // Union covers exactly every reservoir index, once.
        assert_eq!(count, total, "partition size must equal reservoir count");
        assert_eq!(union.len(), total);
        for i in 0..total {
            assert!(union.contains(&i), "index {i} missing from partition");
        }
    }
}
