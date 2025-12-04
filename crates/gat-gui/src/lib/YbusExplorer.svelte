<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as d3 from 'd3';
  import { invoke } from "@tauri-apps/api/core";

  // Types
  interface YbusEntry {
    row: number;
    col: number;
    g: number;
    b: number;
    magnitude: number;
    from_bus_id: number;
    to_bus_id: number;
  }

  interface YbusJson {
    n_bus: number;
    entries: YbusEntry[];
    bus_ids: number[];
  }

  // Types for network info
  interface NetworkJson {
    name: string;
    buses: Array<{ id: number }>;
    branches: Array<{ from: number; to: number }>;
    generators: Array<{ bus: number }>;
    base_mva: number;
  }

  // Props
  let {
    casePath,
    onSelectBus,
    network = null,
    onSolveAc,
    onSolveDc,
    solvingAc = false,
    solvingDc = false
  }: {
    casePath: string | null;
    onSelectBus?: (busId: number) => void;
    network?: NetworkJson | null;
    onSolveAc?: () => void;
    onSolveDc?: () => void;
    solvingAc?: boolean;
    solvingDc?: boolean;
  } = $props();

  // State
  let container: HTMLDivElement;
  let ybus = $state<YbusJson | null>(null);
  let loading = $state(false);
  let selectedEntry = $state<YbusEntry | null>(null);
  let hoveredEntry = $state<YbusEntry | null>(null);

  // Magnitude scale for colors
  const magnitudeColor = d3.scaleSequentialLog(d3.interpolateViridis)
    .domain([0.001, 100]);

  // Load Y-bus when path changes
  $effect(() => {
    if (casePath) {
      loadYbus(casePath);
    } else {
      ybus = null;
    }
  });

  async function loadYbus(path: string) {
    loading = true;
    try {
      ybus = await invoke<YbusJson>("get_ybus", { path });
    } catch (e) {
      console.error("Failed to load Y-bus:", e);
      ybus = null;
    } finally {
      loading = false;
    }
  }

  function initVisualization() {
    if (!container || !ybus) return;

    // Clear previous
    d3.select(container).selectAll('*').remove();

    const width = container.clientWidth;
    const height = container.clientHeight;
    const margin = { top: 40, right: 20, bottom: 20, left: 40 };
    const innerWidth = width - margin.left - margin.right;
    const innerHeight = height - margin.top - margin.bottom;

    // Cell size based on matrix dimension
    const n = ybus.n_bus;
    const cellSize = Math.min(
      innerWidth / n,
      innerHeight / n,
      20 // Max cell size
    );

    const matrixSize = cellSize * n;

    // Create SVG
    const svg = d3.select(container)
      .append('svg')
      .attr('width', width)
      .attr('height', height);

    // Group for matrix
    const g = svg.append('g')
      .attr('transform', `translate(${margin.left},${margin.top})`);

    // Add zoom for large matrices
    if (n > 30) {
      svg.call(d3.zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.5, 10])
        .on('zoom', (event) => g.attr('transform',
          `translate(${margin.left + event.transform.x},${margin.top + event.transform.y}) scale(${event.transform.k})`
        )));
    }

    // Background grid
    g.append('rect')
      .attr('width', matrixSize)
      .attr('height', matrixSize)
      .attr('fill', 'var(--bg-tertiary)')
      .attr('stroke', 'var(--border)')
      .attr('stroke-width', 1);

    // Draw matrix entries
    const cells = g.selectAll<SVGRectElement, YbusEntry>('rect.cell')
      .data(ybus.entries)
      .join('rect')
      .attr('class', 'cell')
      .attr('x', d => d.col * cellSize)
      .attr('y', d => d.row * cellSize)
      .attr('width', cellSize - 0.5)
      .attr('height', cellSize - 0.5)
      .attr('fill', d => {
        // Diagonal entries are darker
        if (d.row === d.col) {
          return d3.color(magnitudeColor(Math.max(d.magnitude, 0.001)))?.darker(0.5)?.toString() || '#666';
        }
        return magnitudeColor(Math.max(d.magnitude, 0.001));
      })
      .attr('stroke', 'none')
      .attr('rx', 1)
      .style('cursor', 'pointer')
      .on('mouseenter', (event, d) => {
        hoveredEntry = d;
        d3.select(event.currentTarget)
          .attr('stroke', 'var(--accent)')
          .attr('stroke-width', 2);
      })
      .on('mouseleave', (event, d) => {
        hoveredEntry = null;
        d3.select(event.currentTarget)
          .attr('stroke', d.row === d.col ? 'var(--accent)' : 'none')
          .attr('stroke-width', d.row === d.col ? 0.5 : 0);
      })
      .on('click', (_, d) => {
        // Toggle selection - clicking same entry deselects
        selectedEntry = selectedEntry?.row === d.row && selectedEntry?.col === d.col ? null : d;
      });

    // Highlight diagonal
    cells.filter(d => d.row === d.col)
      .attr('stroke', 'var(--accent)')
      .attr('stroke-width', 0.5);

    // Row labels (bus IDs) - only for smaller matrices
    if (n <= 50 && cellSize >= 10) {
      g.selectAll('text.row-label')
        .data(ybus.bus_ids)
        .join('text')
        .attr('class', 'row-label')
        .attr('x', -4)
        .attr('y', (_, i) => i * cellSize + cellSize / 2)
        .attr('text-anchor', 'end')
        .attr('dominant-baseline', 'middle')
        .attr('fill', 'var(--text-muted)')
        .attr('font-size', Math.min(cellSize - 2, 10))
        .text(d => d);

      // Column labels
      g.selectAll('text.col-label')
        .data(ybus.bus_ids)
        .join('text')
        .attr('class', 'col-label')
        .attr('x', (_, i) => i * cellSize + cellSize / 2)
        .attr('y', -4)
        .attr('text-anchor', 'middle')
        .attr('fill', 'var(--text-muted)')
        .attr('font-size', Math.min(cellSize - 2, 10))
        .text(d => d);
    }

    // Title is now rendered as HUD overlay, not in SVG
  }

  // Re-render on ybus change
  $effect(() => {
    if (ybus && container) {
      initVisualization();
    }
  });

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      if (ybus) initVisualization();
    });
    resizeObserver.observe(container);

    return () => resizeObserver.disconnect();
  });

  function formatComplex(g: number, b: number): string {
    const sign = b >= 0 ? '+' : '-';
    return `${g.toFixed(4)} ${sign} j${Math.abs(b).toFixed(4)}`;
  }
