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
    let wrapped = format!(
        "try {{ {} }} catch(e) {{ console.warn('CWR JS call failed:', e); }}",
        code
    );
    let _ = js_sys::eval(&wrapped);
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
    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");
    call_js(&format!(
        r#"
        (function() {{
            console.log('[CWR Debug] Initiating polling for multi-line-chart');
            var poll = setInterval(function() {{
                console.log('[CWR Debug] Poll attempt:', {{
                    chartsReady: !!window.__cwrChartsReady,
                    functionAvailable: typeof window.renderMultiLineChart !== 'undefined',
                    domExists: !!document.getElementById('{container_id}'),
                    timestamp: Date.now()
                }});
                if (window.__cwrChartsReady &&
                    typeof window.renderMultiLineChart !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    try {{
                        window.renderMultiLineChart('{container_id}', '{escaped_data}', '{escaped_config}');
                    }} catch(e) {{ console.error('[CWR] renderMultiLineChart error:', e); }}
                }}
            }}, 100);
        }})();
        "#,
    ));
}

/// Render a water years overlay chart.
///
/// Uses a polling loop to wait for D3.js to load, chart scripts to initialize,
/// and the container DOM element to exist before rendering.
pub fn render_water_years_chart(container_id: &str, data_json: &str, config_json: &str) {
    let escaped_data = data_json.replace('\'', "\\'").replace('\n', "");
    let escaped_config = config_json.replace('\'', "\\'").replace('\n', "");
    call_js(&format!(
        r#"
        (function() {{
            console.log('[CWR Debug] Initiating polling for water-years-chart');
            var poll = setInterval(function() {{
                console.log('[CWR Debug] Poll attempt:', {{
                    chartsReady: !!window.__cwrChartsReady,
                    functionAvailable: typeof window.renderWaterYearsChart !== 'undefined',
                    domExists: !!document.getElementById('{container_id}'),
                    timestamp: Date.now()
                }});
                if (window.__cwrChartsReady &&
                    typeof window.renderWaterYearsChart !== 'undefined' &&
                    document.getElementById('{container_id}')) {{
                    clearInterval(poll);
                    try {{
                        window.renderWaterYearsChart('{container_id}', '{escaped_data}', '{escaped_config}');
                    }} catch(e) {{ console.error('[CWR] renderWaterYearsChart error:', e); }}
                }}
            }}, 100);
        }})();
        "#,
    ));
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

/// Destroy/clean up a chart in the given container.
pub fn destroy_chart(container_id: &str) {
    call_js(&format!(
        "var el = document.getElementById('{}'); if (el) el.innerHTML = '';",
        container_id
    ));
}
