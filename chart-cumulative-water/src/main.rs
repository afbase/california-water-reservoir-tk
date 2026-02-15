//! Cumulative California Water Storage (CA-only, excludes Lake Mead and Lake Powell)
//!
//! Displays a single line chart of the total water stored across all
//! California-only reservoirs over time. This replaces the former `yew-da-best`
//! crate with an equivalent Dioxus 0.7 + D3.js implementation.
//!
//! Data flow:
//! 1. `build.rs` copies `capacity-no-powell-no-mead.csv` and `observations.csv`
//!    into `OUT_DIR` at compile time.
//! 2. `include_str!` embeds these CSVs into the WASM binary.
//! 3. On mount, the CSVs are loaded into an in-memory SQLite database (`cwr-db`).
//! 4. CA-only totals are derived on-the-fly via SQL `SUM/GROUP BY` with a JOIN
//!    that excludes Lake Mead (MEA) and Lake Powell (PWL).
//! 5. The line chart is rendered via the D3.js bridge in `cwr-chart-ui`.

use cwr_chart_ui::components::{ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::Database;
use dioxus::prelude::*;

/// CA-only reservoir capacity (excludes Mead/Powell).
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));
/// Daily observation data for all reservoirs.
const OBSERVATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/observations.csv"));

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "cumulative-water-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut state = use_context_provider(AppState::new);

    // Initialize database on mount
    use_effect(move || {
        match Database::new() {
            Ok(db) => {
                if let Err(e) = db.load_reservoirs(CAPACITY_CSV) {
                    log::error!("Failed to load CA-only reservoirs: {}", e);
                    state.error_msg.set(Some(format!("Failed to load reservoir data: {}", e)));
                    state.loading.set(false);
                    return;
                }
                if !OBSERVATIONS_CSV.is_empty() {
                    if let Err(e) = db.load_observations(OBSERVATIONS_CSV) {
                        log::error!("Failed to load observations: {}", e);
                        state.error_msg.set(Some(format!("Failed to load observations: {}", e)));
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
                state.error_msg.set(Some(format!("Database initialization failed: {}", e)));
                state.loading.set(false);
            }
        }
    });

    // Render chart after data loaded
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

        // Initialize the D3.js chart scripts
        js_bridge::init_charts();

        // Query the full date range of cumulative CA-only data
        let (start, end) = match db.query_date_range() {
            Ok(range) => range,
            Err(e) => {
                log::warn!("No date range available: {}", e);
                return;
            }
        };

        let data = match db.query_total_water_ca_only(&start, &end) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to query cumulative CA-only water: {}", e);
                return;
            }
        };

        if data.is_empty() {
            log::warn!("No cumulative CA-only water data found");
            return;
        }

        let data_json = serde_json::to_string(&data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": "Cumulative California Water Storage",
            "yAxisLabel": "Acre-Feet (AF)",
            "lineColor": "#2196F3",
            "tooltipFormat": "date_value",
            "dateFormat": "YYYYMMDD",
            "valueLabel": "Total Storage (AF)"
        }))
        .unwrap_or_default();

        js_bridge::render_line_chart(CHART_ID, &data_json, &config_json);
    });

    rsx! {
        div {
            style: "padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;",

            ChartHeader {
                title: "Cumulative California Water Storage".to_string(),
                unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year".to_string(),
            }

            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            } else if (state.loading)() {
                LoadingSpinner {}
            } else {
                ChartContainer {
                    id: CHART_ID.to_string(),
                    loading: false,
                    min_height: 450,
                }
            }

            // Footer note about excluded reservoirs
            div {
                style: "margin-top: 12px; padding: 8px 12px; background: #FFF3E0; border-radius: 4px; font-size: 12px; color: #E65100; border: 1px solid #FFE0B2;",
                strong { "Note: " }
                "This chart excludes Lake Mead and Lake Powell (Colorado River) to show California-only storage."
            }
        }
    }
}
