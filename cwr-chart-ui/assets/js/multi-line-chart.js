// California Water Reservoir — Multi-Line Overlay Chart
// Used by: chart-reservoir-history, chart-snow-history
// One line per station, with a shared crosshair tooltip listing every series.
//
// Data format (JSON array of series):
// [{ station_id: "SHA", name: "Shasta Lake",
//    data: [{ date: "YYYY-MM-DD" | "YYYYMMDD", value: <number> }, ...] }, ...]
//
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,       // e.g. "Acre-Feet (AF)" or "Snow Water Equivalent (inches)"
//   yUnit: string,            // e.g. "AF" or "in"
//   capacity: number (optional), // draws a dashed reference line
//   width, height: number (optional)
// }
//
// Colors come from the three validated categorical slots, assigned in series
// order. Callers pass at most three series; any extras share the neutral
// "other" color rather than getting an invented hue.

(function () {
    "use strict";
    var CWR = window.CWR;

    CWR.renderMultiLineChart = function (containerId, series, config) {
        series = (series || [])
            .map(function (s, i) {
                return {
                    id: s.station_id,
                    name: s.name || s.station_id,
                    color: CWR.color.series[i] || CWR.color.other,
                    data: (s.data || [])
                        .map(function (d) { return { date: CWR.parseDate(d.date), value: +d.value }; })
                        .filter(function (d) { return d.date != null && !isNaN(d.value); })
                        .sort(function (a, b) { return a.date - b.date; })
                };
            })
            .filter(function (s) { return s.data.length > 0; });

        var container = CWR.prepare(containerId, series.length > 0);
        if (!container) return;

        // A single series is named by the title; only reserve a legend gutter for several.
        var showLegend = series.length > 1;
        var f = CWR.frame(container, config,
            { top: 40, right: showLegend ? 150 : 20, bottom: 34, left: 68 }, 800, 440);
        showLegend = showLegend && f.margin.right >= 150;
        var unit = config.yUnit || "AF";

        var allPoints = d3.merge(series.map(function (s) { return s.data; }));
        var x = d3.scaleTime()
            .domain(d3.extent(allPoints, function (d) { return d.date; }))
            .range([0, f.width]);
        var y = d3.scaleLinear()
            .domain([0, CWR.yMax(d3.max(allPoints, function (d) { return d.value; }), config) * 1.1])
            .nice()
            .range([f.height, 0]);

        CWR.axes(f, x, y, config, d3.axisBottom(x).ticks(Math.min(f.width / 80, 12)).tickSizeOuter(0));

        CWR.capacityLine(f, y, config);

        var line = d3.line()
            .x(function (d) { return x(d.date); })
            .y(function (d) { return y(d.value); });

        var lines = f.g.append("g").selectAll("path")
            .data(series)
            .join("path")
            .attr("fill", "none")
            .attr("stroke-width", 2)
            .attr("stroke-linejoin", "round")
            .attr("stroke-linecap", "round")
            .style("stroke", function (s) { return s.color; })
            .attr("d", function (s) { return line(s.data); });

        function highlight(id) {
            lines.style("opacity", function (s) { return id == null || s.id === id ? 1 : 0.2; });
        }

        if (showLegend) {
            var legend = f.svg.append("g")
                .attr("transform", "translate(" + (f.margin.left + f.width + 16) + "," + (f.margin.top + 6) + ")");
            series.forEach(function (s, i) {
                var item = legend.append("g")
                    .attr("transform", "translate(0," + (i * 20) + ")")
                    .on("pointerenter", function () { highlight(s.id); })
                    .on("pointerleave", function () { highlight(null); });
                item.append("line")
                    .attr("x2", 16)
                    .attr("stroke-width", 2.5)
                    .style("stroke", s.color);
                item.append("text")
                    .attr("x", 22).attr("y", 4)
                    .style("font-size", "11px")
                    .style("fill", CWR.color.textSecondary)
                    .text(s.name);
            });
        }

        var crosshair = CWR.crosshair(f);
        series.forEach(function (s) { s.dot = CWR.focusDot(f, s.color); });
        var byDate = function (p) { return p.date; };

        CWR.overlay(f, function (event) {
            var x0 = x.invert(d3.pointer(event)[0]);

            // Snap the crosshair to the nearest actual observation across all series.
            var nearest = null;
            series.forEach(function (s) {
                var d = s.data[CWR.nearestIndex(s.data, x0, byDate)];
                if (nearest == null || Math.abs(d.date - x0) < Math.abs(nearest - x0)) nearest = d.date;
            });

            crosshair.style("opacity", 1).attr("x1", x(nearest)).attr("x2", x(nearest));

            var rows = series.map(function (s) {
                var d = s.data[CWR.nearestIndex(s.data, nearest, byDate)];
                s.dot.style("opacity", 1).attr("cx", x(d.date)).attr("cy", y(d.value));
                return CWR.tipKey(s.color) + CWR.escapeHtml(s.name) + ": <strong>" +
                    CWR.formatValue(d.value) + "</strong> " + CWR.escapeHtml(unit);
            });

            CWR.showTooltip("<strong>" + CWR.formatDate(nearest) + "</strong><br/>" + rows.join("<br/>"), event);
        }, function () {
            crosshair.style("opacity", 0);
            series.forEach(function (s) { s.dot.style("opacity", 0); });
            CWR.hideTooltip();
        });
    };
})();
