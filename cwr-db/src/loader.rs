//! CSV data loading functions for populating the in-memory SQLite database.
//!
//! Each loader method parses CSV data from a string slice and inserts rows
//! into the corresponding table. The CSV formats match the fixture files
//! produced by the CLI query tool and the CDEC API data pipeline.
//!
//! # CSV Formats
//!
//! - **Reservoirs** (has headers): `ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL`
//! - **Observations** (no headers): `station_id,duration,date(YYYYMMDD),value`
//! - **Snow stations** (has headers): `ID,NAME,ELEVATION,RIVER_BASIN,COUNTY,LATITUDE,LONGITUDE`
//! - **Snow observations** (no headers): `station_id,date(YYYYMMDD),swe,depth`

use crate::Database;
use rusqlite::params;

impl Database {
    /// Load reservoir metadata from CSV string.
    ///
    /// Expected format (with headers): `ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL`
    ///
    /// # Example CSV
    /// ```text
    /// ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
    /// SHA,Shasta,Lake Shasta,Sacramento River,4552000,1954
    /// ```
    pub fn load_reservoirs(&self, csv_data: &str) -> anyhow::Result<()> {
        let conn = self.conn.borrow();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(csv_data.as_bytes());

        let mut count = 0u32;
        for result in rdr.records() {
            let r = result?;
            let station_id = r.get(0).unwrap_or("").trim();
            let dam = r.get(1).unwrap_or("").trim();
            let lake = r.get(2).unwrap_or("").trim();
            let stream = r.get(3).unwrap_or("").trim();
            let capacity: i64 = r.get(4).unwrap_or("0").trim().parse()?;
            let fill_year: i64 = r.get(5).unwrap_or("0").trim().parse()?;

            conn.execute(
                "INSERT OR REPLACE INTO reservoirs (station_id, dam, lake, stream, capacity, fill_year)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![station_id, dam, lake, stream, capacity, fill_year],
            )?;
            count += 1;
        }
        log::info!("[CWR Debug] loader: Loaded {} reservoirs", count);
        Ok(())
    }

    /// Load water observations from CSV string.
    ///
    /// Expected format (no headers): `station_id,duration,date(YYYYMMDD),value`
    ///
    /// The `duration` field is either `D` (daily) or `M` (monthly) and is not stored;
    /// only the station_id, date, and numeric value are persisted. Rows with
    /// non-numeric values (e.g., `ART`, `BRT`, `---`) are skipped.
    ///
    /// # Example CSV
    /// ```text
    /// SHA,M,19631031,2828000
    /// SHA,D,20220218,2100000
    /// ```
    pub fn load_observations(&self, csv_data: &str) -> anyhow::Result<()> {
        let conn = self.conn.borrow();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(csv_data.as_bytes());

        let mut count = 0u32;
        let mut skipped = 0u32;
        for result in rdr.records() {
            let r = result?;
            let station_id = r.get(0).unwrap_or("").trim();
            // field 1 is duration (D or M) -- we don't store it
            let date = r.get(2).unwrap_or("").trim();
            let value_str = r.get(3).unwrap_or("").trim();

            // Skip non-numeric values (ART, BRT, ---)
            let value: f64 = match value_str.parse::<f64>() {
                Ok(v) => v,
                Err(_) => { skipped += 1; continue; }
            };

            // Skip if station_id or date is empty
            if station_id.is_empty() || date.is_empty() {
                skipped += 1;
                continue;
            }

            conn.execute(
                "INSERT OR REPLACE INTO observations (station_id, date, value)
                 VALUES (?1, ?2, ?3)",
                params![station_id, date, value],
            )?;
            count += 1;
        }
        log::info!("[CWR Debug] loader: Loaded {} observations, skipped {} non-numeric", count, skipped);
        Ok(())
    }

    /// Load snow station metadata from CSV string.
    ///
    /// Expected format (with headers): `ID,NAME,ELEVATION,RIVER_BASIN,COUNTY,LATITUDE,LONGITUDE`
    ///
    /// # Example CSV
    /// ```text
    /// ID,NAME,ELEVATION,RIVER_BASIN,COUNTY,LATITUDE,LONGITUDE
    /// GRZ,Grizzly Ridge,5280,Feather River,Plumas,39.95,-120.68
    /// ```
    pub fn load_snow_stations(&self, csv_data: &str) -> anyhow::Result<()> {
        let conn = self.conn.borrow();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(csv_data.as_bytes());

        let mut count = 0u32;
        for result in rdr.records() {
            let r = result?;
            let station_id = r.get(0).unwrap_or("").trim();
            let name = r.get(1).unwrap_or("").trim();
            let elevation: i64 = r.get(2).unwrap_or("0").trim().parse().unwrap_or(0);
            let river_basin = r.get(3).unwrap_or("").trim();
            let county = r.get(4).unwrap_or("").trim();
            let latitude: Option<f64> = r.get(5).and_then(|s| s.trim().parse().ok());
            let longitude: Option<f64> = r.get(6).and_then(|s| s.trim().parse().ok());

            conn.execute(
                "INSERT OR REPLACE INTO snow_stations
                 (station_id, name, elevation, river_basin, county, latitude, longitude)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![station_id, name, elevation, river_basin, county, latitude, longitude],
            )?;
            count += 1;
        }
        log::info!("[CWR Debug] loader: Loaded {} snow stations", count);
        Ok(())
    }

