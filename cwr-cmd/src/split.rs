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
pub fn run_split(input: &str, output_dir: &str) -> Result<()> {
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
        stations
            .entry(station.to_string())
            .or_default()
            .insert(daynum, v.round() as i64);
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

    let manifest_path = Path::new(output_dir).join("observations_manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string(&manifest)?)
        .with_context(|| format!("writing {}", manifest_path.display()))?;

    println!(
        "split-observations: read {rows_read} rows, kept {rows_kept}, wrote {} stations ({} KiB brotli) + manifest to {output_dir}",
        manifest.len(),
        total_bytes / 1024
    );
    Ok(())
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
}
