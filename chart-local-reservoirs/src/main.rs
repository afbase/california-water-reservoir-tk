//! Local Reservoir Observations
//!
//! Displays water level line charts for specific local reservoirs:
//! - Alpine Lake (station ID: LGT)
//! - Lake Lagunitas (station ID: APN)
//!
//! This replaces the former `yew-tew` crate with an equivalent Dioxus 0.7
//! + D3.js implementation. Unlike the other chart apps that let the user
//! select a reservoir, this app is hardcoded to show two specific local
//! reservoirs side by side (or stacked).
//!
//! Data flow:
//! 1. `build.rs` copies `capacity.csv` and `observations.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database.
//! 4. The app queries `query_reservoir_history()` for both LGT and APN
//!    station IDs and renders a line chart for each.

use cwr_chart_ui::components::{ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// All reservoir metadata.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));
/// Daily observation data for all reservoirs.
const OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/observations.csv"));

/// Chart container DOM element IDs used by D3.js to render into.
const CHART_LGT_ID: &str = "local-reservoir-lgt-chart";
const CHART_APN_ID: &str = "local-reservoir-apn-chart";

/// Hardcoded local reservoir station IDs.
const STATION_LGT: &str = "LGT";
const STATION_APN: &str = "APN";

/// Human-readable names for the local reservoirs.
const NAME_LGT: &str = "Alpine Lake";
const NAME_APN: &str = "Lake Lagunitas";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("local-reservoirs-root"))
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

                if let Ok(reservoirs) = db.query_reservoirs() {
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

    // Render charts after data loaded
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

        // Initialize D3.js chart scripts
        js_bridge::init_charts();

        // Get the full date range for both reservoirs
        let (start, end) = match db.query_date_range() {
            Ok(range) => range,
            Err(e) => {
                log::warn!("No date range available: {}", e);
                return;
            }
        };

        // Find capacity for each local reservoir
        let lgt_capacity = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == STATION_LGT)
            .map(|r| r.capacity)
            .unwrap_or(0);

        let apn_capacity = state
            .reservoirs
            .read()
            .iter()
            .find(|r| r.station_id == STATION_APN)
            .map(|r| r.capacity)
            .unwrap_or(0);

        // Render Alpine Lake (LGT) chart
        render_local_chart(
            &db,
            STATION_LGT,
            NAME_LGT,
            CHART_LGT_ID,
            &start,
            &end,
            lgt_capacity,
            "#1565C0", // Dark blue
        );

        // Render Lake Lagunitas (APN) chart
        render_local_chart(
            &db,
            STATION_APN,
            NAME_APN,
            CHART_APN_ID,
            &start,
            &end,
            apn_capacity,
            "#2E7D32", // Dark green
        );
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Local Reservoir Observations".to_string(),
                unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year".to_string(),
            }

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                // Alpine Lake (LGT) section
                div {
                    style: "margin-bottom: 24px;",
                    h4 {
                        style: "margin: 0 0 4px 0; font-size: 14px; color: #1565C0;",
                        "{NAME_LGT} ({STATION_LGT})"
                    }
                    ChartContainer {
                        id: CHART_LGT_ID.to_string(),
                        loading: false,
                        min_height: 350,
                    }
                }

                // Lake Lagunitas (APN) section
                div {
                    style: "margin-bottom: 16px;",
                    h4 {
                        style: "margin: 0 0 4px 0; font-size: 14px; color: #2E7D32;",
                        "{NAME_APN} ({STATION_APN})"
                    }
                    ChartContainer {
                        id: CHART_APN_ID.to_string(),
                        loading: false,
                        min_height: 350,
                    }
                }

                // Footer with reservoir info
                div {
                    style: "margin-top: 12px; padding: 8px 12px; background: #F5F5F5; border-radius: 4px; font-size: 12px; color: #616161; border: 1px solid #E0E0E0;",
                    "These are local Marin County reservoirs managed by the Marin Municipal Water District."
                }
            }
        }
    }
}

/// Render a line chart for a single local reservoir.
fn render_local_chart(
    db: &Database,
    station_id: &str,
    station_name: &str,
    chart_id: &str,
    start: &str,
    end: &str,
    capacity: i32,
    line_color: &str,
) {
    let data = match db.query_reservoir_history(station_id, start, end) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("No data for station {}: {}", station_id, e);
            return;
        }
    };

    if data.is_empty() {
        log::warn!("No observations found for station {}", station_id);
        return;
    }

    let data_json = serde_json::to_string(&data).unwrap_or_default();
    let config_json = serde_json::to_string(&serde_json::json!({
        "title": format!("{} ({})", station_name, station_id),
        "yAxisLabel": "Acre-Feet (AF)",
        "lineColor": line_color,
        "tooltipFormat": "date_value",
        "dateFormat": "YYYYMMDD",
        "valueLabel": "Storage (AF)",
        "capacity": capacity,
        "showCapacityLine": capacity > 0,
    }))
    .unwrap_or_default();

    js_bridge::render_line_chart(chart_id, &data_json, &config_json);
}
