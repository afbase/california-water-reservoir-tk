// California Water Reservoir — Multi-Line Overlay Chart
// Used by: chart-reservoir-history, chart-snow-history
// Renders multiple reservoir/station time-series as overlaid lines with
// per-series colors, hover highlighting, and a multi-series bisector tooltip.
// Called from Rust/WASM via js_sys::eval(). No ES modules.
//
// Data format (JSON array of series):
// [
//   {
//     station_id: "SHA",
//     name: "Shasta Lake",
//     data: [{ date: "YYYY-MM-DD", value: <number> }, ...]
//   },
//   ...
// ]
//
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,       // e.g. "Acre-Feet (AF)" or "Snow Water Equivalent (inches)"
//   yUnit: string,            // e.g. "AF" or "in"
//   width: number (optional),
//   height: number (optional),
//   colors: [string] (optional) // override d3.schemeCategory10
// }

function renderMultiLineChart(containerId, seriesJson, configJson) {
    var series = JSON.parse(seriesJson);
    var config = JSON.parse(configJson);

    var container = d3.select("#" + containerId);
    container.selectAll("*").remove();

    if (!series || !series.length) {
        container.append("p")
            .style("text-align", "center")
            .style("color", "#999")
            .text("No data available.");
        return;
    }

    // Ensure shared tooltip is ready
    if (typeof initTooltip === "function" && !_cwrTooltipDiv) initTooltip();

    var margin = { top: 44, right: 140, bottom: 54, left: 85 };
    var totalW = config.width || container.node().clientWidth || 800;
    var totalH = config.height || 440;
    var width  = totalW - margin.left - margin.right;
    var height = totalH - margin.top - margin.bottom;

    var svg = container.append("svg")
        .attr("viewBox", "0 0 " + totalW + " " + totalH)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .style("width", "100%")
        .style("max-width", totalW + "px");

    var g = svg.append("g")
        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");

    // Color scale
    var colorPalette = config.colors || d3.schemeCategory10;
    var colorScale = d3.scaleOrdinal()
        .domain(series.map(function(s) { return s.station_id; }))
        .range(colorPalette);

    // Parse dates for every series
    var parseDate = d3.timeParse("%Y-%m-%d");
    var globalXMin = null, globalXMax = null, globalYMax = 0;

    series.forEach(function(s) {
        s.data.forEach(function(d) {
            d._date  = parseDate(d.date);
            d._value = +d.value;
        });
        // Remove invalid
        s.data = s.data.filter(function(d) { return d._date != null && !isNaN(d._value); });
        // Sort chronologically
        s.data.sort(function(a, b) { return a._date - b._date; });

        if (s.data.length) {
            var ext = d3.extent(s.data, function(d) { return d._date; });
            if (!globalXMin || ext[0] < globalXMin) globalXMin = ext[0];
            if (!globalXMax || ext[1] > globalXMax) globalXMax = ext[1];
            var localMax = d3.max(s.data, function(d) { return d._value; });
            if (localMax > globalYMax) globalYMax = localMax;
        }
    });

    if (!globalXMin) return;

    // --- Scales ---
    var x = d3.scaleTime()
        .domain([globalXMin, globalXMax])
        .range([0, width]);

    var y = d3.scaleLinear()
        .domain([0, globalYMax * 1.1])
        .nice()
        .range([height, 0]);

    // --- Axes ---
    g.append("g")
        .attr("class", "x-axis")
        .attr("transform", "translate(0," + height + ")")
        .call(d3.axisBottom(x).ticks(Math.min(width / 80, 12)))
        .selectAll("text")
            .style("font-size", "11px");

    g.append("g")
        .attr("class", "y-axis")
        .call(d3.axisLeft(y).ticks(8).tickFormat(d3.format(",.0f")))
        .selectAll("text")
            .style("font-size", "11px");

    // --- Y-axis label ---
    g.append("text")
        .attr("transform", "rotate(-90)")
        .attr("y", -margin.left + 16)
        .attr("x", -height / 2)
        .attr("text-anchor", "middle")
        .style("font-size", "12px")
        .style("fill", "#555")
        .text(config.yAxisLabel || "Acre-Feet (AF)");

    // --- Title ---
    svg.append("text")
        .attr("x", totalW / 2)
        .attr("y", 22)
        .attr("text-anchor", "middle")
        .style("font-size", "15px")
        .style("font-weight", "bold")
        .text(config.title || "");

    // --- Grid lines ---
    g.append("g")
        .attr("class", "grid")
        .call(
            d3.axisLeft(y)
                .ticks(8)
                .tickSize(-width)
                .tickFormat("")
        )
        .selectAll("line")
            .style("stroke", "#e0e0e0")
            .style("stroke-dasharray", "2,2");
    g.select(".grid .domain").remove();

    // --- Lines ---
    var lineGen = d3.line()
        .defined(function(d) { return !isNaN(d._value); })
        .x(function(d) { return x(d._date); })
        .y(function(d) { return y(d._value); });

    var linesGroup = g.append("g").attr("class", "lines-group");

    series.forEach(function(s) {
        if (!s.data.length) return;
        linesGroup.append("path")
            .datum(s.data)
            .attr("class", "series-line")
            .attr("data-station", s.station_id)
            .attr("fill", "none")
            .attr("stroke", colorScale(s.station_id))
            .attr("stroke-width", 2)
            .attr("stroke-linejoin", "round")
            .attr("stroke-linecap", "round")
            .attr("d", lineGen);
    });

    // --- Legend (right side) ---
    var legendGroup = svg.append("g")
        .attr("class", "legend")
        .attr("transform", "translate(" + (margin.left + width + 14) + "," + (margin.top + 4) + ")");

    series.forEach(function(s, i) {
        var lg = legendGroup.append("g")
            .attr("transform", "translate(0," + (i * 18) + ")")
            .style("cursor", "pointer");

        lg.append("line")
            .attr("x1", 0).attr("y1", 0)
            .attr("x2", 16).attr("y2", 0)
            .attr("stroke", colorScale(s.station_id))
            .attr("stroke-width", 2.5);

        lg.append("text")
            .attr("x", 20).attr("y", 4)
            .style("font-size", "11px")
            .style("fill", "#333")
            .text(s.name || s.station_id);

        // Legend hover: highlight the corresponding line
        lg.on("mouseover", function() {
                _highlightSeries(s.station_id, linesGroup, colorScale);
            })
            .on("mouseout", function() {
                _resetSeriesHighlight(linesGroup, colorScale);
            });
    });

    // --- Bisector tooltip (multi-series) ---
    var bisect = d3.bisector(function(d) { return d._date; }).left;

    var focusLine = g.append("line")
        .attr("class", "focus-line")
        .style("stroke", "#999")
        .style("stroke-dasharray", "3,3")
        .style("opacity", 0)
        .attr("y1", 0)
        .attr("y2", height);

    // One focus dot per series
    var focusDots = {};
    series.forEach(function(s) {
        focusDots[s.station_id] = g.append("circle")
            .attr("class", "focus-dot")
            .attr("r", 4)
            .style("fill", colorScale(s.station_id))
            .style("stroke", "#fff")
            .style("stroke-width", 1.5)
            .style("opacity", 0);
    });

    g.append("rect")
        .attr("class", "overlay")
        .attr("width", width)
        .attr("height", height)
        .style("fill", "none")
        .style("pointer-events", "all")
        .on("mousemove", function(event) {
            var coords = d3.pointer(event);
            var x0 = x.invert(coords[0]);
            var fmt = d3.format(",.0f");
            var dateFmt = d3.timeFormat("%b %d, %Y");

            // Find nearest date across all series
            var nearestDate = null;
            var minDist = Infinity;
            series.forEach(function(s) {
                if (!s.data.length) return;
                var i = bisect(s.data, x0, 1);
                if (i >= s.data.length) i = s.data.length - 1;
                if (i < 1) i = 1;
                var d0 = s.data[i - 1];
                var d1 = s.data[i];
                var d = (!d1 || (x0 - d0._date > d1._date - x0)) ? (d1 || d0) : d0;
                if (d) {
                    var dist = Math.abs(d._date - x0);
                    if (dist < minDist) {
                        minDist = dist;
                        nearestDate = d._date;
                    }
                }
            });

            if (!nearestDate) return;

            focusLine
                .style("opacity", 1)
                .attr("x1", x(nearestDate))
                .attr("x2", x(nearestDate));

            // Build tooltip rows and position dots
            var rows = [];
            var displayDate = dateFmt(nearestDate);
            series.forEach(function(s) {
                if (!s.data.length) return;
                var i = bisect(s.data, nearestDate, 1);
                if (i >= s.data.length) i = s.data.length - 1;
                if (i < 1) i = 1;
                var d0 = s.data[i - 1];
                var d1 = s.data[i];
                var d = (!d1 || (nearestDate - d0._date > d1._date - nearestDate)) ? (d1 || d0) : d0;
                if (d) {
                    focusDots[s.station_id]
                        .style("opacity", 1)
                        .attr("cx", x(d._date))
                        .attr("cy", y(d._value));
                    var colorHex = colorScale(s.station_id);
                    rows.push(
                        '<span style="color:' + colorHex + ';font-weight:bold;">' +
                        (s.name || s.station_id) + '</span>: ' +
                        fmt(d._value) + ' ' + (config.yUnit || 'AF')
                    );
                }
            });

            showTooltip(
                "<strong>" + displayDate + "</strong><br/>" + rows.join("<br/>"),
                event.pageX,
                event.pageY
            );
        })
        .on("mouseleave", function() {
            focusLine.style("opacity", 0);
            series.forEach(function(s) {
                focusDots[s.station_id].style("opacity", 0);
            });
            hideTooltip();
        });

    // --- Line hover highlighting ---
    linesGroup.selectAll(".series-line")
        .on("mouseover", function() {
            var sid = d3.select(this).attr("data-station");
            _highlightSeries(sid, linesGroup, colorScale);
        })
        .on("mouseout", function() {
            _resetSeriesHighlight(linesGroup, colorScale);
        });
}

// Internal: highlight one series, dim others
function _highlightSeries(stationId, linesGroup, colorScale) {
    linesGroup.selectAll(".series-line").each(function() {
        var el = d3.select(this);
        var sid = el.attr("data-station");
        if (sid === stationId) {
            el.attr("stroke-width", 4).style("opacity", 1);
        } else {
            el.attr("stroke-width", 1.5).style("opacity", 0.2);
        }
    });
}

// Internal: reset all series to default
function _resetSeriesHighlight(linesGroup, colorScale) {
    linesGroup.selectAll(".series-line")
        .attr("stroke-width", 2)
        .style("opacity", 1);
}

function destroyMultiLineChart(containerId) {
    d3.select("#" + containerId).selectAll("*").remove();
}
