//! Snow Year Statistics Table
//!
//! Displays a sortable table of snow year statistics per station, showing
//! the lowest and highest observed SWE values for each snow year.
//! Rows for the driest and wettest years are dynamically highlighted.
//!
//! Data flow:
//! 1. `build.rs` copies `snow_stations.csv` and `snow_observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. When the user selects a station, `query_snow_year_stats()` is called
//!    and the results are passed to `renderDataTable()` for D3.js rendering.

use cwr_chart_ui::components::{
    ChartCard, ChartContainer, ErrorDisplay, LoadingSpinner, SnowStationSelector,
};
use cwr_chart_ui::dates::compact_to_iso;
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// All snow station metadata.
const SNOW_STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_stations.csv"));

/// Runtime-fetched gzip-compressed snow observation data (served alongside WASM).
const SNOW_OBSERVATIONS_GZ_URL: &str = "./snow_observations.csv.gz";

/// Table container DOM element ID used by D3.js to render into.
const TABLE_ID: &str = "snow-stats-table";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("snow-stats-root"))
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

    // Re-render table whenever station selection changes
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

        // Query snow year stats (already has is_driest/is_wettest computed dynamically)
        let stats = match db.query_snow_year_stats(&station) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to query snow year stats: {}", e);
                return;
            }
        };

        if stats.is_empty() {
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

        // Find station name for context
        let station_name = state
            .snow_stations
            .read()
            .iter()
            .find(|s| s.station_id == station)
            .map(|s| format!("{} ({})", s.name, s.station_id))
            .unwrap_or_else(|| station.clone());

        let data_json = serde_json::to_string(&table_data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": format!("Snow Year Statistics: {}", station_name),
            "yUnit": "in",
            "columns": [
                {"key": "year", "label": "Snow Year", "type": "water_year"},
                {"key": "date_highest", "label": "Date of Highest", "type": "date"},
                {"key": "highest_value", "label": "Highest SWE", "type": "number"},
                {"key": "date_lowest", "label": "Date of Lowest", "type": "date"},
                {"key": "lowest_value", "label": "Lowest SWE", "type": "number"},
            ],
        }))
        .unwrap_or_default();

        js_bridge::render_data_table(TABLE_ID, &data_json, &config_json);
    });

    rsx! {
        ChartCard {
            title: "Snow Year Statistics",
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
                }

                ChartContainer { id: TABLE_ID, min_height: 300 }
            }
        }
    }
}
