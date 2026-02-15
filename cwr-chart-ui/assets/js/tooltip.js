// California Water Reservoir — Shared Tooltip Management
// Global tooltip used by all CWR chart modules.
// Called from Rust/WASM via js_sys::eval(). No ES modules.

var _cwrTooltipDiv = null;

function initTooltip() {
    // Remove any stale tooltip from a prior render
    if (_cwrTooltipDiv) {
        try { _cwrTooltipDiv.remove(); } catch (e) { /* ignore */ }
        _cwrTooltipDiv = null;
    }

    _cwrTooltipDiv = d3.select("body").append("div")
        .attr("class", "cwr-tooltip")
        .style("position", "absolute")
        .style("visibility", "hidden")
        .style("background-color", "rgba(0, 0, 0, 0.85)")
        .style("color", "#fff")
        .style("padding", "8px 12px")
        .style("border-radius", "4px")
        .style("font-size", "13px")
        .style("font-family", "system-ui, -apple-system, sans-serif")
        .style("line-height", "1.4")
        .style("pointer-events", "none")
        .style("z-index", "10000")
        .style("white-space", "nowrap")
        .style("box-shadow", "0 2px 8px rgba(0,0,0,0.3)")
        .style("max-width", "360px");
}

function showTooltip(html, x, y) {
    if (!_cwrTooltipDiv) initTooltip();
    _cwrTooltipDiv
        .html(html)
        .style("visibility", "visible")
        .style("left", (x + 15) + "px")
        .style("top", (y - 10) + "px");

    // Prevent tooltip from overflowing the right edge of the viewport
    var node = _cwrTooltipDiv.node();
    if (node) {
        var rect = node.getBoundingClientRect();
        var viewportW = window.innerWidth || document.documentElement.clientWidth;
        if (rect.right > viewportW - 8) {
            _cwrTooltipDiv.style("left", (x - rect.width - 15) + "px");
        }
    }
}

function hideTooltip() {
    if (_cwrTooltipDiv) {
        _cwrTooltipDiv.style("visibility", "hidden");
    }
}