    /// Load snow observations from CSV string.
    ///
    /// Expected format (no headers): `station_id,date(YYYYMMDD),swe,depth`
    ///
    /// Both `swe` (snow water equivalent) and `depth` may be empty or non-numeric,
    /// in which case they are stored as NULL. Rows where both values are missing
    /// are skipped entirely.
    ///
    /// # Example CSV
    /// ```text
    /// GRZ,20220101,12.5,36.0
    /// GRZ,20220102,13.0,
    /// ```
    pub fn load_snow_observations(&self, csv_data: &str) -> anyhow::Result<()> {
        let conn = self.conn.borrow();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(csv_data.as_bytes());

        let mut count = 0u32;
        let mut skipped = 0u32;
        for result in rdr.records() {
            let r = result?;
            let station_id = r.get(0).unwrap_or("").trim();
            let date = r.get(1).unwrap_or("").trim();
            let swe: Option<f64> = r.get(2).and_then(|s| s.trim().parse().ok());
            let depth: Option<f64> = r.get(3).and_then(|s| s.trim().parse().ok());

            if station_id.is_empty() || date.is_empty() {
                skipped += 1;
                continue;
            }

            // Skip rows where both values are missing
            if swe.is_none() && depth.is_none() {
                skipped += 1;
                continue;
            }

            conn.execute(
                "INSERT OR REPLACE INTO snow_observations
                 (station_id, date, snow_water_equivalent, snow_depth)
                 VALUES (?1, ?2, ?3, ?4)",
                params![station_id, date, swe, depth],
            )?;
            count += 1;
        }
        log::info!("[CWR Debug] loader: Loaded {} snow observations, skipped {} invalid", count, skipped);
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use crate::Database;

    #[test]
    fn load_reservoirs_from_csv() {
        let db = Database::new().unwrap();
        let csv = "\
ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
SHA,Shasta,Lake Shasta,Sacramento River,4552000,1954
ORO,Oroville,Lake Oroville,Feather River,3537577,1969
";
        db.load_reservoirs(csv).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM reservoirs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let dam: String = conn
            .query_row(
                "SELECT dam FROM reservoirs WHERE station_id = 'SHA'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dam, "Shasta");

        let capacity: i64 = conn
            .query_row(
                "SELECT capacity FROM reservoirs WHERE station_id = 'ORO'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(capacity, 3537577);
    }

    #[test]
    fn load_reservoirs_replaces_on_conflict() {
        let db = Database::new().unwrap();
        let csv1 = "\
ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
SHA,Shasta,Lake Shasta,Sacramento River,4552000,1954
";
        let csv2 = "\
ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL
SHA,Shasta Updated,Lake Shasta,Sacramento River,4552000,1954
";
        db.load_reservoirs(csv1).unwrap();
        db.load_reservoirs(csv2).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM reservoirs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "Should have 1 row after upsert");

        let dam: String = conn
            .query_row(
                "SELECT dam FROM reservoirs WHERE station_id = 'SHA'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dam, "Shasta Updated");
    }

    #[test]
    fn load_observations_from_csv() {
        let db = Database::new().unwrap();
        let csv = "\
SHA,M,19631031,2828000
SHA,D,20220218,2100000
ORO,M,19690101,500000
";
        db.load_observations(csv).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);

        let value: f64 = conn
            .query_row(
                "SELECT value FROM observations WHERE station_id = 'SHA' AND date = '19631031'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!((value - 2828000.0).abs() < 0.01);
    }

    #[test]
    fn load_observations_skips_non_numeric() {
        let db = Database::new().unwrap();
        let csv = "\
SHA,D,20220101,1000
SHA,D,20220102,ART
SHA,D,20220103,BRT
SHA,D,20220104,---
SHA,D,20220105,2000
";
        db.load_observations(csv).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Should only load rows with numeric values");
    }

    #[test]
    fn load_snow_stations_from_csv() {
        let db = Database::new().unwrap();
        let csv = "\
ID,NAME,ELEVATION,RIVER_BASIN,COUNTY,LATITUDE,LONGITUDE
GRZ,Grizzly Ridge,5280,Feather River,Plumas,39.95,-120.68
HNT,Huntington Lake,7000,San Joaquin River,Fresno,37.23,-119.22
";
        db.load_snow_stations(csv).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snow_stations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let name: String = conn
            .query_row(
                "SELECT name FROM snow_stations WHERE station_id = 'GRZ'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Grizzly Ridge");
    }

    #[test]
    fn load_snow_observations_from_csv() {
        let db = Database::new().unwrap();
        let csv = "\
GRZ,20220101,12.5,36.0
GRZ,20220102,13.0,37.5
GRZ,20220103,,
";
        db.load_snow_observations(csv).unwrap();

        let conn = db.conn.borrow();
        // The third row has both values missing, so it should be skipped
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snow_observations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "Should skip rows where both SWE and depth are missing");
    }

    #[test]
    fn load_snow_observations_partial_values() {
        let db = Database::new().unwrap();
        // Only SWE provided, no depth
        let csv = "\
GRZ,20220101,12.5,
";
        db.load_snow_observations(csv).unwrap();

        let conn = db.conn.borrow();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snow_observations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Should load row with partial values");

        let depth: Option<f64> = conn
            .query_row(
                "SELECT snow_depth FROM snow_observations WHERE station_id = 'GRZ'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(depth.is_none(), "Depth should be NULL when not provided");
    }

}
