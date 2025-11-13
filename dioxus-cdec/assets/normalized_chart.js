import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";

export function createNormalizedChart(containerId, dataJson) {
    const data = JSON.parse(dataJson);

    if (!data || data.length === 0) {
        return;
    }

    // Group data by year
    const yearGroups = d3.group(data, d => d.year);
    const years = Array.from(yearGroups.keys()).sort((a, b) => b - a);

    // Clear previous chart
    const container = d3.select(`#${containerId}`);
    container.selectAll("*").remove();

    // Set up dimensions
    const margin = { top: 40, right: 120, bottom: 50, left: 60 };
    const width = container.node().offsetWidth - margin.left - margin.right;
    const height = 500 - margin.top - margin.bottom;

    // Create SVG
    const svg = container
        .append("svg")
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom)
        .append("g")
        .attr("transform", `translate(${margin.left},${margin.top})`);

    // Set up scales
    const xScale = d3.scaleLinear()
        .domain([1, 366])
        .range([0, width]);

    const yScale = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.value) * 1.1])
        .range([height, 0]);

    // Color scale for years
    const colorScale = d3.scaleOrdinal()
        .domain(years)
        .range(d3.schemeCategory10);

    // Add X axis
    const xAxis = svg.append("g")
        .attr("transform", `translate(0,${height})`)
        .call(d3.axisBottom(xScale).ticks(12).tickFormat(d => {
            // Convert water year day to month name
            const months = ["Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep"];
            const monthIndex = Math.floor((d - 1) / 30.5);
            return months[Math.min(monthIndex, 11)];
        }));

    xAxis.selectAll("text")
        .style("font-size", "12px");

    // Add Y axis
    const yAxis = svg.append("g")
        .call(d3.axisLeft(yScale).tickFormat(d => {
            if (d >= 1e6) return `${(d / 1e6).toFixed(1)}M`;
            if (d >= 1e3) return `${(d / 1e3).toFixed(0)}K`;
            return d;
        }));

    yAxis.selectAll("text")
        .style("font-size", "12px");

    // Add axis labels
    svg.append("text")
        .attr("x", width / 2)
        .attr("y", height + 40)
        .attr("text-anchor", "middle")
        .style("font-size", "14px")
        .style("fill", "#666")
        .text("Water Year Day");

    svg.append("text")
        .attr("transform", "rotate(-90)")
        .attr("x", -height / 2)
        .attr("y", -45)
        .attr("text-anchor", "middle")
        .style("font-size", "14px")
        .style("fill", "#666")
        .text("Water Level (acre-feet)");

    // Add title
    svg.append("text")
        .attr("x", width / 2)
        .attr("y", -15)
        .attr("text-anchor", "middle")
        .style("font-size", "16px")
        .style("font-weight", "600")
        .style("fill", "#2c3e50")
        .text("Water Year Comparison");

    // Line generator
    const line = d3.line()
        .x(d => xScale(d.day))
        .y(d => yScale(d.value))
        .curve(d3.curveMonotoneX);

    // Draw lines for each year
    years.forEach(year => {
        const yearData = yearGroups.get(year).sort((a, b) => a.day - b.day);

        svg.append("path")
            .datum(yearData)
            .attr("fill", "none")
            .attr("stroke", colorScale(year))
            .attr("stroke-width", 2)
            .attr("opacity", 0.8)
            .attr("d", line);
    });

    // Add legend
    const legend = svg.append("g")
        .attr("transform", `translate(${width + 10}, 0)`);

    years.slice(0, 10).forEach((year, i) => {
        const legendRow = legend.append("g")
            .attr("transform", `translate(0, ${i * 20})`);

        legendRow.append("line")
            .attr("x1", 0)
            .attr("x2", 20)
            .attr("y1", 10)
            .attr("y2", 10)
            .attr("stroke", colorScale(year))
            .attr("stroke-width", 2);

        legendRow.append("text")
            .attr("x", 25)
            .attr("y", 14)
            .style("font-size", "12px")
            .style("fill", "#333")
            .text(year);
    });

    // Add tooltip
    const tooltip = container.append("div")
        .style("position", "absolute")
        .style("background", "rgba(0, 0, 0, 0.8)")
        .style("color", "white")
        .style("padding", "8px")
        .style("border-radius", "4px")
        .style("font-size", "12px")
        .style("pointer-events", "none")
        .style("opacity", 0);

    // Add invisible overlay for hover interactions
    const overlay = svg.append("rect")
        .attr("width", width)
        .attr("height", height)
        .attr("fill", "none")
        .attr("pointer-events", "all")
        .on("mousemove", function(event) {
            const [mouseX] = d3.pointer(event);
            const day = Math.round(xScale.invert(mouseX));

            // Find closest points for each year at this day
            const points = years.map(year => {
                const yearData = yearGroups.get(year);
                const closest = yearData.reduce((prev, curr) =>
                    Math.abs(curr.day - day) < Math.abs(prev.day - day) ? curr : prev
                );
                return { year, ...closest };
            }).sort((a, b) => b.value - a.value);

            const tooltipHtml = points.slice(0, 5).map(p =>
                `<div><span style="color:${colorScale(p.year)}">●</span> ${p.year}: ${p.value.toLocaleString()} AF</div>`
            ).join('');

            tooltip
                .html(tooltipHtml)
                .style("opacity", 1)
                .style("left", `${event.pageX - container.node().offsetLeft + 10}px`)
                .style("top", `${event.pageY - container.node().offsetTop - 10}px`);
        })
        .on("mouseleave", () => {
            tooltip.style("opacity", 0);
        });
}
