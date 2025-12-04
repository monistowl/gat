<script lang="ts">
  import { onMount } from 'svelte';

  // Animation state
  let visible = $state(false);
  let activeLayer = $state<string | null>(null);
  let revealStep = $state(0);

  // Layer definitions with descriptions
  const layers = [
    {
      id: 'demo',
      name: 'gat-demo',
      subtitle: 'Visualization Layer',
      color: '#8b5cf6',
      y: 60,
      description: 'Tauri + Svelte + D3.js desktop application for interactive power grid visualization',
      features: ['Force-directed grid layout', 'Y-bus matrix explorer', 'Real-time power flow animation'],
    },
    {
      id: 'algo',
      name: 'gat-algo',
      subtitle: 'Algorithm Layer',
      color: '#0066ff',
      y: 160,
      description: 'High-performance power system algorithms with sparse matrix optimizations',
      features: ['Newton-Raphson AC Power Flow', 'AC-OPF via IPOPT', 'Sparse Y-bus matrix (CSR)'],
    },
    {
      id: 'io',
      name: 'gat-io',
      subtitle: 'Import/Export Layer',
      color: '#22c55e',
      y: 260,
      description: 'Multi-format parser supporting industry-standard power system data formats',
      features: ['MATPOWER (.m)', 'PSS/E RAW', 'IEEE CDF', 'CGMES/CIM'],
    },
    {
      id: 'core',
      name: 'gat-core',
      subtitle: 'Data Model Layer',
      color: '#f59e0b',
      y: 360,
      description: 'Graph-based network model using petgraph for efficient traversal and analysis',
      features: ['Bus, Branch, Gen, Load', 'Transformer models', 'Type-safe IDs (BusId, GenId)'],
    },
  ];

  // Data flow arrows
  const dataFlows = [
    { from: 'demo', to: 'algo', label: 'solve()' },
    { from: 'algo', to: 'core', label: 'Network' },
    { from: 'io', to: 'core', label: 'parse()' },
    { from: 'demo', to: 'io', label: 'load_case()' },
  ];

  function handleLayerClick(layerId: string) {
    activeLayer = activeLayer === layerId ? null : layerId;
  }

  function startReveal() {
    revealStep = 0;
    const interval = setInterval(() => {
      revealStep++;
      if (revealStep >= layers.length + dataFlows.length) {
        clearInterval(interval);
      }
    }, 300);
  }

  onMount(() => {
    // Trigger entrance animation
    setTimeout(() => {
      visible = true;
      startReveal();
    }, 100);
  });
</script>

