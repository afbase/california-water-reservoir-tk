use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use crate::database::Database;
use wasm_bindgen::prelude::*;

// Import the D3 chart function from our assets/chart.js
#[wasm_bindgen(module = "/assets/chart.js")]
extern "C" {
    #[wasm_bindgen(js_name = createD3Chart)]
    fn create_d3_chart(container_id: &str, data_json: &str);
}

#[component]
pub fn PerReservoirChart(
    database: Database,
    station_id: String,
    start_date: String,
    end_date: String,
) -> Element {
    let mut chart_data = use_signal(|| Vec::<(String, u32)>::new());
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    // Load data when station or date range changes
    use_effect(move || {
        let db = database.clone();
        let station = station_id.clone();
        let start = start_date.clone();
        let end = end_date.clone();

        spawn(async move {
            loading.set(true);
            error_msg.set(None);
            match db.get_reservoir_data(&station, &start, &end).await {
                Ok(data) => {
                    info!("Loaded {} data points for reservoir {}", data.len(), station);
                    chart_data.set(data);
                    loading.set(false);
                }
                Err(e) => {
                    info!("Error loading reservoir chart data: {}", e);
                    error_msg.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    // Update D3 chart when data changes
    use_effect(move || {
        if !loading() && !chart_data().is_empty() {
            let data = chart_data();

            // Build JSON string
            let json_data: Vec<String> = data.iter()
                .map(|(date, value)| {
                    format!(r#"{{"date":"{}","value":{}}}"#, date, value)
                })
                .collect();

            let json_str = format!("[{}]", json_data.join(","));

            // Call the D3 chart function from our JavaScript module
            create_d3_chart("per-reservoir-chart-container", &json_str);
        }
    });

    rsx! {
        div {
            class: "per-reservoir-chart-wrapper",
            style: "margin: 20px 0;",

            if let Some(error) = error_msg() {
                div {
                    class: "error-message",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "Error: {error}"
                }
            }

            if loading() {
                div {
                    class: "loading-indicator",
                    style: "text-align: center; padding: 20px; color: #666;",
                    "Loading reservoir data..."
                }
            } else if chart_data().is_empty() {
                div {
                    class: "no-data-message",
                    style: "text-align: center; padding: 20px; color: #666;",
                    "No data available for this reservoir in the selected date range"
                }
            }

            div {
                id: "per-reservoir-chart-container",
                style: "width: 100%; min-height: 500px; background: #f9f9f9; border-radius: 8px; padding: 10px;"
            }

            div {
                class: "data-count",
                style: "text-align: center; margin-top: 10px; color: #666; font-size: 14px;",
                "Data points: {chart_data().len()}"
            }
        }
    }
}
