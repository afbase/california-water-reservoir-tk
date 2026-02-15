// California Water Reservoir — Single Time-Series Line Chart
// Used by: chart-total-water, chart-cumulative-water, chart-local-reservoirs
// Renders a single line with bisector-based tooltip showing date + value.
// Called from Rust/WASM via js_sys::eval(). No ES modules.
//
// Data format (JSON array): [{ date: "YYYY-MM-DD", value: <number> }, ...]
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,       // e.g. "Acre-Feet (AF)"
//   yUnit: string,            // e.g. "AF" — shown in tooltip
//   color: string,            // e.g. "#2196F3"
//   width: number (optional), // falls back to container width or 700
//   height: number (optional) // falls back to 400
// }

function renderLineChart(containerId, dataJson, configJson) {
    var data = JSON.parse(dataJson);
    var config = JSON.parse(configJson);

    var container = d3.select("#" + containerId);
    container.selectAll("*").remove();

    if (!data || !data.length) {
        container.append("p")
            .style("text-align", "center")
            .style("color", "#999")
            .text("No data available.");
        return;
    }

    // Ensure shared tooltip is ready
    if (typeof initTooltip === "function" && !_cwrTooltipDiv) initTooltip();

    var margin = { top: 44, right: 30, bottom: 54, left: 85 };
    var totalW = config.width || container.node().clientWidth || 700;
    var totalH = config.height || 400;
    var width  = totalW - margin.left - margin.right;
    var height = totalH - margin.top - margin.bottom;

    var svg = container.append("svg")
        .attr("viewBox", "0 0 " + totalW + " " + totalH)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .style("width", "100%")
        .style("max-width", totalW + "px");

    var g = svg.append("g")
        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");

    // Parse dates and values
    var parseDate = d3.timeParse("%Y-%m-%d");
    data.forEach(function(d) {
        d._date  = parseDate(d.date);
        d._value = +d.value;
    });

    // Filter out rows with invalid dates
    data = data.filter(function(d) { return d._date != null; });
    if (!data.length) return;

    // Sort chronologically for bisector
    data.sort(function(a, b) { return a._date - b._date; });

    // --- Scales ---
    var x = d3.scaleTime()
        .domain(d3.extent(data, function(d) { return d._date; }))
        .range([0, width]);

    var yMax = d3.max(data, function(d) { return d._value; });
    var y = d3.scaleLinear()
        .domain([0, yMax * 1.1])
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

    // --- Grid lines (light horizontal) ---
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

    // --- Line ---
    var lineColor = config.color || "steelblue";

    var line = d3.line()
        .defined(function(d) { return !isNaN(d._value); })
        .curve(d3.curveMonotoneX)
        .x(function(d) { return x(d._date); })
        .y(function(d) { return y(d._value); });

    g.append("path")
        .datum(data)
        .attr("fill", "none")
        .attr("stroke", lineColor)
        .attr("stroke-width", 1.5)
        .attr("stroke-linejoin", "round")
        .attr("stroke-linecap", "round")
        .attr("d", line);

    // --- Bisector tooltip interaction ---
    var bisect = d3.bisector(function(d) { return d._date; }).left;

    var focusLine = g.append("line")
        .attr("class", "focus-line")
        .style("stroke", "#999")
        .style("stroke-dasharray", "3,3")
        .style("opacity", 0)
        .attr("y1", 0)
        .attr("y2", height);

    var focusDot = g.append("circle")
        .attr("class", "focus-dot")
        .attr("r", 5)
        .style("fill", lineColor)
        .style("stroke", "#fff")
        .style("stroke-width", 2)
        .style("opacity", 0);

    // Invisible overlay rect for mouse events
    g.append("rect")
        .attr("class", "overlay")
        .attr("width", width)
        .attr("height", height)
        .style("fill", "none")
        .style("pointer-events", "all")
        .on("mousemove", function(event) {
            var coords = d3.pointer(event);
            var x0 = x.invert(coords[0]);
            var i = bisect(data, x0, 1);
            if (i >= data.length) i = data.length - 1;
            if (i < 1) i = 1;
            var d0 = data[i - 1];
            var d1 = data[i];
            var d = (!d1 || (x0 - d0._date > d1._date - x0)) ? (d1 || d0) : d0;
            if (!d) return;

            focusLine
                .style("opacity", 1)
                .attr("x1", x(d._date))
                .attr("x2", x(d._date));

            focusDot
                .style("opacity", 1)
                .attr("cx", x(d._date))
                .attr("cy", y(d._value));

            var fmt = d3.format(",.0f");
            var dateFmt = d3.timeFormat("%b %d, %Y");
            showTooltip(
                "<strong>" + dateFmt(d._date) + "</strong><br/>" +
                fmt(d._value) + " " + (config.yUnit || "AF"),
                event.pageX,
                event.pageY
            );
        })
        .on("mouseleave", function() {
            focusLine.style("opacity", 0);
            focusDot.style("opacity", 0);
            hideTooltip();
        });
}

function destroyLineChart(containerId) {
    d3.select("#" + containerId).selectAll("*").remove();
}
