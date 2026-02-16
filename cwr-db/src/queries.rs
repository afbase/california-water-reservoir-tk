//! Typed query methods for retrieving water and snow data from the database.
//!
//! All queries return typed structs from [`crate::models`] that can be
//! serialized to JSON for consumption by D3.js chart components.
//!
//! # Water Year Convention
//!
//! A "water year" runs from October 1 through September 30. Water year 2023
//! spans October 1, 2022 through September 30, 2023. This convention is
//! standard in California water resource management and allows overlaying
//! different years on the same x-axis for comparison.

use crate::models::{
    DateValue, ReservoirInfo, SnowStationInfo, StationDateValue, WaterYearData, WaterYearStats,
};
use crate::Database;
use rusqlite::params;

impl Database {
    // ───────────────────── Water Queries ─────────────────────

    /// Get total water level for a date range (for cumulative line chart).
    ///
    /// Returns daily total storage in acre-feet, derived by summing all
    /// individual station observations grouped by date. Ordered chronologically.
    pub fn query_total_water(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<DateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, SUM(value) as total_af
             FROM observations
             WHERE date >= ?1 AND date <= ?2
             GROUP BY date
             ORDER BY date",
        )?;
        let rows = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(DateValue {
                    date: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_total_water returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get total water level for California-only reservoirs in a date range.
    ///
    /// Same as [`query_total_water`](Self::query_total_water) but excludes
    /// out-of-state reservoirs (Lake Mead = MEA, Lake Powell = PWL) by
    /// joining against the `reservoirs` table and filtering station IDs.
    pub fn query_total_water_ca_only(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<DateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT o.date, SUM(o.value) as total_af
             FROM observations o
             INNER JOIN reservoirs r ON o.station_id = r.station_id
             WHERE r.station_id NOT IN ('MEA', 'PWL')
               AND o.date >= ?1 AND o.date <= ?2
             GROUP BY o.date
             ORDER BY o.date",
        )?;
        let rows = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(DateValue {
                    date: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_total_water_ca_only returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get observation history for a specific reservoir station.
    ///
    /// Returns storage values in acre-feet (AF) for the given station
    /// within the specified date range, ordered chronologically.
    pub fn query_reservoir_history(
        &self,
        station_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<DateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, value FROM observations
             WHERE station_id = ?1 AND date >= ?2 AND date <= ?3
             ORDER BY date",
        )?;
        let rows = stmt
            .query_map(params![station_id, start_date, end_date], |row| {
                Ok(DateValue {
                    date: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_reservoir_history returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get all reservoir histories for a date range (for multi-line chart).
    ///
    /// Returns observations for all stations in the specified date range,
    /// ordered by station_id then date. This enables drawing one line per
    /// reservoir on the same chart.
    pub fn query_all_reservoir_histories(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<StationDateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT station_id, date, value FROM observations
             WHERE date >= ?1 AND date <= ?2
             ORDER BY station_id, date",
        )?;
        let rows = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(StationDateValue {
                    station_id: row.get(0)?,
                    date: row.get(1)?,
                    value: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_all_reservoir_histories returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get water year data for a specific reservoir.
    ///
    /// Partitions observations into water years (Oct 1 - Sep 30) and
    /// normalizes each date to a day-of-water-year index (0 = Oct 1,
    /// 364 = Sep 30). This enables overlaying multiple years on the
    /// same x-axis for the water years comparison chart.
    ///
    /// Only complete or partial water years with data are returned.
    pub fn query_water_years(&self, station_id: &str) -> anyhow::Result<Vec<WaterYearData>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, value FROM observations
             WHERE station_id = ?1
             ORDER BY date",
        )?;
        let raw_rows: Vec<(String, f64)> = stmt
            .query_map(params![station_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut results = Vec::new();
        for (date_str, value) in raw_rows {
            if let Some((water_year, day_of_year)) = date_to_water_year_day(&date_str) {
                results.push(WaterYearData {
                    year: water_year,
                    day_of_year,
                    date: date_str,
                    value,
                });
            }
        }
        log::info!(
            "[CWR Debug] query: query_water_years returned {} records",
            results.len()
        );
        Ok(results)
    }

    /// Get water year statistics (min/max per year) for a specific reservoir.
    ///
    /// For each water year, computes the lowest and highest observed storage
    /// values. Then dynamically determines which year is the driest (lowest
    /// minimum value across all years) and which is the wettest (highest
    /// maximum value across all years).
    ///
    /// This replaces the old hard-coded driest/wettest year approach with
    /// a data-driven computation.
    pub fn query_water_year_stats(&self, station_id: &str) -> anyhow::Result<Vec<WaterYearStats>> {
        // First get all water year data
        let water_years = self.query_water_years(station_id)?;

        // Group by year and compute per-year min/max
        let mut year_stats: std::collections::BTreeMap<i32, (String, f64, String, f64)> =
            std::collections::BTreeMap::new();

        for wy in &water_years {
            let entry = year_stats
                .entry(wy.year)
                .or_insert_with(|| (wy.date.clone(), wy.value, wy.date.clone(), wy.value));
            // Update minimum
            if wy.value < entry.1 {
                entry.0 = wy.date.clone();
                entry.1 = wy.value;
            }
            // Update maximum
            if wy.value > entry.3 {
                entry.2 = wy.date.clone();
                entry.3 = wy.value;
            }
        }

        if year_stats.is_empty() {
            return Ok(Vec::new());
        }

        // Find the global driest (lowest min) and wettest (highest max) years
        let driest_year = year_stats
            .iter()
            .min_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap())
            .map(|(y, _)| *y)
            .unwrap();

        let wettest_year = year_stats
            .iter()
            .max_by(|a, b| a.1 .3.partial_cmp(&b.1 .3).unwrap())
            .map(|(y, _)| *y)
            .unwrap();

        let results: Vec<WaterYearStats> = year_stats
            .into_iter()
            .map(
                |(year, (date_lowest, lowest_value, date_highest, highest_value))| WaterYearStats {
                    year,
                    date_lowest,
                    lowest_value,
                    date_highest,
                    highest_value,
                    is_driest: year == driest_year,
                    is_wettest: year == wettest_year,
                },
            )
            .collect();

        log::info!(
            "[CWR Debug] query: query_water_year_stats returned {} records",
            results.len()
        );
        Ok(results)
    }

    /// Get list of all reservoirs.
    ///
    /// Returns metadata for all reservoirs in the database, ordered by
    /// capacity descending (largest reservoirs first).
    pub fn query_reservoirs(&self) -> anyhow::Result<Vec<ReservoirInfo>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT station_id, dam, lake, capacity FROM reservoirs
             ORDER BY capacity DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReservoirInfo {
                    station_id: row.get(0)?,
                    dam: row.get(1)?,
                    lake: row.get(2)?,
                    capacity: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_reservoirs returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get the (min, max) date range for all observations.
    ///
    /// Returns the earliest and latest dates across all station observations
    /// in YYYYMMDD format.
    pub fn query_date_range(&self) -> anyhow::Result<(String, String)> {
        let conn = self.conn.borrow();
        let (min_date, max_date) =
            conn.query_row("SELECT MIN(date), MAX(date) FROM observations", [], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
        log::info!(
            "[CWR Debug] query: query_date_range returned ({}, {})",
            min_date,
            max_date
        );
        Ok((min_date, max_date))
    }

    // ───────────────────── Snow Queries ─────────────────────

    /// Get total snow water equivalent for a date range (for cumulative snow chart).
    ///
    /// Returns daily total SWE derived by summing all station snow water
    /// equivalent values grouped by date. Rows where SWE is NULL are excluded.
    /// Ordered chronologically.
    pub fn query_total_snow(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<DateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, SUM(snow_water_equivalent) as total_swe
             FROM snow_observations
             WHERE snow_water_equivalent IS NOT NULL
               AND date >= ?1 AND date <= ?2
             GROUP BY date
             ORDER BY date",
        )?;
        let rows = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(DateValue {
                    date: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_total_snow returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get snow observation history for a specific station.
    ///
    /// Returns snow water equivalent (SWE) values for the given station
    /// within the specified date range, ordered chronologically.
    /// Rows where SWE is NULL are excluded.
    pub fn query_snow_station_history(
        &self,
        station_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<DateValue>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, snow_water_equivalent FROM snow_observations
             WHERE station_id = ?1 AND date >= ?2 AND date <= ?3
               AND snow_water_equivalent IS NOT NULL
             ORDER BY date",
        )?;
        let rows = stmt
            .query_map(params![station_id, start_date, end_date], |row| {
                Ok(DateValue {
                    date: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_snow_station_history returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get snow water year data for a specific station.
    ///
    /// Same water year convention as [`query_water_years`](Self::query_water_years)
    /// but uses snow water equivalent (SWE) values from `snow_observations`.
    pub fn query_snow_years(&self, station_id: &str) -> anyhow::Result<Vec<WaterYearData>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT date, snow_water_equivalent FROM snow_observations
             WHERE station_id = ?1 AND snow_water_equivalent IS NOT NULL
             ORDER BY date",
        )?;
        let raw_rows: Vec<(String, f64)> = stmt
            .query_map(params![station_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut results = Vec::new();
        for (date_str, value) in raw_rows {
            if let Some((water_year, day_of_year)) = date_to_water_year_day(&date_str) {
                results.push(WaterYearData {
                    year: water_year,
                    day_of_year,
                    date: date_str,
                    value,
                });
            }
        }
        log::info!(
            "[CWR Debug] query: query_snow_years returned {} records",
            results.len()
        );
        Ok(results)
    }

    /// Get snow year statistics (min/max per year) for a specific station.
    ///
    /// Same approach as [`query_water_year_stats`](Self::query_water_year_stats)
    /// but uses snow SWE values. Dynamically determines driest/wettest years.
    pub fn query_snow_year_stats(&self, station_id: &str) -> anyhow::Result<Vec<WaterYearStats>> {
        let snow_years = self.query_snow_years(station_id)?;

        let mut year_stats: std::collections::BTreeMap<i32, (String, f64, String, f64)> =
            std::collections::BTreeMap::new();

        for sy in &snow_years {
            let entry = year_stats
                .entry(sy.year)
                .or_insert_with(|| (sy.date.clone(), sy.value, sy.date.clone(), sy.value));
            if sy.value < entry.1 {
                entry.0 = sy.date.clone();
                entry.1 = sy.value;
            }
            if sy.value > entry.3 {
                entry.2 = sy.date.clone();
                entry.3 = sy.value;
            }
        }

        if year_stats.is_empty() {
            return Ok(Vec::new());
        }

        let driest_year = year_stats
            .iter()
            .min_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap())
            .map(|(y, _)| *y)
            .unwrap();

        let wettest_year = year_stats
            .iter()
            .max_by(|a, b| a.1 .3.partial_cmp(&b.1 .3).unwrap())
            .map(|(y, _)| *y)
            .unwrap();

        let results: Vec<WaterYearStats> = year_stats
            .into_iter()
            .map(
                |(year, (date_lowest, lowest_value, date_highest, highest_value))| WaterYearStats {
                    year,
                    date_lowest,
                    lowest_value,
                    date_highest,
                    highest_value,
                    is_driest: year == driest_year,
                    is_wettest: year == wettest_year,
                },
            )
            .collect();

        log::info!(
            "[CWR Debug] query: query_snow_year_stats returned {} records",
            results.len()
        );
        Ok(results)
    }

    /// Get list of all snow stations.
    ///
    /// Returns metadata for all snow stations in the database, ordered
    /// by station name alphabetically.
    pub fn query_snow_stations(&self) -> anyhow::Result<Vec<SnowStationInfo>> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT station_id, name, elevation, COALESCE(river_basin, '') FROM snow_stations
             ORDER BY name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SnowStationInfo {
                    station_id: row.get(0)?,
                    name: row.get(1)?,
                    elevation: row.get(2)?,
                    river_basin: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        log::info!(
            "[CWR Debug] query: query_snow_stations returned {} records",
            rows.len()
        );
        Ok(rows)
    }

    /// Get the (min, max) date range for all snow observations.
    ///
    /// Returns the earliest and latest dates across all snow station observations
    /// in YYYYMMDD format.
    pub fn query_snow_date_range(&self) -> anyhow::Result<(String, String)> {
        let conn = self.conn.borrow();
        let (min_date, max_date) = conn.query_row(
            "SELECT MIN(date), MAX(date) FROM snow_observations",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        Ok((min_date, max_date))
    }
}

// ───────────────────── Helper Functions ─────────────────────

/// Convert a date string (YYYYMMDD) to (water_year, day_of_water_year).
///
/// Water years run from October 1 to September 30:
/// - October 1, 2022 is day 0 of water year 2023
/// - September 30, 2023 is day 364 of water year 2023
///
/// Returns `None` if the date string cannot be parsed.
fn date_to_water_year_day(date_str: &str) -> Option<(i32, i32)> {
    if date_str.len() < 8 {
        return None;
    }

    let year: i32 = date_str[0..4].parse().ok()?;
    let month: u32 = date_str[4..6].parse().ok()?;
    let day: u32 = date_str[6..8].parse().ok()?;

    // Validate basic ranges
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Determine the water year
    // Oct-Dec of calendar year Y belong to water year Y+1
    // Jan-Sep of calendar year Y belong to water year Y
    let water_year = if month >= 10 { year + 1 } else { year };

    // Calculate day of water year (Oct 1 = day 0)
    // We need the number of days from Oct 1 of the previous calendar year
    let wy_start_year = water_year - 1;

    // Use chrono for accurate day-of-year calculation
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    let wy_start = chrono::NaiveDate::from_ymd_opt(wy_start_year, 10, 1)?;

    let day_of_year = (date - wy_start).num_days() as i32;

    // Sanity check: day_of_year should be between 0 and 365 (leap years)
    if !(0..=365).contains(&day_of_year) {
        return None;
    }

    Some((water_year, day_of_year))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    /// Helper to create a database with sample water data.
    fn sample_water_db() -> Database {
        let db = Database::new().unwrap();

        let reservoirs_csv = "\
ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
SHA,Shasta,Lake Shasta,Sacramento River,4552000,1954
ORO,Oroville,Lake Oroville,Feather River,3537577,1969
";
        db.load_reservoirs(reservoirs_csv).unwrap();

        // Observations spanning two water years:
        // Water year 2022: Oct 2021 - Sep 2022
        // Water year 2023: Oct 2022 - Sep 2023
        let observations_csv = "\
SHA,D,20211001,2000000
SHA,D,20211115,2200000
SHA,D,20220101,2500000
SHA,D,20220301,3000000
SHA,D,20220601,2800000
SHA,D,20220930,1800000
SHA,D,20221001,1900000
SHA,D,20221115,2100000
SHA,D,20230101,2400000
SHA,D,20230301,4000000
SHA,D,20230601,3500000
SHA,D,20230930,2000000
ORO,D,20220101,1500000
ORO,D,20220601,1200000
";
        db.load_observations(observations_csv).unwrap();

        db
    }

    /// Helper to create a database with sample snow data.
    fn sample_snow_db() -> Database {
        let db = Database::new().unwrap();

        let stations_csv = "\
ID,NAME,ELEVATION,RIVER_BASIN,COUNTY,LATITUDE,LONGITUDE
GRZ,Grizzly Ridge,5280,Feather River,Plumas,39.95,-120.68
HNT,Huntington Lake,7000,San Joaquin River,Fresno,37.23,-119.22
";
        db.load_snow_stations(stations_csv).unwrap();

        // Snow observations spanning two water years
        let snow_obs_csv = "\
GRZ,20211001,0.0,0.0
GRZ,20220101,15.0,45.0
GRZ,20220301,25.0,75.0
GRZ,20220601,5.0,15.0
GRZ,20220930,0.0,0.0
GRZ,20221001,0.0,0.0
GRZ,20230101,20.0,60.0
GRZ,20230301,35.0,105.0
GRZ,20230601,8.0,24.0
GRZ,20230930,0.0,0.0
HNT,20220101,10.0,30.0
HNT,20220601,3.0,9.0
";
        db.load_snow_observations(snow_obs_csv).unwrap();

        db
    }

    // ───────────────────── date_to_water_year_day tests ─────────────────────

    #[test]
    fn water_year_day_october_1() {
        // Oct 1, 2022 is day 0 of water year 2023
        let (wy, day) = date_to_water_year_day("20221001").unwrap();
        assert_eq!(wy, 2023);
        assert_eq!(day, 0);
    }

    #[test]
    fn water_year_day_december_31() {
        // Dec 31, 2022 is day 91 of water year 2023
        let (wy, day) = date_to_water_year_day("20221231").unwrap();
        assert_eq!(wy, 2023);
        assert_eq!(day, 91);
    }

    #[test]
    fn water_year_day_january_1() {
        // Jan 1, 2023 is day 92 of water year 2023 (non-leap: Oct has 31, Nov has 30, Dec has 31 = 92 days)
        let (wy, day) = date_to_water_year_day("20230101").unwrap();
        assert_eq!(wy, 2023);
        assert_eq!(day, 92);
    }

    #[test]
    fn water_year_day_september_30() {
        // Sep 30, 2023 is last day of water year 2023
        let (wy, day) = date_to_water_year_day("20230930").unwrap();
        assert_eq!(wy, 2023);
        // Oct: 31, Nov: 30, Dec: 31, Jan: 31, Feb: 28, Mar: 31, Apr: 30, May: 31, Jun: 30, Jul: 31, Aug: 31, Sep: 30 = 365
        // day_of_year = 365 - 1 = 364
        assert_eq!(day, 364);
    }

    #[test]
    fn water_year_day_leap_year() {
        // Feb 29, 2024 in water year 2024
        let (wy, day) = date_to_water_year_day("20240229").unwrap();
        assert_eq!(wy, 2024);
        // Oct: 31 + Nov: 30 + Dec: 31 + Jan: 31 + Feb 1-29 = 31+30+31+31+29-1 = 151
        assert_eq!(day, 151);
    }

    #[test]
    fn water_year_day_invalid_date() {
        assert!(date_to_water_year_day("invalid").is_none());
        assert!(date_to_water_year_day("2022").is_none());
        assert!(date_to_water_year_day("20221301").is_none()); // month 13
        assert!(date_to_water_year_day("20220230").is_none()); // Feb 30
    }

    // ───────────────────── Water Query Tests ─────────────────────

    #[test]
    fn query_total_water_returns_ordered_results() {
        let db = sample_water_db();
        let results = db.query_total_water("20220101", "20220601").unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].date, "20220101");
        assert_eq!(results[1].date, "20220301");
        assert_eq!(results[2].date, "20220601");
        // 20220101: SHA(2500000) + ORO(1500000) = 4000000
        assert!((results[0].value - 4000000.0).abs() < 0.01);
        // 20220301: SHA(3000000) only
        assert!((results[1].value - 3000000.0).abs() < 0.01);
        // 20220601: SHA(2800000) + ORO(1200000) = 4000000
        assert!((results[2].value - 4000000.0).abs() < 0.01);
    }

    #[test]
    fn query_total_water_filters_by_date_range() {
        let db = sample_water_db();
        let results = db.query_total_water("20220101", "20220301").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_total_water_empty_range() {
        let db = sample_water_db();
        let results = db.query_total_water("20200101", "20200301").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn query_total_water_ca_only() {
        let db = sample_water_db();
        // Both SHA and ORO are CA reservoirs (not MEA or PWL), so totals match query_total_water
        let results = db
            .query_total_water_ca_only("20220101", "20220601")
            .unwrap();
        assert_eq!(results.len(), 3);
        // 20220101: SHA(2500000) + ORO(1500000) = 4000000
        assert!((results[0].value - 4000000.0).abs() < 0.01);
    }

    #[test]
    fn query_total_water_ca_only_excludes_colorado() {
        let db = Database::new().unwrap();
        let reservoirs_csv = "\
ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
SHA,Shasta,Lake Shasta,Sacramento River,4552000,1954
MEA,Hoover,Lake Mead,Colorado River,26159000,1936
PWL,Glen Canyon,Lake Powell,Colorado River,24322000,1963
";
        db.load_reservoirs(reservoirs_csv).unwrap();
        let observations_csv = "\
SHA,D,20220101,2500000
MEA,D,20220101,10000000
PWL,D,20220101,8000000
";
        db.load_observations(observations_csv).unwrap();

        let all = db.query_total_water("20220101", "20220101").unwrap();
        // All three stations: 2500000 + 10000000 + 8000000 = 20500000
        assert!((all[0].value - 20500000.0).abs() < 0.01);

        let ca_only = db
            .query_total_water_ca_only("20220101", "20220101")
            .unwrap();
        // Only SHA: 2500000
        assert_eq!(ca_only.len(), 1);
        assert!((ca_only[0].value - 2500000.0).abs() < 0.01);
    }

    #[test]
    fn query_reservoir_history() {
        let db = sample_water_db();
        let results = db
            .query_reservoir_history("SHA", "20220101", "20220930")
            .unwrap();
        // SHA has 4 observations in range: 20220101, 20220301, 20220601, 20220930
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].date, "20220101");
        assert!((results[0].value - 2500000.0).abs() < 0.01);
    }

    #[test]
    fn query_reservoir_history_nonexistent_station() {
        let db = sample_water_db();
        let results = db
            .query_reservoir_history("NOPE", "20220101", "20220930")
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn query_all_reservoir_histories() {
        let db = sample_water_db();
        let results = db
            .query_all_reservoir_histories("20220101", "20220601")
            .unwrap();
        // SHA has 20220101, 20220301, 20220601 = 3
        // ORO has 20220101, 20220601 = 2
        assert_eq!(results.len(), 5);

        // Should be ordered by station_id, then date
        let station_ids: Vec<&str> = results.iter().map(|r| r.station_id.as_str()).collect();
        assert_eq!(station_ids, vec!["ORO", "ORO", "SHA", "SHA", "SHA"]);
    }

    #[test]
    fn query_water_years_partitions_correctly() {
        let db = sample_water_db();
        let results = db.query_water_years("SHA").unwrap();

        // SHA observations span WY 2022 (Oct 2021 - Sep 2022) and WY 2023 (Oct 2022 - Sep 2023)
        let wy_2022: Vec<&WaterYearData> = results.iter().filter(|r| r.year == 2022).collect();
        let wy_2023: Vec<&WaterYearData> = results.iter().filter(|r| r.year == 2023).collect();

        assert_eq!(wy_2022.len(), 6, "WY 2022 should have 6 observations");
        assert_eq!(wy_2023.len(), 6, "WY 2023 should have 6 observations");

        // Oct 1, 2021 is day 0 of WY 2022
        let oct1 = wy_2022.iter().find(|r| r.date == "20211001").unwrap();
        assert_eq!(oct1.day_of_year, 0);

        // Sep 30, 2022 is last day of WY 2022
        let sep30 = wy_2022.iter().find(|r| r.date == "20220930").unwrap();
        assert_eq!(sep30.day_of_year, 364);
    }

    #[test]
    fn query_water_year_stats_computes_min_max() {
        let db = sample_water_db();
        let stats = db.query_water_year_stats("SHA").unwrap();

        assert_eq!(stats.len(), 2, "Should have stats for 2 water years");

        let wy_2022 = stats.iter().find(|s| s.year == 2022).unwrap();
        let wy_2023 = stats.iter().find(|s| s.year == 2023).unwrap();

        // WY 2022: min = 1800000 (Sep 30), max = 3000000 (Mar 1)
        assert!((wy_2022.lowest_value - 1800000.0).abs() < 0.01);
        assert!((wy_2022.highest_value - 3000000.0).abs() < 0.01);
        assert_eq!(wy_2022.date_lowest, "20220930");
        assert_eq!(wy_2022.date_highest, "20220301");

        // WY 2023: min = 1900000 (Oct 1), max = 4000000 (Mar 1)
        assert!((wy_2023.lowest_value - 1900000.0).abs() < 0.01);
        assert!((wy_2023.highest_value - 4000000.0).abs() < 0.01);
    }

    #[test]
    fn query_water_year_stats_driest_wettest_dynamic() {
        let db = sample_water_db();
        let stats = db.query_water_year_stats("SHA").unwrap();

        let wy_2022 = stats.iter().find(|s| s.year == 2022).unwrap();
        let wy_2023 = stats.iter().find(|s| s.year == 2023).unwrap();

        // WY 2022 has lowest min (1800000) so it is driest
        assert!(wy_2022.is_driest, "WY 2022 should be driest (min 1800000)");
        assert!(!wy_2022.is_wettest);

        // WY 2023 has highest max (4000000) so it is wettest
        assert!(
            wy_2023.is_wettest,
            "WY 2023 should be wettest (max 4000000)"
        );
        assert!(!wy_2023.is_driest);
    }

    #[test]
    fn query_water_year_stats_empty_station() {
        let db = sample_water_db();
        let stats = db.query_water_year_stats("NOPE").unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn query_reservoirs_ordered_by_capacity() {
        let db = sample_water_db();
        let reservoirs = db.query_reservoirs().unwrap();
        assert_eq!(reservoirs.len(), 2);
        // SHA (4552000) should come before ORO (3537577)
        assert_eq!(reservoirs[0].station_id, "SHA");
        assert_eq!(reservoirs[1].station_id, "ORO");
        assert_eq!(reservoirs[0].capacity, 4552000);
    }

    #[test]
    fn query_date_range() {
        let db = sample_water_db();
        let (min_date, max_date) = db.query_date_range().unwrap();
        assert_eq!(min_date, "20211001");
        assert_eq!(max_date, "20230930");
    }

    // ───────────────────── Snow Query Tests ─────────────────────

    #[test]
    fn query_total_snow() {
        let db = sample_snow_db();
        let results = db.query_total_snow("20220101", "20220601").unwrap();
        // Dates with snow data in range:
        // 20220101: GRZ(15.0) + HNT(10.0) = 25.0
        // 20220301: GRZ(25.0) = 25.0
        // 20220601: GRZ(5.0) + HNT(3.0) = 8.0
        assert_eq!(results.len(), 3);
        assert!((results[0].value - 25.0).abs() < 0.01);
        assert!((results[1].value - 25.0).abs() < 0.01);
        assert!((results[2].value - 8.0).abs() < 0.01);
    }

    #[test]
    fn query_total_snow_empty_range() {
        let db = sample_snow_db();
        let results = db.query_total_snow("20200101", "20200301").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn query_snow_station_history() {
        let db = sample_snow_db();
        let results = db
            .query_snow_station_history("GRZ", "20220101", "20220601")
            .unwrap();
        assert_eq!(results.len(), 3);
        assert!((results[0].value - 15.0).abs() < 0.01); // Jan 1
        assert!((results[1].value - 25.0).abs() < 0.01); // Mar 1
        assert!((results[2].value - 5.0).abs() < 0.01); // Jun 1
    }

    #[test]
    fn query_snow_years_partitions_correctly() {
        let db = sample_snow_db();
        let results = db.query_snow_years("GRZ").unwrap();

        let wy_2022: Vec<&WaterYearData> = results.iter().filter(|r| r.year == 2022).collect();
        let wy_2023: Vec<&WaterYearData> = results.iter().filter(|r| r.year == 2023).collect();

        assert_eq!(wy_2022.len(), 5, "WY 2022 should have 5 observations");
        assert_eq!(wy_2023.len(), 5, "WY 2023 should have 5 observations");
    }

    #[test]
    fn query_snow_year_stats_driest_wettest() {
        let db = sample_snow_db();
        let stats = db.query_snow_year_stats("GRZ").unwrap();
        assert_eq!(stats.len(), 2);

        let wy_2022 = stats.iter().find(|s| s.year == 2022).unwrap();
        let wy_2023 = stats.iter().find(|s| s.year == 2023).unwrap();

        // WY 2022: max SWE = 25.0, min = 0.0
        assert!((wy_2022.highest_value - 25.0).abs() < 0.01);
        assert!((wy_2022.lowest_value - 0.0).abs() < 0.01);

        // WY 2023: max SWE = 35.0, min = 0.0
        assert!((wy_2023.highest_value - 35.0).abs() < 0.01);
        assert!((wy_2023.lowest_value - 0.0).abs() < 0.01);

        // WY 2023 should be wettest (highest max)
        assert!(wy_2023.is_wettest);
        // Both have min of 0.0, so the first one (earlier year) wins for driest
        // due to BTreeMap ordering - this is still valid since they're tied
        assert!(wy_2022.is_driest || wy_2023.is_driest);
    }

    #[test]
    fn query_snow_stations_ordered_by_name() {
        let db = sample_snow_db();
        let stations = db.query_snow_stations().unwrap();
        assert_eq!(stations.len(), 2);
        // Alphabetical: Grizzly Ridge before Huntington Lake
        assert_eq!(stations[0].station_id, "GRZ");
        assert_eq!(stations[1].station_id, "HNT");
        assert_eq!(stations[0].elevation, 5280);
        assert_eq!(stations[0].river_basin, "Feather River");
    }

    // ───────────────────── Integration Tests ─────────────────────

    #[test]
    fn full_water_workflow() {
        let db = sample_water_db();

        // 1. List reservoirs
        let reservoirs = db.query_reservoirs().unwrap();
        assert!(!reservoirs.is_empty());

        // 2. Get date range
        let (min, max) = db.query_date_range().unwrap();
        assert!(!min.is_empty());
        assert!(!max.is_empty());
        assert!(min < max);

        // 3. Get cumulative data
        let cumulative = db.query_total_water(&min, &max).unwrap();
        assert!(!cumulative.is_empty());

        // 4. Get individual reservoir history
        let station_id = &reservoirs[0].station_id;
        let history = db.query_reservoir_history(station_id, &min, &max).unwrap();
        assert!(!history.is_empty());

        // 5. Get water years
        let water_years = db.query_water_years(station_id).unwrap();
        assert!(!water_years.is_empty());

        // 6. Get water year stats
        let stats = db.query_water_year_stats(station_id).unwrap();
        assert!(!stats.is_empty());

        // Verify exactly one driest and one wettest year
        let driest_count = stats.iter().filter(|s| s.is_driest).count();
        let wettest_count = stats.iter().filter(|s| s.is_wettest).count();
        assert_eq!(driest_count, 1, "Should have exactly one driest year");
        assert_eq!(wettest_count, 1, "Should have exactly one wettest year");
    }

    #[test]
    fn full_snow_workflow() {
        let db = sample_snow_db();

        // 1. List stations
        let stations = db.query_snow_stations().unwrap();
        assert!(!stations.is_empty());

        // 2. Get cumulative snow
        let cumulative = db.query_total_snow("20220101", "20220601").unwrap();
        assert!(!cumulative.is_empty());

        // 3. Get station history
        let station_id = &stations[0].station_id;
        let history = db
            .query_snow_station_history(station_id, "20200101", "20250101")
            .unwrap();
        assert!(!history.is_empty());

        // 4. Get snow years
        let snow_years = db.query_snow_years(station_id).unwrap();
        assert!(!snow_years.is_empty());

        // 5. Get snow year stats
        let stats = db.query_snow_year_stats(station_id).unwrap();
        assert!(!stats.is_empty());
    }

    #[test]
    fn water_year_data_day_of_year_is_contiguous() {
        let db = Database::new().unwrap();

        // Create a dense set of daily observations for one month
        let mut obs_lines = String::new();
        for day in 1..=31 {
            obs_lines.push_str(&format!("TST,D,202210{:02},1000\n", day));
        }
        db.load_observations(&obs_lines).unwrap();

        let results = db.query_water_years("TST").unwrap();
        assert_eq!(results.len(), 31);

        // All should be WY 2023
        for r in &results {
            assert_eq!(r.year, 2023);
        }

        // Days should be 0-30
        let days: Vec<i32> = results.iter().map(|r| r.day_of_year).collect();
        for (i, &d) in days.iter().enumerate() {
            assert_eq!(d, i as i32, "Day {} should have day_of_year {}", i, i);
        }
    }

    #[test]
    fn query_snow_date_range() {
        let db = sample_snow_db();
        let (min_date, max_date) = db.query_snow_date_range().unwrap();
        assert_eq!(min_date, "20211001");
        assert_eq!(max_date, "20230930");
    }
}
