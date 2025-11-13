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
pub fn ChartComponent(database: Database, start_date: String, end_date: String) -> Element {
    let mut chart_data = use_signal(|| Vec::<(String, u32)>::new());
    let mut loading = use_signal(|| true);

    // Load data when date range changes
    use_effect(move || {
        let db = database.clone();
        let start = start_date.clone();
        let end = end_date.clone();

        spawn(async move {
            loading.set(true);
            match db.get_data(&start, &end).await {
                Ok(data) => {
                    info!("Loaded {} data points for chart", data.len());
                    chart_data.set(data);
                    loading.set(false);
                }
                Err(e) => {
                    info!("Error loading chart data: {}", e);
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
            create_d3_chart("chart-container", &json_str);
        }
    });

    rsx! {
        div {
            class: "chart-wrapper",
            style: "margin: 20px 0;",

            if loading() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "Loading chart data..."
                }
            } else if chart_data().is_empty() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "No data available for selected date range"
                }
            }

            div {
                id: "chart-container",
                style: "width: 100%; min-height: 500px; background: #f9f9f9; border-radius: 8px; padding: 10px;"
            }

            div {
                style: "text-align: center; margin-top: 10px; color: #666; font-size: 14px;",
                "Data points: {chart_data().len()}"
            }
        }
    }
}
