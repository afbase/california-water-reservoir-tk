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
    ChartCard, ChartContainer, DateRangePicker, ErrorDisplay, LoadingSpinner, SnowStationSelector,
};
use cwr_chart_ui::dates::{compact_to_iso, iso_to_compact};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// All snow station metadata.
const SNOW_STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_stations.csv"));

/// Runtime-fetched gzip-compressed snow observation data (served alongside WASM).
const SNOW_OBSERVATIONS_GZ_URL: &str = "./snow_observations.csv.gz";

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
    let mut state = use_context_provider(AppState::new);

    // Initialize database on mount
    use_effect(move || {
        spawn(async move {
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

                    match js_bridge::fetch_gz_csv(SNOW_OBSERVATIONS_GZ_URL).await {
                        Ok(csv_data) => {
                            if !csv_data.is_empty()
                                && let Err(e) = db.load_snow_observations(&csv_data)
                            {
                                log::error!("Failed to load snow observations: {}", e);
                                state
                                    .error_msg
                                    .set(Some(format!("Failed to load snow observations: {}", e)));
                                state.loading.set(false);
                                return;
                            }
                        }
                        Err(e) => {
                            state.error_msg.set(Some(format!(
                                "Failed to fetch snow observation data: {}",
                                e
                            )));
                            state.loading.set(false);
                            return;
                        }
                    }

                    // Populate snow station list for the dropdown
                    if let Ok(stations) = db.query_snow_stations() {
                        let default_station = stations
                            .first()
                            .map(|s| s.station_id.clone())
                            .unwrap_or_default();

                        if !default_station.is_empty() {
                            state.selected_station.set(default_station);
                        }
                        state.snow_stations.set(stations);
                    }

                    // Set default date range from the available snow data
                    if let Ok((min_date, max_date)) = db.query_snow_date_range() {
                        // Convert YYYYMMDD to YYYY-MM-DD for HTML date inputs
                        if min_date.len() == 8 {
                            let formatted_min = compact_to_iso(&min_date);
                            state.start_date.set(formatted_min);
                        }
                        if max_date.len() == 8 {
                            let formatted_max = compact_to_iso(&max_date);
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
    });

    // Re-render chart whenever selection or date range changes
    use_effect(move || {
        let loading_state = (state.loading)();

        if loading_state {
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => return,
        };

        let station = (state.selected_station)();
        let start_date_html = (state.start_date)();
        let end_date_html = (state.end_date)();

        if station.is_empty() || start_date_html.is_empty() || end_date_html.is_empty() {
            return;
        }

        // Convert YYYY-MM-DD back to YYYYMMDD for DB queries
        let start_date = iso_to_compact(&start_date_html);
        let end_date = iso_to_compact(&end_date_html);

        // Query the selected station's history within the date range
        let data = match db.query_snow_station_history(&station, &start_date, &end_date) {
            Ok(d) => d,
            Err(_e) => {
                return;
            }
        };

        if data.is_empty() {
            let station_name = state
                .snow_stations
                .read()
                .iter()
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

        // multi-line-chart.js expects an array of series: [{station_id, name, data:[{date,value},...]}]
        let points: Vec<serde_json::Value> = data
            .iter()
            .map(|dv| serde_json::json!({ "date": dv.date, "value": dv.value }))
            .collect();
        let series = serde_json::json!([{
            "station_id": station,
            "name": station_name.clone(),
            "data": points,
        }]);

        let data_json = serde_json::to_string(&series).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Snow Water Equivalent: {}", station_name),
            "yAxisLabel": "Snow water equivalent (inches)",
            "yUnit": "in",
        }))
        .unwrap_or_default();

        js_bridge::render_multi_line_chart(CHART_ID, &data_json, &config_json);
    });

    rsx! {
        ChartCard {
            title: "Historical Snow Water Equivalent by Station",
            unit_description: "Snow Water Equivalent (SWE) in inches - the depth of water that would result from melting the snowpack",

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
                    SnowStationSelector {}
                    DateRangePicker {}
                }

                ChartContainer { id: CHART_ID, min_height: 450 }
            }
        }
    }
}
