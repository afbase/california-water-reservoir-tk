// D3.js chart implementation
let chart = null;

export function createChart(containerId, data, config) {
    console.log('Creating chart with', data.length, 'data points');

    // Clear existing chart
    const container = document.getElementById(containerId);
    if (!container) {
        console.error('Chart container not found:', containerId);
        return;
    }
    container.innerHTML = '';

    // Setup dimensions
    const margin = config.margin;
    const width = config.width - margin.left - margin.right;
    const height = config.height - margin.top - margin.bottom;

    // Create SVG
    const svg = d3.select(`#${containerId}`)
        .append('svg')
        .attr('width', config.width)
        .attr('height', config.height)
        .append('g')
        .attr('transform', `translate(${margin.left},${margin.top})`);

    // Parse dates
    const parseDate = d3.timeParse('%Y-%m-%d');
    data.forEach(d => {
        d.date = parseDate(d.date);
        d.value = +d.value;
    });

    // Create scales
    const x = d3.scaleTime()
        .domain(d3.extent(data, d => d.date))
        .range([0, width]);

    const y = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.value) * 1.1])
        .range([height, 0]);

    // Create line generator
    const line = d3.line()
        .x(d => x(d.date))
        .y(d => y(d.value))
        .curve(d3.curveMonotoneX);

    // Add X axis
    svg.append('g')
        .attr('transform', `translate(0,${height})`)
        .call(d3.axisBottom(x))
        .selectAll('text')
        .style('text-anchor', 'end')
        .attr('dx', '-.8em')
        .attr('dy', '.15em')
        .attr('transform', 'rotate(-45)');

    // Add Y axis
    svg.append('g')
        .call(d3.axisLeft(y)
            .tickFormat(d => {
                if (d >= 1000000) return (d / 1000000).toFixed(1) + 'M';
                if (d >= 1000) return (d / 1000).toFixed(0) + 'K';
                return d;
            }));

    // Add Y axis label
    svg.append('text')
        .attr('transform', 'rotate(-90)')
        .attr('y', 0 - margin.left)
        .attr('x', 0 - (height / 2))
        .attr('dy', '1em')
        .style('text-anchor', 'middle')
        .style('font-size', '12px')
        .text('Water Level (acre-feet)');

    // Add grid lines
    svg.append('g')
        .attr('class', 'grid')
        .attr('opacity', 0.1)
        .call(d3.axisLeft(y)
            .tickSize(-width)
            .tickFormat(''));

    // Add the line path
    svg.append('path')
        .datum(data)
        .attr('fill', 'none')
        .attr('stroke', '#2196F3')
        .attr('stroke-width', 2)
        .attr('d', line);

    // Add area under the line
    const area = d3.area()
        .x(d => x(d.date))
        .y0(height)
        .y1(d => y(d.value))
        .curve(d3.curveMonotoneX);

    svg.append('path')
        .datum(data)
        .attr('fill', '#2196F3')
        .attr('opacity', 0.2)
        .attr('d', area);

    // Add tooltip
    const focus = svg.append('g')
        .attr('class', 'focus')
        .style('display', 'none');

    focus.append('circle')
        .attr('r', 5)
        .attr('fill', '#2196F3');

    focus.append('rect')
        .attr('class', 'tooltip')
        .attr('width', 120)
        .attr('height', 50)
        .attr('x', 10)
        .attr('y', -22)
        .attr('rx', 4)
        .attr('ry', 4)
        .attr('fill', 'white')
        .attr('stroke', '#999')
        .attr('opacity', 0.9);

    focus.append('text')
        .attr('class', 'tooltip-date')
        .attr('x', 18)
        .attr('y', -2);

    focus.append('text')
        .attr('class', 'tooltip-value')
        .attr('x', 18)
        .attr('y', 18);

    svg.append('rect')
        .attr('class', 'overlay')
        .attr('width', width)
        .attr('height', height)
        .attr('opacity', 0)
        .on('mouseover', () => focus.style('display', null))
        .on('mouseout', () => focus.style('display', 'none'))
        .on('mousemove', function(event) {
            const bisect = d3.bisector(d => d.date).left;
            const x0 = x.invert(d3.pointer(event)[0]);
            const i = bisect(data, x0, 1);
            const d0 = data[i - 1];
            const d1 = data[i];
            const d = x0 - d0.date > d1.date - x0 ? d1 : d0;

            focus.attr('transform', `translate(${x(d.date)},${y(d.value)})`);
            focus.select('.tooltip-date').text(d3.timeFormat('%Y-%m-%d')(d.date));
            focus.select('.tooltip-value').text(d.value.toLocaleString() + ' AF');
        });

    chart = { svg, data, x, y, line };
    console.log('Chart created successfully');
}

export function updateChart(data) {
    if (!chart) {
        console.warn('Chart not initialized');
        return;
    }
    console.log('Updating chart with', data.length, 'data points');
    // For simplicity, recreate the chart
    // In production, you'd update the existing chart for better performance
}
