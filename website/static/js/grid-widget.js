/**
 * Grid Widget - Cytoscape.js wrapper for power system topology visualization
 *
 * Usage: <div class="grid-widget" data-network="three-bus"></div>
 */

// Node styling by bus type
const BUS_STYLES = {
  slack: { color: '#22c55e', shape: 'diamond', label: '⟲' },
  pv:    { color: '#ef4444', shape: 'triangle', label: '⚡' },
  pq:    { color: '#3b82f6', shape: 'ellipse', label: '' }
};

// Cytoscape stylesheet
const CYTOSCAPE_STYLE = [
  // Base node style
  {
    selector: 'node',
    style: {
      'label': 'data(label)',
      'text-valign': 'center',
      'text-halign': 'center',
      'font-size': '12px',
      'font-weight': 'bold',
      'color': '#ffffff',
      'text-outline-color': 'data(color)',
      'text-outline-width': 2,
      'width': 40,
      'height': 40,
      'background-color': 'data(color)',
      'shape': 'data(shape)',
      'border-width': 2,
      'border-color': '#1e293b'
    }
  },
  // Highlighted node
  {
    selector: 'node.highlighted',
    style: {
      'border-width': 4,
      'border-color': '#fbbf24',
      'width': 50,
      'height': 50
    }
  },
  // Selected node
  {
    selector: 'node:selected',
    style: {
      'border-width': 4,
      'border-color': '#8b5cf6'
    }
  },
  // Base edge style
  {
    selector: 'edge',
    style: {
      'width': 'data(width)',
      'line-color': '#64748b',
      'curve-style': 'bezier',
      'label': 'data(label)',
      'font-size': '10px',
      'color': '#475569',
      'text-background-color': '#ffffff',
      'text-background-opacity': 0.8,
      'text-background-padding': '2px'
    }
  },
  // Highlighted edge
  {
    selector: 'edge.highlighted',
    style: {
      'line-color': '#fbbf24',
      'width': 4
    }
  },
  // Edge on hover
  {
    selector: 'edge:selected',
    style: {
      'line-color': '#8b5cf6',
      'width': 4
    }
  }
];

/**
 * Helper: create a text element with optional class
 */
