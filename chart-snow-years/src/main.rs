//! Snow Years Comparison Chart
//!
//! Overlays multiple snow years for a selected station on the same x-axis
//! (Oct 1 = day 0 through Sep 30 = day 364). The chart dynamically identifies
//! and highlights the driest year (lowest minimum), wettest year (highest
//! maximum), and most recent complete snow year.
//!
//! Data flow:
//! 1. `build.rs` copies `snow_stations.csv` and `snow_observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a station and sort mode, the app queries
//!    `query_snow_years()` and `query_snow_year_stats()`, then enriches
//!    the data with `is_most_recent` flags before rendering.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner, SnowStationSelector, SortSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;


/// All snow station metadata.
const SNOW_STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_stations.csv"));
/// Daily snow observation data for all stations.
const SNOW_OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_observations.csv"));

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "snow-years-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("snow-years-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    // CRITICAL DEBUG: This fires immediately when component mounts
    web_sys::console::log_1(&"[CWR CRITICAL] snow-years App component mounted".into());

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
                        web_sys::console::log_1(&format!("[CWR Debug] snow-years: Default selection: {}", default_station).into());
                        state.selected_station.set(default_station);
                    }
                    state.snow_stations.set(stations);
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

    // Re-render chart whenever station selection, sort mode, or display count changes
    use_effect(move || {
        web_sys::console::log_1(&"[CWR CRITICAL] use_effect triggered".into());
        web_sys::console::log_1(&"[CWR Debug Rust] snow-years use_effect triggered".into());

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

        web_sys::console::log_1(&format!("[CWR Debug Rust] Querying snow years for: {}", station).into());
        // 1. Query all snow year data for the selected station
        let snow_years = match db.query_snow_years(&station) {
            Ok(sy) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Query returned {} snow year records", sy.len()).into());
                sy
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Snow years query failed: {}", e).into());
                return;
            }
        };

        if snow_years.is_empty() {
            web_sys::console::log_1(&"[CWR Debug Rust] No snow years data, destroying chart".into());
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

        web_sys::console::log_1(&"[CWR Debug Rust] Querying snow year stats".into());
        // 2. Query snow year stats (has is_driest/is_wettest already computed dynamically)
        let stats = match db.query_snow_year_stats(&station) {
            Ok(s) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Stats returned {} years", s.len()).into());
                s
            }
            Err(e) => {
                web_sys::console::log_1(&format!("[CWR Debug Rust] Stats query failed: {}", e).into());
                return;
            }
        };

        // 3. Determine the most recent complete snow year.
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

        // 5. Filter snow year data to only include years we want to display
        let filtered_data: Vec<serde_json::Value> = snow_years
            .iter()
            .filter(|sy| years_to_show.contains(&sy.year))
            .map(|sy| {
                let is_driest = sy.year == driest_year;
                let is_wettest = sy.year == wettest_year;
                let is_most_recent = sy.year == most_recent_year;
                serde_json::json!({
                    "year": sy.year,
                    "day_of_year": sy.day_of_year,
                    "date": sy.date,
                    "value": sy.value,
                    "is_driest": is_driest,
                    "is_wettest": is_wettest,
                    "is_most_recent": is_most_recent,
                })
            })
            .collect();

        // Find the station name for the chart
        let station_name = state
            .snow_stations
            .read()
            .iter()
            .find(|s| s.station_id == station)
            .map(|s| format!("{} ({})", s.name, s.station_id))
            .unwrap_or_else(|| station.clone());

        let data_json = serde_json::to_string(&filtered_data).unwrap_or_default();
        web_sys::console::log_1(&format!(
            "Sending to renderWaterYearsChart: {}",
            &data_json[..200.min(data_json.len())]
        ).into());
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Snow Years: {}", station_name),
            "yAxisLabel": "Inches (SWE)",
            "valueLabel": "SWE (inches)",
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
                title: "Snow Years Comparison".to_string(),
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
                    SortSelector {}
                }

                ChartContainer {
                    id: CHART_ID.to_string(),
                    loading: false,
                    min_height: 450,
                }

                // Legend showing driest/wettest/most recent color coding
                SnowYearLegend {}
            }
        }
    }
}

/// Legend component explaining the color coding for highlighted snow years.
#[component]
fn SnowYearLegend() -> Element {
    rsx! {
        div {
            style: "margin-top: 12px; padding: 8px 12px; background: #FAFAFA; border-radius: 4px; border: 1px solid #E0E0E0; font-size: 12px; display: flex; gap: 16px; flex-wrap: wrap;",
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #FF5722;",
                }
                "Driest Year (lowest minimum SWE)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #2196F3;",
                }
                "Wettest Year (highest maximum SWE)"
            }
            div {
                style: "display: flex; align-items: center; gap: 4px;",
                span {
                    style: "display: inline-block; width: 16px; height: 3px; background: #4CAF50;",
                }
                "Most Recent Snow Year"
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
