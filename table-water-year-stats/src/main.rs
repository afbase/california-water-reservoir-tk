//! Water Year Statistics Table
//!
//! Displays a sortable table of water year statistics per reservoir, showing
//! the lowest and highest observed storage values for each water year.
//! Rows for the driest and wettest years are dynamically highlighted (not
//! hard-coded to any specific years like the old 1976/1977 approach).
//!
//! This replaces the former `yew-nani` crate and FIXES THE BUG where
//! driest/wettest year highlighting was hard-coded. Now the table
//! dynamically determines which year had the lowest minimum (driest)
//! and which had the highest maximum (wettest) for the selected reservoir.
//!
//! Data flow:
//! 1. `build.rs` copies `capacity.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds this CSV into the WASM binary.
//! 3. On mount, the CSV is loaded into an in-memory SQLite database. The
//!    selected reservoir's per-station observation file is then fetched on
//!    demand (and whenever the selection changes) via `fetch_observations_br`.
//! 4. When the user selects a reservoir, `query_water_year_stats()` is called
//!    and the results are passed to `renderDataTable()` for D3.js rendering.

use cwr_chart_ui::components::{
    ChartCard, ChartContainer, ErrorDisplay, LoadingSpinner, ReservoirSelector,
};
use cwr_chart_ui::dates::compact_to_iso;
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;
use std::collections::HashSet;

/// All reservoir metadata.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));

/// Table container DOM element ID used by D3.js to render into.
const TABLE_ID: &str = "water-year-stats-table";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("water-year-stats-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut state = use_context_provider(AppState::new);
    // Tracks which stations have already been fetched into the DB, so switching
    // back to a previously viewed reservoir does not refetch its data.
    let mut loaded_stations = use_signal(HashSet::<String>::new);

    // Initialize database + reservoir metadata on mount (no observations yet;
    // those are fetched per-station when a reservoir is selected).
    use_effect(move || {
        spawn(async move {
            match Database::new() {
                Ok(db) => {
                    if let Err(e) = db.load_reservoirs(CAPACITY_CSV) {
                        log::error!("Failed to load reservoirs: {}", e);
                        state
                            .error_msg
                            .set(Some(format!("Failed to load reservoir data: {}", e)));
                        state.loading.set(false);
                        return;
                    }

                    // Populate reservoir list for the dropdown
                    if let Ok(reservoirs) = db.query_reservoirs() {
                        let default_station = reservoirs
                            .iter()
                            .find(|r| r.station_id == "ORO")
                            .or_else(|| reservoirs.first())
                            .map(|r| r.station_id.clone())
                            .unwrap_or_default();

                        if !default_station.is_empty() {
                            state.selected_station.set(default_station);
                        }
                        state.reservoirs.set(reservoirs);
                    }

                    state.db.set(Some(db));
                }
                Err(e) => {
                    state
                        .error_msg
                        .set(Some(format!("Database initialization failed: {}", e)));
                    state.loading.set(false);
                }
            }
        });
    });

    // Fetch the selected station's observations whenever the selection changes.
    use_effect(move || {
        let station = (state.selected_station)();
        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => return,
        };
        if station.is_empty() {
            return;
        }
        // Skip stations we've already fetched (peek so this effect does not
        // re-trigger itself when the loaded set is updated below).
        if loaded_stations.peek().contains(&station) {
            return;
        }

        state.loading.set(true);
        spawn(async move {
            match js_bridge::fetch_observations_br(&station).await {
                Ok(csv) => {
                    if let Err(e) = db.load_observations(&csv) {
                        log::error!("Failed to load observations for {}: {}", station, e);
                        state
                            .error_msg
                            .set(Some(format!("Failed to load observations: {}", e)));
                        state.loading.set(false);
                        return;
                    }
                    loaded_stations.write().insert(station.clone());
                    state.loading.set(false);
                }
                Err(e) => {
                    state
                        .error_msg
                        .set(Some(format!("Failed to fetch observation data: {}", e)));
                    state.loading.set(false);
                }
            }
        });
    });

    // Re-render table whenever reservoir selection changes
    use_effect(move || {
        if (state.loading)() {
            return;
        }
        if (state.error_msg)().is_some() {
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => return,
        };

        let station = (state.selected_station)();

        if station.is_empty() {
            return;
        }

        // Query water year stats (already has is_driest/is_wettest computed dynamically)
        let stats = match db.query_water_year_stats(&station) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to query water year stats: {}", e);
                return;
            }
        };

        if stats.is_empty() {
            let reservoir_name = state
                .reservoirs
                .read()
                .iter()
                .find(|r| r.station_id == station)
                .map(|r| format!("{} ({})", r.dam, r.station_id))
                .unwrap_or_else(|| station.clone());
            state.error_msg.set(Some(format!(
                "No observation data available for {}. This reservoir may not have data in our database yet. Please select another reservoir from the dropdown.",
                reservoir_name
            )));
            js_bridge::destroy_chart(TABLE_ID);
            return;
        }
        // Clear any previous error when data IS available
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        // Reshape stats for the table, with ISO dates
        let table_data: Vec<serde_json::Value> = stats
            .iter()
            .map(|s| {
                serde_json::json!({
                    "year": s.year,
                    "date_lowest": compact_to_iso(&s.date_lowest),
                    "lowest_value": s.lowest_value,
                    "date_highest": compact_to_iso(&s.date_highest),
                    "highest_value": s.highest_value,
                    "is_driest": s.is_driest,
                    "is_wettest": s.is_wettest,
                })
            })
            .collect();

        // Find reservoir name for context
        let reservoir_name = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == station)
            .map(|r| format!("{} ({})", r.dam, r.station_id))
            .unwrap_or_else(|| station.clone());

        let data_json = serde_json::to_string(&table_data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Water Year Statistics: {}", reservoir_name),
            "yUnit": "AF",
            "columns": [
                {"key": "year", "label": "Water Year", "type": "water_year"},
                {"key": "date_highest", "label": "Date of Highest", "type": "date"},
                {"key": "highest_value", "label": "Highest Storage", "type": "number"},
                {"key": "date_lowest", "label": "Date of Lowest", "type": "date"},
                {"key": "lowest_value", "label": "Lowest Storage", "type": "number"},
            ],
        }))
        .unwrap_or_default();

        js_bridge::render_data_table(TABLE_ID, &data_json, &config_json);
    });

    rsx! {
        ChartCard {
            title: "Water Year Statistics",
            unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year",

            // Errors are shown alongside the controls (not instead of them) so
            // the reader can always pick a different station or range.
            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            }

            if (state.loading)() {
                LoadingSpinner {}
            } else {
                div {
                    class: "cwr-controls",
                    ReservoirSelector {}
                }

                ChartContainer { id: TABLE_ID, min_height: 300 }
            }
        }
    }
}
