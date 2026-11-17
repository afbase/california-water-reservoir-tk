// California Water Reservoir — shared chart helpers.
//
// Loaded first; every other chart script hangs its entry point off `window.CWR`.
// Evaluated from Rust/WASM as a classic script once D3 is available (no ES modules).
//
// Colors are never hard-coded here: marks and text reference the `--cwr-*`
// custom properties defined in assets/css/theme.css, so the charts follow
// whatever theme the surrounding `.cwr-card` resolves to.

(function () {
    "use strict";

    var CWR = (window.CWR = window.CWR || {});

    CWR.color = {
        text: "var(--cwr-text)",
        textSecondary: "var(--cwr-text-secondary)",
        textMuted: "var(--cwr-text-muted)",
        surface: "var(--cwr-surface)",
        grid: "var(--cwr-grid)",
        axis: "var(--cwr-axis)",
        series: ["var(--cwr-series-1)", "var(--cwr-series-2)", "var(--cwr-series-3)"],
        other: "var(--cwr-series-other)"
    };

    // ── Formatting ──────────────────────────────────────────────────────

    var parseDashed = d3.timeParse("%Y-%m-%d");
    var parseCompact = d3.timeParse("%Y%m%d");

    /** Parse "YYYY-MM-DD" or "YYYYMMDD" into a Date (null when invalid). */
    CWR.parseDate = function (s) {
        if (s == null) return null;
        s = String(s);
        return s.indexOf("-") >= 0 ? parseDashed(s) : parseCompact(s);
    };

    CWR.formatDate = d3.timeFormat("%b %d, %Y");

    /** Thousands-separated value; keeps one decimal for small magnitudes (snow inches). */
    CWR.formatValue = function (v) {
        return Math.abs(v) < 100 && v % 1 !== 0 ? d3.format(",.1f")(v) : d3.format(",.0f")(v);
    };

    /** Compact axis ticks (1.2M, 450k) so wide numbers never crowd the y-axis label. */
    CWR.formatTick = function (v) {
        return Math.abs(v) >= 10000 ? d3.format("~s")(v) : d3.format(",")(v);
    };

    CWR.escapeHtml = function (s) {
        return String(s).replace(/[&<>"']/g, function (c) {
            return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c];
        });
    };

    // ── Scaffolding shared by every SVG chart ───────────────────────────

    /**
     * Clear the container and return it, or null (after writing an empty-state
     * message) when there is nothing to draw.
     */
    CWR.prepare = function (containerId, hasData) {
        var container = d3.select("#" + containerId);
        if (container.empty()) return null;
        container.selectAll("*").remove();
        container.classed("cwr-chart", true);
        if (!hasData) {
            container.append("p").attr("class", "cwr-empty").text("No data available.");
            return null;
        }
        return container;
    };

    /** Create the responsive SVG plus the inner plot group. */
    CWR.frame = function (container, config, margin, fallbackW, fallbackH) {
        var totalW = config.width || container.node().clientWidth || fallbackW;
        var totalH = config.height || fallbackH;
        // Narrow (phone) layouts cannot afford a wide legend gutter.
        if (totalW < 560 && margin.right > 24) margin.right = 24;
        var svg = container.append("svg")
            .attr("viewBox", "0 0 " + totalW + " " + totalH)
            .attr("preserveAspectRatio", "xMidYMid meet")
            .attr("role", "img")
            .attr("aria-label", config.title || "Chart")
            .style("max-width", totalW + "px");
        var g = svg.append("g")
            .attr("transform", "translate(" + margin.left + "," + margin.top + ")");
        return {
            svg: svg,
            g: g,
            totalW: totalW,
            totalH: totalH,
            width: totalW - margin.left - margin.right,
            height: totalH - margin.top - margin.bottom,
            margin: margin
        };
    };

    /** Recessive axis styling: muted tick text, hairline domain, no tick clutter. */
    function styleAxis(sel) {
        sel.selectAll("text")
            .style("font-size", "11px")
            .style("fill", CWR.color.textSecondary);
        sel.selectAll("line").style("stroke", CWR.color.axis);
        sel.select(".domain").style("stroke", CWR.color.axis);
    }

    /** Draw gridlines, both axes, the y-axis label and the title. */
    CWR.axes = function (f, x, y, config, xAxis) {
        var g = f.g;

        var grid = g.append("g").attr("class", "cwr-grid")
            .call(d3.axisLeft(y).ticks(6).tickSize(-f.width).tickFormat(""));
        grid.select(".domain").remove();
        grid.selectAll("line").style("stroke", CWR.color.grid);

        g.append("g")
            .attr("class", "cwr-x-axis")
            .attr("transform", "translate(0," + f.height + ")")
            .call(xAxis)
            .call(styleAxis);

        var yAxis = g.append("g")
            .attr("class", "cwr-y-axis")
            .call(d3.axisLeft(y).ticks(6).tickSize(0).tickPadding(8).tickFormat(CWR.formatTick))
            .call(styleAxis);
        yAxis.select(".domain").remove();

        g.append("text")
            .attr("transform", "rotate(-90)")
            .attr("x", -f.height / 2)
            .attr("y", -f.margin.left + 14)
            .attr("text-anchor", "middle")
            .style("font-size", "12px")
            .style("fill", CWR.color.textSecondary)
            .text(config.yAxisLabel || "Acre-Feet (AF)");

        if (config.title) {
            f.svg.append("text")
                .attr("x", f.margin.left)
                .attr("y", 20)
                .style("font-size", "14px")
                .style("font-weight", "600")
                .style("fill", CWR.color.text)
                .text(config.title);
        }
    };

    /** Upper bound for the y scale: the data maximum, or the capacity if that is higher. */
    CWR.yMax = function (dataMax, config) {
        return Math.max(dataMax || 0, +config.capacity || 0) || 1;
    };

    /** Dashed reference line at the reservoir's capacity (no-op when `config.capacity` is unset). */
    CWR.capacityLine = function (f, y, config) {
        var capacity = +config.capacity;
        if (!(capacity > 0)) return;
        var unit = config.yUnit || "AF";
        f.g.append("line")
            .attr("x1", 0).attr("x2", f.width)
            .attr("y1", y(capacity)).attr("y2", y(capacity))
            .style("stroke", CWR.color.textMuted)
            .style("stroke-dasharray", "2,4");
        f.g.append("text")
            .attr("x", f.width)
            .attr("y", y(capacity) - 5)
            .attr("text-anchor", "end")
            .style("font-size", "11px")
            .style("fill", CWR.color.textSecondary)
            .text("Capacity " + CWR.formatValue(capacity) + " " + unit);
    };

    /** Vertical crosshair that follows the pointer. */
    CWR.crosshair = function (f) {
        return f.g.append("line")
            .attr("y1", 0)
            .attr("y2", f.height)
            .style("stroke", CWR.color.axis)
            .style("stroke-dasharray", "3,3")
            .style("pointer-events", "none")
            .style("opacity", 0);
    };

    /** Hover marker: a dot ringed in the surface color so it reads over any line. */
    CWR.focusDot = function (f, color) {
        return f.g.append("circle")
            .attr("r", 4.5)
            .style("fill", color)
            .style("stroke", CWR.color.surface)
            .style("stroke-width", 2)
            .style("pointer-events", "none")
            .style("opacity", 0);
    };

    /** Transparent rect over the plot area that captures pointer events. */
    CWR.overlay = function (f, onMove, onLeave) {
        return f.g.append("rect")
            .attr("width", Math.max(f.width, 0))
            .attr("height", Math.max(f.height, 0))
            .style("fill", "none")
            .style("pointer-events", "all")
            .on("pointermove", onMove)
            .on("pointerleave", onLeave);
    };

    /** Index of the point in sorted `data` whose `key` is nearest `target`. */
    CWR.nearestIndex = function (data, target, key) {
        var i = d3.bisector(key).left(data, target);
        if (i <= 0) return 0;
        if (i >= data.length) return data.length - 1;
        return target - key(data[i - 1]) > key(data[i]) - target ? i : i - 1;
    };

    // ── Tooltip (one shared element, appended to <body>) ────────────────

    var tooltip = null;

    function tooltipNode() {
        if (!tooltip || !tooltip.node() || !tooltip.node().isConnected) {
            tooltip = d3.select("body").append("div").attr("class", "cwr-tooltip");
        }
        return tooltip;
    }

    /** A colored key swatch for a tooltip row (identity lives in the swatch, text stays neutral). */
    CWR.tipKey = function (color) {
        return '<span class="cwr-tip-key" style="background:' + color + '"></span>';
    };

    CWR.showTooltip = function (html, event) {
        var tip = tooltipNode();
        tip.html(html)
            .style("visibility", "visible")
            .style("left", (event.pageX + 15) + "px")
            .style("top", (event.pageY - 10) + "px");

        // Flip to the left of the pointer rather than overflow the viewport.
        var rect = tip.node().getBoundingClientRect();
        var viewportW = window.innerWidth || document.documentElement.clientWidth;
        if (rect.right > viewportW - 8) {
            tip.style("left", Math.max(8, event.pageX - rect.width - 15) + "px");
        }
    };

    CWR.hideTooltip = function () {
        if (tooltip) tooltip.style("visibility", "hidden");
    };
})();
