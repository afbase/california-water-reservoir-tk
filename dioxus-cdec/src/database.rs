use dioxus_logger::tracing::info;
use rusqlite::Connection;
use std::rc::Rc;

const COMPRESSED_DB: &[u8] = include_bytes!("../data/reservoir_data.db.zst");

#[derive(Clone)]
pub struct Database {
    conn: Rc<Connection>,
    // Keep the decompressed buffer alive for the lifetime of the connection
    _buffer: Rc<Vec<u8>>,
}

// Manual PartialEq since Connection doesn't implement it
impl PartialEq for Database {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.conn, &other.conn)
    }
}

impl Database {
    pub async fn new() -> Result<Self, String> {
        info!("Decompressing SQLite database...");

        // Decompress the database
        let decompressed = zstd::decode_all(COMPRESSED_DB)
            .map_err(|e| format!("Failed to decompress database: {}", e))?;

        info!("Database decompressed, size: {} bytes", decompressed.len());

        // Open SQLite database from memory
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to create in-memory database: {}", e))?;

        // Deserialize the database from bytes
        unsafe {
            let db_handle = conn.handle();
            let result = rusqlite::ffi::sqlite3_deserialize(
                db_handle,
                b"main\0".as_ptr() as *const i8,
                decompressed.as_ptr() as *mut u8,
                decompressed.len() as i64,
                decompressed.len() as i64,
                rusqlite::ffi::SQLITE_DESERIALIZE_READONLY,
            );

            if result != rusqlite::ffi::SQLITE_OK {
                return Err(format!("Failed to deserialize database: {}", result));
            }
        }

        info!("SQLite database loaded successfully");

        Ok(Database {
            conn: Rc::new(conn),
            // Keep buffer alive - SQLite's READONLY mode doesn't take ownership
            _buffer: Rc::new(decompressed),
        })
    }

    pub async fn get_date_range(&self) -> Result<(String, String), String> {
        let conn = &self.conn;

        let min_date: String = conn
            .query_row(
                "SELECT date FROM statewide_observations ORDER BY date ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get min date: {}", e))?;

        let max_date: String = conn
            .query_row(
                "SELECT date FROM statewide_observations ORDER BY date DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get max date: {}", e))?;

        Ok((min_date, max_date))
    }

    pub async fn get_data(&self, start_date: &str, end_date: &str) -> Result<Vec<(String, u32)>, String> {
        let conn = &self.conn;

        let mut stmt = conn
            .prepare("SELECT date, water_level FROM statewide_observations WHERE date >= ?1 AND date <= ?2 ORDER BY date ASC")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map([start_date, end_date], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut data = Vec::new();
        for row in rows {
            let (date, water_level) = row.map_err(|e| format!("Failed to read row: {}", e))?;
            data.push((date, water_level));
        }

        info!("Retrieved {} data points for range {} to {}", data.len(), start_date, end_date);
        Ok(data)
    }

    pub async fn get_reservoirs(&self) -> Result<Vec<Reservoir>, String> {
        let conn = &self.conn;

        let mut stmt = conn
            .prepare("SELECT station_id, dam_name, lake_name, stream_name, capacity, year_fill FROM reservoirs ORDER BY lake_name")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Reservoir {
                    station_id: row.get(0)?,
                    dam_name: row.get(1)?,
                    lake_name: row.get(2)?,
                    stream_name: row.get(3)?,
                    capacity: row.get(4)?,
                    year_fill: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut reservoirs = Vec::new();
        for row in rows {
            reservoirs.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }

        info!("Retrieved {} reservoirs", reservoirs.len());
        Ok(reservoirs)
    }

    pub async fn get_reservoir_data(
        &self,
        station_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<(String, u32)>, String> {
        let conn = &self.conn;

        let mut stmt = conn
            .prepare(
                "SELECT date, water_level FROM reservoir_observations \
                 WHERE station_id = ?1 AND date >= ?2 AND date <= ?3 \
                 ORDER BY date ASC"
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map([station_id, start_date, end_date], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut data = Vec::new();
        for row in rows {
            let (date, water_level) = row.map_err(|e| format!("Failed to read row: {}", e))?;
            data.push((date, water_level));
        }

        info!("Retrieved {} data points for reservoir {} from {} to {}",
              data.len(), station_id, start_date, end_date);
        Ok(data)
    }

    pub async fn get_reservoir_date_range(&self, station_id: &str) -> Result<(String, String), String> {
        let conn = &self.conn;

        let min_date: String = conn
            .query_row(
                "SELECT date FROM reservoir_observations WHERE station_id = ?1 ORDER BY date ASC LIMIT 1",
                [station_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get min date: {}", e))?;

        let max_date: String = conn
            .query_row(
                "SELECT date FROM reservoir_observations WHERE station_id = ?1 ORDER BY date DESC LIMIT 1",
                [station_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get max date: {}", e))?;

        Ok((min_date, max_date))
    }
}

#[derive(Clone, Debug)]
pub struct Reservoir {
    pub station_id: String,
    pub dam_name: Option<String>,
    pub lake_name: Option<String>,
    pub stream_name: Option<String>,
    pub capacity: Option<i32>,
    pub year_fill: Option<i32>,
}
