// California Water Reservoir — Sortable Data Table
// Used by: table-water-year-stats, table-snow-stats
//
// Data format (JSON array):
// [{ year: 2023, date_lowest: "YYYY-MM-DD", lowest_value: <number>,
//    date_highest: "YYYY-MM-DD", highest_value: <number>,
//    is_driest: bool, is_wettest: bool }, ...]
//
// Config format (JSON object): {
//   title: string,
//   yUnit: string,          // e.g. "AF" or "in" — shown beside numeric headers
//   columns: [              // type is one of "water_year" | "date" | "number" | "text"
//     { key: "year", label: "Water Year", type: "water_year" }, ...
//   ]
// }
//
// The wettest and driest rows are tinted AND tagged with a text label, so the
// highlight never depends on color alone.

(function () {
    "use strict";
    var CWR = window.CWR;

    var DEFAULT_COLUMNS = [
        { key: "year", label: "Water Year", type: "water_year" },
        { key: "date_highest", label: "Date of Peak", type: "date" },
        { key: "highest_value", label: "Peak", type: "number" },
        { key: "date_lowest", label: "Date of Low", type: "date" },
        { key: "lowest_value", label: "Low", type: "number" }
    ];

    var LEGEND = [
        { tint: "var(--cwr-tint-wettest)", label: "Wettest year" },
        { tint: "var(--cwr-tint-driest)", label: "Driest year" },
        { tint: "var(--cwr-tint-empty)", label: "No measurable value" }
    ];

    function isEmptyRow(row) {
        return row.highest_value != null && +row.highest_value === 0 &&
            row.lowest_value != null && +row.lowest_value === 0;
    }

    function rowClass(row) {
        if (row.is_wettest) return "cwr-row-wettest";
        if (row.is_driest) return "cwr-row-driest";
        return isEmptyRow(row) ? "cwr-row-empty" : null;
    }

    /** Water year 2023 runs Oct 2022 – Sep 2023 and is written "2022-23". */
    function waterYearLabel(year) {
        var yr = +year;
        return yr > 0 ? (yr - 1) + "-" + String(yr % 100).padStart(2, "0") : String(year);
    }

    function compare(a, b) {
        if (a == null && b == null) return 0;
        if (a == null) return 1;
        if (b == null) return -1;
        if (typeof a === "number" && typeof b === "number") return a - b;
        return String(a) < String(b) ? -1 : String(a) > String(b) ? 1 : 0;
    }

    CWR.renderDataTable = function (containerId, data, config) {
        data = data || [];
        var container = CWR.prepare(containerId, data.length > 0);
        if (!container) return;

        var columns = config.columns || DEFAULT_COLUMNS;
        var unit = config.yUnit || "";
        var sortKey = "year";
        var ascending = false; // most recent first

        if (config.title) {
            container.append("h4").attr("class", "cwr-subtitle").text(config.title);
        }

        var legend = container.append("div").attr("class", "cwr-legend");
        LEGEND.forEach(function (item) {
            var el = legend.append("span").attr("class", "cwr-legend-item");
            el.append("span").attr("class", "cwr-swatch").style("background", item.tint);
            el.append("span").text(item.label);
        });

        var table = container.append("div").attr("class", "cwr-table-wrap")
            .append("table").attr("class", "cwr-table");
        var headerRow = table.append("thead").append("tr");
        var tbody = table.append("tbody");
        table.append("tfoot").append("tr").append("td")
            .attr("colspan", columns.length)
            .text(data.length + " water years");

        columns.forEach(function (col) {
            var th = headerRow.append("th")
                .classed("cwr-num", col.type === "number")
                .attr("scope", "col");
            th.append("span").text(col.label || col.header || col.key);
            if (col.type === "number" && unit) {
                th.append("span").attr("class", "cwr-th-unit").text("(" + unit + ")");
            }
            th.append("span").attr("class", "cwr-sort-arrow");
            th.on("click", function () {
                ascending = sortKey === col.key ? !ascending : true;
                sortKey = col.key;
                renderBody();
            });
        });

        function renderBody() {
            headerRow.selectAll(".cwr-sort-arrow").text(function (_, i) {
                return columns[i].key === sortKey ? (ascending ? "▲" : "▼") : "";
            });
            headerRow.selectAll("th").attr("aria-sort", function (_, i) {
                return columns[i].key !== sortKey ? null : ascending ? "ascending" : "descending";
            });

            var sorted = data.slice().sort(function (a, b) {
                var c = compare(a[sortKey], b[sortKey]);
                // Missing values stay at the bottom in either direction.
                if (a[sortKey] == null || b[sortKey] == null) return c;
                return ascending ? c : -c;
            });

            tbody.selectAll("tr").remove();
            sorted.forEach(function (row) {
                var tr = tbody.append("tr").attr("class", rowClass(row));
                columns.forEach(function (col) {
                    var td = tr.append("td").classed("cwr-num", col.type === "number");
                    var val = row[col.key];

                    if (col.type === "water_year") {
                        td.append("span").text(waterYearLabel(val));
                        if (row.is_wettest) td.append("span").attr("class", "cwr-tag").text("Wettest");
                        if (row.is_driest) td.append("span").attr("class", "cwr-tag").text("Driest");
                    } else if (val == null || val === "" || (col.type === "number" && isNaN(+val))) {
                        td.classed("cwr-missing", true).text("—");
                    } else if (col.type === "date") {
                        var parsed = CWR.parseDate(val);
                        td.text(parsed ? CWR.formatDate(parsed) : val);
                    } else if (col.type === "number") {
                        td.text(CWR.formatValue(+val));
                    } else {
                        td.text(val);
                    }
                });
            });
        }

        renderBody();
    };
})();
