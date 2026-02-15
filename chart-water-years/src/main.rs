//! Water Years Comparison Chart
//!
//! Overlays multiple water years for a selected reservoir on the same x-axis
//! (Oct 1 = day 0 through Sep 30 = day 364). The chart dynamically identifies
//! and highlights the driest year (lowest minimum), wettest year (highest
//! maximum), and most recent complete water year.
//!
//! This replaces the former `yew-wot_m8` crate and FIXES THE BUG where
//! driest/wettest years were previously hard-coded. Now all three highlighted
//! years (driest, wettest, most recent) are computed dynamically from the data.
//!
//! Data flow:
//! 1. `build.rs` copies `capacity.csv` and `observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a reservoir and sort mode, the app queries
//!    `query_water_years()` and `query_water_year_stats()`, then enriches
//!    the data with `is_most_recent` flags before rendering.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner, ReservoirSelector, SortSelector,
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

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "water-years-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("water-years-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    // CRITICAL DEBUG: This fires immediately when component mounts
    web_sys::console::log_1(&"[CWR CRITICAL] water-years App component mounted".into());

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
                        web_sys::console::log_1(&format!("[CWR Debug] water-years: Default selection: {}", default_station).into());
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

    // Re-render chart whenever reservoir selection, sort mode, or display count changes
    use_effect(move || {
        web_sys::console::log_1(&"[CWR CRITICAL] use_effect triggered".into());
        web_sys::console::log_1(&"[CWR Debug Rust] water-years use_effect triggered".into());

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
        let sort_mode = (state.sort_mode)();
        let display_count = (state.display_count)();
        web_sys::console::log_1(&format!("[CWR Debug Rust] Selected station: {}, sort: {}, count: {}", station, sort_mode, display_count).into());

        if station.is_empty() {
            web_sys::console::log_1(&"[CWR Debug Rust] Exiting: empty station".into());
            return;
        }

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        web_sys::console::log_1(&format!("[CWR Debug Rust] Querying water years for: {}", station).into());
        // 1. Query all water year data for the selected reservoir
        let water_years = match db.query_water_years(&station) {
            Ok(wy) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Query returned {} water year records", wy.len()).into());
                wy
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Water years query failed: {}", e).into());
                return;
            }
        };

        if water_years.is_empty() {
            web_sys::console::log_1(&"[CWR Debug Rust] No water years data, destroying chart".into());
            let reservoir_name = state.reservoirs.read().iter()
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
        state.error_msg.set(None);

        web_sys::console::log_1(&"[CWR Debug Rust] Querying water year stats".into());
        // 2. Query water year stats (has is_driest/is_wettest already computed dynamically)
        let stats = match db.query_water_year_stats(&station) {
            Ok(s) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Stats returned {} years", s.len()).into());
                s
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Stats query failed: {}", e).into());
                return;
            }
        };

        // 3. Determine the most recent complete water year.
        // A water year is "complete" if it has data near both the start (Oct) and end (Sep).
        // We find the maximum year from the stats.
        let all_years: Vec<i32> = stats.iter().map(|s| s.year).collect();
        let most_recent_year = all_years.iter().copied().max().unwrap_or(0);

        // Find the driest and wettest years from the stats
        let driest_year = stats
            .iter()
            .find(|s| s.is_driest)
            .map(|s| s.year)
            .unwrap_or(0);
        let wettest_year = stats
            .iter()
            .find(|s| s.is_wettest)
            .map(|s| s.year)
            .unwrap_or(0);

        // 4. Determine which years to display based on sort mode and count
        let mut sorted_stats = stats.clone();
        match sort_mode.as_str() {
            "driest" => {
                // Sort by lowest_value ascending (driest first)
                sorted_stats.sort_by(|a, b| {
                    a.lowest_value
                        .partial_cmp(&b.lowest_value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            "wettest" => {
                // Sort by highest_value descending (wettest first)
                sorted_stats.sort_by(|a, b| {
                    b.highest_value
                        .partial_cmp(&a.highest_value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            _ => {
                // "most_recent" - sort by year descending
                sorted_stats.sort_by(|a, b| b.year.cmp(&a.year));
            }
        }

        // Take the requested number of years
        let display_years: Vec<i32> = sorted_stats
            .iter()
            .take(display_count)
            .map(|s| s.year)
            .collect();

        // Always include driest, wettest, and most recent regardless of sort
        let mut years_to_show: Vec<i32> = display_years;
        for special_year in [driest_year, wettest_year, most_recent_year] {
            if special_year > 0 && !years_to_show.contains(&special_year) {
                years_to_show.push(special_year);
            }
        }

        // 5. Filter water year data to only include years we want to display
        let filtered_data: Vec<serde_json::Value> = water_years
            .iter()
            .filter(|wy| years_to_show.contains(&wy.year))
            .map(|wy| {
                let is_driest = wy.year == driest_year;
                let is_wettest = wy.year == wettest_year;
                let is_most_recent = wy.year == most_recent_year;
                serde_json::json!({
                    "year": wy.year,
                    "day_of_year": wy.day_of_year,
                    "date": wy.date,
                    "value": wy.value,
                    "is_driest": is_driest,
                    "is_wettest": is_wettest,
                    "is_most_recent": is_most_recent,
                })
            })
            .collect();

        // Find the reservoir name and capacity for the chart
        let reservoir_name = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == station)
            .map(|r| format!("{} ({})", r.dam, r.station_id))
            .unwrap_or_else(|| station.clone());

        let capacity = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == station)
            .map(|r| r.capacity)
            .unwrap_or(0);

        let data_json = serde_json::to_string(&filtered_data).unwrap_or_default();
        web_sys::console::log_1(&format!(
            "Sending to renderWaterYearsChart: {}",
            &data_json[..200.min(data_json.len())]
        ).into());
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Water Years: {}", reservoir_name),
            "yAxisLabel": "Acre-Feet (AF)",
            "valueLabel": "Storage (AF)",
            "capacity": capacity,
            "showCapacityLine": capacity > 0,
            "driestYear": driest_year,
            "wettestYear": wettest_year,
            "mostRecentYear": most_recent_year,
            "driestColor": "#FF5722",
            "wettestColor": "#2196F3",
            "mostRecentColor": "#4CAF50",
            "defaultColor": "#BDBDBD",
            "tooltipFormat": "water_year",
        }))
        .unwrap_or_default();

        web_sys::console::log_1(&"[CWR Debug Rust] Calling render_water_years_chart".into());
        js_bridge::render_water_years_chart(CHART_ID, &data_json, &config_json);
        web_sys::console::log_1(&"[CWR Debug Rust] render_water_years_chart returned".into());
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Water Years Comparison".to_string(),
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
                    SortSelector {}
                }

                ChartContainer {
                    id: CHART_ID.to_string(),
                    loading: false,
                    min_height: 450,
                }

                // Legend showing driest/wettest/most recent color coding
                WaterYearLegend {}
            }
        }
    }
}

/// Legend component explaining the color coding for highlighted water years.
#[component]
fn WaterYearLegend() -> Element {
    rsx! {
        div {
            style: "margin-top: 12px; padding: 8px 12px; background: #FAFAFA; border-radius: 4px; border: 1px solid #E0E0E0; font-size: 12px; display: flex; gap: 16px; flex-wrap: wrap;",
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #FF5722;",
                }
                "Driest Year (lowest minimum storage)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #2196F3;",
                }
                "Wettest Year (highest maximum storage)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #4CAF50;",
                }
                "Most Recent Water Year"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #BDBDBD;",
                }
                "Other Years"
            }
        }
    }
}
