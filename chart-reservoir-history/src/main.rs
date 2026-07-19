//! Historical Water Levels by Reservoir
//!
//! Displays a multi-line chart showing historical water levels for selected
//! reservoirs. The user picks a reservoir from a dropdown and optionally
//! adjusts a date range; the chart renders one line per selected reservoir
//! in the chosen time window.
//!
//! This replaces the former `yew-avin_a_laf` crate with an equivalent
//! Dioxus 0.7 + D3.js implementation.
//!
//! Data flow:
//! 1. `build.rs` copies `capacity.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds this CSV into the WASM binary.
//! 3. On mount, the CSV is loaded into an in-memory SQLite database. The
//!    selected reservoir's per-station observation file is then fetched on
//!    demand (and whenever the selection changes) via `fetch_observations_br`.
//! 4. When the user selects a reservoir and date range, the app queries
//!    `query_all_reservoir_histories()` and renders a multi-line chart.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, DateRangePicker, ErrorDisplay, LoadingSpinner, ReservoirSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;
use std::collections::HashSet;

/// All reservoir metadata including Mead/Powell.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "reservoir-history-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("reservoir-history-root"))
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

                    // Seed the date range defaults from the loaded data (only if unset).
                    if state.start_date.peek().is_empty() || state.end_date.peek().is_empty() {
                        if let Ok((min_date, max_date)) = db.query_date_range() {
                            if min_date.len() == 8 {
                                state.start_date.set(format!(
                                    "{}-{}-{}",
                                    &min_date[0..4],
                                    &min_date[4..6],
                                    &min_date[6..8]
                                ));
                            }
                            if max_date.len() == 8 {
                                state.end_date.set(format!(
                                    "{}-{}-{}",
                                    &max_date[0..4],
                                    &max_date[4..6],
                                    &max_date[6..8]
                                ));
                            }
                        }
                    }

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

    // Re-render chart whenever selection or date range changes
    use_effect(move || {
        let loading_state = (state.loading)();

        if loading_state {
            return;
        }

        let error_state = (state.error_msg)().is_some();

        if error_state {
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => {
                return;
            }
        };

        let station = (state.selected_station)();
        let start_date_html = (state.start_date)();
        let end_date_html = (state.end_date)();

        if station.is_empty() || start_date_html.is_empty() || end_date_html.is_empty() {
            return;
        }

        // Convert YYYY-MM-DD back to YYYYMMDD for DB queries
        let start_date = start_date_html.replace('-', "");
        let end_date = end_date_html.replace('-', "");

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        // Query the selected reservoir's history within the date range
        let data = match db.query_reservoir_history(&station, &start_date, &end_date) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Reservoir history query failed: {}", e);
                return;
            }
        };

        if data.is_empty() {
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
            js_bridge::destroy_chart(CHART_ID);
            return;
        }
        // Clear any previous error when data IS available
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        // Find the reservoir name for the chart title
        let reservoir_name = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == station)
            .map(|r| format!("{} ({})", r.dam, r.station_id))
            .unwrap_or_else(|| station.clone());

        // Find capacity for the selected reservoir
        let capacity = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == station)
            .map(|r| r.capacity)
            .unwrap_or(0);

        // Wrap single reservoir data as StationDateValue-like structure for multi-line chart
        let station_data: Vec<serde_json::Value> = data
            .iter()
            .map(|dv| {
                serde_json::json!({
                    "station_id": station,
                    "date": dv.date,
                    "value": dv.value,
                })
            })
            .collect();

        let data_json = serde_json::to_string(&station_data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Water Levels: {}", reservoir_name),
            "yAxisLabel": "Acre-Feet (AF)",
            "dateFormat": "YYYYMMDD",
            "tooltipFormat": "station_date_value",
            "valueLabel": "Storage (AF)",
            "capacity": capacity,
            "showCapacityLine": capacity > 0,
        }))
        .unwrap_or_default();

        js_bridge::render_multi_line_chart(CHART_ID, &data_json, &config_json);
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Historical Water Levels by Reservoir".to_string(),
                unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year".to_string(),
            }

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                div {
                    style: "display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; margin-bottom: 8px;",
                    ReservoirSelector {}
                    DateRangePicker {}
                }

                ChartContainer {
                    id: CHART_ID.to_string(),
                    loading: false,
                    min_height: 450,
                }
            }
        }
    }
}
