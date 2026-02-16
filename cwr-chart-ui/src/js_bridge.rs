//! Typed wrappers around JS interop via `js_sys::eval()`.
//!
//! D3.js chart functions are split across `assets/js/*.js` and loaded at runtime.
//! They are evaluated as globals (no ES modules) and exposed via `window.*`.
//! This module provides safe Rust wrappers that serialize data and call those globals.

// Embed all D3 chart JS files at compile time
static TOOLTIP_JS: &str = include_str!("../assets/js/tooltip.js");
static LINE_CHART_JS: &str = include_str!("../assets/js/line-chart.js");
static MULTI_LINE_CHART_JS: &str = include_str!("../assets/js/multi-line-chart.js");
static WATER_YEARS_CHART_JS: &str = include_str!("../assets/js/water-years-chart.js");
static DATA_TABLE_JS: &str = include_str!("../assets/js/data-table.js");

/// Execute arbitrary JS, wrapping in try/catch to avoid panics.
pub fn call_js(code: &str) {
    log::info!(
        "[CWR Debug CallJS] Executing {} bytes of JavaScript",
        code.len()
    );

    let wrapped = format!(
        "try {{ {} }} catch(e) {{ console.error('CWR JS call failed:', e); console.error('Stack:', e.stack); }}",
        code
    );

    match js_sys::eval(&wrapped) {
        Ok(_) => log::info!("[CWR Debug CallJS] eval() succeeded"),
        Err(e) => log::error!("[CWR Debug CallJS] eval() failed: {:?}", e),
    }
}

/// Load and evaluate all chart JS scripts. Call once at app startup.
pub fn load_chart_scripts() {
    let all_js = [
        TOOLTIP_JS,
        LINE_CHART_JS,
        MULTI_LINE_CHART_JS,
        WATER_YEARS_CHART_JS,
        DATA_TABLE_JS,
    ]
    .join("\n");
    let _ = js_sys::eval(&all_js);
}

/// Initialize chart scripts with a wait-for-D3 polling loop.
///
/// The chart JS files define functions like `renderLineChart(...)` via
/// `function` declarations. To ensure they become globally accessible
/// (not block-scoped inside the setInterval callback), we evaluate them
/// at global scope via a separate `eval()` call once D3 is ready,
/// and then explicitly promote each function to `window.*`.
pub fn init_charts() {
    let all_js = [
        TOOLTIP_JS,
        LINE_CHART_JS,
        MULTI_LINE_CHART_JS,
        WATER_YEARS_CHART_JS,
        DATA_TABLE_JS,
    ]
    .join("\n");

    // Store the scripts on window so the polling callback can eval them
    // at global scope (not block-scoped inside setInterval).
    let store_js = format!(
        "window.__cwrChartScripts = {};",
        serde_json::to_string(&all_js).unwrap_or_default()
    );
    let _ = js_sys::eval(&store_js);

    let init_js = r#"
        (function() {
            var waitForD3 = setInterval(function() {
                if (typeof d3 !== 'undefined') {
                    clearInterval(waitForD3);
                    // Eval at global scope via indirect eval
                    (0, eval)(window.__cwrChartScripts);
                    delete window.__cwrChartScripts;
                    // Promote function declarations to window explicitly
                    if (typeof renderLineChart !== 'undefined') window.renderLineChart = renderLineChart;
                    if (typeof destroyLineChart !== 'undefined') window.destroyLineChart = destroyLineChart;
                    if (typeof renderMultiLineChart !== 'undefined') window.renderMultiLineChart = renderMultiLineChart;
                    if (typeof renderWaterYearsChart !== 'undefined') window.renderWaterYearsChart = renderWaterYearsChart;
                    if (typeof renderDataTable !== 'undefined') window.renderDataTable = renderDataTable;
                    if (typeof initTooltip !== 'undefined') window.initTooltip = initTooltip;
                    if (typeof showTooltip !== 'undefined') window.showTooltip = showTooltip;
                    if (typeof hideTooltip !== 'undefined') window.hideTooltip = hideTooltip;
                    window.__cwrChartsReady = true;
                    console.log('CWR charts initialized');
                }
            }, 100);
        })();
    "#;
    let _ = js_sys::eval(init_js);
}

/// Render a single line chart (total water, cumulative water, local reservoirs).
///
/// Uses a polling loop to wait for D3.js to load, chart scripts to initialize,
/// and the container DOM element to exist before rendering.
pub fn render_line_chart(container_id: &str, data_json: &str, config_json: &str) {
    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");
    call_js(&format!(
        r#"
        (function() {{
            var poll = setInterval(function() {{
                if (window.__cwrChartsReady &&
                    typeof window.renderLineChart !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    try {{
                        window.renderLineChart('{container_id}', '{escaped_data}', '{escaped_config}');
                    }} catch(e) {{ console.error('[CWR] renderLineChart error:', e); }}
                }}
            }}, 100);
        }})();
        "#,
    ));
}