<div class="architecture-diagram" class:visible>
  <div class="diagram-container">
    <svg viewBox="0 0 800 480" class="arch-svg">
      <defs>
        <!-- Gradient backgrounds for layers -->
        {#each layers as layer}
          <linearGradient id="grad-{layer.id}" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" style="stop-color:{layer.color};stop-opacity:0.2" />
            <stop offset="100%" style="stop-color:{layer.color};stop-opacity:0.05" />
          </linearGradient>
        {/each}

        <!-- Arrow marker -->
        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="var(--text-muted)" />
        </marker>
      </defs>

      <!-- Layer boxes -->
      {#each layers as layer, i}
        <g
          class="layer"
          class:active={activeLayer === layer.id}
          class:revealed={revealStep > i}
          style="--delay: {i * 0.15}s"
          onclick={() => handleLayerClick(layer.id)}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && handleLayerClick(layer.id)}
        >
          <!-- Background -->
          <rect
            x="100"
            y={layer.y}
            width="400"
            height="80"
            rx="8"
            fill="url(#grad-{layer.id})"
            stroke={layer.color}
            stroke-width={activeLayer === layer.id ? 2 : 1}
            class="layer-bg"
          />

          <!-- Layer name -->
          <text x="120" y={layer.y + 35} class="layer-name" fill={layer.color}>
            {layer.name}
          </text>

          <!-- Subtitle -->
          <text x="120" y={layer.y + 55} class="layer-subtitle">
            {layer.subtitle}
          </text>

          <!-- Crate icon -->
          <g transform="translate(460, {layer.y + 25})">
            <rect x="0" y="0" width="24" height="24" rx="4" fill={layer.color} opacity="0.2" />
            <text x="12" y="17" text-anchor="middle" fill={layer.color} font-size="14">📦</text>
          </g>
        </g>
      {/each}

      <!-- Data flow arrows -->
      {#each dataFlows as flow, i}
        {@const fromLayer = layers.find(l => l.id === flow.from)}
        {@const toLayer = layers.find(l => l.id === flow.to)}
        {#if fromLayer && toLayer}
          <g class="data-flow" class:revealed={revealStep > layers.length + i}>
            <!-- Vertical connector -->
            {#if flow.from === 'demo' && flow.to === 'algo'}
              <path
                d="M 300 {fromLayer.y + 80} L 300 {toLayer.y}"
                stroke="var(--text-muted)"
                stroke-width="2"
                fill="none"
                marker-end="url(#arrowhead)"
                stroke-dasharray="4,4"
              />
              <text x="310" y={(fromLayer.y + 80 + toLayer.y) / 2} class="flow-label">{flow.label}</text>
            {:else if flow.from === 'algo' && flow.to === 'core'}
              <path
                d="M 250 {fromLayer.y + 80} L 250 {toLayer.y}"
                stroke="var(--text-muted)"
                stroke-width="2"
                fill="none"
                marker-end="url(#arrowhead)"
                stroke-dasharray="4,4"
              />
              <text x="260" y={(fromLayer.y + 80 + toLayer.y) / 2 + 50} class="flow-label">{flow.label}</text>
            {:else if flow.from === 'io' && flow.to === 'core'}
              <path
                d="M 350 {fromLayer.y + 80} L 350 {toLayer.y}"
                stroke="var(--text-muted)"
                stroke-width="2"
                fill="none"
                marker-end="url(#arrowhead)"
                stroke-dasharray="4,4"
              />
              <text x="360" y={(fromLayer.y + 80 + toLayer.y) / 2} class="flow-label">{flow.label}</text>
            {:else if flow.from === 'demo' && flow.to === 'io'}
              <path
                d="M 450 {fromLayer.y + 80} Q 550 {(fromLayer.y + toLayer.y) / 2 + 40} 450 {toLayer.y}"
                stroke="var(--text-muted)"
                stroke-width="2"
                fill="none"
                marker-end="url(#arrowhead)"
                stroke-dasharray="4,4"
              />
              <text x="560" y={(fromLayer.y + toLayer.y) / 2 + 40} class="flow-label">{flow.label}</text>
            {/if}
          </g>
        {/if}
      {/each}

      <!-- External dependencies -->
      <g class="externals" class:revealed={revealStep > 2}>
        <rect x="550" y="160" width="200" height="180" rx="8" fill="var(--bg-tertiary)" stroke="var(--border)" />
        <text x="650" y="185" text-anchor="middle" class="external-title">Dependencies</text>

        <text x="570" y="215" class="external-item">petgraph - Graph data structure</text>
        <text x="570" y="240" class="external-item">sprs - Sparse matrices (CSR)</text>
        <text x="570" y="265" class="external-item">num-complex - Complex numbers</text>
        <text x="570" y="290" class="external-item">serde - Serialization</text>
        <text x="570" y="315" class="external-item">tauri - Desktop runtime</text>
      </g>

      <!-- Title -->
      <text x="300" y="35" text-anchor="middle" class="diagram-title">
        GAT Architecture
      </text>
    </svg>
  </div>

  <!-- Detail panel -->
  <div class="detail-panel">
    {#if activeLayer}
      {@const layer = layers.find(l => l.id === activeLayer)}
      {#if layer}
        <div class="detail-header" style="border-color: {layer.color}">
          <h3 style="color: {layer.color}">{layer.name}</h3>
          <span class="detail-subtitle">{layer.subtitle}</span>
        </div>
        <p class="detail-description">{layer.description}</p>
        <div class="detail-features">
          <h4>Key Features</h4>
          <ul>
            {#each layer.features as feature}
              <li>{feature}</li>
            {/each}
          </ul>
        </div>
      {/if}
    {:else}
      <div class="detail-placeholder">
        <p>Click a layer to see details</p>
        <div class="legend">
          <h4>Layer Overview</h4>
          {#each layers as layer}
            <div class="legend-item">
              <span class="legend-color" style="background: {layer.color}"></span>
              <span class="legend-name">{layer.name}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="replay-btn-container">
      <button class="replay-btn" onclick={startReveal}>
        Replay Animation
      </button>
    </div>
  </div>
</div>

<style>
  .architecture-diagram {
    display: flex;
    height: 100%;
    gap: 16px;
    opacity: 0;
    transform: translateY(20px);
    transition: opacity 0.5s ease, transform 0.5s ease;
  }

  .architecture-diagram.visible {
    opacity: 1;
    transform: translateY(0);
  }

  .diagram-container {
    flex: 1;
    background: var(--bg-tertiary);
    border-radius: 8px;
    padding: 16px;
    overflow: hidden;
  }

  .arch-svg {
    width: 100%;
    height: 100%;
  }

  .diagram-title {
    font-size: 18px;
    font-weight: 600;
    fill: var(--text-primary);
  }

  .layer {
    cursor: pointer;
    opacity: 0;
    transform: translateX(-20px);
    transition: opacity 0.4s ease, transform 0.4s ease;
  }

  .layer.revealed {
    opacity: 1;
    transform: translateX(0);
    transition-delay: var(--delay);
  }

  .layer:hover .layer-bg {
    filter: brightness(1.1);
  }

  .layer.active .layer-bg {
    filter: brightness(1.2);
  }

  .layer-name {
    font-size: 18px;
    font-weight: 600;
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .layer-subtitle {
    font-size: 12px;
    fill: var(--text-muted);
  }

  .data-flow {
    opacity: 0;
    transition: opacity 0.4s ease;
  }

  .data-flow.revealed {
    opacity: 1;
  }

  .flow-label {
    font-size: 11px;
    fill: var(--text-muted);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .externals {
    opacity: 0;
    transition: opacity 0.5s ease;
  }

  .externals.revealed {
    opacity: 1;
  }

  .external-title {
    font-size: 14px;
    font-weight: 600;
    fill: var(--text-secondary);
  }

  .external-item {
    font-size: 11px;
    fill: var(--text-muted);
  }

  /* Detail panel */
  .detail-panel {
    width: 280px;
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 16px;
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }

  .detail-header {
    padding-bottom: 12px;
    margin-bottom: 12px;
    border-bottom: 2px solid;
  }

  .detail-header h3 {
    font-size: 18px;
    font-weight: 600;
    font-family: 'SF Mono', 'Fira Code', monospace;
    margin: 0;
  }

  .detail-subtitle {
    font-size: 12px;
    color: var(--text-muted);
  }

  .detail-description {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 16px;
  }

  .detail-features h4 {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }

  .detail-features ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .detail-features li {
    font-size: 13px;
    color: var(--text-secondary);
    padding: 4px 0;
    padding-left: 16px;
    position: relative;
  }

  .detail-features li::before {
    content: '•';
    position: absolute;
    left: 0;
    color: var(--accent);
  }

  .detail-placeholder {
    flex: 1;
  }

  .detail-placeholder > p {
    color: var(--text-muted);
    font-style: italic;
    margin-bottom: 24px;
  }

  .legend h4 {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 12px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
  }

  .legend-color {
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }

  .legend-name {
    font-size: 13px;
    color: var(--text-secondary);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .replay-btn-container {
    margin-top: auto;
    padding-top: 16px;
  }

  .replay-btn {
    width: 100%;
    padding: 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .replay-btn:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
