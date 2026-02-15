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
//! 1. `build.rs` copies `capacity.csv` and `observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a reservoir, `query_water_year_stats()` is called
//!    and the results are passed to `renderDataTable()` for D3.js rendering.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner, ReservoirSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;
use wasm_bindgen::JsValue;

/// All reservoir metadata.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));
/// Daily observation data for all reservoirs.
const OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/observations.csv"));

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
                    let default_station = reservoirs.iter()
                        .find(|r| r.station_id == "ORO")
                        .or_else(|| reservoirs.first())
                        .map(|r| r.station_id.clone())
                        .unwrap_or_default();

                    if !default_station.is_empty() {
                        web_sys::console::log_1(&format!("[CWR Debug] table-water-year-stats: Default selection: {}", default_station).into());
                        state.selected_station.set(default_station);
                    }
                    state.reservoirs.set(reservoirs);
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

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        // Query water year stats (already has is_driest/is_wettest computed dynamically)
        let stats = match db.query_water_year_stats(&station) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to query water year stats: {}", e);
                return;
            }
        };

        if stats.is_empty() {
            let reservoir_name = state.reservoirs.read().iter()
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

        // Determine the most recent year for additional highlighting
        let most_recent_year = stats.iter().map(|s| s.year).max().unwrap_or(0);

        // Enrich stats data with is_most_recent flag and formatted dates
        let table_data: Vec<serde_json::Value> = stats
            .iter()
            .map(|s| {
                // Format YYYYMMDD dates to YYYY-MM-DD for display
                let fmt_date = |d: &str| -> String {
                    if d.len() == 8 {
                        format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
                    } else {
                        d.to_string()
                    }
                };

                serde_json::json!({
                    "year": s.year,
                    "date_lowest": fmt_date(&s.date_lowest),
                    "lowest_value": s.lowest_value,
                    "date_highest": fmt_date(&s.date_highest),
                    "highest_value": s.highest_value,
                    "is_driest": s.is_driest,
                    "is_wettest": s.is_wettest,
                    "is_most_recent": s.year == most_recent_year,
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
            "columns": [
                {"key": "year", "label": "Water Year", "sortable": true, "type": "number"},
                {"key": "date_lowest", "label": "Date of Lowest", "sortable": true, "type": "date"},
                {"key": "lowest_value", "label": "Lowest Storage (AF)", "sortable": true, "type": "number", "format": "comma"},
                {"key": "date_highest", "label": "Date of Highest", "sortable": true, "type": "date"},
                {"key": "highest_value", "label": "Highest Storage (AF)", "sortable": true, "type": "number", "format": "comma"},
            ],
            "highlightRules": [
                {"field": "is_driest", "color": "#FFEBEE", "borderColor": "#FF5722", "label": "Driest Year"},
                {"field": "is_wettest", "color": "#E3F2FD", "borderColor": "#2196F3", "label": "Wettest Year"},
                {"field": "is_most_recent", "color": "#E8F5E9", "borderColor": "#4CAF50", "label": "Most Recent Year"},
            ],
            "defaultSort": {"key": "year", "direction": "desc"},
            "valueUnit": "Acre-Feet (AF)",
        }))
        .unwrap_or_default();

        js_bridge::render_data_table(TABLE_ID, &data_json, &config_json);
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Water Year Statistics".to_string(),
                unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year".to_string(),
            }

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                div {
                    style: "margin-bottom: 8px;",
                    ReservoirSelector {}
                }

                ChartContainer {
                    id: TABLE_ID.to_string(),
                    loading: false,
                    min_height: 300,
                }

                // Legend for row highlighting
                TableLegend {}
            }
        }
    }
}

/// Legend component explaining the row highlighting colors.
#[component]
fn TableLegend() -> Element {
    rsx! {
        div {
            style: "margin-top: 12px; padding: 8px 12px; background: #FAFAFA; border-radius: 4px; border: 1px solid #E0E0E0; font-size: 12px; display: flex; gap: 16px; flex-wrap: wrap;",
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 12px; background: #FFEBEE; border: 1px solid #FF5722; border-radius: 2px;",
                }
                "Driest Year (lowest minimum storage across all years)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 12px; background: #E3F2FD; border: 1px solid #2196F3; border-radius: 2px;",
                }
                "Wettest Year (highest maximum storage across all years)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 12px; background: #E8F5E9; border: 1px solid #4CAF50; border-radius: 2px;",
                }
                "Most Recent Water Year"
            }
        }
    }
}
