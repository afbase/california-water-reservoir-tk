use dioxus_logger::tracing::info;
use rusqlite::Connection;
use std::rc::Rc;

const COMPRESSED_DB: &[u8] = include_bytes!("../data/reservoir_data.db.zst");

#[derive(Clone)]
pub struct Database {
    conn: Rc<Connection>,
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

            // Prevent decompressed from being freed
            std::mem::forget(decompressed);
        }

        info!("SQLite database loaded successfully");

        Ok(Database {
            conn: Rc::new(conn),
        })
    }

    pub async fn get_date_range(&self) -> Result<(String, String), String> {
        let conn = &self.conn;

        let min_date: String = conn
            .query_row(
                "SELECT date FROM observations ORDER BY date ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get min date: {}", e))?;

        let max_date: String = conn
            .query_row(
                "SELECT date FROM observations ORDER BY date DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get max date: {}", e))?;

        Ok((min_date, max_date))
    }

    pub async fn get_data(&self, start_date: &str, end_date: &str) -> Result<Vec<(String, u32)>, String> {
        let conn = &self.conn;

        let mut stmt = conn
            .prepare("SELECT date, water_level FROM observations WHERE date >= ?1 AND date <= ?2 ORDER BY date ASC")
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
}