</script>

<div class="ybus-explorer">
  <div class="matrix-container" bind:this={container}>
    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <span>Building Y-bus matrix...</span>
      </div>
    {:else if !ybus}
      <div class="empty">
        <p>Load a case to view Y-bus matrix</p>
      </div>
    {/if}
  </div>

  {#if ybus}
    <!-- Matrix title HUD -->
    <div class="matrix-title-hud">
      Y-bus Matrix ({ybus.n_bus}×{ybus.n_bus}, {ybus.entries.length} non-zeros)
    </div>

    <!-- Hover tooltip HUD (small, follows mouse) -->
    {#if hoveredEntry && !selectedEntry}
      <div class="hover-hud">
        <span class="hover-label">Y<sub>{hoveredEntry.row},{hoveredEntry.col}</sub></span>
        <span class="hover-value">{formatComplex(hoveredEntry.g, hoveredEntry.b)}</span>
      </div>
    {/if}

    <!-- Stats HUD -->
    <div class="stats-hud">
      <div class="stats-row">
        <span class="label">Size</span>
        <span class="value">{ybus.n_bus}×{ybus.n_bus}</span>
      </div>
      <div class="stats-row">
        <span class="label">NNZ</span>
        <span class="value">{ybus.entries.length}</span>
      </div>
      <div class="stats-row">
        <span class="label">Fill</span>
        <span class="value">{((ybus.entries.length / (ybus.n_bus * ybus.n_bus)) * 100).toFixed(1)}%</span>
      </div>
    </div>

    <!-- Color scale legend HUD -->
    <div class="legend-hud">
      <span class="legend-label">|Y| (pu)</span>
      <div class="legend-bar"></div>
      <div class="legend-ticks">
        <span>0.001</span>
        <span>100</span>
      </div>
    </div>

    <!-- Detail Sidebar (slides in when entry selected) -->
    <aside class="detail-sidebar" class:open={selectedEntry !== null}>
      {#if selectedEntry}
        <div class="sidebar-header">
          <h3>Matrix Entry</h3>
          <button class="close-btn" onclick={() => selectedEntry = null} aria-label="Close">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>

        <div class="sidebar-content">
          <!-- Position & Type -->
          <div class="info-card">
            <div class="card-header">
              <span class="entry-position">Y<sub>{selectedEntry.row},{selectedEntry.col}</sub></span>
              <span class="entry-type" class:diagonal={selectedEntry.row === selectedEntry.col}>
                {selectedEntry.row === selectedEntry.col ? 'Diagonal' : 'Off-diagonal'}
              </span>
            </div>
            <p class="entry-description">
              {#if selectedEntry.row === selectedEntry.col}
                Self-admittance at bus {selectedEntry.from_bus_id}. Sum of all admittances connected to this bus.
              {:else}
                Mutual admittance between buses {selectedEntry.from_bus_id} and {selectedEntry.to_bus_id}. Negative of branch admittance.
              {/if}
            </p>
          </div>

          <!-- Bus Connection -->
          <div class="info-section">
            <h4>Connection</h4>
            <div class="connection-display">
              <div class="bus-badge">Bus {selectedEntry.from_bus_id}</div>
              {#if selectedEntry.row !== selectedEntry.col}
                <span class="connection-arrow">↔</span>
                <div class="bus-badge">Bus {selectedEntry.to_bus_id}</div>
              {/if}
            </div>
          </div>

          <!-- Complex Value -->
          <div class="info-section">
            <h4>Admittance Value</h4>
            <div class="complex-display">
              <div class="complex-full">
                <span class="complex-label">Y<sub>ij</sub> =</span>
                <span class="complex-value">{formatComplex(selectedEntry.g, selectedEntry.b)}</span>
              </div>
            </div>
          </div>

          <!-- Components -->
          <div class="info-section">
            <h4>Components</h4>
            <div class="components-grid">
              <div class="component">
                <span class="component-label">G (conductance)</span>
                <span class="component-value">{selectedEntry.g.toFixed(6)} pu</span>
              </div>
              <div class="component">
                <span class="component-label">B (susceptance)</span>
                <span class="component-value">{selectedEntry.b.toFixed(6)} pu</span>
              </div>
              <div class="component">
                <span class="component-label">|Y| (magnitude)</span>
                <span class="component-value">{selectedEntry.magnitude.toFixed(6)} pu</span>
              </div>
              <div class="component">
                <span class="component-label">∠Y (angle)</span>
                <span class="component-value">{(Math.atan2(selectedEntry.b, selectedEntry.g) * 180 / Math.PI).toFixed(2)}°</span>
              </div>
            </div>
          </div>

          <!-- Physical Interpretation -->
          <div class="info-section">
            <h4>Physical Meaning</h4>
            <div class="interpretation">
              {#if selectedEntry.row === selectedEntry.col}
                <p>This diagonal entry represents the <strong>self-admittance</strong> of bus {selectedEntry.from_bus_id}.</p>
                <p>It equals the sum of admittances of all branches connected to this bus, plus any shunt elements.</p>
              {:else}
                <p>This off-diagonal entry represents the <strong>mutual admittance</strong> between buses {selectedEntry.from_bus_id} and {selectedEntry.to_bus_id}.</p>
                <p>It equals the negative of the series admittance of the connecting branch: Y<sub>ij</sub> = -y<sub>series</sub></p>
              {/if}
            </div>
          </div>

          <!-- Actions -->
          <div class="sidebar-actions">
            <button class="action-btn" onclick={() => { if (onSelectBus) onSelectBus(selectedEntry!.from_bus_id); }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <circle cx="12" cy="12" r="3"/>
              </svg>
              View Bus {selectedEntry.from_bus_id} in Grid
            </button>
          </div>
        </div>
      {/if}
    </aside>
  {/if}

  <!-- Network Info HUD -->
  {#if network}
    <div class="network-hud">
      <div class="network-name">{network.name}</div>
      <div class="network-stats">
        <span class="stat"><strong>{network.buses.length}</strong> buses</span>
        <span class="stat"><strong>{network.branches.length}</strong> branches</span>
        <span class="stat"><strong>{network.generators.length}</strong> gens</span>
      </div>
      <div class="solve-buttons">
        {#if onSolveDc}
          <button class="solve-btn dc" onclick={onSolveDc} disabled={solvingDc || solvingAc} title="DC Power Flow (fast linearized)">
            {#if solvingDc}
              <span class="loading-spinner"></span>
              DC...
            {:else}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
              DC
            {/if}
          </button>
        {/if}
        {#if onSolveAc}
          <button class="solve-btn ac" onclick={onSolveAc} disabled={solvingAc || solvingDc} title="AC Power Flow (Newton-Raphson)">
            {#if solvingAc}
              <span class="loading-spinner"></span>
              AC...
            {:else}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 12c0-5.523-4.477-10-10-10S2 6.477 2 12s4.477 10 10 10"/>
                <path d="M12 2v10l4.5 4.5"/>
              </svg>
              AC
            {/if}
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ybus-explorer {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .matrix-container {
    width: 100%;
    height: 100%;
    background: var(--bg-tertiary);
    overflow: hidden;
  }

  .loading, .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-muted);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Matrix title HUD - top center */
  .matrix-title-hud {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 16px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    backdrop-filter: blur(8px);
    z-index: 10;
  }

  /* Hover HUD - small tooltip */
  .hover-hud {
    position: absolute;
    top: 16px;
    right: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    backdrop-filter: blur(8px);
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .hover-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .hover-value {
    font-size: 12px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--accent);
  }

  /* Detail Sidebar */
  .detail-sidebar {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 320px;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform 0.25s ease;
    z-index: 20;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .detail-sidebar.open {
    transform: translateX(0);
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .sidebar-header h3 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .sidebar-header .close-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-muted);
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .sidebar-header .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Info Card */
  .info-card {
    background: var(--bg-tertiary);
    border-radius: 8px;
    padding: 14px;
    border: 1px solid var(--border);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .entry-position {
    font-size: 18px;
    font-weight: 700;
    color: var(--accent);
  }

  .entry-type {
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .entry-type.diagonal {
    background: var(--accent);
    color: white;
  }

  .entry-description {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }

  /* Info Sections */
  .info-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .info-section h4 {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0;
  }

  /* Connection Display */
  .connection-display {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bus-badge {
    background: var(--accent);
    color: white;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
  }

  .connection-arrow {
    color: var(--text-muted);
    font-size: 16px;
  }

  /* Complex Display */
  .complex-display {
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 12px;
  }

  .complex-full {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .complex-label {
    font-size: 13px;
    color: var(--text-muted);
  }

  .complex-value {
    font-size: 14px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-primary);
    font-weight: 500;
  }

  /* Components Grid */
  .components-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .component {
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .component-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .component-value {
    font-size: 13px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-primary);
    font-weight: 500;
  }

  /* Interpretation */
  .interpretation {
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 12px;
    border-left: 3px solid var(--accent);
  }

  .interpretation p {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
    margin: 0 0 8px 0;
  }

  .interpretation p:last-child {
    margin-bottom: 0;
  }

  .interpretation strong {
    color: var(--text-primary);
  }

  /* Sidebar Actions */
  .sidebar-actions {
    margin-top: auto;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .action-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  /* Stats HUD - top left */
  .stats-hud {
    position: absolute;
    top: 16px;
    left: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    backdrop-filter: blur(8px);
    z-index: 10;
    display: flex;
    gap: 16px;
  }

  .stats-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: center;
  }

  .stats-row .label {
    font-size: 9px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stats-row .value {
    font-size: 13px;
    font-weight: 600;
    color: var(--accent);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  /* Legend HUD - bottom left */
  .legend-hud {
    position: absolute;
    bottom: 16px;
    left: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    backdrop-filter: blur(8px);
    z-index: 10;
    min-width: 120px;
  }

  .legend-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: block;
    margin-bottom: 6px;
  }

  .legend-bar {
    height: 10px;
    border-radius: 4px;
    background: linear-gradient(to right,
      #440154, #482878, #3e4989, #31688e,
      #26828e, #1f9e89, #35b779, #6ece58,
      #b5de2b, #fde725
    );
    margin-bottom: 4px;
  }

  .legend-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 9px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  /* Network Info HUD - bottom right */
  .network-hud {
    position: absolute;
    bottom: 16px;
    right: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    backdrop-filter: blur(8px);
    z-index: 10;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 160px;
  }

  .network-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .network-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .network-stats .stat strong {
    color: var(--accent);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .solve-buttons {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .network-hud .solve-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 14px;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    flex: 1;
    min-width: 70px;
  }

  .network-hud .solve-btn.dc {
    background: #059669; /* Green for fast/simple */
  }

  .network-hud .solve-btn.dc:hover:not(:disabled) {
    background: #047857;
  }

  .network-hud .solve-btn.ac {
    background: var(--accent); /* Blue for full solve */
  }

  .network-hud .solve-btn.ac:hover:not(:disabled) {
    background: #0052cc;
  }

  .network-hud .solve-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .loading-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
</style>
