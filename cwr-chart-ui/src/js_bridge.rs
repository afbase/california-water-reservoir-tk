//! Bridge between the Dioxus apps and the D3.js chart scripts.
//!
//! The chart scripts (`assets/js/*.js`) and the theme stylesheet
//! (`assets/css/theme.css`) are embedded at compile time. [`init_charts`]
//! installs them into the page exactly once — even when several chart apps
//! share a page — and the `render_*` functions queue a draw call that runs as
//! soon as D3, the scripts and the target container element all exist.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// Chart scripts in load order (`chart-base.js` defines `window.CWR`).
const CHART_SCRIPTS: [&str; 5] = [
    include_str!("../assets/js/chart-base.js"),
    include_str!("../assets/js/line-chart.js"),
    include_str!("../assets/js/multi-line-chart.js"),
    include_str!("../assets/js/water-years-chart.js"),
    include_str!("../assets/js/data-table.js"),
];

const THEME_CSS: &str = include_str!("../assets/css/theme.css");

/// Page-level bootstrap. `__CSS__` / `__SCRIPTS__` are replaced with JSON
/// string literals. It is idempotent: the first chart app on the page wins and
/// later ones reuse its `window.__cwrBoot`.
///
/// `__cwrBoot.draw(id, fn)` runs `fn` once the scripts are loaded and the
/// element `id` is in the DOM, giving up (with a console error) after 30s so a
/// blocked D3 download cannot leave a timer spinning forever.
const BOOTSTRAP_JS: &str = r##"
(function () {
    if (window.__cwrBoot) return;

    var style = document.createElement("style");
    style.id = "cwr-theme";
    style.textContent = __CSS__;
    document.head.appendChild(style);

    var INTERVAL_MS = 100, MAX_TRIES = 300;

    function poll(isReady, onReady, what) {
        if (isReady()) return onReady();
        var tries = 0;
        var timer = setInterval(function () {
            if (isReady()) {
                clearInterval(timer);
                onReady();
            } else if (++tries >= MAX_TRIES) {
                clearInterval(timer);
                console.error("[CWR] timed out waiting for " + what);
            }
        }, INTERVAL_MS);
    }

    var boot = (window.__cwrBoot = {
        ready: false,
        pending: [],
        draw: function (id, fn) {
            var run = function () {
                poll(
                    function () { return !!document.getElementById(id); },
                    function () {
                        try { fn(window.CWR); } catch (e) { console.error("[CWR] render failed for #" + id, e); }
                    },
                    "#" + id
                );
            };
            if (boot.ready) run(); else boot.pending.push(run);
        }
    });

    poll(
        function () { return typeof d3 !== "undefined"; },
        function () {
            (0, eval)(__SCRIPTS__);
            boot.ready = true;
            boot.pending.splice(0).forEach(function (run) { run(); });
        },
        "D3.js"
    );
})();
"##;

/// Evaluate JS, logging (rather than panicking on) any exception.
fn eval(code: &str) {
    if let Err(e) = js_sys::eval(code) {
        log::error!("[CWR] JS evaluation failed: {e:?}");
    }
}

/// Encode a Rust string as a JS string literal.
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Install the theme stylesheet and chart scripts into the page.
///
/// Safe to call any number of times, from any number of chart apps; only the
/// first call per page does any work. [`crate::components::ChartCard`] calls
/// this on mount, so apps normally never need to.
pub fn init_charts() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.replace(true) {
        return;
    }
    let bootstrap = BOOTSTRAP_JS
        .replace("__CSS__", &js_string(THEME_CSS))
        .replace("__SCRIPTS__", &js_string(&CHART_SCRIPTS.join("\n")));
    eval(&bootstrap);
}

/// Queue `window.CWR.<function>(container_id, data, config)`.
///
/// `data_json` and `config_json` must be valid JSON; they are spliced into the
/// call as object literals, so no string escaping is involved.
fn render(function: &str, container_id: &str, data_json: &str, config_json: &str) {
    init_charts();
    let id = js_string(container_id);
    eval(&format!(
        "window.__cwrBoot.draw({id}, function (CWR) {{ CWR.{function}({id}, {data_json}, {config_json}); }});"
    ));
}

/// Render a single-series line chart (total water/snow, cumulative water, local reservoirs).
pub fn render_line_chart(container_id: &str, data_json: &str, config_json: &str) {
    render("renderLineChart", container_id, data_json, config_json);
}

/// Render a multi-series line chart (reservoir history, snow history).
pub fn render_multi_line_chart(container_id: &str, data_json: &str, config_json: &str) {
    render("renderMultiLineChart", container_id, data_json, config_json);
}

/// Render a water-years overlay chart.
pub fn render_water_years_chart(container_id: &str, data_json: &str, config_json: &str) {
    render(
        "renderWaterYearsChart",
        container_id,
        data_json,
        config_json,
    );
}

/// Render a sortable data table.
pub fn render_data_table(container_id: &str, data_json: &str, config_json: &str) {
    render("renderDataTable", container_id, data_json, config_json);
}

/// Fetch a URL and return its raw bytes.
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no window")?;

    let resp: Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| "response cast failed".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), url));
    }

    let buf = JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;

    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Fetch and decompress a gzip-compressed CSV file at the given URL.
pub async fn fetch_gz_csv(url: &str) -> Result<String, String> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let compressed = fetch_bytes(url).await?;
    let mut csv_text = String::new();
    GzDecoder::new(&compressed[..])
        .read_to_string(&mut csv_text)
        .map_err(|e| e.to_string())?;
    Ok(csv_text)
}

/// Base path where per-station observation files and the manifest are served.
pub const CWR_DATA_BASE: &str = "/cwr-data";

/// Fetch one station's per-station observation file (`observations_<ID>.csv.br`),
/// decompress it, and reconstruct the 4-column CSV that
/// [`cwr_db::Database::load_observations`] accepts.
pub async fn fetch_observations_br(station_id: &str) -> Result<String, String> {
    let url = format!("{CWR_DATA_BASE}/observations_{station_id}.csv.br");
    let bytes = fetch_bytes(&url).await?;
    cwr_data::codec::decode_station_file(station_id, &bytes)
}

/// Fetch and parse the station manifest — the JSON array of station IDs that
/// have a per-station data file available.
pub async fn fetch_observations_manifest() -> Result<Vec<String>, String> {
    let url = format!("{CWR_DATA_BASE}/observations_manifest.json");
    let bytes = fetch_bytes(&url).await?;
    serde_json::from_slice::<Vec<String>>(&bytes).map_err(|e| format!("manifest parse: {e}"))
}

/// Fetch the precomputed California-only cumulative daily total series
/// (`observations_cumulative.csv.br`) as `(YYYYMMDD, acre-feet)` pairs. The
/// cumulative chart fetches this single precomputed file instead of downloading
/// every per-station file.
pub async fn fetch_cumulative_series() -> Result<Vec<(String, i64)>, String> {
    let url = format!("{CWR_DATA_BASE}/observations_cumulative.csv.br");
    let bytes = fetch_bytes(&url).await?;
    cwr_data::codec::decode_dated_series(&bytes)
}

/// Destroy/clean up a chart in the given container.
pub fn destroy_chart(container_id: &str) {
    if let Some(window) = web_sys::window()
        && let Some(document) = window.document()
        && let Some(el) = document.get_element_by_id(container_id)
    {
        el.set_inner_html("");
    }
}