/// Render a multi-line chart (reservoir history, snow history).
///
/// Uses a polling loop to wait for D3.js to load, chart scripts to initialize,
/// and the container DOM element to exist before rendering.
pub fn render_multi_line_chart(container_id: &str, data_json: &str, config_json: &str) {
    log::info!(
        "[CWR Debug Bridge] render_multi_line_chart called for container: {}",
        container_id
    );
    log::info!("[CWR Debug Bridge] Data length: {} bytes", data_json.len());
    log::info!(
        "[CWR Debug Bridge] Config length: {} bytes",
        config_json.len()
    );

    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");

    log::info!("[CWR Debug Bridge] Calling call_js");
    call_js(&format!(
        r#"
        (function() {{
            console.log('[CWR Debug JS] Polling started for multi-line-chart');
            console.log('[CWR Debug JS] Container ID:', '{container_id}');

            var pollCount = 0;
            var poll = setInterval(function() {{
                pollCount++;
                console.log('[CWR Debug JS] Poll attempt #' + pollCount);
                console.log('[CWR Debug JS] chartsReady:', !!window.__cwrChartsReady);
                console.log('[CWR Debug JS] functionAvailable:', typeof window.renderMultiLineChart !== 'undefined');
                console.log('[CWR Debug JS] domExists:', !!document.getElementById('{container_id}'));

                if (window.__cwrChartsReady &&
                    typeof window.renderMultiLineChart !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    console.log('[CWR Debug JS] All conditions met, calling renderMultiLineChart');
                    try {{
                        window.renderMultiLineChart('{container_id}', '{escaped_data}', '{escaped_config}');
                        console.log('[CWR Debug JS] renderMultiLineChart returned successfully');
                    }} catch(e) {{
                        console.error('[CWR Debug JS] renderMultiLineChart error:', e);
                        console.error('[CWR Debug JS] Stack:', e.stack);
                    }}
                }}

                // Stop polling after 50 attempts (5 seconds)
                if (pollCount > 50) {{
                    clearInterval(poll);
                    console.error('[CWR Debug JS] Polling timeout after 50 attempts');
                }}
            }}, 100);
        }})();
        "#,
    ));
    log::info!("[CWR Debug Bridge] call_js returned");
}

/// Render a water years overlay chart.
///
/// Uses a polling loop to wait for D3.js to load, chart scripts to initialize,
/// and the container DOM element to exist before rendering.
pub fn render_water_years_chart(container_id: &str, data_json: &str, config_json: &str) {
    log::info!(
        "[CWR Debug Bridge] render_water_years_chart called for container: {}",
        container_id
    );
    log::info!("[CWR Debug Bridge] Data length: {} bytes", data_json.len());
    log::info!(
        "[CWR Debug Bridge] Config length: {} bytes",
        config_json.len()
    );

    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");

    log::info!("[CWR Debug Bridge] Calling call_js");
    call_js(&format!(
        r#"
        (function() {{
            console.log('[CWR Debug JS] Polling started for water-years-chart');
            console.log('[CWR Debug JS] Container ID:', '{container_id}');

            var pollCount = 0;
            var poll = setInterval(function() {{
                pollCount++;
                console.log('[CWR Debug JS] Poll attempt #' + pollCount);
                console.log('[CWR Debug JS] chartsReady:', !!window.__cwrChartsReady);
                console.log('[CWR Debug JS] functionAvailable:', typeof window.renderWaterYearsChart !== 'undefined');
                console.log('[CWR Debug JS] domExists:', !!document.getElementById('{container_id}'));

                if (window.__cwrChartsReady &&
                    typeof window.renderWaterYearsChart !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    console.log('[CWR Debug JS] All conditions met, calling renderWaterYearsChart');
                    try {{
                        window.renderWaterYearsChart('{container_id}', '{escaped_data}', '{escaped_config}');
                        console.log('[CWR Debug JS] renderWaterYearsChart returned successfully');
                    }} catch(e) {{
                        console.error('[CWR Debug JS] renderWaterYearsChart error:', e);
                        console.error('[CWR Debug JS] Stack:', e.stack);
                    }}
                }}

                // Stop polling after 50 attempts (5 seconds)
                if (pollCount > 50) {{
                    clearInterval(poll);
                    console.error('[CWR Debug JS] Polling timeout after 50 attempts');
                }}
            }}, 100);
        }})();
        "#,
    ));
    log::info!("[CWR Debug Bridge] call_js returned");
}

/// Render a sortable data table.
///
/// Uses a polling loop to wait for D3.js to load, chart scripts to initialize,
/// and the container DOM element to exist before rendering.
pub fn render_data_table(container_id: &str, data_json: &str, config_json: &str) {
    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");
    call_js(&format!(
        r#"
        (function() {{
            console.log('[CWR Debug] Initiating polling for data-table');
            var poll = setInterval(function() {{
                console.log('[CWR Debug] Poll attempt:', {{
                    chartsReady: !!window.__cwrChartsReady,
                    functionAvailable: typeof window.renderDataTable !== 'undefined',
                    domExists: !!document.getElementById('{container_id}'),
                    timestamp: Date.now()
                }});
                if (window.__cwrChartsReady &&
                    typeof window.renderDataTable !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    try {{
                        window.renderDataTable('{container_id}', '{escaped_data}', '{escaped_config}');
                    }} catch(e) {{ console.error('[CWR] renderDataTable error:', e); }}
                }}
            }}, 100);
        }})();
        "#,
    ));
}

/// Fetch and decompress a gzip-compressed CSV file at the given URL.
/// No init step needed — pure Rust decompression via `flate2`.
pub async fn fetch_gz_csv(url: &str) -> Result<String, String> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    let window = web_sys::window().ok_or("no window")?;

    let resp: Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("{:?}", e))?
        .dyn_into()
        .map_err(|_| "response cast failed".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), url));
    }

    let buf = JsFuture::from(resp.array_buffer().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let compressed = js_sys::Uint8Array::new(&buf).to_vec();

    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut csv_text = String::new();
    decoder
        .read_to_string(&mut csv_text)
        .map_err(|e| e.to_string())?;

    Ok(csv_text)
}

/// Destroy/clean up a chart in the given container.
pub fn destroy_chart(container_id: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(el) = document.get_element_by_id(container_id) {
                el.set_inner_html("");
            }
        }
    }
}
