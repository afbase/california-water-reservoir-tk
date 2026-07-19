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
    ChartContainer, ChartHeader, DateRangePicker, ErrorDisplay, LoadingSpinner,
};
use cwr_chart_ui::js_bridge;
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

/// Convert a `YYYYMMDD` date to the `YYYY-MM-DD` form used by HTML date inputs.
fn to_html_date(yyyymmdd: &str) -> String {
    if yyyymmdd.len() == 8 {
        format!(
            "{}-{}-{}",
            &yyyymmdd[0..4],
            &yyyymmdd[4..6],
            &yyyymmdd[6..8]
        )
    } else {
        yyyymmdd.to_string()
    }
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
                        state.start_date.set(to_html_date(&first.0));
                        state.end_date.set(to_html_date(&last.0));
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

        js_bridge::init_charts();

        // HTML date inputs are YYYY-MM-DD; our data keys are YYYYMMDD.
        let start = start_html.replace('-', "");
        let end = end_html.replace('-', "");

        let filtered: Vec<&(String, i64)> = points
            .iter()
            .filter(|(d, _)| d.as_str() >= start.as_str() && d.as_str() <= end.as_str())
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

        // Downsample to ~2000 points for crisp, performant rendering.
        let display: Vec<&(String, i64)> = if filtered.len() > 2000 {
            let step = filtered.len() as f64 / 2000.0;
            let mut r = Vec::with_capacity(2001);
            let mut idx = 0.0;
            while (idx as usize) < filtered.len() {
                r.push(filtered[idx as usize]);
                idx += step;
            }
            // Always keep the final point so the line reaches the latest date.
            if r.last().map(|p| &p.0) != filtered.last().map(|p| &p.0) {
                r.push(*filtered.last().unwrap());
            }
            r
        } else {
            filtered
        };

        let data: Vec<serde_json::Value> = display
            .iter()
            .map(|(date, value)| serde_json::json!({ "date": date, "value": value }))
            .collect();
        let data_json = serde_json::to_string(&data).unwrap_or_default();
        let config_json = serde_json::to_string(&serde_json::json!({
            "title": "Cumulative California Water Storage",
            "yAxisLabel": "Acre-Feet (AF)",
            "yUnit": "AF",
            "color": "#2196F3",
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
