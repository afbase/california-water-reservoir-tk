// California Water Reservoir — Water Year Overlay Chart
// Used by: chart-water-years, chart-snow-years
// Renders each water year as a separate line overlaid on a single chart.
// The X-axis is day-of-water-year (0-364), labeled as months Oct..Sep.
// Three special years are always highlighted with thick distinct lines:
//   - Most recent complete water year: blue (#2196F3)
//   - Driest water year (lowest minimum): red (#F44336)
//   - Wettest water year (highest maximum): green (#4CAF50)
// Other years are rendered in grey with low opacity.
// Called from Rust/WASM via js_sys::eval(). No ES modules.
//
// Data format (JSON array):
// [
//   {
//     year: 2023,            // water year (starts Oct of year-1)
//     day_of_year: 0,        // 0 = Oct 1, 364 = Sep 30
//     date: "YYYY-MM-DD",    // actual calendar date (for tooltip)
//     value: <number>,
//     is_driest: bool,
//     is_wettest: bool,
//     is_most_recent: bool
//   },
//   ...
// ]
//
// Config format (JSON object): {
//   title: string,
//   yAxisLabel: string,       // e.g. "Acre-Feet (AF)" or "Snow Water Equivalent (inches)"
//   yUnit: string,            // e.g. "AF" or "in"
//   stationName: string,      // reservoir or snow station name
//   maxAdditionalYears: number, // default 20 — how many grey years to show
//   width: number (optional),
//   height: number (optional)
// }

// Water-year month labels positioned at approximate day offsets
var _WY_MONTH_TICKS = [
    { day:   0, label: "Oct" },
    { day:  31, label: "Nov" },
    { day:  61, label: "Dec" },
    { day:  92, label: "Jan" },
    { day: 122, label: "Feb" },
    { day: 150, label: "Mar" },
    { day: 181, label: "Apr" },
    { day: 211, label: "May" },
    { day: 242, label: "Jun" },
    { day: 272, label: "Jul" },
    { day: 303, label: "Aug" },
    { day: 334, label: "Sep" }
];

// Colors for the three special years
var _WY_COLOR_RECENT  = "#2196F3"; // blue
var _WY_COLOR_DRIEST  = "#F44336"; // red
var _WY_COLOR_WETTEST = "#4CAF50"; // green
var _WY_COLOR_NORMAL  = "#9E9E9E"; // grey

