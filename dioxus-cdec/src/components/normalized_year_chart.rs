use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use crate::database::Database;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/assets/normalized_chart.js")]
extern "C" {
    #[wasm_bindgen(js_name = createNormalizedChart)]
    fn create_normalized_chart(container_id: &str, data_json: &str);
}

fn parse_date(date: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0].parse().ok()?;
        let month = parts[1].parse().ok()?;
        let day = parts[2].parse().ok()?;
        Some((year, month, day))
    } else {
        None
    }
}

fn get_water_year(date: &str) -> Option<i32> {
    let (year, month, _) = parse_date(date)?;
    if month >= 10 {
        Some(year + 1)
    } else {
        Some(year)
    }
}

fn get_water_year_day(date: &str) -> Option<i32> {
    let (year, month, day) = parse_date(date)?;

    // Days in each month
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut day_of_year = day;
    for i in 0..(month - 1) {
        day_of_year += days_in_month[i as usize];
    }

    // Adjust for leap years
    if month > 2 && is_leap_year(year) {
        day_of_year += 1;
    }

    // Water year starts Oct 1 (day 274 of calendar year)
    let wy_day = if month >= 10 {
        day_of_year - 273
    } else {
        let prev_year = year - 1;
        let prev_year_days = if is_leap_year(prev_year) { 366 } else { 365 };
        (prev_year_days - 273) + day_of_year
    };

    Some(wy_day)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[derive(Clone, Debug)]
struct NormalizedDataPoint {
    water_year: i32,
    water_year_day: i32,
    value: u32,
}

fn normalize_data(data: &[(String, u32)]) -> Vec<NormalizedDataPoint> {
    data.iter()
        .filter_map(|(date, value)| {
            let wy = get_water_year(date)?;
            let wy_day = get_water_year_day(date)?;
            Some(NormalizedDataPoint {
                water_year: wy,
                water_year_day: wy_day,
                value: *value,
            })
        })
        .collect()
}

#[component]
pub fn NormalizedYearChart(
    database: Database,
    station_id: Option<String>,
    selected_years: Vec<i32>,
) -> Element {
    let mut chart_data = use_signal(|| Vec::<NormalizedDataPoint>::new());
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    // Load data when inputs change
    use_effect(move || {
        let db = database.clone();
        let station = station_id.clone();

        spawn(async move {
            loading.set(true);
            error_msg.set(None);

            // Get all available data
            let result = if let Some(sid) = station {
                if let Ok((min, max)) = db.get_reservoir_date_range(&sid).await {
                    db.get_reservoir_data(&sid, &min, &max).await
                } else {
                    Err("Failed to get date range".to_string())
                }
            } else {
                if let Ok((min, max)) = db.get_date_range().await {
                    db.get_data(&min, &max).await
                } else {
                    Err("Failed to get date range".to_string())
                }
            };

            match result {
                Ok(data) => {
                    info!("Normalizing {} data points", data.len());
                    let normalized = normalize_data(&data);
                    chart_data.set(normalized);
                    loading.set(false);
                }
                Err(e) => {
                    info!("Error loading data for normalized chart: {}", e);
                    error_msg.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    // Update chart when data or selected years change
    use_effect(move || {
        if !loading() && !chart_data().is_empty() {
            let data = chart_data();
            let years = selected_years.clone();

            // Filter to selected years if any specified
            let filtered: Vec<_> = if years.is_empty() {
                data.clone()
            } else {
                data.iter()
                    .filter(|d| years.contains(&d.water_year))
                    .cloned()
                    .collect()
            };

            if filtered.is_empty() {
                return;
            }

            // Build JSON: [{"year": 2024, "day": 1, "value": 12345}, ...]
            let json_data: Vec<String> = filtered.iter()
                .map(|d| {
                    format!(
                        r#"{{"year":{},"day":{},"value":{}}}"#,
                        d.water_year, d.water_year_day, d.value
                    )
                })
                .collect();

            let json_str = format!("[{}]", json_data.join(","));
            create_normalized_chart("normalized-chart-container", &json_str);
        }
    });

    rsx! {
        div {
            class: "normalized-chart-wrapper",
            style: "margin: 20px 0;",

            h3 {
                style: "color: #2c3e50; margin-bottom: 15px;",
                "Normalized Water Year Comparison"
            }

            p {
                style: "color: #666; font-size: 14px; margin-bottom: 15px;",
                "Comparing water years by aligning all years to start on October 1 (day 1 of water year)"
            }

            if let Some(error) = error_msg() {
                div {
                    class: "error-message",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "Error: {error}"
                }
            }

            if loading() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "Loading normalized data..."
                }
            } else if chart_data().is_empty() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "No data available"
                }
            }

            div {
                id: "normalized-chart-container",
                style: "width: 100%; min-height: 500px; background: #f9f9f9; border-radius: 8px; padding: 10px;"
            }
        }
    }
}
