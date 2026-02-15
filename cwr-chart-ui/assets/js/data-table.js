// California Water Reservoir — Sortable Data Table with Row Highlighting
// Used by: table-water-year-stats, table-snow-stats
// Renders a dynamically sortable HTML table with conditional row coloring.
// Called from Rust/WASM via js_sys::eval(). No ES modules.
//
// Data format (JSON array):
// [
//   {
//     year: 2023,
//     date_lowest: "YYYY-MM-DD",
//     lowest_value: <number>,
//     date_highest: "YYYY-MM-DD",
//     highest_value: <number>,
//     is_driest: bool,        // row gets red background
//     is_wettest: bool,       // row gets green background
//     range: <number>         // optional: highest - lowest
//   },
//   ...
// ]
//
// Config format (JSON object): {
//   title: string,
//   yUnit: string,            // e.g. "AF" or "in"
//   stationName: string,
//   columns: [                // column definitions
//     { key: "year",           header: "Water Year",     type: "water_year" },
//     { key: "date_highest",   header: "Date of Peak",   type: "date" },
//     { key: "highest_value",  header: "Peak Value",     type: "number" },
//     { key: "date_lowest",    header: "Date of Low",    type: "date" },
//     { key: "lowest_value",   header: "Low Value",      type: "number" },
//     { key: "range",          header: "Range",          type: "number" }
//   ]
// }

// Row background colors
var _DT_COLOR_DRIEST  = "#FFCDD2"; // light red
var _DT_COLOR_WETTEST = "#C8E6C9"; // light green
var _DT_COLOR_ZERO    = "#FFF9C4"; // light yellow (zero-value years)

function renderDataTable(containerId, dataJson, configJson) {
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

    // Default columns if not provided
    var columns = config.columns || [
        { key: "year",          header: "Water Year",   type: "water_year" },
        { key: "date_highest",  header: "Date of Peak", type: "date" },
        { key: "highest_value", header: "Peak Value",   type: "number" },
        { key: "date_lowest",   header: "Date of Low",  type: "date" },
        { key: "lowest_value",  header: "Low Value",    type: "number" }
    ];

    var unit = config.yUnit || "AF";

    // --- Title ---
    if (config.title) {
        container.append("h3")
            .style("text-align", "center")
            .style("margin", "0 0 4px 0")
            .style("font-size", "15px")
            .text(config.title);
    }
    if (config.stationName) {
        container.append("p")
            .style("text-align", "center")
            .style("margin", "0 0 10px 0")
            .style("font-size", "12px")
            .style("color", "#777")
            .text(config.stationName);
    }

    // --- Legend for row colors ---
    var legendDiv = container.append("div")
        .style("display", "flex")
        .style("justify-content", "center")
        .style("gap", "16px")
        .style("margin-bottom", "8px")
        .style("font-size", "12px");

    var legendItems = [
        { color: _DT_COLOR_WETTEST, label: "Wettest year" },
        { color: _DT_COLOR_DRIEST,  label: "Driest year" },
        { color: _DT_COLOR_ZERO,    label: "Zero-value year" }
    ];
    legendItems.forEach(function(item) {
        var span = legendDiv.append("span")
            .style("display", "inline-flex")
            .style("align-items", "center")
            .style("gap", "4px");
        span.append("span")
            .style("display", "inline-block")
            .style("width", "14px")
            .style("height", "14px")
            .style("background", item.color)
            .style("border", "1px solid #ccc")
            .style("border-radius", "2px");
        span.append("span").text(item.label);
    });

    // --- Table wrapper (scrollable) ---
    var wrapper = container.append("div")
        .style("overflow-x", "auto")
        .style("max-height", "500px")
        .style("overflow-y", "auto");

    var table = wrapper.append("table")
        .style("width", "100%")
        .style("border-collapse", "collapse")
        .style("font-size", "13px")
        .style("font-family", "system-ui, -apple-system, sans-serif");

    // --- Header ---
    var thead = table.append("thead");
    var headerRow = thead.append("tr")
        .style("position", "sticky")
        .style("top", "0")
        .style("background", "#f5f5f5")
        .style("z-index", "1");

    // Track sort state
    var currentSortKey = "year";
    var currentSortAsc = false; // most recent first by default

    columns.forEach(function(col) {
        var th = headerRow.append("th")
            .style("padding", "8px 10px")
            .style("text-align", col.type === "number" ? "right" : "left")
            .style("border-bottom", "2px solid #999")
            .style("cursor", "pointer")
            .style("user-select", "none")
            .style("white-space", "nowrap")
            .attr("data-key", col.key);

        th.append("span").text(col.header);

        // Unit indicator for number columns
        if (col.type === "number") {
            th.append("span")
                .style("font-size", "10px")
                .style("color", "#999")
                .style("margin-left", "4px")
                .text("(" + unit + ")");
        }

        // Sort arrow indicator
        th.append("span")
            .attr("class", "sort-arrow")
            .style("margin-left", "4px")
            .style("font-size", "10px")
            .text(col.key === currentSortKey ? (currentSortAsc ? "\u25B2" : "\u25BC") : "");

        th.on("click", function() {
            if (currentSortKey === col.key) {
                currentSortAsc = !currentSortAsc;
            } else {
                currentSortKey = col.key;
                currentSortAsc = true;
            }
            _sortAndRender(table, data, columns, currentSortKey, currentSortAsc, unit);
            // Update arrows
            headerRow.selectAll(".sort-arrow").text("");
            th.select(".sort-arrow").text(currentSortAsc ? "\u25B2" : "\u25BC");
        });
    });

    // --- Body (initial render sorted by year descending) ---
    table.append("tbody");
    _sortAndRender(table, data, columns, currentSortKey, currentSortAsc, unit);
}

