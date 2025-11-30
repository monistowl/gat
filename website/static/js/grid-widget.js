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
 * Attach interaction handlers
 */
function attachInteractions(cy, data, container) {
  // Create info panel
  const infoPanel = document.createElement('div');
  infoPanel.className = 'grid-widget-info';
  infoPanel.style.display = 'none';
  container.appendChild(infoPanel);

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
function createZoomControls(cy, container) {
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
    // Fetch network data
    const response = await fetch(`/examples/${networkName}.json`);
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

    // Attach interactions
    attachInteractions(cy, data, container);

    // Add zoom controls (unless disabled)
    if (container.dataset.controls !== 'false') {
      createZoomControls(cy, container);
    }

    // Apply highlights if specified
    if (container.dataset.highlight) {
      highlightNodes(cy, container.dataset.highlight);
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
