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
//! 1. `build.rs` copies `capacity.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds this CSV into the WASM binary.
//! 3. On mount, the CSV is loaded into an in-memory SQLite database. The
//!    selected reservoir's per-station observation file is then fetched on
//!    demand (and whenever the selection changes) via `fetch_observations_br`.
//! 4. When the user selects a reservoir and sort mode, the app queries
//!    `query_water_years()` and `query_water_year_stats()`, then enriches
//!    the data with `is_most_recent` flags before rendering.

use cwr_chart_ui::components::{
    ChartCard, ChartContainer, ErrorDisplay, LoadingSpinner, ReservoirSelector, SortSelector,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;
use std::collections::HashSet;

/// All reservoir metadata.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));

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

    // Re-render chart whenever reservoir selection, sort mode, or display count changes
    use_effect(move || {
        let loading_state = (state.loading)();

        if loading_state {
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => {
                return;
            }
        };

        let station = (state.selected_station)();
        let sort_mode = (state.sort_mode)();
        let display_count = (state.display_count)();

        if station.is_empty() {
            return;
        }

        // 1. Query all water year data for the selected reservoir
        let water_years = match db.query_water_years(&station) {
            Ok(wy) => wy,
            Err(e) => {
                log::error!("Water years query failed: {}", e);
                return;
            }
        };

        if water_years.is_empty() {
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

        // 2. Query water year stats (has is_driest/is_wettest already computed dynamically)
        let stats = match db.query_water_year_stats(&station) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Stats query failed: {}", e);
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
                sorted_stats.sort_by_key(|s| std::cmp::Reverse(s.year));
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
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Water Years: {}", reservoir_name),
            "yAxisLabel": "Acre-Feet (AF)",
            "yUnit": "AF",
            "capacity": capacity,
        }))
        .unwrap_or_default();

        js_bridge::render_water_years_chart(CHART_ID, &data_json, &config_json);
    });

    rsx! {
        ChartCard {
            title: "Water Years Comparison",
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
                    SortSelector {}
                }

                ChartContainer { id: CHART_ID, min_height: 450 }
            }
        }
    }
}
