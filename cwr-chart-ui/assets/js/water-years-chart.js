// California Water Reservoir — Water Year Overlay Chart
// Used by: chart-water-years, chart-snow-years
// Each water year is one line; the x-axis is day-of-water-year (Oct 1 .. Sep 30).
// Three years are emphasised, everything else recedes into grey context:
//   - most recent water year: series 1 (blue)
//   - driest water year:      series 2 (orange, dashed)
//   - wettest water year:     series 3 (aqua)
//
// Data format (JSON array):
// [{ year: 2023, day_of_year: 0, date: "YYYY-MM-DD", value: <number>,
//    is_driest: bool, is_wettest: bool, is_most_recent: bool }, ...]
//
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,         // e.g. "Acre-Feet (AF)"
//   yUnit: string,              // e.g. "AF" or "in"
//   capacity: number (optional), // draws a dashed reference line
//   maxAdditionalYears: number, // default 20 — how many grey context years to draw
//   width, height: number (optional)
// }

(function () {
    "use strict";
    var CWR = window.CWR;

    // Month tick positions as day offsets from Oct 1 (non-leap year).
    var MONTHS = [
        { day: 0, label: "Oct" }, { day: 31, label: "Nov" }, { day: 61, label: "Dec" },
        { day: 92, label: "Jan" }, { day: 123, label: "Feb" }, { day: 151, label: "Mar" },
        { day: 182, label: "Apr" }, { day: 212, label: "May" }, { day: 243, label: "Jun" },
        { day: 273, label: "Jul" }, { day: 304, label: "Aug" }, { day: 335, label: "Sep" }
    ];

    /** "Jan 12" for a day offset into the water year. */
    function dayLabel(day) {
        var month = MONTHS[0];
        MONTHS.forEach(function (m) { if (day >= m.day) month = m; });
        return month.label + " " + (day - month.day + 1);
    }

    CWR.renderWaterYearsChart = function (containerId, data, config) {
        data = data || [];
        var container = CWR.prepare(containerId, data.length > 0);
        if (!container) return;

        // Group rows by water year and note which years are special.
        var byYear = new Map();
        var special = new Map(); // year -> { color, label, dash }
        data.forEach(function (d) {
            var yr = +d.year;
            if (!byYear.has(yr)) byYear.set(yr, []);
            byYear.get(yr).push({ day: +d.day_of_year, value: +d.value, date: d.date });
            // Later assignments win, so "most recent" outranks wettest/driest for the same year.
            if (d.is_driest && !special.has(yr)) special.set(yr, { color: CWR.color.series[1], label: "Driest", dash: "6,4" });
            if (d.is_wettest && !special.has(yr)) special.set(yr, { color: CWR.color.series[2], label: "Wettest", dash: null });
        });
        data.forEach(function (d) {
            if (d.is_most_recent) special.set(+d.year, { color: CWR.color.series[0], label: "Most recent", dash: null });
        });
        byYear.forEach(function (rows) {
            rows.sort(function (a, b) { return a.day - b.day; });
        });

        var allYears = Array.from(byYear.keys()).sort(d3.ascending);
        var maxExtra = config.maxAdditionalYears != null ? config.maxAdditionalYears : 20;
        var contextYears = allYears.filter(function (yr) { return !special.has(yr); });
        var totalContext = contextYears.length;
        contextYears = maxExtra > 0 ? contextYears.slice(-maxExtra) : [];
        var specialYears = allYears.filter(function (yr) { return special.has(yr); });
        var visibleYears = contextYears.concat(specialYears);

        var f = CWR.frame(container, config, { top: 40, right: 150, bottom: 50, left: 68 }, 850, 460);
        var showLegend = f.margin.right >= 150;
        var unit = config.yUnit || "AF";

        var x = d3.scaleLinear().domain([0, 365]).range([0, f.width]);
        var yMax = d3.max(visibleYears, function (yr) {
            return d3.max(byYear.get(yr), function (d) { return d.value; });
        });
        var y = d3.scaleLinear().domain([0, CWR.yMax(yMax, config) * 1.1]).nice().range([f.height, 0]);

        CWR.axes(f, x, y, config, d3.axisBottom(x)
            .tickValues(MONTHS.map(function (m) { return m.day; }))
            .tickFormat(function (_, i) { return MONTHS[i].label; })
            .tickSizeOuter(0));

        f.g.append("text")
            .attr("x", f.width / 2)
            .attr("y", f.height + 38)
            .attr("text-anchor", "middle")
            .style("font-size", "12px")
            .style("fill", CWR.color.textSecondary)
            .text("Water year (Oct 1 – Sep 30)");

        CWR.capacityLine(f, y, config);

        var line = d3.line()
            .defined(function (d) { return !isNaN(d.value); })
            .x(function (d) { return x(d.day); })
            .y(function (d) { return y(d.value); });

        // Context years first (thin, recessive), special years on top.
        f.g.append("g").selectAll("path")
            .data(contextYears)
            .join("path")
            .attr("fill", "none")
            .attr("stroke-width", 1)
            .style("stroke", CWR.color.other)
            .style("opacity", 0.45)
            .attr("d", function (yr) { return line(byYear.get(yr)); });

        f.g.append("g").selectAll("path")
            .data(specialYears)
            .join("path")
            .attr("fill", "none")
            .attr("stroke-width", 2.5)
            .attr("stroke-linejoin", "round")
            .attr("stroke-linecap", "round")
            .attr("stroke-dasharray", function (yr) { return special.get(yr).dash; })
            .style("stroke", function (yr) { return special.get(yr).color; })
            .attr("d", function (yr) { return line(byYear.get(yr)); });

        // Legend: the special years (newest first), then the context bucket.
        var legendItems = specialYears.slice().reverse().map(function (yr) {
            var s = special.get(yr);
            return { label: "WY " + yr + " · " + s.label, color: s.color, dash: s.dash, width: 2.5 };
        });
        if (contextYears.length) {
            legendItems.push({ label: contextYears.length + " other years", color: CWR.color.other, dash: null, width: 1.5 });
        }

        if (showLegend) {
            var legend = f.svg.append("g")
                .attr("transform", "translate(" + (f.margin.left + f.width + 16) + "," + (f.margin.top + 6) + ")");
            legendItems.forEach(function (item, i) {
                var row = legend.append("g").attr("transform", "translate(0," + (i * 20) + ")");
                row.append("line")
                    .attr("x2", 18)
                    .attr("stroke-width", item.width)
                    .attr("stroke-dasharray", item.dash)
                    .style("stroke", item.color);
                row.append("text")
                    .attr("x", 24).attr("y", 4)
                    .style("font-size", "11px")
                    .style("fill", CWR.color.textSecondary)
                    .text(item.label);
            });
        } else {
            // Narrow layout: the legend moves below the plot as HTML.
            var htmlLegend = container.append("div").attr("class", "cwr-legend");
            legendItems.forEach(function (item) {
                var el = htmlLegend.append("span").attr("class", "cwr-legend-item");
                el.append("span")
                    .style("display", "inline-block")
                    .style("width", "18px")
                    .style("border-top", item.width + "px " + (item.dash ? "dashed " : "solid ") + item.color);
                el.append("span").text(item.label);
            });
        }

        // Crosshair tooltip: special years individually, context years as a range.
        var crosshair = CWR.crosshair(f);
        var dots = new Map();
        specialYears.forEach(function (yr) { dots.set(yr, CWR.focusDot(f, special.get(yr).color)); });
        var byDay = function (p) { return p.day; };

        /** The year's observation within 5 days of `day`, if any. */
        function pointNear(yr, day) {
            var rows = byYear.get(yr);
            var d = rows[CWR.nearestIndex(rows, day, byDay)];
            return d && Math.abs(d.day - day) <= 5 && !isNaN(d.value) ? d : null;
        }

        CWR.overlay(f, function (event) {
            var day = Math.max(0, Math.min(365, Math.round(x.invert(d3.pointer(event)[0]))));
            crosshair.style("opacity", 1).attr("x1", x(day)).attr("x2", x(day));

            var rows = ["<strong>" + dayLabel(day) + "</strong>"];
            specialYears.slice().reverse().forEach(function (yr) {
                var s = special.get(yr);
                var d = pointNear(yr, day);
                dots.get(yr).style("opacity", d ? 1 : 0);
                if (!d) return;
                dots.get(yr).attr("cx", x(d.day)).attr("cy", y(d.value));
                rows.push(CWR.tipKey(s.color) + "WY " + yr + " (" + s.label.toLowerCase() + "): <strong>" +
                    CWR.formatValue(d.value) + "</strong> " + CWR.escapeHtml(unit));
            });

            var context = contextYears
                .map(function (yr) { return pointNear(yr, day); })
                .filter(Boolean)
                .map(function (d) { return d.value; });
            if (context.length) {
                rows.push('<span class="cwr-tip-muted">' + CWR.tipKey(CWR.color.other) + context.length +
                    " other years: " + CWR.formatValue(d3.min(context)) + " – " +
                    CWR.formatValue(d3.max(context)) + " " + CWR.escapeHtml(unit) + "</span>");
            }

            CWR.showTooltip(rows.join("<br/>"), event);
        }, function () {
            crosshair.style("opacity", 0);
            dots.forEach(function (dot) { dot.style("opacity", 0); });
            CWR.hideTooltip();
        });

        // Slider: how many grey context years to draw.
        if (totalContext > 0) {
            var sliderId = containerId + "-slider";
            var slider = container.append("div").attr("class", "cwr-slider");
            var label = slider.append("label").attr("for", sliderId);
            var setLabel = function (n) { label.text("Other years shown: " + n + " "); };
            setLabel(contextYears.length);
            slider.append("input")
                .attr("id", sliderId)
                .attr("type", "range")
                .attr("min", 0)
                .attr("max", totalContext)
                .attr("value", contextYears.length)
                .style("width", "200px")
                .style("vertical-align", "middle")
                .on("input", function () { setLabel(+this.value); })
                .on("change", function () {
                    var next = Object.assign({}, config, { maxAdditionalYears: +this.value });
                    CWR.renderWaterYearsChart(containerId, data, next);
                });
        }

    };
})();
