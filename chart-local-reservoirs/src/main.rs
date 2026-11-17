//! Local Reservoir Observations
//!
//! Displays water level line charts for specific local reservoirs:
//! - Alpine Lake (station ID: LGT)
//! - Lake Lagunitas (station ID: APN)
//!
//! This replaces the former `yew-tew` crate with an equivalent
//! Dioxus 0.7 + D3.js implementation. Unlike the other chart apps that
//! let the user select a reservoir, this app is hardcoded to show two
//! specific local reservoirs side by side (or stacked).
//!
//! Data flow:
//! 1. `build.rs` copies `capacity.csv` into `OUT_DIR`.
//! 2. `include_str!` embeds this CSV into the WASM binary.
//! 3. On mount, the CSV is loaded into an in-memory SQLite database, then the
//!    two local reservoirs' per-station observation files (LGT + APN) are
//!    fetched via `fetch_observations_br` and loaded into the DB.
//! 4. The app queries `query_reservoir_history()` for both LGT and APN
//!    station IDs and renders a line chart for each.

use cwr_chart_ui::components::{Callout, ChartCard, ChartContainer, ErrorDisplay, LoadingSpinner};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// All reservoir metadata.
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));

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

                    // Fetch only the two local reservoirs' per-station files.
                    // A failure for one station is non-fatal so the other chart
                    // can still render; the render effect surfaces the error if
                    // neither reservoir ends up with data.
                    for station in [STATION_LGT, STATION_APN] {
                        match js_bridge::fetch_observations_br(station).await {
                            Ok(csv) => {
                                if let Err(e) = db.load_observations(&csv) {
                                    log::warn!(
                                        "Failed to load observations for {}: {}",
                                        station,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to fetch observations for {}: {}", station, e);
                            }
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
        let lgt_ok = render_local_chart(
            &db,
            STATION_LGT,
            NAME_LGT,
            CHART_LGT_ID,
            &start,
            &end,
            lgt_capacity,
            1,
        );

        // Render Lake Lagunitas (APN) chart
        let apn_ok = render_local_chart(
            &db,
            STATION_APN,
            NAME_APN,
            CHART_APN_ID,
            &start,
            &end,
            apn_capacity,
            3,
        );

        if !lgt_ok && !apn_ok {
            state.error_msg.set(Some("No observation data available for local reservoirs. These reservoirs may not have data in our database yet.".to_string()));
        } else if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }
    });

    rsx! {
        ChartCard {
            title: "Local Reservoir Observations",
            unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year",

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                ChartContainer { id: CHART_LGT_ID, min_height: 320 }
                ChartContainer { id: CHART_APN_ID, min_height: 320 }

                Callout {
                    "These are local Marin County reservoirs managed by the Marin Municipal Water District."
                }
            }
        }
    }
}

/// Render a line chart for a single local reservoir. Returns true if data was found and rendered.
#[allow(clippy::too_many_arguments)]
fn render_local_chart(
    db: &Database,
    station_id: &str,
    station_name: &str,
    chart_id: &str,
    start: &str,
    end: &str,
    capacity: i32,
    series: u8,
) -> bool {
    let data = match db.query_reservoir_history(station_id, start, end) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("No data for station {}: {}", station_id, e);
            return false;
        }
    };

    if data.is_empty() {
        log::warn!("No observations found for station {}", station_id);
        return false;
    }

    let data_json = serde_json::to_string(&data).unwrap_or_default();
    let config_json = serde_json::to_string(&serde_json::json!({
        "title": format!("{} ({})", station_name, station_id),
        "yAxisLabel": "Acre-Feet (AF)",
        "yUnit": "AF",
        "capacity": capacity,
        "series": series,
    }))
    .unwrap_or_default();

    js_bridge::render_line_chart(chart_id, &data_json, &config_json);
    true
}
