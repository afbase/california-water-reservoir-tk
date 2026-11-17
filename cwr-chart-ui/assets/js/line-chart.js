// California Water Reservoir — Single Time-Series Line Chart
// Used by: chart-total-water, chart-total-snow, chart-cumulative-water, chart-local-reservoirs
//
// Data format (JSON array): [{ date: "YYYY-MM-DD" | "YYYYMMDD", value: <number> }, ...]
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,       // e.g. "Acre-Feet (AF)"
//   yUnit: string,            // e.g. "AF" — shown in tooltip
//   capacity: number (optional), // draws a dashed reference line
//   series: number (optional),// 1-based categorical slot for the line color (default 1)
//   width, height: number (optional)
// }

(function () {
    "use strict";
    var CWR = window.CWR;

    CWR.renderLineChart = function (containerId, data, config) {
        data = (data || [])
            .map(function (d) { return { date: CWR.parseDate(d.date), value: +d.value }; })
            .filter(function (d) { return d.date != null && !isNaN(d.value); })
            .sort(function (a, b) { return a.date - b.date; });

        var container = CWR.prepare(containerId, data.length > 0);
        if (!container) return;

        var f = CWR.frame(container, config, { top: config.title ? 40 : 14, right: 20, bottom: 34, left: 68 }, 700, 400);
        var color = CWR.color.series[(config.series || 1) - 1] || CWR.color.series[0];
        var unit = config.yUnit || "AF";

        var x = d3.scaleTime()
            .domain(d3.extent(data, function (d) { return d.date; }))
            .range([0, f.width]);
        var y = d3.scaleLinear()
            .domain([0, CWR.yMax(d3.max(data, function (d) { return d.value; }), config) * 1.1])
            .nice()
            .range([f.height, 0]);

        CWR.axes(f, x, y, config, d3.axisBottom(x).ticks(Math.min(f.width / 80, 12)).tickSizeOuter(0));

        CWR.capacityLine(f, y, config);

        f.g.append("path")
            .datum(data)
            .attr("fill", "none")
            .attr("stroke-width", 2)
            .attr("stroke-linejoin", "round")
            .attr("stroke-linecap", "round")
            .style("stroke", color)
            .attr("d", d3.line()
                .x(function (d) { return x(d.date); })
                .y(function (d) { return y(d.value); }));

        var crosshair = CWR.crosshair(f);
        var dot = CWR.focusDot(f, color);

        CWR.overlay(f, function (event) {
            var d = data[CWR.nearestIndex(data, x.invert(d3.pointer(event)[0]), function (p) { return p.date; })];
            crosshair.style("opacity", 1).attr("x1", x(d.date)).attr("x2", x(d.date));
            dot.style("opacity", 1).attr("cx", x(d.date)).attr("cy", y(d.value));
            CWR.showTooltip(
                "<strong>" + CWR.formatDate(d.date) + "</strong><br/>" +
                CWR.formatValue(d.value) + " " + CWR.escapeHtml(unit),
                event
            );
        }, function () {
            crosshair.style("opacity", 0);
            dot.style("opacity", 0);
            CWR.hideTooltip();
        });
    };
})();
