//! Historical Snow Water Equivalent by Station
//!
//! Displays a multi-line chart showing historical SWE levels for a selected
//! snow station. The user picks a station from a dropdown and optionally
//! adjusts a date range; the chart renders the station's SWE history
//! in the chosen time window.
//!
//! Data flow:
//! 1. `build.rs` copies `snow_stations.csv` and `snow_observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a station and date range, the app queries
//!    `query_snow_station_history()` and renders a multi-line chart.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, DateRangePicker, ErrorDisplay, LoadingSpinner, SnowStationSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;
use wasm_bindgen::JsValue;

/// All snow station metadata.
const SNOW_STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_stations.csv"));
/// Daily snow observation data for all stations.
const SNOW_OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_observations.csv"));

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "snow-history-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("snow-history-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    // CRITICAL DEBUG: This fires immediately when component mounts
    web_sys::console::log_1(&"[CWR CRITICAL] snow-history App component mounted".into());

    let mut state = use_context_provider(AppState::new);

    // Initialize database on mount
    use_effect(move || {
        match Database::new() {
            Ok(db) => {
                if let Err(e) = db.load_snow_stations(SNOW_STATIONS_CSV) {
                    log::error!("Failed to load snow stations: {}", e);
                    state
                        .error_msg
                        .set(Some(format!("Failed to load snow station data: {}", e)));
                    state.loading.set(false);
                    return;
                }
                if !SNOW_OBSERVATIONS_CSV.is_empty() {
                    if let Err(e) = db.load_snow_observations(SNOW_OBSERVATIONS_CSV) {
                        log::error!("Failed to load snow observations: {}", e);
                        state
                            .error_msg
                            .set(Some(format!("Failed to load snow observations: {}", e)));
                        state.loading.set(false);
                        return;
                    }
                }

                // Populate snow station list for the dropdown
                if let Ok(stations) = db.query_snow_stations() {
                    let default_station = stations.first()
                        .map(|s| s.station_id.clone())
                        .unwrap_or_default();

                    if !default_station.is_empty() {
                        web_sys::console::log_1(&format!("[CWR Debug] snow-history: Default selection: {}", default_station).into());
                        state.selected_station.set(default_station);
                    }
                    state.snow_stations.set(stations);
                }

                // Set default date range from the available snow data
                if let Ok((min_date, max_date)) = db.query_snow_date_range() {
                    // Convert YYYYMMDD to YYYY-MM-DD for HTML date inputs
                    if min_date.len() == 8 {
                        let formatted_min = format!(
                            "{}-{}-{}",
                            &min_date[0..4],
                            &min_date[4..6],
                            &min_date[6..8]
                        );
                        state.start_date.set(formatted_min);
                    }
                    if max_date.len() == 8 {
                        let formatted_max = format!(
                            "{}-{}-{}",
                            &max_date[0..4],
                            &max_date[4..6],
                            &max_date[6..8]
                        );
                        state.end_date.set(formatted_max);
                    }
                }

                state.db.set(Some(db));
                state.loading.set(false);
            }
            Err(e) => {
                state
                    .error_msg
                    .set(Some(format!("Database initialization failed: {}", e)));
                state.loading.set(false);
            }
        }
    });

    // Re-render chart whenever selection or date range changes
    use_effect(move || {
        web_sys::console::log_1(&"[CWR CRITICAL] use_effect triggered".into());
        web_sys::console::log_1(&"[CWR Debug Rust] snow-history use_effect triggered".into());

        let loading_state = (state.loading)();
        web_sys::console::log_1(&format!("[CWR CRITICAL] loading={}", loading_state).into());

        if loading_state {
            web_sys::console::log_1(&"[CWR Debug Rust] Exiting: still loading".into());
            return;
        }

        let error_state = (state.error_msg)().is_some();
        web_sys::console::log_1(&format!("[CWR CRITICAL] has_error={}", error_state).into());

        if error_state {
            web_sys::console::log_1(&"[CWR Debug Rust] Exiting: error present".into());
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => {
                web_sys::console::log_1(&"[CWR Debug Rust] Database available".into());
                db.clone()
            }
            None => {
                web_sys::console::log_1(&"[CWR Debug Rust] Exiting: no database".into());
                return;
            }
        };

        let station = (state.selected_station)();
        let start_date_html = (state.start_date)();
        let end_date_html = (state.end_date)();
        web_sys::console::log_1(&format!("[CWR Debug Rust] Selected station: {}", station).into());

        if station.is_empty() || start_date_html.is_empty() || end_date_html.is_empty() {
            web_sys::console::log_1(&"[CWR Debug Rust] Exiting: empty station or date range".into());
            return;
        }

        // Convert YYYY-MM-DD back to YYYYMMDD for DB queries
        let start_date = start_date_html.replace('-', "");
        let end_date = end_date_html.replace('-', "");

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        web_sys::console::log_1(&format!("[CWR Debug Rust] Querying snow station history for: {}", station).into());
        // Query the selected station's history within the date range
        let data = match db.query_snow_station_history(&station, &start_date, &end_date) {
            Ok(d) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Query returned {} records", d.len()).into());
                d
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Query failed: {}", e).into());
                return;
            }
        };

        if data.is_empty() {
            web_sys::console::log_1(&"[CWR Debug Rust] No data returned, destroying chart".into());
            let station_name = state.snow_stations.read().iter()
                .find(|s| s.station_id == station)
                .map(|s| format!("{} ({})", s.name, s.station_id))
                .unwrap_or_else(|| station.clone());
            state.error_msg.set(Some(format!(
                "No observation data available for {}. This station may not have data in our database yet. Please select another station from the dropdown.",
                station_name
            )));
            js_bridge::destroy_chart(CHART_ID);
            return;
        }
        // Clear any previous error when data IS available
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        // Find the station name for the chart title
        let station_name = state
            .snow_stations
            .read()
            .iter()
            .find(|s| s.station_id == station)
            .map(|s| format!("{} ({})", s.name, s.station_id))
            .unwrap_or_else(|| station.clone());

        // Wrap single station data as station_date_value format for multi-line chart
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
        web_sys::console::log_1(&format!(
            "Sending to renderMultiLineChart: {}",
            &data_json[..200.min(data_json.len())]
        ).into());
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Snow Water Equivalent: {}", station_name),
            "yAxisLabel": "Inches (SWE)",
            "dateFormat": "YYYYMMDD",
            "tooltipFormat": "station_date_value",
            "valueLabel": "SWE (inches)",
        }))
        .unwrap_or_default();

        web_sys::console::log_1(&"[CWR Debug Rust] Calling render_multi_line_chart".into());
        js_bridge::render_multi_line_chart(CHART_ID, &data_json, &config_json);
        web_sys::console::log_1(&"[CWR Debug Rust] render_multi_line_chart returned".into());
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Historical Snow Water Equivalent by Station".to_string(),
                unit_description: "Snow Water Equivalent (SWE) in inches - the depth of water that would result from melting the snowpack".to_string(),
            }

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                div {
                    style: "display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; margin-bottom: 8px;",
                    SnowStationSelector {}
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
