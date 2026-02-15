//! In-memory SQLite database layer for California water and snow data.
//!
//! This crate provides a shared database abstraction that loads CSV data
//! into an in-memory SQLite database and exposes typed query methods for
//! consumption by Dioxus/D3.js chart applications compiled to WASM.
//!
//! # Architecture
//!
//! Follows the pattern established in `slave-trade-map/src/db.rs`:
//! - `Rc<RefCell<Connection>>` wrapper for interior mutability in single-threaded WASM
//! - In-memory SQLite via `rusqlite` (compiles to WASM via `wasm32-unknown-unknown`)
//! - CSV data loaded via `include_str!` at compile time in consuming crates
//! - Typed query methods returning serializable structs for JSON export to D3.js
//!
//! # Usage
//!
//! ```rust
//! use cwr_db::Database;
//!
//! let db = Database::new().unwrap();
//!
//! // Load CSV data (typically via include_str! in the consuming crate)
//! db.load_reservoirs("ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL\nSHA,Shasta,Lake Shasta,Sacramento River,4552000,1954\n").unwrap();
//! db.load_observations("SHA,D,20220101,2500000\n").unwrap();
//!
//! // Query typed results
//! let reservoirs = db.query_reservoirs().unwrap();
//! let history = db.query_reservoir_history("SHA", "20220101", "20221231").unwrap();
//! ```
//!
//! # Tables
//!
//! See [`schema::create_schema`] for the full SQL schema.
//!
//! ## Water Tables
//! - `reservoirs` - Station metadata
//! - `observations` - Daily/monthly storage values (acre-feet)
//!
//! ## Snow Tables
//! - `snow_stations` - Snow sensor metadata
//! - `snow_observations` - Snow water equivalent and depth readings
//!
//! Cumulative totals (total water, CA-only water, total snow) are derived
//! on-the-fly via SQL `GROUP BY date` + `SUM(value)` queries against the
//! base observation tables.

pub mod schema;
mod loader;
mod queries;
pub mod models;

use rusqlite::Connection;
use std::cell::RefCell;
use std::rc::Rc;

/// In-memory SQLite database wrapping California water and snow data.
///
/// This struct is cheaply cloneable (via `Rc`) and suitable for sharing
/// across Dioxus components in a single-threaded WASM environment.
///
/// # Example
///
/// ```rust
/// use cwr_db::Database;
///
/// let db = Database::new().unwrap();
/// db.load_reservoirs("ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL\nSHA,Shasta,Lake Shasta,Sacramento River,4552000,1954\n").unwrap();
/// let reservoirs = db.query_reservoirs().unwrap();
/// assert_eq!(reservoirs.len(), 1);
/// ```
#[derive(Clone)]
pub struct Database {
    conn: Rc<RefCell<Connection>>,
}

impl Database {
    /// Create a new in-memory database with the full schema applied.
    ///
    /// The database is empty after creation; use the `load_*` methods
    /// to populate it with CSV data.
    pub fn new() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(schema::create_schema())?;
        Ok(Self {
            conn: Rc::new(RefCell::new(conn)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_creates_successfully() {
        let db = Database::new();
        assert!(db.is_ok(), "Database should create without errors");
    }

    #[test]
    fn database_is_cloneable() {
        let db = Database::new().unwrap();
        let db2 = db.clone();
        // Both should reference the same underlying connection
        db.load_reservoirs(
            "ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL\nSHA,Shasta,Lake Shasta,Sacramento River,4552000,1954\n",
        )
        .unwrap();
        let reservoirs = db2.query_reservoirs().unwrap();
        assert_eq!(
            reservoirs.len(),
            1,
            "Clone should see same data via shared Rc"
        );
    }

    #[test]
    fn database_starts_empty() {
        let db = Database::new().unwrap();
        let reservoirs = db.query_reservoirs().unwrap();
        assert!(reservoirs.is_empty(), "New database should have no reservoirs");
    }
}
