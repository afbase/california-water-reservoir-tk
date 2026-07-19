//! Split a monolithic observations CSV into per-station, delta-encoded,
//! brotli-compressed files plus a JSON manifest.
//!
//! Input (4-col, headerless): `STATION_ID,DURATION,YYYYMMDD,VALUE`
//!
//! Output, one file per station: `observations_<ID>.csv.br` (brotli-compressed,
//! delta-encoded — see [`cwr_data::codec`] for the wire format and the matching
//! decoder used by the WASM chart apps). Also writes `observations_manifest.json`
//! = `["ID", ...]` listing every station that has at least one numeric
//! observation, so the cumulative chart can enumerate available files without
//! 404-ing on the handful of capacity IDs that carry no data.
//!
//! This replaces the single monolithic `observations.csv.gz`, which grew past
//! GitHub's 100 MB file limit and broke the website deploy.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use cwr_data::codec;
use std::collections::BTreeMap;
use std::path::Path;

/// Physically-implausible storage cap in acre-feet. The largest reservoir in the
/// dataset is Lake Mead (~26M AF), so any reading beyond this is a CDEC data
/// error (e.g. station `ELC` once reported 14.2 *billion* AF; `JNC` 207M). Such
/// rows — and negatives — are dropped so they can't corrupt per-station charts
/// or the cumulative total.
const MAX_PLAUSIBLE_AF: i64 = 30_000_000;

/// Parse a date field into a day number (days from CE), or `None`.
///
/// The monolithic CSV mixes two date formats: clean `YYYYMMDD` (older monthly
/// rows) and `YYYYMMDD HHMM` (daily rows, where the raw CDEC obs-date time
/// suffix leaked through `incremental-query`). We normalize both by taking the
/// leading run of digits and using its first 8.
fn parse_daynum(s: &str) -> Option<i32> {
    let digits: String = s.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    NaiveDate::parse_from_str(&digits[..8], "%Y%m%d")
        .ok()
        .map(|d| d.num_days_from_ce())
}