function renderWaterYearsChart(containerId, dataJson, configJson) {
    console.log('[CWR Debug D3] renderWaterYearsChart called');
    console.log('[CWR Debug D3] containerId:', containerId);
    console.log('[CWR Debug D3] dataJson length:', dataJson.length);
    console.log('[CWR Debug D3] configJson length:', configJson.length);

    try {
        var data = JSON.parse(dataJson);
        console.log('[CWR Debug D3] Parsed data:', {
            isArray: Array.isArray(data),
            length: data ? data.length : 0,
            firstItem: data ? data[0] : null
        });
    } catch(e) {
        console.error('[CWR Debug D3] JSON parse error (data):', e);
        return;
    }

    try {
        var config = JSON.parse(configJson);
        console.log('[CWR Debug D3] Parsed config successfully');
    } catch(e) {
        console.error('[CWR Debug D3] JSON parse error (config):', e);
        return;
    }

    var container = d3.select("#" + containerId);
    console.log('[CWR Debug D3] Container selected:', container.empty() ? 'EMPTY' : 'found');
    container.selectAll("*").remove();

    if (!data || !data.length) {
        console.log('[CWR Debug D3] No data available');
        container.append("p")
            .style("text-align", "center")
            .style("color", "#999")
            .text("No data available.");
        return;
    }

    // Ensure shared tooltip is ready
    if (typeof initTooltip === "function" && !_cwrTooltipDiv) initTooltip();

    var margin = { top: 48, right: 160, bottom: 70, left: 85 };
    var totalW = config.width || container.node().clientWidth || 850;
    var totalH = config.height || 460;
    var width  = totalW - margin.left - margin.right;
    var height = totalH - margin.top - margin.bottom;

    // --- Group data by water year ---
    var yearMap = {};  // year -> [{day_of_year, date, value, is_*}]
    var driestYear = null, wettestYear = null, recentYear = null;

    data.forEach(function(d) {
        d._value = +d.value;
        d._day   = +d.day_of_year;
        var yr = +d.year;
        if (!yearMap[yr]) yearMap[yr] = [];
        yearMap[yr].push(d);
        if (d.is_driest)     driestYear  = yr;
        if (d.is_wettest)    wettestYear = yr;
        if (d.is_most_recent) recentYear = yr;
    });

    // Sort each year's data by day_of_year
    var allYears = Object.keys(yearMap).map(Number).sort(function(a, b) { return a - b; });
    allYears.forEach(function(yr) {
        yearMap[yr].sort(function(a, b) { return a._day - b._day; });
    });

    // Identify special years set for quick lookup
    var specialYears = {};
    if (driestYear  != null) specialYears[driestYear]  = { color: _WY_COLOR_DRIEST,  label: "Driest",      dash: "6,3" };
    if (wettestYear != null) specialYears[wettestYear] = { color: _WY_COLOR_WETTEST, label: "Wettest",     dash: null  };
    if (recentYear  != null) specialYears[recentYear]  = { color: _WY_COLOR_RECENT,  label: "Most Recent", dash: null  };

    // Determine which "normal" years to show (limit to maxAdditionalYears)
    var maxExtra = (config.maxAdditionalYears != null) ? config.maxAdditionalYears : 20;
    var normalYears = allYears.filter(function(yr) { return !specialYears[yr]; });

    // If we have too many normal years, take the most recent ones
    if (normalYears.length > maxExtra) {
        normalYears = normalYears.slice(normalYears.length - maxExtra);
    }

    // Combined list: normal first (behind), special on top
    var visibleYears = normalYears.concat(
        allYears.filter(function(yr) { return !!specialYears[yr]; })
    );

    // --- Scales ---
    var x = d3.scaleLinear()
        .domain([0, 364])
        .range([0, width]);

    var globalYMax = 0;
    visibleYears.forEach(function(yr) {
        var localMax = d3.max(yearMap[yr], function(d) { return d._value; });
        if (localMax > globalYMax) globalYMax = localMax;
    });

    var y = d3.scaleLinear()
        .domain([0, globalYMax * 1.1])
        .nice()
        .range([height, 0]);

    // --- SVG ---
    var svg = container.append("svg")
        .attr("viewBox", "0 0 " + totalW + " " + totalH)
        .attr("preserveAspectRatio", "xMidYMid meet")
        .style("width", "100%")
        .style("max-width", totalW + "px");

    var g = svg.append("g")
        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");

    // --- X-axis (month labels) ---
    var xAxis = d3.axisBottom(x)
        .tickValues(_WY_MONTH_TICKS.map(function(t) { return t.day; }))
        .tickFormat(function(d) {
            var match = _WY_MONTH_TICKS.find(function(t) { return t.day === d; });
            return match ? match.label : "";
        });

    g.append("g")
        .attr("class", "x-axis")
        .attr("transform", "translate(0," + height + ")")
        .call(xAxis)
        .selectAll("text")
            .style("font-size", "11px");

    // X-axis label
    g.append("text")
        .attr("x", width / 2)
        .attr("y", height + 40)
        .attr("text-anchor", "middle")
        .style("font-size", "12px")
        .style("fill", "#555")
        .text("Day of Water Year (Oct 1 \u2013 Sep 30)");

    // --- Y-axis ---
    g.append("g")
        .attr("class", "y-axis")
        .call(d3.axisLeft(y).ticks(8).tickFormat(d3.format(",.0f")))
        .selectAll("text")
            .style("font-size", "11px");

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
        .text(config.title || "Water Years");

    // Subtitle (station name)
    if (config.stationName) {
        svg.append("text")
            .attr("x", totalW / 2)
            .attr("y", 38)
            .attr("text-anchor", "middle")
            .style("font-size", "12px")
            .style("fill", "#777")
            .text(config.stationName);
    }

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

    // --- Draw lines ---
    var lineGen = d3.line()
        .defined(function(d) { return !isNaN(d._value); })
        .x(function(d) { return x(d._day); })
        .y(function(d) { return y(d._value); });

    var linesGroup = g.append("g").attr("class", "wy-lines");

    // Normal years first (thin grey, low opacity)
    normalYears.forEach(function(yr) {
        linesGroup.append("path")
            .datum(yearMap[yr])
            .attr("class", "wy-line wy-normal")
            .attr("data-year", yr)
            .attr("fill", "none")
            .attr("stroke", _WY_COLOR_NORMAL)
            .attr("stroke-width", 1)
            .attr("stroke-opacity", 0.3)
            .attr("d", lineGen);
    });

    // Special years on top (thick, distinctive)
    var specialKeys = Object.keys(specialYears).map(Number);
    specialKeys.forEach(function(yr) {
        var spec = specialYears[yr];
        var path = linesGroup.append("path")
            .datum(yearMap[yr])
            .attr("class", "wy-line wy-special")
            .attr("data-year", yr)
            .attr("fill", "none")
            .attr("stroke", spec.color)
            .attr("stroke-width", 3)
            .attr("stroke-linejoin", "round")
            .attr("stroke-linecap", "round")
            .attr("d", lineGen);

        if (spec.dash) {
            path.attr("stroke-dasharray", spec.dash);
        }
    });

    // --- Legend (right side) ---
    var legendGroup = svg.append("g")
        .attr("class", "legend")
        .attr("transform", "translate(" + (margin.left + width + 14) + "," + (margin.top + 4) + ")");

    var legendItems = [];
    if (recentYear != null) {
        legendItems.push({ year: recentYear, label: "WY " + recentYear + " (Recent)", color: _WY_COLOR_RECENT, dash: null });
    }
    if (wettestYear != null) {
        legendItems.push({ year: wettestYear, label: "WY " + wettestYear + " (Wettest)", color: _WY_COLOR_WETTEST, dash: null });
    }
    if (driestYear != null) {
        legendItems.push({ year: driestYear, label: "WY " + driestYear + " (Driest)", color: _WY_COLOR_DRIEST, dash: "6,3" });
    }
    legendItems.push({ year: null, label: "Other years", color: _WY_COLOR_NORMAL, dash: null });

    legendItems.forEach(function(item, i) {
        var lg = legendGroup.append("g")
            .attr("transform", "translate(0," + (i * 20) + ")");

        var legendLine = lg.append("line")
            .attr("x1", 0).attr("y1", 0)
            .attr("x2", 18).attr("y2", 0)
            .attr("stroke", item.color)
            .attr("stroke-width", item.year ? 3 : 1.5);

        if (item.dash) {
            legendLine.attr("stroke-dasharray", item.dash);
        }
        if (!item.year) {
            legendLine.attr("stroke-opacity", 0.5);
        }

        lg.append("text")
            .attr("x", 24).attr("y", 4)
            .style("font-size", "11px")
            .style("fill", "#333")
            .text(item.label);
    });

    // --- Bisector tooltip (all visible years at hovered day) ---
    var bisect = d3.bisector(function(d) { return d._day; }).left;

    var focusLine = g.append("line")
        .attr("class", "focus-line")
        .style("stroke", "#999")
        .style("stroke-dasharray", "3,3")
        .style("opacity", 0)
        .attr("y1", 0)
        .attr("y2", height);

    // Focus dots for special years only (to avoid clutter)
    var specialDots = {};
    specialKeys.forEach(function(yr) {
        specialDots[yr] = g.append("circle")
            .attr("class", "focus-dot")
            .attr("r", 5)
            .style("fill", specialYears[yr].color)
            .style("stroke", "#fff")
            .style("stroke-width", 2)
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
            var dayX = x.invert(coords[0]);
            var dayIdx = Math.round(dayX);
            if (dayIdx < 0) dayIdx = 0;
            if (dayIdx > 364) dayIdx = 364;

            focusLine
                .style("opacity", 1)
                .attr("x1", x(dayIdx))
                .attr("x2", x(dayIdx));

            var fmt = d3.format(",.0f");
            var dateFmt = d3.timeFormat("%b %d, %Y");
            var rows = [];

            // Determine the water-year month label for this day
            var monthLabel = "";
            for (var mi = _WY_MONTH_TICKS.length - 1; mi >= 0; mi--) {
                if (dayIdx >= _WY_MONTH_TICKS[mi].day) {
                    monthLabel = _WY_MONTH_TICKS[mi].label;
                    break;
                }
            }
            var dayInMonth = dayIdx - (_WY_MONTH_TICKS.find(function(t) { return t.label === monthLabel; }) || {day:0}).day + 1;

            // Header
            rows.push("<strong>Day " + dayIdx + " (" + monthLabel + " ~" + dayInMonth + ")</strong>");

            // Show special years first, then a summary of others
            var specialRows = [];
            var normalValues = [];

            visibleYears.forEach(function(yr) {
                var yearData = yearMap[yr];
                if (!yearData || !yearData.length) return;

                var i = bisect(yearData, dayIdx, 0);
                // Clamp
                if (i >= yearData.length) i = yearData.length - 1;
                var d = yearData[i];
                // Try to find the closest day
                if (i > 0) {
                    var d0 = yearData[i - 1];
                    if (Math.abs(d0._day - dayIdx) < Math.abs(d._day - dayIdx)) {
                        d = d0;
                    }
                }
                // Only show if the point is within 5 days of the hovered position
                if (Math.abs(d._day - dayIdx) > 5) return;

                if (specialYears[yr]) {
                    var spec = specialYears[yr];
                    // Position dot
                    if (specialDots[yr]) {
                        specialDots[yr]
                            .style("opacity", 1)
                            .attr("cx", x(d._day))
                            .attr("cy", y(d._value));
                    }
                    var dateStr = d.date ? " (" + d.date + ")" : "";
                    specialRows.push(
                        '<span style="color:' + spec.color + ';font-weight:bold;">WY ' +
                        yr + ' ' + spec.label + '</span>: ' +
                        fmt(d._value) + ' ' + (config.yUnit || 'AF') + dateStr
                    );
                } else {
                    normalValues.push({ year: yr, value: d._value });
                }
            });

            specialRows.forEach(function(r) { rows.push(r); });

            // Summarize normal years (show range)
            if (normalValues.length > 0) {
                var minNorm = d3.min(normalValues, function(v) { return v.value; });
                var maxNorm = d3.max(normalValues, function(v) { return v.value; });
                rows.push(
                    '<span style="color:#999;">' + normalValues.length + ' other years</span>: ' +
                    fmt(minNorm) + ' \u2013 ' + fmt(maxNorm) + ' ' + (config.yUnit || 'AF')
                );
            }

            showTooltip(rows.join("<br/>"), event.pageX, event.pageY);
        })
        .on("mouseleave", function() {
            focusLine.style("opacity", 0);
            specialKeys.forEach(function(yr) {
                if (specialDots[yr]) specialDots[yr].style("opacity", 0);
            });
            hideTooltip();
        });

    // --- Line hover: highlight individual year on mouseover ---
    linesGroup.selectAll(".wy-line")
        .style("cursor", "pointer")
        .on("mouseover", function() {
            var hoveredYear = +d3.select(this).attr("data-year");
            linesGroup.selectAll(".wy-normal")
                .attr("stroke-opacity", function() {
                    return +d3.select(this).attr("data-year") === hoveredYear ? 0.9 : 0.1;
                })
                .attr("stroke-width", function() {
                    return +d3.select(this).attr("data-year") === hoveredYear ? 2.5 : 1;
                });
            // Dim special lines slightly if hovering a normal line
            if (!specialYears[hoveredYear]) {
                linesGroup.selectAll(".wy-special")
                    .style("opacity", 0.3);
            }
        })
        .on("mouseout", function() {
            linesGroup.selectAll(".wy-normal")
                .attr("stroke-opacity", 0.3)
                .attr("stroke-width", 1);
            linesGroup.selectAll(".wy-special")
                .style("opacity", 1);
        });

    // --- Slider control for number of additional years ---
    _renderWaterYearsSlider(container, containerId, dataJson, configJson, maxExtra, allYears.length);
}

