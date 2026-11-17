//! Cumulative California Water Storage (CA-only, excludes Lake Mead and Lake Powell)
//!
//! Displays a single line chart of the total water stored across all
//! California-only reservoirs over time.
//!
//! Data flow:
//! 1. CI precomputes a smoothed (forward-filled) CA-only daily total and ships
//!    it as a single `observations_cumulative.csv.br` under `/cwr-data/`.
//! 2. On mount the app fetches that one file (delta + brotli) and decodes it to
//!    `(YYYYMMDD, acre-feet)` pairs — no per-station downloads, no in-browser DB.
//! 3. A `DateRangePicker` filters the range; the series is downsampled to ~2000
//!    points and rendered via the D3.js line chart in `cwr-chart-ui`.

use cwr_chart_ui::components::{
    Callout, ChartCard, ChartContainer, DateRangePicker, ErrorDisplay, LoadingSpinner,
};
use cwr_chart_ui::dates::{compact_to_iso, iso_to_compact};
use cwr_chart_ui::js_bridge;
use cwr_chart_ui::series::{MAX_CHART_POINTS, downsample};
use cwr_chart_ui::state::AppState;
use dioxus::prelude::*;

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
    // Precomputed CA-only daily totals as (YYYYMMDD, acre-feet) pairs.
    let mut series = use_signal(Vec::<(String, i64)>::new);

    // Fetch the single precomputed cumulative file on mount.
    use_effect(move || {
        spawn(async move {
            match js_bridge::fetch_cumulative_series().await {
                Ok(points) => {
                    // Seed the date-range picker from the data extent.
                    if let (Some(first), Some(last)) = (points.first(), points.last()) {
                        state.start_date.set(compact_to_iso(&first.0));
                        state.end_date.set(compact_to_iso(&last.0));
                    }
                    series.set(points);
                    state.loading.set(false);
                }
                Err(e) => {
                    state
                        .error_msg
                        .set(Some(format!("Failed to load cumulative data: {}", e)));
                    state.loading.set(false);
                }
            }
        });
    });

    // Re-render whenever the data or the selected date range changes.
    use_effect(move || {
        let loading = (state.loading)();
        let start_html = (state.start_date)();
        let end_html = (state.end_date)();
        let points = series.read();

        if loading || start_html.is_empty() || end_html.is_empty() || points.is_empty() {
            return;
        }

        // HTML date inputs are YYYY-MM-DD; our data keys are YYYYMMDD.
        let start = iso_to_compact(&start_html);
        let end = iso_to_compact(&end_html);

        let filtered: Vec<(String, i64)> = points
            .iter()
            .filter(|(d, _)| d.as_str() >= start.as_str() && d.as_str() <= end.as_str())
            .cloned()
            .collect();

        if filtered.is_empty() {
            state.error_msg.set(Some(
                "No cumulative water data available for the selected date range.".to_string(),
            ));
            return;
        }
        if state.error_msg.peek().is_some() {
            state.error_msg.set(None);
        }

        let data: Vec<serde_json::Value> = downsample(&filtered, MAX_CHART_POINTS)
            .into_iter()
            .map(|(date, value)| serde_json::json!({ "date": date, "value": value }))
            .collect();
        let data_json = serde_json::to_string(&data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": serde_json::Value::Null,
            "yAxisLabel": "Acre-Feet (AF)",
            "yUnit": "AF",
        }))
        .unwrap_or_default();

        js_bridge::render_line_chart(CHART_ID, &data_json, &config_json);
    });

    rsx! {
        ChartCard {
            title: "Cumulative California Water Storage",
            unit_description: "Acre-Feet (AF) - 1 acre-foot = ~326,000 gallons, enough for 1-2 households per year",

            // Shown above the chart (non-exclusively) so the date picker stays
            // usable even if a chosen range yields no data.
            if let Some(err) = (state.error_msg)() {
                ErrorDisplay { message: err }
            }

            if (state.loading)() {
                LoadingSpinner {}
            } else {
                ChartContainer { id: CHART_ID, min_height: 400 }

                div {
                    class: "cwr-controls cwr-controls-footer",
                    DateRangePicker {}
                }
            }

            Callout {
                strong { "Note: " }
                "This chart excludes Lake Mead and Lake Powell (Colorado River) to show California-only storage."
            }
        }
    }
}