function createTextElement(tag, text, className) {
  const el = document.createElement(tag);
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

/**
 * Transform bus data to Cytoscape node
 */
function busToNode(bus, generators, loads) {
  const style = BUS_STYLES[bus.type] || BUS_STYLES.pq;
  const gen = generators.find(g => g.bus === bus.id);
  const load = loads.find(l => l.bus === bus.id);

  let label = bus.name || `Bus ${bus.id}`;
  if (style.label) {
    label = `${style.label} ${label}`;
  }

  // Add generation/load info to label
  const annotations = [];
  if (gen && gen.p > 0) annotations.push(`+${gen.p}MW`);
  if (load && load.p > 0) annotations.push(`-${load.p}MW`);
  if (annotations.length > 0) {
    label += `\n${annotations.join(' ')}`;
  }

  return {
    data: {
      id: `bus-${bus.id}`,
      label: label,
      color: style.color,
      shape: style.shape,
      busType: bus.type,
      busData: bus,
      genData: gen,
      loadData: load
    }
  };
}

/**
 * Transform branch data to Cytoscape edge
 */
function branchToEdge(branch, index) {
  // Line width proportional to rating (normalized)
  const width = Math.max(2, Math.min(6, branch.rating / 30));

  // Format impedance label
  const rStr = branch.r.toFixed(3);
  const xStr = branch.x.toFixed(3);
  const label = `${rStr}+j${xStr}`;

  return {
    data: {
      id: `branch-${index}`,
      source: `bus-${branch.from}`,
      target: `bus-${branch.to}`,
      label: label,
      width: width,
      branchData: branch
    }
  };
}

/**
 * Build bus info panel content using safe DOM methods
 */
function buildBusInfoContent(busData, genData, loadData) {
  const fragment = document.createDocumentFragment();

  // Bus header
  const header = createTextElement('strong', `Bus ${busData.id}: ${busData.name || ''}`);
  fragment.appendChild(header);
  fragment.appendChild(document.createElement('br'));

  // Type
  const typeLabel = document.createElement('span');
  typeLabel.textContent = 'Type: ';
  const typeValue = createTextElement('span', busData.type.toUpperCase(), `bus-type-${busData.type}`);
  fragment.appendChild(typeLabel);
  fragment.appendChild(typeValue);
  fragment.appendChild(document.createElement('br'));

  // Base voltage
  fragment.appendChild(createTextElement('span', `Base Voltage: ${busData.v_base} kV`));
  fragment.appendChild(document.createElement('br'));

  // Voltage magnitude
  if (busData.v_mag !== undefined) {
    fragment.appendChild(createTextElement('span', `|V|: ${busData.v_mag} p.u.`));
    fragment.appendChild(document.createElement('br'));
  }

  // Voltage angle
  if (busData.v_ang !== undefined) {
    fragment.appendChild(createTextElement('span', `δ: ${busData.v_ang}°`));
    fragment.appendChild(document.createElement('br'));
  }

  // Generator info
  if (genData) {
    fragment.appendChild(document.createElement('br'));
    fragment.appendChild(createTextElement('strong', 'Generator:'));
    fragment.appendChild(document.createElement('br'));
    let genText = `P: ${genData.p} MW`;
    if (genData.q !== undefined) genText += `, Q: ${genData.q} MVAr`;
    fragment.appendChild(createTextElement('span', genText));
    fragment.appendChild(document.createElement('br'));
  }

  // Load info
  if (loadData) {
    fragment.appendChild(document.createElement('br'));
    fragment.appendChild(createTextElement('strong', 'Load:'));
    fragment.appendChild(document.createElement('br'));
    fragment.appendChild(createTextElement('span', `P: ${loadData.p} MW, Q: ${loadData.q} MVAr`));
    fragment.appendChild(document.createElement('br'));
  }

  return fragment;
}

/**
 * Build branch info panel content using safe DOM methods
 */
function buildBranchInfoContent(branch) {
  const fragment = document.createDocumentFragment();

  fragment.appendChild(createTextElement('strong', `Branch ${branch.from} → ${branch.to}`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `R: ${branch.r} p.u.`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `X: ${branch.x} p.u.`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `B: ${branch.b} p.u.`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `Rating: ${branch.rating} MVA`));
  fragment.appendChild(document.createElement('br'));

  return fragment;
}

/**
 * Build tooltip content for a bus node using safe DOM methods
 */
function buildBusTooltipContent(busData, busType) {
  const fragment = document.createDocumentFragment();

  const name = createTextElement('strong', busData.name || `Bus ${busData.id}`);
  fragment.appendChild(name);
  fragment.appendChild(document.createElement('br'));

  const typeSpan = createTextElement('span', busType.toUpperCase(), `tooltip-type tooltip-type-${busType}`);
  fragment.appendChild(typeSpan);

  if (busData.v_mag !== undefined) {
    fragment.appendChild(document.createElement('br'));
    fragment.appendChild(createTextElement('span', `|V|: ${busData.v_mag} p.u.`));
  }

  return fragment;
}

/**
 * Build tooltip content for a branch edge using safe DOM methods
 */
function buildBranchTooltipContent(branch) {
  const fragment = document.createDocumentFragment();

  fragment.appendChild(createTextElement('strong', `Branch ${branch.from}→${branch.to}`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `Z: ${branch.r.toFixed(3)}+j${branch.x.toFixed(3)}`));
  fragment.appendChild(document.createElement('br'));
  fragment.appendChild(createTextElement('span', `Rating: ${branch.rating} MVA`));

  return fragment;
}

/**
 * Create and attach hover tooltip
 */
function createTooltip(cy, container) {
  const tooltip = document.createElement('div');
  tooltip.className = 'grid-widget-tooltip';
  tooltip.style.display = 'none';
  container.appendChild(tooltip);

  let tooltipTimeout = null;

  // Node hover
  cy.on('mouseover', 'node', function(evt) {
    const node = evt.target;
    const busData = node.data('busData');
    const busType = node.data('busType');

    clearTimeout(tooltipTimeout);

    tooltip.textContent = '';
    tooltip.appendChild(buildBusTooltipContent(busData, busType));
    tooltip.style.display = 'block';

    const pos = evt.renderedPosition;
    tooltip.style.left = (pos.x + 15) + 'px';
    tooltip.style.top = (pos.y - 10) + 'px';
  });

  // Edge hover
  cy.on('mouseover', 'edge', function(evt) {
    const edge = evt.target;
    const branch = edge.data('branchData');

    clearTimeout(tooltipTimeout);

    tooltip.textContent = '';
    tooltip.appendChild(buildBranchTooltipContent(branch));
    tooltip.style.display = 'block';

    const pos = evt.renderedPosition;
    tooltip.style.left = (pos.x + 15) + 'px';
    tooltip.style.top = (pos.y - 10) + 'px';
  });

  // Hide tooltip on mouseout
  cy.on('mouseout', 'node, edge', function() {
    tooltipTimeout = setTimeout(() => {
      tooltip.style.display = 'none';
    }, 100);
  });

  return tooltip;
}

/**
 * Attach interaction handlers
 */
function attachInteractions(cy, data, container, options = {}) {
  // Create info panel
  const infoPanel = document.createElement('div');
  infoPanel.className = 'grid-widget-info';
  infoPanel.style.display = 'none';
  container.appendChild(infoPanel);

  // Add hover tooltips if enabled
  if (options.tooltips !== false) {
    createTooltip(cy, container);
  }

  // Node click - show info
  cy.on('tap', 'node', function(evt) {
    const node = evt.target;
    const busData = node.data('busData');
    const genData = node.data('genData');
    const loadData = node.data('loadData');

    // Clear and rebuild info panel with safe DOM methods
    infoPanel.textContent = '';
    infoPanel.appendChild(buildBusInfoContent(busData, genData, loadData));
    infoPanel.style.display = 'block';

    // Highlight connected edges
    cy.edges().removeClass('highlighted');
    node.connectedEdges().addClass('highlighted');
  });

  // Edge click - show branch info
  cy.on('tap', 'edge', function(evt) {
    const edge = evt.target;
    const branch = edge.data('branchData');

    // Clear and rebuild info panel with safe DOM methods
    infoPanel.textContent = '';
    infoPanel.appendChild(buildBranchInfoContent(branch));
    infoPanel.style.display = 'block';
  });

  // Click background - hide info panel
  cy.on('tap', function(evt) {
    if (evt.target === cy) {
      infoPanel.style.display = 'none';
      cy.edges().removeClass('highlighted');
    }
  });
}

/**
 * Create zoom control buttons
 */
function createZoomControls(cy, container, data, options = {}) {
  const controls = document.createElement('div');
  controls.className = 'grid-widget-controls';

  // Zoom in button
  const zoomIn = document.createElement('button');
  zoomIn.className = 'grid-widget-btn';
  zoomIn.title = 'Zoom in';
  zoomIn.textContent = '+';
  zoomIn.addEventListener('click', () => {
    cy.zoom({ level: cy.zoom() * 1.3, renderedPosition: { x: cy.width() / 2, y: cy.height() / 2 } });
  });

  // Zoom out button
  const zoomOut = document.createElement('button');
  zoomOut.className = 'grid-widget-btn';
  zoomOut.title = 'Zoom out';
  zoomOut.textContent = '−';
  zoomOut.addEventListener('click', () => {
    cy.zoom({ level: cy.zoom() / 1.3, renderedPosition: { x: cy.width() / 2, y: cy.height() / 2 } });
  });

  // Fit button
  const fitBtn = document.createElement('button');
  fitBtn.className = 'grid-widget-btn';
  fitBtn.title = 'Fit to view';
  fitBtn.textContent = '⊡';
  fitBtn.addEventListener('click', () => {
    cy.fit(undefined, 40);
  });

  controls.appendChild(zoomIn);
  controls.appendChild(zoomOut);
  controls.appendChild(fitBtn);

  // Add flow toggle if enabled
  if (options.showFlowToggle) {
    const flowBtn = createFlowToggle(cy, container, data);
    controls.appendChild(flowBtn);
  }

  // Add voltage toggle if enabled
  if (options.showVoltageToggle) {
    const voltageBtn = createVoltageToggle(cy, container);
    controls.appendChild(voltageBtn);
  }

  // Add contingency toggle if enabled
  if (options.showContingencyToggle) {
    const contingencyBtn = createContingencyToggle(cy, container);
    controls.appendChild(contingencyBtn);
  }

  // Add Y-bus toggle if enabled
  if (options.showYbusToggle) {
    const ybusBtn = createYbusToggle(cy, container, data);
    controls.appendChild(ybusBtn);
  }

  // Add LMP toggle if enabled
  if (options.showLmpToggle) {
    const lmpBtn = createLmpToggle(cy, container, data);
    controls.appendChild(lmpBtn);
  }

  container.appendChild(controls);
}

/**
 * Highlight specific nodes by ID
 */
function highlightNodes(cy, nodeIds) {
  const ids = nodeIds.split(',').map(id => `bus-${id.trim()}`);
  ids.forEach(id => {
    cy.$id(id).addClass('highlighted');
  });
}

/**
 * Create legend panel showing bus types and edge meanings
 */
function createLegend(container, options = {}) {
  const legend = document.createElement('div');
  legend.className = 'grid-widget-legend';

  // Bus type legend items
  const busTypes = [
    { type: 'slack', name: 'Slack (Reference)', desc: 'Sets angle ref, balances power' },
    { type: 'pv', name: 'PV (Generator)', desc: 'Controls P and |V|' },
    { type: 'pq', name: 'PQ (Load)', desc: 'Fixed P and Q demand' }
  ];

  busTypes.forEach(item => {
    const style = BUS_STYLES[item.type];
    const row = document.createElement('div');
    row.className = 'legend-item';

    // Create shape indicator
    const shape = document.createElement('span');
    shape.className = `legend-shape legend-shape-${item.type}`;
    shape.style.backgroundColor = style.color;

    // Create label
    const label = document.createElement('span');
    label.className = 'legend-label';
    label.textContent = item.name;

    row.appendChild(shape);
    row.appendChild(label);

    // Add tooltip with description
    row.title = item.desc;

    legend.appendChild(row);
  });

  // Add edge legend if requested
  if (options.showEdges !== false) {
    const edgeRow = document.createElement('div');
    edgeRow.className = 'legend-item legend-edge-item';

    const edgeLine = document.createElement('span');
    edgeLine.className = 'legend-edge-line';

    const edgeLabel = document.createElement('span');
    edgeLabel.className = 'legend-label';
    edgeLabel.textContent = 'Branch (R+jX)';

    edgeRow.appendChild(edgeLine);
    edgeRow.appendChild(edgeLabel);
    edgeRow.title = 'Transmission line or transformer';

    legend.appendChild(edgeRow);
  }

  container.appendChild(legend);
  return legend;
}

/**
 * Apply voltage profile coloring to nodes
 * Colors nodes based on voltage magnitude (green=1.0, yellow=deviation, red=severe)
 */
function applyVoltageProfile(cy) {
  cy.nodes().forEach(node => {
    const busData = node.data('busData');
    if (busData && busData.v_mag !== undefined) {
      const vMag = busData.v_mag;
      // Calculate color based on voltage deviation from 1.0 p.u.
      const deviation = Math.abs(vMag - 1.0);
      let color;
      if (deviation < 0.02) {
        color = '#22c55e'; // Green - nominal
      } else if (deviation < 0.05) {
        color = '#eab308'; // Yellow - slight deviation
      } else if (deviation < 0.10) {
        color = '#f97316'; // Orange - moderate deviation
      } else {
        color = '#ef4444'; // Red - severe deviation
      }
      node.style('background-color', color);
      node.data('voltageColor', color);
    }
  });
}

/**
 * Setup power flow animation on edges
 * Shows animated particles flowing in the direction of power flow
 */
function setupFlowAnimation(cy, data) {
  // Add flow direction and loading data to edges
  cy.edges().forEach((edge, index) => {
    const branch = edge.data('branchData');
    if (branch) {
      // Calculate loading percentage (simplified - uses rating)
      const loading = branch.flow_mw !== undefined
        ? Math.abs(branch.flow_mw) / branch.rating
        : 0.5; // Default 50% if no flow data

      // Color edge by loading
      let lineColor;
      if (loading < 0.5) {
        lineColor = '#22c55e'; // Green - light loading
      } else if (loading < 0.8) {
        lineColor = '#eab308'; // Yellow - moderate loading
      } else if (loading < 1.0) {
        lineColor = '#f97316'; // Orange - heavy loading
      } else {
        lineColor = '#ef4444'; // Red - overloaded
      }

      edge.style('line-color', lineColor);
      edge.data('loading', loading);
      edge.data('loadingColor', lineColor);
    }
  });

  // Add arrow markers to show flow direction
  cy.style()
    .selector('edge.flow-animated')
    .style({
      'target-arrow-shape': 'triangle',
      'target-arrow-color': 'data(loadingColor)',
      'arrow-scale': 0.8
    })
    .update();

  // Mark edges for animation
  cy.edges().addClass('flow-animated');
}

/**
 * Create flow animation toggle button
 */
function createFlowToggle(cy, container, data) {
  const btn = document.createElement('button');
  btn.className = 'grid-widget-btn grid-widget-flow-btn';
  btn.title = 'Toggle power flow animation';
  btn.textContent = '⚡';

  let flowActive = false;

  btn.addEventListener('click', () => {
    flowActive = !flowActive;
    if (flowActive) {
      setupFlowAnimation(cy, data);
      btn.classList.add('active');
    } else {
      // Reset edge colors
      cy.edges().removeClass('flow-animated');
      cy.edges().style('line-color', '#64748b');
      cy.edges().removeStyle('target-arrow-shape');
      btn.classList.remove('active');
    }
  });

  return btn;
}

/**
 * Create voltage profile toggle button
 */
function createVoltageToggle(cy, container) {
  const btn = document.createElement('button');
  btn.className = 'grid-widget-btn grid-widget-voltage-btn';
  btn.title = 'Toggle voltage profile coloring';
  btn.textContent = 'V';

  let voltageActive = false;
  const originalColors = new Map();

  // Store original colors
  cy.nodes().forEach(node => {
    originalColors.set(node.id(), node.data('color'));
  });

  btn.addEventListener('click', () => {
    voltageActive = !voltageActive;
    if (voltageActive) {
      applyVoltageProfile(cy);
      btn.classList.add('active');
    } else {
      // Restore original colors
      cy.nodes().forEach(node => {
        const originalColor = originalColors.get(node.id());
        if (originalColor) {
          node.style('background-color', originalColor);
        }
      });
      btn.classList.remove('active');
    }
  });

  return btn;
}

/**
 * Create contingency mode toggle - allows clicking edges to "open" them
 */
function createContingencyToggle(cy, container) {
  const btn = document.createElement('button');
  btn.className = 'grid-widget-btn grid-widget-contingency-btn';
  btn.title = 'Toggle contingency mode (click branches to open/close)';
  btn.textContent = '✂';

  let contingencyActive = false;
  const openedEdges = new Set();

  // Create status indicator
  const statusDiv = document.createElement('div');
  statusDiv.className = 'grid-widget-contingency-status';
  statusDiv.style.display = 'none';
  container.appendChild(statusDiv);

  function updateStatus() {
    if (openedEdges.size > 0) {
      statusDiv.textContent = `${openedEdges.size} branch(es) open`;
      statusDiv.style.display = 'block';
    } else {
      statusDiv.style.display = 'none';
    }
  }

  // Edge click handler for contingency mode
  const edgeClickHandler = function(evt) {
    if (!contingencyActive) return;

    const edge = evt.target;
    const edgeId = edge.id();

    if (openedEdges.has(edgeId)) {
      // Close the branch (restore)
      openedEdges.delete(edgeId);
      edge.removeClass('branch-open');
      edge.style({
        'line-style': 'solid',
        'opacity': 1
      });
    } else {
      // Open the branch (simulate outage)
      openedEdges.add(edgeId);
      edge.addClass('branch-open');
      edge.style({
        'line-style': 'dashed',
        'opacity': 0.3
      });
    }
    updateStatus();
  };

  btn.addEventListener('click', () => {
    contingencyActive = !contingencyActive;
    if (contingencyActive) {
      btn.classList.add('active');
      cy.on('tap', 'edge', edgeClickHandler);
      statusDiv.textContent = 'Click branches to simulate outages';
      statusDiv.style.display = 'block';
    } else {
      btn.classList.remove('active');
      cy.off('tap', 'edge', edgeClickHandler);
      // Restore all edges
      openedEdges.forEach(edgeId => {
        const edge = cy.$id(edgeId);
        edge.removeClass('branch-open');
        edge.style({
          'line-style': 'solid',
          'opacity': 1
        });
      });
      openedEdges.clear();
      statusDiv.style.display = 'none';
    }
  });

  return btn;
}

/**
 * Build Y-bus matrix display from network data
 */
function buildYbusMatrix(data) {
  const n = data.buses.length;
  const busIdToIndex = new Map();
  data.buses.forEach((bus, i) => {
    busIdToIndex.set(bus.id, i);
  });

  // Initialize Y-bus as complex matrix (stored as [real, imag] pairs)
  const ybus = Array(n).fill(null).map(() =>
    Array(n).fill(null).map(() => [0, 0])
  );

  // Build Y-bus from branches
  data.branches.forEach(branch => {
    const i = busIdToIndex.get(branch.from);
    const j = busIdToIndex.get(branch.to);

    if (i === undefined || j === undefined) return;

    // Calculate admittance y = 1/(r + jx) = (r - jx)/(r² + x²)
    const r = branch.r;
    const x = branch.x;
    const denom = r * r + x * x;
    const g = r / denom;  // conductance
    const b = -x / denom; // susceptance

    // Off-diagonal: Y_ij = -y_ij
    ybus[i][j][0] -= g;
    ybus[i][j][1] -= b;
    ybus[j][i][0] -= g;
    ybus[j][i][1] -= b;

    // Diagonal: Y_ii += y_ij + shunt/2
    const bShunt = branch.b / 2 || 0;
    ybus[i][i][0] += g;
    ybus[i][i][1] += b + bShunt;
    ybus[j][j][0] += g;
    ybus[j][j][1] += b + bShunt;
  });

  return { ybus, busIdToIndex, buses: data.buses };
}

/**
 * Create Y-bus matrix visualization panel
 */
function createYbusPanel(container, data, cy) {
  const panel = document.createElement('div');
  panel.className = 'grid-widget-ybus-panel';

  const header = createTextElement('div', 'Y-Bus Matrix', 'ybus-header');
  panel.appendChild(header);

  const { ybus, buses } = buildYbusMatrix(data);
  const n = buses.length;

  // Create matrix table
  const table = document.createElement('table');
  table.className = 'ybus-matrix';

  // Header row with bus IDs
  const headerRow = document.createElement('tr');
  headerRow.appendChild(document.createElement('th')); // Empty corner cell
  buses.forEach(bus => {
    const th = createTextElement('th', bus.id.toString());
    headerRow.appendChild(th);
  });
  table.appendChild(headerRow);

  // Data rows
  buses.forEach((rowBus, i) => {
    const row = document.createElement('tr');

    // Row header
    const rowHeader = createTextElement('th', rowBus.id.toString());
    row.appendChild(rowHeader);

    // Matrix cells
    buses.forEach((colBus, j) => {
      const cell = document.createElement('td');
      const [real, imag] = ybus[i][j];

      if (Math.abs(real) < 0.0001 && Math.abs(imag) < 0.0001) {
        cell.textContent = '0';
        cell.className = 'ybus-zero';
      } else {
        const realStr = real.toFixed(2);
        const imagStr = imag >= 0 ? `+j${imag.toFixed(2)}` : `-j${Math.abs(imag).toFixed(2)}`;
        cell.textContent = `${realStr}${imagStr}`;
        cell.className = i === j ? 'ybus-diagonal' : 'ybus-offdiag';
      }

      // Add hover interaction to highlight corresponding edge
      if (i !== j && (Math.abs(real) > 0.0001 || Math.abs(imag) > 0.0001)) {
        cell.classList.add('ybus-clickable');
        cell.addEventListener('mouseenter', () => {
          // Find and highlight the edge between these buses
          cy.edges().forEach(edge => {
            const branch = edge.data('branchData');
            if ((branch.from === rowBus.id && branch.to === colBus.id) ||
                (branch.from === colBus.id && branch.to === rowBus.id)) {
              edge.addClass('highlighted');
            }
          });
        });
        cell.addEventListener('mouseleave', () => {
          cy.edges().removeClass('highlighted');
        });
      }

      row.appendChild(cell);
    });

    table.appendChild(row);
  });

  panel.appendChild(table);
  container.appendChild(panel);

  return panel;
}

/**
 * Apply LMP coloring to nodes - colors by price level
 * Low prices = green, high prices = red
 */
function applyLmpColoring(cy, data) {
  // Find min/max LMP values for scaling
  let minLmp = Infinity, maxLmp = -Infinity;
  data.buses.forEach(bus => {
    if (bus.lmp !== undefined) {
      minLmp = Math.min(minLmp, bus.lmp);
      maxLmp = Math.max(maxLmp, bus.lmp);
    }
  });

  // Default range if no LMP data
  if (minLmp === Infinity) {
    minLmp = 0;
    maxLmp = 100;
  }
  const range = maxLmp - minLmp || 1;

  cy.nodes().forEach(node => {
    const busData = node.data('busData');
    if (busData && busData.lmp !== undefined) {
      // Normalize LMP to 0-1 range
      const normalized = (busData.lmp - minLmp) / range;

      // Color gradient: green (low) -> yellow (mid) -> red (high)
      let color;
      if (normalized < 0.33) {
        color = '#22c55e'; // Green - low price
      } else if (normalized < 0.66) {
        color = '#eab308'; // Yellow - medium price
      } else {
        color = '#ef4444'; // Red - high price
      }

      node.style('background-color', color);
      node.data('lmpColor', color);

      // Update label to show price
      const currentLabel = node.data('label');
      const priceLabel = `$${busData.lmp}/MWh`;
      if (!currentLabel.includes('$')) {
        node.data('label', currentLabel + '\n' + priceLabel);
      }
    }
  });

  // Highlight congested branches
  cy.edges().forEach(edge => {
    const branch = edge.data('branchData');
    if (branch && branch.congested) {
      edge.style({
        'line-color': '#ef4444',
        'width': 5,
        'line-style': 'solid'
      });
    }
  });
}

/**
 * Create LMP visualization toggle button
 */
function createLmpToggle(cy, container, data) {
  const btn = document.createElement('button');
  btn.className = 'grid-widget-btn grid-widget-lmp-btn';
  btn.title = 'Toggle LMP price coloring';
  btn.textContent = '$';

  let lmpActive = false;
  const originalColors = new Map();
  const originalLabels = new Map();

  // Store original colors and labels
  cy.nodes().forEach(node => {
    originalColors.set(node.id(), node.data('color'));
    originalLabels.set(node.id(), node.data('label'));
  });

  btn.addEventListener('click', () => {
    lmpActive = !lmpActive;
    if (lmpActive) {
      applyLmpColoring(cy, data);
      btn.classList.add('active');
    } else {
      // Restore original colors and labels
      cy.nodes().forEach(node => {
        const originalColor = originalColors.get(node.id());
        const originalLabel = originalLabels.get(node.id());
        if (originalColor) {
          node.style('background-color', originalColor);
        }
        if (originalLabel) {
          node.data('label', originalLabel);
        }
      });
      // Reset edge styles
      cy.edges().forEach(edge => {
        const branch = edge.data('branchData');
        if (branch && branch.congested) {
          edge.style({
            'line-color': '#64748b',
            'width': edge.data('width'),
            'line-style': 'solid'
          });
        }
      });
      btn.classList.remove('active');
    }
  });

  return btn;
}

/**
 * Create Y-bus toggle button
 */
function createYbusToggle(cy, container, data) {
  const btn = document.createElement('button');
  btn.className = 'grid-widget-btn grid-widget-ybus-btn';
  btn.title = 'Toggle Y-bus matrix display';
  btn.textContent = 'Y';

  let ybusPanel = null;
  let ybusActive = false;

  btn.addEventListener('click', () => {
    ybusActive = !ybusActive;
    if (ybusActive) {
      btn.classList.add('active');
      ybusPanel = createYbusPanel(container, data, cy);
    } else {
      btn.classList.remove('active');
      if (ybusPanel) {
        ybusPanel.remove();
        ybusPanel = null;
      }
    }
  });

  return btn;
}

/**
 * Show error message in widget container using safe DOM methods
 */
function showError(container, message) {
  container.textContent = '';

  const errorDiv = document.createElement('div');
  errorDiv.className = 'grid-widget-error';

  const iconSpan = createTextElement('span', '⚠️', 'error-icon');
  const messageSpan = createTextElement('span', message, 'error-message');

  errorDiv.appendChild(iconSpan);
  errorDiv.appendChild(messageSpan);
  container.appendChild(errorDiv);
}

/**
 * Initialize a single grid widget
 */
async function initWidget(container) {
  const networkName = container.dataset.network;

  if (!networkName) {
    showError(container, 'Missing data-network attribute');
    return;
  }

  // Set container height
  const height = container.dataset.height || '400';
  container.style.height = `${height}px`;

  // Create Cytoscape container
  const cyContainer = document.createElement('div');
  cyContainer.className = 'grid-widget-canvas';
  container.appendChild(cyContainer);

  try {
    // Fetch network data (use base URL for subdirectory deployments)
    const baseUrl = (window.GAT_BASE_URL || '').replace(/\/$/, '');
    const response = await fetch(`${baseUrl}/examples/${networkName}.json`);
    if (!response.ok) {
      throw new Error(`Network "${networkName}" not found`);
    }
    const data = await response.json();

    // Transform to Cytoscape elements
    const elements = [
      ...data.buses.map(bus => busToNode(bus, data.generators || [], data.loads || [])),
      ...data.branches.map((branch, i) => branchToEdge(branch, i))
    ];

    // Initialize Cytoscape (without layout - we'll run it separately)
    const cy = cytoscape({
      container: cyContainer,
      elements: elements,
      style: CYTOSCAPE_STYLE,
      userZoomingEnabled: container.dataset.zoom !== 'false',
      userPanningEnabled: container.dataset.zoom !== 'false',
      boxSelectionEnabled: false,
      minZoom: 0.3,
      maxZoom: 3
    });

    // Calculate padding (extra at bottom for caption)
    const hasCaption = !!container.dataset.caption;
    const basePadding = 40;

    // Run layout and fit to container when done
    const layout = cy.layout({
      name: container.dataset.layout || 'cose',
      animate: false,
      padding: basePadding,
      nodeRepulsion: 6000,
      idealEdgeLength: 80,
      stop: function() {
        // Fit with asymmetric padding if caption present
        if (hasCaption) {
          cy.fit(undefined, basePadding);
          // Pan up slightly to make room for caption bar
          cy.panBy({ x: 0, y: -15 });
        } else {
          cy.fit(undefined, basePadding);
        }
      }
    });
    layout.run();

    // Attach interactions (with tooltips enabled by default)
    attachInteractions(cy, data, container, {
      tooltips: container.dataset.tooltips !== 'false'
    });

    // Add zoom controls (unless disabled)
    if (container.dataset.controls !== 'false') {
      createZoomControls(cy, container, data, {
        showFlowToggle: container.dataset.flow === 'true',
        showVoltageToggle: container.dataset.voltage === 'true',
        showContingencyToggle: container.dataset.contingency === 'true',
        showYbusToggle: container.dataset.ybus === 'true',
        showLmpToggle: container.dataset.lmp === 'true'
      });
    }

    // Apply highlights if specified
    if (container.dataset.highlight) {
      highlightNodes(cy, container.dataset.highlight);
    }

    // Add legend if requested (data-legend="true")
    if (container.dataset.legend === 'true') {
      createLegend(container, { showEdges: true });
    }

    // Add caption if specified
    if (container.dataset.caption) {
      const caption = createTextElement('div', container.dataset.caption, 'grid-widget-caption');
      container.appendChild(caption);
    }

    // Add title if network has name
    if (data.name) {
      const title = createTextElement('div', data.name, 'grid-widget-title');
      container.insertBefore(title, cyContainer);
    }

  } catch (error) {
    console.error('Grid widget error:', error);
    showError(container, error.message);
  }
}

/**
 * Initialize all grid widgets on page load
 */
function initAllWidgets() {
  const widgets = document.querySelectorAll('.grid-widget');
  widgets.forEach(initWidget);
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initAllWidgets);
} else {
  initAllWidgets();
}