// Render a slider below the chart to control how many "other" years are visible
function _renderWaterYearsSlider(container, containerId, dataJson, configJson, currentMax, totalYears) {
    var sliderDiv = container.append("div")
        .style("text-align", "center")
        .style("margin-top", "8px")
        .style("font-size", "13px")
        .style("color", "#555");

    sliderDiv.append("label")
        .attr("for", containerId + "-slider")
        .text("Additional years shown: " + currentMax + " ");

    var slider = sliderDiv.append("input")
        .attr("id", containerId + "-slider")
        .attr("type", "range")
        .attr("min", 0)
        .attr("max", Math.max(totalYears, 50))
        .attr("value", currentMax)
        .style("width", "200px")
        .style("vertical-align", "middle");

    var valueLabel = sliderDiv.append("span")
        .text(" (" + currentMax + ")");

    slider.on("input", function() {
        var newMax = +this.value;
        valueLabel.text(" (" + newMax + ")");
        sliderDiv.select("label").text("Additional years shown: " + newMax + " ");

        // Re-render the chart with updated maxAdditionalYears
        var newConfig = JSON.parse(configJson);
        newConfig.maxAdditionalYears = newMax;
        // Remove just the SVG and re-render (preserve slider)
        container.select("svg").remove();
        container.selectAll("div").remove();
        renderWaterYearsChart(containerId, dataJson, JSON.stringify(newConfig));
    });
}

function destroyWaterYearsChart(containerId) {
    d3.select("#" + containerId).selectAll("*").remove();
}
