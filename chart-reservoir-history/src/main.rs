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
//! 1. `build.rs` copies `capacity.csv` and `observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a reservoir and date range, the app queries
//!    `query_all_reservoir_histories()` and renders a multi-line chart.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, DateRangePicker, ErrorDisplay, LoadingSpinner, ReservoirSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// All reservoir metadata including Mead/Powell.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));
/// Daily observation data for all reservoirs.
const OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/observations.csv"));

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

    // Initialize database on mount
    use_effect(move || {
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
                if !OBSERVATIONS_CSV.is_empty() {
                    if let Err(e) = db.load_observations(OBSERVATIONS_CSV) {
                        log::error!("Failed to load observations: {}", e);
                        state
                            .error_msg
                            .set(Some(format!("Failed to load observations: {}", e)));
                        state.loading.set(false);
                        return;
                    }
                }

                // Populate reservoir list for the dropdown
                if let Ok(reservoirs) = db.query_reservoirs() {
                    if !reservoirs.is_empty() {
                        // Default to the largest reservoir
                        state
                            .selected_station
                            .set(reservoirs[0].station_id.clone());
                    }
                    state.reservoirs.set(reservoirs);
                }

                // Set default date range from the available data
                if let Ok((min_date, max_date)) = db.query_date_range() {
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
        log::info!("[CWR Debug Rust] reservoir-history use_effect triggered");

        if (state.loading)() {
            log::info!("[CWR Debug Rust] Exiting: still loading");
            return;
        }
        if (state.error_msg)().is_some() {
            log::info!("[CWR Debug Rust] Exiting: error present");
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => {
                log::info!("[CWR Debug Rust] Database available");
                db.clone()
            }
            None => {
                log::info!("[CWR Debug Rust] Exiting: no database");
                return;
            }
        };

        let station = (state.selected_station)();
        let start_date_html = (state.start_date)();
        let end_date_html = (state.end_date)();
        log::info!("[CWR Debug Rust] Selected station: {}", station);

        if station.is_empty() || start_date_html.is_empty() || end_date_html.is_empty() {
            log::info!("[CWR Debug Rust] Exiting: empty station or date range");
            return;
        }

        // Convert YYYY-MM-DD back to YYYYMMDD for DB queries
        let start_date = start_date_html.replace('-', "");
        let end_date = end_date_html.replace('-', "");

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        log::info!("[CWR Debug Rust] Querying reservoir history for: {}", station);
        // Query the selected reservoir's history within the date range
        let data = match db.query_reservoir_history(&station, &start_date, &end_date) {
            Ok(d) => {
                log::info!("[CWR Debug Rust] Query returned {} records", d.len());
                d
            }
            Err(e) => {
                log::error!("[CWR Debug Rust] Query failed: {}", e);
                return;
            }
        };

        if data.is_empty() {
            log::info!("[CWR Debug Rust] No data returned, destroying chart");
            js_bridge::destroy_chart(CHART_ID);
            return;
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
        log::info!(
            "Sending to renderMultiLineChart: {}",
            &data_json[..200.min(data_json.len())]
        );
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

        log::info!("[CWR Debug Rust] Calling render_multi_line_chart");
        js_bridge::render_multi_line_chart(CHART_ID, &data_json, &config_json);
        log::info!("[CWR Debug Rust] render_multi_line_chart returned");
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
