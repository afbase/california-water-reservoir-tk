//! Cumulative California Water Storage (CA-only, excludes Lake Mead and Lake Powell)
//!
//! Displays a single line chart of the total water stored across all
//! California-only reservoirs over time. This replaces the former `yew-da-best`
//! crate with an equivalent Dioxus 0.7 + D3.js implementation.
//!
//! Data flow:
//! 1. `build.rs` copies `capacity-no-powell-no-mead.csv` into `OUT_DIR`
//!    at compile time.
//! 2. `include_str!` embeds this CSV into the WASM binary.
//! 3. On mount, the CSV is loaded into an in-memory SQLite database (`cwr-db`),
//!    then the per-station observation manifest is fetched and every station's
//!    `observations_<ID>.csv.br` file is fetched concurrently and loaded.
//! 4. CA-only totals are derived via a forward-filled, smoothed query that
//!    excludes Lake Mead (MEA) and Lake Powell (PWL).
//! 5. A `DateRangePicker` filters the range; the smoothed series is downsampled
//!    to ~2000 points and rendered via the D3.js bridge in `cwr-chart-ui`.

use cwr_chart_ui::components::{
    ChartContainer, ChartHeader, DateRangePicker, ErrorDisplay, LoadingSpinner,
};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use cwr_db::models::DateValue;
use cwr_db::Database;
use dioxus::prelude::*;

/// CA-only reservoir capacity (excludes Mead/Powell).
const CAPACITY_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/capacity.csv"));

/// Chart container DOM element ID used by D3.js to render into.
const CHART_ID: &str = "cumulative-water-chart";

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("cumulative-water-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut state = use_context_provider(AppState::new);

    // Initialize database on mount, then load EVERY station's observations.
    // The cumulative chart needs all CA reservoirs, so we fetch the manifest
    // and pull each per-station file concurrently before loading into the DB.
    use_effect(move || {
        spawn(async move {
            match Database::new() {
                Ok(db) => {
                    if let Err(e) = db.load_reservoirs(CAPACITY_CSV) {
                        log::error!("Failed to load CA-only reservoirs: {}", e);
                        state
                            .error_msg
                            .set(Some(format!("Failed to load reservoir data: {}", e)));
                        state.loading.set(false);
                        return;
                    }

                    // Which stations have per-station data files available?
                    let station_ids = match js_bridge::fetch_observations_manifest().await {
                        Ok(ids) => ids,
                        Err(e) => {
                            state.error_msg.set(Some(format!(
                                "Failed to fetch observation manifest: {}",
                                e
                            )));
                            state.loading.set(false);
                            return;
                        }
                    };

                    // Fetch all per-station files CONCURRENTLY (network-bound),
                    // then load them into the DB sequentially since DB access is
                    // single-threaded. Loading PWL/MEA is harmless — the smoothed
                    // query excludes them via the reservoirs join.
                    let results = futures::future::join_all(
                        station_ids
                            .iter()
                            .map(|id| js_bridge::fetch_observations_br(id)),
                    )
                    .await;

                    for (id, result) in station_ids.iter().zip(results) {
                        match result {
                            Ok(csv) => {
                                if let Err(e) = db.load_observations(&csv) {
                                    log::warn!(
                                        "Failed to load observations for {}: {}",
                                        id,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to fetch observations for {}: {}", id, e);
                            }
                        }
                    }

                    if let Ok(reservoirs) = db.query_reservoirs() {
                        state.reservoirs.set(reservoirs);
                    }

                    // Seed the date-range picker defaults from the loaded data.
                    if let Ok((min_date, max_date)) = db.query_date_range() {
                        if min_date.len() == 8 {
                            state.start_date.set(format!(
                                "{}-{}-{}",
                                &min_date[0..4],
                                &min_date[4..6],
                                &min_date[6..8]
                            ));
                        }
                        if max_date.len() == 8 {
                            state.end_date.set(format!(
                                "{}-{}-{}",
                                &max_date[0..4],
                                &max_date[4..6],
                                &max_date[6..8]
                            ));
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

    // Re-render chart whenever the date range changes (or loading finishes).
    use_effect(move || {
        let loading = (state.loading)();
        let start_html = (state.start_date)();
        let end_html = (state.end_date)();

        if loading || start_html.is_empty() || end_html.is_empty() {
            return;
        }

        let db = match &*state.db.read() {
            Some(db) => db.clone(),
            None => return,
        };

        // Initialize the D3.js chart scripts
        js_bridge::init_charts();

        // Convert YYYY-MM-DD (HTML date inputs) back to YYYYMMDD for the query.
        let start = start_html.replace('-', "");
        let end = end_html.replace('-', "");

        // Forward-filled/smoothed CA-only totals (excludes MEA/PWL).
        let data = match db.query_total_water_ca_only_smoothed(&start, &end) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to query cumulative CA-only water: {}", e);
                return;
            }
        };

        if data.is_empty() {
            log::warn!("No cumulative CA-only water data found");
            state.error_msg.set(Some(
                "No cumulative water data available for the selected date range.".to_string(),
            ));
            return;
        }
        // Clear any previous error when data IS available.
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        // Downsample to ~2000 points for crisp, performant rendering.
        let display_data: Vec<&DateValue> = if data.len() > 2000 {
            let step = data.len() as f64 / 2000.0;
            let mut result = Vec::with_capacity(2001);
            let mut idx = 0.0;
            while (idx as usize) < data.len() {
                result.push(&data[idx as usize]);
                idx += step;
            }
            // Always keep the final point so the line reaches the latest date.
            if result.last().map(|d| &d.date) != data.last().map(|d| &d.date) {
                result.push(data.last().unwrap());
            }
            result
        } else {
            data.iter().collect()
        };

        let data_json = serde_json::to_string(&display_data).unwrap_or_default();
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

            // Shown above the chart (non-exclusively) so the date picker stays
            // usable even if a chosen range yields no data.
            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            }

            if (state.loading)() {
                LoadingSpinner {}
            } else {
                ChartContainer {
                    id: CHART_ID.to_string(),
                    loading: false,
                    min_height: 450,
                }

                // Date range picker for filtering the chart
                div {
                    style: "margin-top: 12px; padding-top: 8px; border-top: 1px solid #e0e0e0;",
                    p {
                        style: "font-size: 12px; color: #666; margin: 0 0 4px 0;",
                        "Adjust the date range to filter the data:"
                    }
                    DateRangePicker {}
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
