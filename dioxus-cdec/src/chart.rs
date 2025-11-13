use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use crate::database::Database;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = ["window"], js_name = eval)]
    fn eval_js(s: &str);
}

const D3_CHART_CODE: &str = r#"
function createD3Chart(containerId, dataJson) {
    const data = JSON.parse(dataJson);
    console.log('Creating chart with', data.length, 'data points');

    const container = document.getElementById(containerId);
    if (!container) {
        console.error('Chart container not found:', containerId);
        return;
    }
    container.innerHTML = '';

    const margin = {top: 20, right: 30, bottom: 70, left: 70};
    const width = 900 - margin.left - margin.right;
    const height = 500 - margin.top - margin.bottom;

    const svg = d3.select('#' + containerId)
        .append('svg')
        .attr('width', width + margin.left + margin.right)
        .attr('height', height + margin.top + margin.bottom)
        .append('g')
        .attr('transform', 'translate(' + margin.left + ',' + margin.top + ')');

    const parseDate = d3.timeParse('%Y-%m-%d');
    data.forEach(d => {
        d.date = parseDate(d.date);
        d.value = +d.value;
    });

    const x = d3.scaleTime()
        .domain(d3.extent(data, d => d.date))
        .range([0, width]);

    const y = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.value) * 1.1])
        .range([height, 0]);

    const line = d3.line()
        .x(d => x(d.date))
        .y(d => y(d.value))
        .curve(d3.curveMonotoneX);

    svg.append('g')
        .attr('transform', 'translate(0,' + height + ')')
        .call(d3.axisBottom(x))
        .selectAll('text')
        .style('text-anchor', 'end')
        .attr('dx', '-.8em')
        .attr('dy', '.15em')
        .attr('transform', 'rotate(-45)');

    svg.append('g')
        .call(d3.axisLeft(y)
            .tickFormat(d => {
                if (d >= 1000000) return (d / 1000000).toFixed(1) + 'M';
                if (d >= 1000) return (d / 1000).toFixed(0) + 'K';
                return d;
            }));

    svg.append('text')
        .attr('transform', 'rotate(-90)')
        .attr('y', 0 - margin.left)
        .attr('x', 0 - (height / 2))
        .attr('dy', '1em')
        .style('text-anchor', 'middle')
        .style('font-size', '12px')
        .text('Water Level (acre-feet)');

    svg.append('g')
        .attr('class', 'grid')
        .attr('opacity', 0.1)
        .call(d3.axisLeft(y)
            .tickSize(-width)
            .tickFormat(''));

    svg.append('path')
        .datum(data)
        .attr('fill', 'none')
        .attr('stroke', '#2196F3')
        .attr('stroke-width', 2)
        .attr('d', line);

    const area = d3.area()
        .x(d => x(d.date))
        .y0(height)
        .y1(d => y(d.value))
        .curve(d3.curveMonotoneX);

    svg.append('path')
        .datum(data)
        .attr('fill', '#2196F3')
        .attr('opacity', 0.2)
        .attr('d', area);
}
"#;

#[component]
pub fn ChartComponent(database: Database, start_date: String, end_date: String) -> Element {
    let mut chart_data = use_signal(|| Vec::<(String, u32)>::new());
    let mut loading = use_signal(|| true);
    let mut chart_ready = use_signal(|| false);

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

    // Initialize D3 chart code once
    use_effect(move || {
        if !chart_ready() {
            eval_js(D3_CHART_CODE);
            chart_ready.set(true);
        }
    });

    // Update D3 chart when data changes
    use_effect(move || {
        if !loading() && !chart_data().is_empty() && chart_ready() {
            let data = chart_data();

            let json_data: Vec<_> = data.iter()
                .map(|(date, value)| {
                    format!(r#"{{"date":"{}","value":{}}}"#, date, value)
                })
                .collect();

            let json_str = format!("[{}]", json_data.join(","));
            let js_call = format!(r#"createD3Chart('chart-container', '{}');"#, json_str.replace("'", "\\'"));

            eval_js(&js_call);
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
