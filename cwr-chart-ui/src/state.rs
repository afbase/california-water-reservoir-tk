//! Application state managed via Dioxus context.
//!
//! `AppState` bundles all reactive signals into a single struct provided via
//! `use_context_provider`. Child components retrieve it with `use_context::<AppState>()`.

use cwr_db::models::{ReservoirInfo, SnowStationInfo};
use cwr_db::Database;
use dioxus::prelude::*;

/// Shared application state for all CWR chart apps.
#[derive(Clone, Copy)]
pub struct AppState {
    /// Database instance (None until loaded)
    pub db: Signal<Option<Database>>,
    /// Whether the app is still loading
    pub loading: Signal<bool>,
    /// Error message if something went wrong
    pub error_msg: Signal<Option<String>>,
    /// Currently selected reservoir station ID
    pub selected_station: Signal<String>,
    /// Available reservoirs
    pub reservoirs: Signal<Vec<ReservoirInfo>>,
    /// Available snow stations
    pub snow_stations: Signal<Vec<SnowStationInfo>>,
    /// Start date for date range filtering
    pub start_date: Signal<String>,
    /// End date for date range filtering
    pub end_date: Signal<String>,
    /// Sort mode for water year display ("driest", "wettest", "most_recent")
    pub sort_mode: Signal<String>,
    /// Number of years to display
    pub display_count: Signal<usize>,
}

impl AppState {
    /// Create a new AppState with default signal values.
    pub fn new() -> Self {
        Self {
            db: Signal::new(None),
            loading: Signal::new(true),
            error_msg: Signal::new(None),
            selected_station: Signal::new("SHA".to_string()),
            reservoirs: Signal::new(Vec::new()),
            snow_stations: Signal::new(Vec::new()),
            start_date: Signal::new(String::new()),
            end_date: Signal::new(String::new()),
            sort_mode: Signal::new("most_recent".to_string()),
            display_count: Signal::new(20),
        }
    }
}
