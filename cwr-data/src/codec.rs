//! Shared codec for per-station observation files (`observations_<ID>.csv.br`).
//!
//! The producer (`cwr-cli split-observations`) delta-encodes each station's
//! date-sorted series and brotli-compresses it. The consumer (the WASM chart
//! apps) decompresses and reconstructs the 4-column CSV that
//! [`cwr_db::Database::load_observations`] expects. Keeping both directions in
//! one module guarantees the encode/decode stay in lock-step.
//!
//! ## Wire format (after brotli-decompression)
//! One row per observation, ascending by date:
//! - row 0:  `<abs_daynum>,<abs_value>`
//! - row n:  `<delta_daynum>,<delta_value>`
//!
//! `daynum` is [`chrono::NaiveDate::num_days_from_ce`]; `value` is whole
//! acre-feet. Delta-encoding makes daily series (date deltas ~`1`, small value
//! deltas) compress extremely well.

use chrono::NaiveDate;
use std::io::{Read, Write};

/// Brotli quality (0-11). Produced once in CI, so favor ratio.
const BROTLI_QUALITY: u32 = 11;
/// Brotli window size log2 (matches the `brotli -q 11` CLI default).
const BROTLI_WINDOW: u32 = 22;

/// Delta-encode a date-sorted slice of `(daynum, value)` pairs to text.
///
/// The input MUST be sorted ascending by `daynum` with no duplicate days.
pub fn encode_delta(sorted: &[(i32, i64)]) -> String {
    let mut out = String::new();
    let mut prev: Option<(i32, i64)> = None;
    for &(day, val) in sorted {
        match prev {
            None => out.push_str(&format!("{day},{val}\n")),
            Some((pd, pv)) => out.push_str(&format!("{},{}\n", day - pd, val - pv)),
        }
        prev = Some((day, val));
    }
    out
}

/// Brotli-compress bytes at maximum quality.
pub fn brotli_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, BROTLI_QUALITY, BROTLI_WINDOW);
        w.write_all(data)?;
        w.flush()?;
    }
    Ok(out)
}

/// Brotli-decompress bytes into a UTF-8 string.
pub fn brotli_decompress(data: &[u8]) -> Result<String, String> {
    let mut out = Vec::new();
    brotli::Decompressor::new(data, 4096)
        .read_to_end(&mut out)
        .map_err(|e| format!("brotli decompress failed: {e}"))?;
    String::from_utf8(out).map_err(|e| format!("decoded bytes were not UTF-8: {e}"))
}

/// Reconstruct the 4-column CSV (`station,D,YYYYMMDD,value`) that
/// [`cwr_db::Database::load_observations`] expects, from delta-encoded text.
pub fn reconstruct_csv(station_id: &str, delta_text: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut day = 0i32;
    let mut val = 0i64;
    let mut first = true;
    for line in delta_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (a, b) = line
            .split_once(',')
            .ok_or_else(|| format!("malformed delta row: {line:?}"))?;
        let a: i32 = a
            .trim()
            .parse()
            .map_err(|_| format!("bad daynum field: {a:?}"))?;
        let b: i64 = b
            .trim()
            .parse()
            .map_err(|_| format!("bad value field: {b:?}"))?;
        if first {
            day = a;
            val = b;
            first = false;
        } else {
            day += a;
            val += b;
        }
        let date = NaiveDate::from_num_days_from_ce_opt(day)
            .ok_or_else(|| format!("daynum out of range: {day}"))?;
        out.push_str(station_id);
        out.push_str(",D,");
        out.push_str(&date.format("%Y%m%d").to_string());
        out.push(',');
        out.push_str(&val.to_string());
        out.push('\n');
    }
    Ok(out)
}

/// Convenience: brotli-decompress a per-station file's bytes and reconstruct the
/// 4-column CSV ready for `load_observations`.
pub fn decode_station_file(station_id: &str, compressed: &[u8]) -> Result<String, String> {
    let text = brotli_decompress(compressed)?;
    reconstruct_csv(station_id, &text)
}

/// Decode a delta+brotli file into `(YYYYMMDD, value)` pairs.
///
/// Unlike [`decode_station_file`], this does not wrap rows as per-station CSV
/// lines — it returns bare date/value pairs. Used for the precomputed
/// `observations_cumulative.csv.br` (statewide daily totals), which the
/// cumulative chart fetches as a single file instead of downloading every
/// per-station file.
pub fn decode_dated_series(compressed: &[u8]) -> Result<Vec<(String, i64)>, String> {
    let text = brotli_decompress(compressed)?;
    let mut out = Vec::new();
    let mut day = 0i32;
    let mut val = 0i64;
    let mut first = true;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (a, b) = line
            .split_once(',')
            .ok_or_else(|| format!("malformed delta row: {line:?}"))?;
        let a: i32 = a
            .trim()
            .parse()
            .map_err(|_| format!("bad daynum field: {a:?}"))?;
        let b: i64 = b
            .trim()
            .parse()
            .map_err(|_| format!("bad value field: {b:?}"))?;
        if first {
            day = a;
            val = b;
            first = false;
        } else {
            day += a;
            val += b;
        }
        let date = NaiveDate::from_num_days_from_ce_opt(day)
            .ok_or_else(|| format!("daynum out of range: {day}"))?;
        out.push((date.format("%Y%m%d").to_string(), val));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    fn dn(y: i32, m: u32, d: u32) -> i32 {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().num_days_from_ce()
    }

    #[test]
    fn encode_then_reconstruct_roundtrips() {
        let series = vec![
            (dn(1961, 10, 31), 975_600i64),
            (dn(1961, 11, 30), 986_600),
            (dn(1962, 1, 1), 1_000_000),
            (dn(1962, 1, 2), 999_950), // exercises a negative value delta
        ];
        let text = encode_delta(&series);
        let csv = reconstruct_csv("CLE", &text).unwrap();
        assert_eq!(
            csv,
            "CLE,D,19611031,975600\n\
             CLE,D,19611130,986600\n\
             CLE,D,19620101,1000000\n\
             CLE,D,19620102,999950\n"
        );
    }

    #[test]
    fn brotli_roundtrips() {
        let data = b"715129,975600\n30,11000\n1,-50\n";
        let compressed = brotli_compress(data).unwrap();
        assert_eq!(
            brotli_decompress(&compressed).unwrap().as_bytes(),
            &data[..]
        );
    }

    #[test]
    fn full_pipeline_roundtrips() {
        let series = vec![(dn(2020, 1, 1), 500_000i64), (dn(2020, 1, 2), 500_100)];
        let compressed = brotli_compress(encode_delta(&series).as_bytes()).unwrap();
        let csv = decode_station_file("SHA", &compressed).unwrap();
        assert_eq!(csv, "SHA,D,20200101,500000\nSHA,D,20200102,500100\n");
    }

    #[test]
    fn reconstruct_rejects_garbage() {
        assert!(reconstruct_csv("X", "not-a-row").is_err());
    }

    #[test]
    fn dated_series_roundtrips() {
        let series = vec![
            (dn(2020, 1, 1), 500_000i64),
            (dn(2020, 1, 15), 480_000),
            (dn(2020, 2, 1), 520_000),
        ];
        let compressed = brotli_compress(encode_delta(&series).as_bytes()).unwrap();
        let decoded = decode_dated_series(&compressed).unwrap();
        assert_eq!(
            decoded,
            vec![
                ("20200101".to_string(), 500_000),
                ("20200115".to_string(), 480_000),
                ("20200201".to_string(), 520_000),
            ]
        );
    }
}
