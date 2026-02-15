//! SQL schema definitions for the in-memory SQLite database.
//!
//! Contains CREATE TABLE statements for all water and snow tables.
//! The schema is applied as a single batch when the database is initialized.

/// Returns the full SQL schema as a single batch string.
///
/// This creates the following tables:
///
/// **Water tables:**
/// - `reservoirs` - Reservoir metadata (station ID, dam, lake, stream, capacity, fill year)
/// - `observations` - Individual station storage observations (station_id, date, value in AF)
///
/// **Snow tables:**
/// - `snow_stations` - Snow sensor metadata (station ID, name, elevation, basin, county, lat/lon)
/// - `snow_observations` - Snow sensor readings (station_id, date, SWE, depth)
///
/// Cumulative totals (total water, CA-only water, total snow) are derived on-the-fly
/// via SQL `GROUP BY date` + `SUM(value)` queries against these base tables.
pub fn create_schema() -> &'static str {
    r#"
    CREATE TABLE IF NOT EXISTS reservoirs (
        station_id TEXT PRIMARY KEY,
        dam TEXT NOT NULL,
        lake TEXT NOT NULL,
        stream TEXT NOT NULL,
        capacity INTEGER NOT NULL,
        fill_year INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS observations (
        station_id TEXT NOT NULL,
        date TEXT NOT NULL,
        value REAL NOT NULL,
        PRIMARY KEY (station_id, date)
    );
    CREATE INDEX IF NOT EXISTS idx_obs_station ON observations(station_id);
    CREATE INDEX IF NOT EXISTS idx_obs_date ON observations(date);

    CREATE TABLE IF NOT EXISTS snow_stations (
        station_id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        elevation INTEGER NOT NULL,
        river_basin TEXT,
        county TEXT,
        latitude REAL,
        longitude REAL
    );

    CREATE TABLE IF NOT EXISTS snow_observations (
        station_id TEXT NOT NULL,
        date TEXT NOT NULL,
        snow_water_equivalent REAL,
        snow_depth REAL,
        PRIMARY KEY (station_id, date)
    );
    CREATE INDEX IF NOT EXISTS idx_snow_obs_station ON snow_observations(station_id);
    CREATE INDEX IF NOT EXISTS idx_snow_obs_date ON snow_observations(date);

    "#
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn schema_is_valid_sql() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(create_schema())
            .expect("Schema SQL should be valid");
    }

    #[test]
    fn schema_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(create_schema()).unwrap();

        let expected_tables = [
            "reservoirs",
            "observations",
            "snow_stations",
            "snow_observations",
        ];

        for table in &expected_tables {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Table '{}' should exist", table);
        }
    }

    #[test]
    fn schema_creates_indexes() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(create_schema()).unwrap();

        let expected_indexes = [
            "idx_obs_station",
            "idx_obs_date",
            "idx_snow_obs_station",
            "idx_snow_obs_date",
        ];

        for idx in &expected_indexes {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{}'",
                        idx
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Index '{}' should exist", idx);
        }
    }

    #[test]
    fn schema_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(create_schema()).unwrap();
        // Applying schema a second time should not fail due to IF NOT EXISTS.
        conn.execute_batch(create_schema())
            .expect("Applying schema twice should succeed due to IF NOT EXISTS");
    }
}
