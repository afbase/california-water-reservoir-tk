//! Total California Snow Water Equivalent Levels
//!
//! Shows cumulative snow water equivalent across all CA snow stations
//! as a D3.js time-series line chart with bisector-based tooltip.
//!
//! Data flow:
//! 1. `build.rs` reads `snow_observations.csv` and pre-aggregates daily totals
//!    (SUM by date) into a `total_snow.csv` at compile time.
//! 2. `include_str!` embeds the small aggregated CSV into the WASM binary.
//! 3. On mount: parse the CSV into a vec of (date, value) pairs.
//! 4. On date range change: filter the data and re-render via D3.js.

use cwr_chart_ui::components::{ChartContainer, ChartHeader, ErrorDisplay, LoadingSpinner};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::state::AppState;
use dioxus::prelude::*;

// Embed pre-aggregated total snow CSV (date,total_swe) at compile time.
const TOTAL_SNOW_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/total_snow.csv"));
const SNOW_STATIONS_CSV: &str = include_str!(concat!(env!("OUT_DIR"), "/snow_stations.csv"));

/// DOM id for the D3 chart container div.
const CHART_CONTAINER_ID: &str = "total-snow-chart";

/// A parsed (date_yyyymmdd, date_d3, value) triple.
#[derive(Clone)]
struct DataPoint {
    date_raw: String,   // YYYYMMDD for filtering
    date_d3: String,    // YYYY-MM-DD for D3
    value: f64,
}

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("total-snow-root"))
        .launch(App);
}

/// Convert a date string from YYYYMMDD to YYYY-MM-DD format for D3.js consumption.
fn format_date_for_d3(date: &str) -> String {
    if date.len() == 8 {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

/// Convert a YYYY-MM-DD date back to YYYYMMDD for comparison.
fn format_date_for_db(date: &str) -> String {
    date.replace('-', "")
}

/// Parse the pre-aggregated total_snow.csv into data points.
fn parse_total_snow_csv(csv_data: &str) -> Vec<DataPoint> {
    let mut data = Vec::new();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv_data.as_bytes());

    for result in rdr.records() {
        if let Ok(record) = result {
            let date = record.get(0).unwrap_or("").trim();
            let value_str = record.get(1).unwrap_or("").trim();

            if date.is_empty() {
                continue;
            }

            if let Ok(value) = value_str.parse::<f64>() {
                data.push(DataPoint {
                    date_raw: date.to_string(),
                    date_d3: format_date_for_d3(date),
                    value,
                });
            }
        }
    }

    data
}

#[component]
fn App() -> Element {
    let mut state = use_context_provider(AppState::new);
    // Store all parsed data points in a signal so Effect 2 can filter them.
    let mut all_data: Signal<Vec<DataPoint>> = use_signal(Vec::new);

    // ─── Effect 1: Parse CSV once on mount ───
    use_effect(move || {
        let data = parse_total_snow_csv(TOTAL_SNOW_CSV);

        if data.is_empty() {
            state.error_msg.set(Some("No snow data available.".to_string()));
            state.loading.set(false);
            return;
        }

        let min_date = data.first().map(|d| d.date_raw.clone()).unwrap_or_default();
        let max_date = data.last().map(|d| d.date_raw.clone()).unwrap_or_default();

        // Load snow station metadata
        if let Ok(db) = cwr_db::Database::new() {
            if db.load_snow_stations(SNOW_STATIONS_CSV).is_ok() {
                if let Ok(stations) = db.query_snow_stations() {
                    state.snow_stations.set(stations);
                }
            }
        }

        all_data.set(data);
        state.start_date.set(format_date_for_d3(&min_date));
        state.end_date.set(format_date_for_d3(&max_date));
        state.loading.set(false);

        // Initialize D3 chart scripts (one-time)
        js_bridge::init_charts();
    });

    // ─── Effect 2: Filter data by date range and render chart ───
    // Re-runs whenever loading, start_date, or end_date change.
    use_effect(move || {
        let loading = (state.loading)();
        let start = (state.start_date)();
        let end = (state.end_date)();

        if loading || start.is_empty() || end.is_empty() {
            return;
        }

        // Clone data out of the signal immediately so the read borrow
        // doesn't interfere with Dioxus signal tracking.
        let data: Vec<DataPoint> = all_data.read().clone();
        if data.is_empty() {
            state.error_msg.set(Some("No total snow data available for the selected date range.".to_string()));
            return;
        }

        // Filter by date range (compare YYYYMMDD strings)
        let start_raw = format_date_for_db(&start);
        let end_raw = format_date_for_db(&end);

        let filtered: Vec<&DataPoint> = data
            .iter()
            .filter(|d| d.date_raw >= start_raw && d.date_raw <= end_raw)
            .collect();

        if filtered.is_empty() {
            state.error_msg.set(Some("No total snow data available for the selected date range.".to_string()));
            return;
        }
        // Clear any previous error when data IS available
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        // Downsample to ~2000 points for crisp rendering
        let display_data: Vec<&DataPoint> = if filtered.len() > 2000 {
            let step = filtered.len() as f64 / 2000.0;
            let mut result = Vec::with_capacity(2000);
            let mut idx = 0.0;
            while (idx as usize) < filtered.len() {
                result.push(filtered[idx as usize]);
                idx += step;
            }
            if result.last().map(|d| &d.date_raw) != filtered.last().map(|d| &d.date_raw) {
                result.push(filtered.last().unwrap());
            }
            result
        } else {
            filtered
        };

        let d3_data: Vec<serde_json::Value> = display_data
            .iter()
            .map(|d| {
                serde_json::json!({
                    "date": d.date_d3,
                    "value": d.value,
                })
            })
            .collect();

        let data_json = serde_json::to_string(&d3_data).unwrap_or_default();
        let config_json = serde_json::json!({
            "title": "Total California Snow Water Equivalent Levels",
            "yAxisLabel": "Inches (SWE)",
            "yUnit": "in",
            "color": "#2196F3",
        })
        .to_string();

        js_bridge::render_line_chart(CHART_CONTAINER_ID, &data_json, &config_json);
    });

    // ─── Render ───
    rsx! {
        div {
            style: "max-width: 900px; margin: 0 auto; padding: 8px; font-family: system-ui, -apple-system, sans-serif;",

            if let Some(err) = state.error_msg.read().as_ref() {
                ErrorDisplay { message: err.clone() }
            }

            if *state.loading.read() {
                LoadingSpinner {}
            } else {
                ChartHeader {
                    title: "Total California Snow Water Equivalent Levels".to_string(),
                    unit_description: "Snow Water Equivalent (SWE) in inches - the depth of water that would result from melting the snowpack".to_string(),
                }

                ChartContainer {
                    id: CHART_CONTAINER_ID.to_string(),
                    loading: *state.loading.read(),
                    min_height: 450,
                }

                p {
                    style: "font-size: 11px; color: #888; text-align: center; margin-top: 4px;",
                    "Aggregated across all reporting snow stations with forward-fill interpolation."
                }

                // Date range picker for filtering the chart
                DateRangeSection {}
            }
        }
    }
}

/// Date range section with start/end date inputs.
#[component]
fn DateRangeSection() -> Element {
    rsx! {
        div {
            style: "margin-top: 12px; padding-top: 8px; border-top: 1px solid #e0e0e0;",
            p {
                style: "font-size: 12px; color: #666; margin: 0 0 4px 0;",
                "Adjust the date range to filter the data:"
            }
            cwr_chart_ui::components::DateRangePicker {}
        }
    }
}