// Internal: sort data and re-render tbody
function _sortAndRender(table, data, columns, sortKey, ascending, unit) {
    var sorted = data.slice().sort(function(a, b) {
        var av = a[sortKey];
        var bv = b[sortKey];

        // Handle null/undefined
        if (av == null && bv == null) return 0;
        if (av == null) return 1;
        if (bv == null) return -1;

        // Numeric comparison for numbers
        if (typeof av === "number" && typeof bv === "number") {
            return ascending ? (av - bv) : (bv - av);
        }

        // String comparison
        av = String(av);
        bv = String(bv);
        if (av < bv) return ascending ? -1 : 1;
        if (av > bv) return ascending ? 1 : -1;
        return 0;
    });

    var tbody = table.select("tbody");
    tbody.selectAll("tr").remove();

    var fmt = d3.format(",.0f");
    var parseDateForDisplay = d3.timeParse("%Y-%m-%d");
    var displayDate = d3.timeFormat("%b %d, %Y");

    sorted.forEach(function(row, idx) {
        var bgColor = "transparent";
        if (row.is_wettest) {
            bgColor = _DT_COLOR_WETTEST;
        } else if (row.is_driest) {
            bgColor = _DT_COLOR_DRIEST;
        } else if (_isZeroValueRow(row)) {
            bgColor = _DT_COLOR_ZERO;
        }

        var tr = tbody.append("tr")
            .style("background-color", bgColor)
            .style("border-bottom", "1px solid #e0e0e0");

        // Hover highlight
        tr.on("mouseover", function() {
                if (bgColor === "transparent") {
                    d3.select(this).style("background-color", "#f0f7ff");
                }
            })
            .on("mouseout", function() {
                d3.select(this).style("background-color", bgColor);
            });

        columns.forEach(function(col) {
            var td = tr.append("td")
                .style("padding", "6px 10px")
                .style("text-align", col.type === "number" ? "right" : "left")
                .style("white-space", "nowrap");

            var val = row[col.key];

            if (col.type === "water_year") {
                // Format as "YYYY-YY" (e.g., "2022-23")
                var yr = +val;
                var nextYrShort = String((yr) % 100).padStart(2, "0");
                td.text(yr > 0 ? ((yr - 1) + "-" + nextYrShort) : val);
                td.style("font-weight", (row.is_driest || row.is_wettest) ? "bold" : "normal");
            } else if (col.type === "date") {
                if (val) {
                    var parsed = parseDateForDisplay(val);
                    td.text(parsed ? displayDate(parsed) : val);
                } else {
                    td.text("\u2014"); // em dash
                    td.style("color", "#999");
                }
            } else if (col.type === "number") {
                if (val != null && !isNaN(+val)) {
                    td.text(fmt(+val));
                    // Bold for extreme values in special rows
                    if (row.is_driest || row.is_wettest) {
                        td.style("font-weight", "bold");
                    }
                } else {
                    td.text("\u2014");
                    td.style("color", "#999");
                }
            } else {
                td.text(val != null ? val : "\u2014");
            }
        });
    });

    // Summary footer
    var tfoot = table.select("tfoot");
    if (tfoot.empty()) {
        tfoot = table.append("tfoot");
    }
    tfoot.selectAll("tr").remove();

    // Count and total row
    var footRow = tfoot.append("tr")
        .style("background", "#f9f9f9")
        .style("border-top", "2px solid #999")
        .style("font-size", "12px")
        .style("color", "#666");

    footRow.append("td")
        .attr("colspan", columns.length)
        .style("padding", "6px 10px")
        .style("text-align", "right")
        .text(data.length + " water years");
}

// Check if a row has zero (or near-zero) values, indicating missing data
function _isZeroValueRow(row) {
    var hasZeroHigh = (row.highest_value != null && +row.highest_value === 0);
    var hasZeroLow  = (row.lowest_value  != null && +row.lowest_value  === 0);
    return hasZeroHigh && hasZeroLow;
}

function destroyDataTable(containerId) {
    d3.select("#" + containerId).selectAll("*").remove();
}