/// Split `input` into per-station brotli files + manifest under `output_dir`.
///
/// When `min_coverage` is `Some(min)` (a fraction in `0.0..=1.0`), the number of
/// stations written must be at least `ceil(min * full_reservoir_count)` or the
/// call returns an error so the process exits non-zero.
pub fn run_split(input: &str, output_dir: &str, min_coverage: Option<f64>) -> Result<()> {
    // station -> (daynum -> value). BTreeMap keeps dates sorted and dedups
    // duplicate dates (last write wins, mirroring `INSERT OR REPLACE`).
    let mut stations: BTreeMap<String, BTreeMap<i32, i64>> = BTreeMap::new();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(input)
        .with_context(|| format!("opening {input}"))?;

    let mut rows_read: u64 = 0;
    let mut rows_kept: u64 = 0;
    for rec in rdr.records() {
        let rec = rec?;
        rows_read += 1;
        let station = rec.get(0).unwrap_or("").trim();
        let date = rec.get(2).unwrap_or("").trim();
        let value = rec.get(3).unwrap_or("").trim();
        if station.is_empty() {
            continue;
        }
        let Some(daynum) = parse_daynum(date) else {
            continue;
        };
        // Skip non-numeric readings (ART / BRT / ---), as the DB loader does.
        let Ok(v) = value.parse::<f64>() else {
            continue;
        };
        let vi = v.round() as i64;
        // Drop physically-implausible readings (CDEC data errors).
        if !(0..=MAX_PLAUSIBLE_AF).contains(&vi) {
            continue;
        }
        stations
            .entry(station.to_string())
            .or_default()
            .insert(daynum, vi);
        rows_kept += 1;
    }

    std::fs::create_dir_all(output_dir).with_context(|| format!("creating {output_dir}"))?;

    let mut manifest: Vec<String> = Vec::new();
    let mut total_bytes: u64 = 0;
    for (station, series) in &stations {
        if series.is_empty() {
            continue;
        }
        let sorted: Vec<(i32, i64)> = series.iter().map(|(&d, &v)| (d, v)).collect();
        let text = codec::encode_delta(&sorted);
        let compressed = codec::brotli_compress(text.as_bytes())
            .with_context(|| format!("compressing station {station}"))?;
        total_bytes += compressed.len() as u64;
        let path = Path::new(output_dir).join(format!("observations_{station}.csv.br"));
        std::fs::write(&path, &compressed)
            .with_context(|| format!("writing {}", path.display()))?;
        manifest.push(station.clone());
    }

    // Precompute the smoothed California-only cumulative daily total into a
    // single observations_cumulative.csv.br so the cumulative chart can fetch
    // ONE file instead of every per-station file.
    let cumulative = compute_cumulative_ca_only(&stations);
    let cumulative_written = if cumulative.is_empty() {
        false
    } else {
        let text = codec::encode_delta(&cumulative);
        let compressed =
            codec::brotli_compress(text.as_bytes()).context("compressing cumulative series")?;
        let path = Path::new(output_dir).join("observations_cumulative.csv.br");
        std::fs::write(&path, &compressed)
            .with_context(|| format!("writing {}", path.display()))?;
        true
    };

    let manifest_path = Path::new(output_dir).join("observations_manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string(&manifest)?)
        .with_context(|| format!("writing {}", manifest_path.display()))?;

    println!(
        "split-observations: read {rows_read} rows, kept {rows_kept}, wrote {} stations ({} KiB brotli){} + manifest to {output_dir}",
        manifest.len(),
        total_bytes / 1024,
        if cumulative_written { " + cumulative" } else { "" }
    );

    if let Some(min) = min_coverage {
        let expected = cwr_cdec::reservoir::Reservoir::get_reservoir_vector().len();
        let needed = (min * expected as f64).ceil() as usize;
        let written = manifest.len();
        let pct = if expected == 0 {
            0.0
        } else {
            written as f64 / expected as f64 * 100.0
        };
        println!(
            "coverage: {written}/{expected} stations = {pct:.1}% (min required {needed} = {:.1}%)",
            min * 100.0
        );
        if written < needed {
            anyhow::bail!(
                "coverage gate failed: wrote {written}/{expected} stations, need at least {needed} ({:.1}%)",
                min * 100.0
            );
        }
    }

    Ok(())
}

/// Station IDs excluded from the California-only cumulative total (Colorado
/// River reservoirs). Mirrors `cwr_db::query_total_water_ca_only_smoothed`.
const CUMULATIVE_EXCLUDE: &[&str] = &["MEA", "PWL"];

/// Forward-fill each California reservoir's last-known storage across every
/// observed date and sum per day, producing a smoothed statewide daily total
/// series as sorted `(daynum, total)` pairs. This is the same forward-fill the
/// runtime `query_total_water_ca_only_smoothed` performs, done once at build
/// time so the browser fetches a single precomputed file.
fn compute_cumulative_ca_only(stations: &BTreeMap<String, BTreeMap<i32, i64>>) -> Vec<(i32, i64)> {
    use std::collections::HashMap;
    // day -> [(station, value)] reported that day (CA-only).
    let mut by_day: BTreeMap<i32, Vec<(&str, i64)>> = BTreeMap::new();
    for (station, series) in stations {
        if CUMULATIVE_EXCLUDE.contains(&station.as_str()) {
            continue;
        }
        for (&day, &val) in series {
            by_day.entry(day).or_default().push((station.as_str(), val));
        }
    }
    let mut last: HashMap<&str, i64> = HashMap::new();
    let mut out: Vec<(i32, i64)> = Vec::with_capacity(by_day.len());
    for (day, updates) in by_day {
        for (station, val) in updates {
            last.insert(station, val);
        }
        out.push((day, last.values().sum()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dn(y: i32, m: u32, d: u32) -> i32 {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().num_days_from_ce()
    }

    #[test]
    fn parses_both_date_formats() {
        // Clean YYYYMMDD (monthly rows) and time-suffixed YYYYMMDD HHMM (daily rows).
        assert_eq!(parse_daynum("19611031"), Some(dn(1961, 10, 31)));
        assert_eq!(parse_daynum("20260215 0000"), Some(dn(2026, 2, 15)));
        assert_eq!(parse_daynum(" 20260215 0000 "), Some(dn(2026, 2, 15)));
        assert_eq!(parse_daynum("ART"), None);
        assert_eq!(parse_daynum("2026"), None);
    }

    /// Create a unique temp dir, write a tiny 2-station input CSV into it, and
    /// return `(dir, input_path)`.
    fn tiny_input() -> (std::path::PathBuf, String) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cwr_split_test_{}_{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.csv");
        // Two stations, a couple of numeric daily readings each.
        std::fs::write(
            &input,
            "SHA,D,20260101,1000000\nSHA,D,20260102,1000500\nORO,D,20260101,900000\n",
        )
        .unwrap();
        (dir.clone(), input.to_string_lossy().into_owned())
    }

    #[test]
    fn min_coverage_none_succeeds() {
        let (dir, input) = tiny_input();
        let out = dir.join("out");
        let res = run_split(&input, out.to_str().unwrap(), None);
        assert!(res.is_ok(), "expected Ok with no coverage gate: {res:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn min_coverage_gate_fails_with_few_stations() {
        let (dir, input) = tiny_input();
        let out = dir.join("out");
        // Two stations is far below 90% of the full 217-reservoir list.
        let res = run_split(&input, out.to_str().unwrap(), Some(0.9));
        assert!(res.is_err(), "expected Err when coverage below threshold");
        std::fs::remove_dir_all(&dir).ok();
    }
}
