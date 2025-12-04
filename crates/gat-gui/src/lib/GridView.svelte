<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as d3 from 'd3';

  // Types
  interface BusJson {
    id: number;
    name: string;
    type: string;
    vm: number;
    va: number;
    p_load: number;
    q_load: number;
    voltage_kv: number;
  }

  interface BranchJson {
    from: number;
    to: number;
    r: number;
    x: number;
    b: number;
    p_flow: number;
    loading_pct: number;
    status: boolean;
  }

  interface GeneratorJson {
    bus: number;
    p_gen: number;
    q_gen: number;
    type: string;
  }

  // N-1 Contingency Analysis types
  interface OverloadedBranch {
    from: number;
    to: number;
    loading_pct: number;
    flow_mw: number;
    rating_mva: number;
  }

  interface ContingencyResult {
    outage_from: number;
    outage_to: number;
    has_violations: boolean;
    overloaded_branches: OverloadedBranch[];
    max_loading_pct: number;
    solved: boolean;
  }

  interface N1ContingencyResult {
    total_contingencies: number;
    contingencies_with_violations: number;
    contingencies_failed: number;
    results: ContingencyResult[];
    worst_contingency: ContingencyResult | null;
    solve_time_ms: number;
  }

  interface NetworkJson {
    name: string;
    buses: BusJson[];
    branches: BranchJson[];
    generators: GeneratorJson[];
    base_mva: number;
  }

  // D3 simulation node type
  interface SimNode extends d3.SimulationNodeDatum {
    id: number;
    bus: BusJson;
    isGenerator: boolean;
    genPower: number;
    // For geographic mode - persistent positions
    geoX?: number;
    geoY?: number;
  }

  interface SimLink extends d3.SimulationLinkDatum<SimNode> {
    branch: BranchJson;
  }

  // Layout mode types
  type LayoutMode = 'force' | 'schematic' | 'geographic';

  // Props
  let {
    network,
    selectedBusId = null,
    onSolveAc,
    onSolveDc,
    onRunN1,
    solvingAc = false,
    solvingDc = false,
    runningN1 = false,
    n1Result = null
  }: {
    network: NetworkJson;
    selectedBusId?: number | null;
    onSolveAc?: () => void;
    onSolveDc?: () => void;
    onRunN1?: () => void;
    solvingAc?: boolean;
    solvingDc?: boolean;
    runningN1?: boolean;
    n1Result?: N1ContingencyResult | null;
  } = $props();

  // N-1 panel state
  let n1PanelOpen = $state(false);
  let n1SortBy = $state<'severity' | 'branch'>('severity');

  // Auto-open N-1 panel when results arrive
  $effect(() => {
    if (n1Result && n1Result.total_contingencies > 0) {
      n1PanelOpen = true;
    }
  });

  // Sorted N-1 results
  const sortedN1Results = $derived(() => {
    if (!n1Result) return [];
    const results = [...n1Result.results].filter(r => r.has_violations || !r.solved);
    if (n1SortBy === 'severity') {
      return results.sort((a, b) => b.max_loading_pct - a.max_loading_pct);
    } else {
      return results.sort((a, b) => a.outage_from - b.outage_from || a.outage_to - b.outage_to);
    }
  });

  // Refs
  let container: HTMLDivElement;
  let svg: d3.Selection<SVGSVGElement, unknown, null, undefined>;
  let simulation: d3.Simulation<SimNode, SimLink>;
  let zoomBehavior: d3.ZoomBehavior<SVGSVGElement, unknown>;
  let currentZoom = $state(1);

  // Layout mode state
  let layoutMode = $state<LayoutMode>('force');

  // Geographic mode: persistent positions stored by bus ID
  let geoPositions = $state<Map<number, { x: number; y: number }>>(new Map());

  // Particle animation state
  let particleInterval: number | null = null;

  // Tooltip state
  let hoveredNode = $state<SimNode | null>(null);
  let tooltipPos = $state({ x: 0, y: 0 });
  let connectedBranches = $state<number>(0);

  // Voltage to color scale (green=1.0pu, yellow=0.95, red<0.9)
  const voltageColor = d3.scaleLinear<string>()
    .domain([0.9, 0.95, 1.0, 1.05, 1.1])
    .range(['#ef4444', '#f59e0b', '#22c55e', '#22c55e', '#f59e0b'])
    .clamp(true);

  // Branch loading to color scale (green=low, yellow=moderate, red=overload)
  const loadingColor = d3.scaleLinear<string>()
    .domain([0, 50, 80, 100, 150])
    .range(['#22c55e', '#22c55e', '#f59e0b', '#ef4444', '#dc2626'])
    .clamp(true);

  // Node size based on load/generation
  function nodeRadius(node: SimNode): number {
    const power = Math.max(node.bus.p_load, node.genPower);
    return Math.max(6, Math.min(20, 6 + Math.sqrt(power) * 0.5));
  }

  // Line thickness based on branch impedance (lower = thicker = more capacity)
  function lineWidth(branch: BranchJson): number {
    const impedance = Math.sqrt(branch.r * branch.r + branch.x * branch.x);
    return Math.max(1, Math.min(4, 3 / (impedance * 10 + 0.1)));
  }

  // Grid size for schematic snap
  const GRID_SIZE = 80;

  // HUD margins for safe viewing area (avoid overlapping controls)
  const HUD_MARGINS = {
    top: 60,      // Layout controls height
    right: 60,    // Zoom controls width
    bottom: 20,   // Small buffer
    left: 200     // Legend width
  };

  // Fit the view to show all nodes within the safe area
  function fitToView(nodes: SimNode[], width: number, height: number, animate = true) {
    if (!svg || !zoomBehavior || nodes.length === 0) return;

    // Calculate bounding box of all nodes
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const node of nodes) {
      if (node.x !== undefined && node.y !== undefined) {
        const r = nodeRadius(node);
        minX = Math.min(minX, node.x - r);
        minY = Math.min(minY, node.y - r);
        maxX = Math.max(maxX, node.x + r);
        maxY = Math.max(maxY, node.y + r);
      }
    }

    if (!isFinite(minX)) return; // No valid positions yet

    // Add padding around nodes
    const padding = 40;
    minX -= padding;
    minY -= padding;
    maxX += padding;
    maxY += padding;

    const contentWidth = maxX - minX;
    const contentHeight = maxY - minY;

    // Calculate safe viewing area (excluding HUD)
    const safeWidth = width - HUD_MARGINS.left - HUD_MARGINS.right;
    const safeHeight = height - HUD_MARGINS.top - HUD_MARGINS.bottom;

    // Calculate scale to fit content in safe area
    const scale = Math.min(
      safeWidth / contentWidth,
      safeHeight / contentHeight,
      2 // Max initial zoom
    );

    // Calculate center of content
    const contentCenterX = (minX + maxX) / 2;
    const contentCenterY = (minY + maxY) / 2;

    // Calculate center of safe area
    const safeCenterX = HUD_MARGINS.left + safeWidth / 2;
    const safeCenterY = HUD_MARGINS.top + safeHeight / 2;

    // Calculate translation to center content in safe area
    const translateX = safeCenterX - contentCenterX * scale;
    const translateY = safeCenterY - contentCenterY * scale;

    const transform = d3.zoomIdentity.translate(translateX, translateY).scale(scale);

    if (animate) {
      svg.transition().duration(500).call(zoomBehavior.transform, transform);
    } else {
      svg.call(zoomBehavior.transform, transform);
    }
  }

  // Calculate schematic grid positions - organize by voltage level and connectivity
  function calculateSchematicPositions(
    nodes: SimNode[],
    links: SimLink[],
    width: number,
    height: number
  ): Map<number, { x: number; y: number }> {
    const positions = new Map<number, { x: number; y: number }>();

    // Group buses by voltage level (approximate tiers)
    const voltageTiers = new Map<number, SimNode[]>();
    for (const node of nodes) {
      const kv = node.bus.voltage_kv || 100;
      // Round to nearest standard voltage tier
      const tier = kv >= 300 ? 500 : kv >= 200 ? 230 : kv >= 100 ? 138 : kv >= 50 ? 69 : 13;
      if (!voltageTiers.has(tier)) voltageTiers.set(tier, []);
      voltageTiers.get(tier)!.push(node);
    }

    // Sort tiers by voltage (highest at top)
    const sortedTiers = Array.from(voltageTiers.entries()).sort((a, b) => b[0] - a[0]);

    // Calculate grid layout
    const margin = GRID_SIZE * 1.5;
    const availableWidth = width - margin * 2;
    const tierHeight = (height - margin * 2) / Math.max(sortedTiers.length, 1);

    sortedTiers.forEach(([_, tierNodes], tierIndex) => {
      // Sort nodes within tier: generators first, then by ID
      tierNodes.sort((a, b) => {
        if (a.isGenerator !== b.isGenerator) return a.isGenerator ? -1 : 1;
        return a.id - b.id;
      });

      const nodesPerRow = Math.max(1, Math.floor(availableWidth / GRID_SIZE));
      tierNodes.forEach((node, nodeIndex) => {
        const row = Math.floor(nodeIndex / nodesPerRow);
        const col = nodeIndex % nodesPerRow;

        // Snap to grid with some offset per tier
        const x = margin + col * GRID_SIZE + (tierIndex % 2) * (GRID_SIZE / 2);
        const y = margin + tierIndex * tierHeight + row * GRID_SIZE;

        positions.set(node.id, {
          x: Math.round(x / GRID_SIZE) * GRID_SIZE,
          y: Math.round(y / GRID_SIZE) * GRID_SIZE
        });
      });
    });

    return positions;
  }

  // Initialize geographic positions from current layout or defaults
  function initGeoPositions(nodes: SimNode[], width: number, height: number) {
    for (const node of nodes) {
      if (!geoPositions.has(node.id)) {
        // Initialize with a spread pattern if no existing position
        const angle = (node.id * 137.508) * (Math.PI / 180); // Golden angle
        const r = Math.sqrt(node.id) * 40;
        geoPositions.set(node.id, {
          x: width / 2 + r * Math.cos(angle),
          y: height / 2 + r * Math.sin(angle)
        });
      }
    }
  }

  // Save geographic position when node is dragged
  function saveGeoPosition(node: SimNode) {
    if (layoutMode === 'geographic' && node.x !== undefined && node.y !== undefined) {
      geoPositions.set(node.id, { x: node.x, y: node.y });
    }
  }

  function initVisualization() {
    if (!container || !network) return;

    // Clear previous
    d3.select(container).selectAll('*').remove();
    if (simulation) simulation.stop();
    if (particleInterval) clearInterval(particleInterval);

    const width = container.clientWidth;
    const height = container.clientHeight;

    // Create SVG
    svg = d3.select(container)
      .append('svg')
      .attr('width', width)
      .attr('height', height)
      .attr('viewBox', [0, 0, width, height]);

    // Add zoom behavior
    const g = svg.append('g');
    zoomBehavior = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
        currentZoom = event.transform.k;
      });
    svg.call(zoomBehavior);

    // Build generator lookup
    const genByBus = new Map<number, number>();
    for (const gen of network.generators) {
      genByBus.set(gen.bus, (genByBus.get(gen.bus) || 0) + gen.p_gen);
    }

    // Create nodes
    const nodes: SimNode[] = network.buses.map(bus => ({
      id: bus.id,
      bus,
      isGenerator: genByBus.has(bus.id),
      genPower: genByBus.get(bus.id) || 0,
    }));

    const nodeById = new Map(nodes.map(n => [n.id, n]));

    // Create links
    const links: SimLink[] = network.branches
      .filter(b => b.status && nodeById.has(b.from) && nodeById.has(b.to))
      .map(branch => ({
        source: nodeById.get(branch.from)!,
        target: nodeById.get(branch.to)!,
        branch,
      }));

    // Layout-specific positioning
    if (layoutMode === 'schematic') {
      // Calculate and apply schematic grid positions
      const schematicPos = calculateSchematicPositions(nodes, links, width, height);
      for (const node of nodes) {
        const pos = schematicPos.get(node.id);
        if (pos) {
          node.x = pos.x;
          node.y = pos.y;
          node.fx = pos.x; // Fix position
          node.fy = pos.y;
        }
      }

      // Draw grid background for schematic mode
      const gridGroup = g.append('g').attr('class', 'grid-background');
      for (let x = 0; x <= width; x += GRID_SIZE) {
        gridGroup.append('line')
          .attr('x1', x).attr('y1', 0)
          .attr('x2', x).attr('y2', height)
          .attr('stroke', 'var(--border)')
          .attr('stroke-opacity', 0.3)
          .attr('stroke-dasharray', '2,4');
      }
      for (let y = 0; y <= height; y += GRID_SIZE) {
        gridGroup.append('line')
          .attr('x1', 0).attr('y1', y)
          .attr('x2', width).attr('y2', y)
          .attr('stroke', 'var(--border)')
          .attr('stroke-opacity', 0.3)
          .attr('stroke-dasharray', '2,4');
      }

      // No simulation needed for schematic
      simulation = d3.forceSimulation(nodes).stop();

    } else if (layoutMode === 'geographic') {
      // Use persistent geographic positions
      initGeoPositions(nodes, width, height);
      for (const node of nodes) {
        const pos = geoPositions.get(node.id);
        if (pos) {
          node.x = pos.x;
          node.y = pos.y;
        }
      }

      // Draw map-like background with coordinate grid
      const mapGroup = g.append('g').attr('class', 'map-background');
      // Subtle coordinate overlay
      const gridSpacing = 100;
      for (let x = 0; x <= width; x += gridSpacing) {
        mapGroup.append('line')
          .attr('x1', x).attr('y1', 0)
          .attr('x2', x).attr('y2', height)
          .attr('stroke', '#22c55e')
          .attr('stroke-opacity', 0.1);
        // Coordinate labels
        if (x > 0 && x < width) {
          mapGroup.append('text')
            .attr('x', x).attr('y', 14)
            .attr('fill', 'var(--text-muted)')
            .attr('font-size', 9)
            .attr('text-anchor', 'middle')
            .attr('opacity', 0.5)
            .text(`${x}`);
        }
      }
      for (let y = 0; y <= height; y += gridSpacing) {
        mapGroup.append('line')
          .attr('x1', 0).attr('y1', y)
          .attr('x2', width).attr('y2', y)
          .attr('stroke', '#22c55e')
          .attr('stroke-opacity', 0.1);
        if (y > 0 && y < height) {
          mapGroup.append('text')
            .attr('x', 8).attr('y', y + 3)
            .attr('fill', 'var(--text-muted)')
            .attr('font-size', 9)
            .attr('opacity', 0.5)
            .text(`${y}`);
        }
      }

      // No simulation for geographic mode
      simulation = d3.forceSimulation(nodes).stop();

    } else {
      // Force-directed layout (default)
      simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink<SimNode, SimLink>(links)
          .id(d => d.id)
          .distance(50)
          .strength(0.3))
        .force('charge', d3.forceManyBody()
          .strength(-100)
          .distanceMax(300))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('collision', d3.forceCollide<SimNode>()
          .radius(d => nodeRadius(d) + 5));
    }

    // Draw links (branches) with loading-based coloring
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', d => {
        // Color by loading if solved (loading_pct > 0), otherwise neutral
        const loading = d.branch.loading_pct;
        return loading > 0 ? loadingColor(loading) : '#3f3f46';
      })
      .attr('stroke-width', d => lineWidth(d.branch))
      .attr('stroke-opacity', 0.8);

    // Draw nodes (buses)
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll<SVGGElement, SimNode>('g')
      .data(nodes)
      .join('g')
      .call(drag(simulation) as any);

    if (layoutMode === 'schematic') {
      // Engineering schematic symbols

      // Generator symbol: circle with "G" or sine wave
      node.filter(d => d.isGenerator)
        .append('circle')
        .attr('r', 16)
        .attr('fill', 'var(--bg-secondary)')
        .attr('stroke', '#0066ff')
        .attr('stroke-width', 2);

      node.filter(d => d.isGenerator)
        .append('text')
        .attr('text-anchor', 'middle')
        .attr('dy', 5)
        .attr('fill', '#0066ff')
        .attr('font-size', 14)
        .attr('font-weight', 600)
        .text('G');

      // Load symbol: downward arrow with bar
      node.filter(d => !d.isGenerator && d.bus.p_load > 0)
        .append('path')
        .attr('d', 'M0,-12 L0,8 M-8,2 L0,12 L8,2 M-10,12 L10,12')
        .attr('fill', 'none')
        .attr('stroke', d => voltageColor(d.bus.vm))
        .attr('stroke-width', 2)
        .attr('stroke-linecap', 'round');

      // Regular bus: horizontal bar (busbar symbol)
      node.filter(d => !d.isGenerator && d.bus.p_load === 0)
        .append('rect')
        .attr('x', -12)
        .attr('y', -3)
        .attr('width', 24)
        .attr('height', 6)
        .attr('fill', d => voltageColor(d.bus.vm))
        .attr('stroke', 'var(--text-muted)')
        .attr('stroke-width', 1);

      // Slack bus: diamond shape
      node.filter(d => d.bus.type === 'slack')
        .append('path')
        .attr('d', 'M0,-14 L14,0 L0,14 L-14,0 Z')
        .attr('fill', 'none')
        .attr('stroke', '#8b5cf6')
        .attr('stroke-width', 2);

      // Bus ID labels for schematic
      node.append('text')
        .attr('x', 20)
        .attr('y', 4)
        .attr('fill', 'var(--text-secondary)')
        .attr('font-size', 10)
        .attr('font-family', 'SF Mono, monospace')
        .text(d => `${d.id}`);

    } else {
      // Original force/geographic mode symbols
      // Bus circles
      node.append('circle')
        .attr('r', d => nodeRadius(d))
        .attr('fill', d => voltageColor(d.bus.vm))
        .attr('stroke', d => d.isGenerator ? '#0066ff' : '#52525b')
        .attr('stroke-width', d => d.isGenerator ? 3 : 1.5);

      // Generator triangles
      node.filter(d => d.isGenerator)
        .append('path')
        .attr('d', d => {
          const size = nodeRadius(d) + 8;
          return `M0,${-size} L${size * 0.866},${size * 0.5} L${-size * 0.866},${size * 0.5} Z`;
        })
        .attr('fill', 'none')
        .attr('stroke', '#0066ff')
        .attr('stroke-width', 2)
        .attr('opacity', 0.7);
    }

    // Build connection count map
    const connectionCount = new Map<number, number>();
    for (const link of links) {
      const sourceId = (link.source as SimNode).id;
      const targetId = (link.target as SimNode).id;
      connectionCount.set(sourceId, (connectionCount.get(sourceId) || 0) + 1);
      connectionCount.set(targetId, (connectionCount.get(targetId) || 0) + 1);
    }

    // Rich tooltip on hover
    node.on('mouseenter', (event, d) => {
      const rect = container.getBoundingClientRect();
      tooltipPos = {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top
      };
      connectedBranches = connectionCount.get(d.id) || 0;
      hoveredNode = d;
    })
    .on('mousemove', (event) => {
      const rect = container.getBoundingClientRect();
      tooltipPos = {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top
      };
    })
    .on('mouseleave', () => {
      hoveredNode = null;
    });

    // Particle container for power flow animation
    const particleLayer = g.append('g').attr('class', 'particles');

    // Update positions on tick (only for force layout)
    if (layoutMode === 'force') {
      let hasFitted = false;

      simulation.on('tick', () => {
        link
          .attr('x1', d => (d.source as SimNode).x!)
          .attr('y1', d => (d.source as SimNode).y!)
          .attr('x2', d => (d.target as SimNode).x!)
          .attr('y2', d => (d.target as SimNode).y!);

        node.attr('transform', d => `translate(${d.x},${d.y})`);
      });

      // Fit to view once simulation has mostly stabilized
      simulation.on('end', () => {
        if (!hasFitted) {
          hasFitted = true;
          fitToView(nodes, width, height, true);
          startParticleAnimation(particleLayer, links);
        }
      });

      // Also fit after a timeout in case simulation doesn't fully end
      setTimeout(() => {
        if (!hasFitted) {
          hasFitted = true;
          fitToView(nodes, width, height, true);
          startParticleAnimation(particleLayer, links);
        }
      }, 2500);
    } else {
      // Static layout - set positions immediately
      link
        .attr('x1', d => (d.source as SimNode).x!)
        .attr('y1', d => (d.source as SimNode).y!)
        .attr('x2', d => (d.target as SimNode).x!)
        .attr('y2', d => (d.target as SimNode).y!);

      node.attr('transform', d => `translate(${d.x},${d.y})`);

      // Fit to view and start particle animation immediately for static layouts
      fitToView(nodes, width, height, false);
      startParticleAnimation(particleLayer, links);
    }
  }

  // Drag behavior - varies by layout mode
  function drag(simulation: d3.Simulation<SimNode, SimLink>) {
    function dragstarted(event: d3.D3DragEvent<SVGGElement, SimNode, SimNode>) {
      if (layoutMode === 'force') {
        if (!event.active) simulation.alphaTarget(0.3).restart();
      }
      event.subject.fx = event.subject.x;
      event.subject.fy = event.subject.y;
    }

    function dragged(event: d3.D3DragEvent<SVGGElement, SimNode, SimNode>) {
      if (layoutMode === 'schematic') {
        // Snap to grid
        event.subject.fx = Math.round(event.x / GRID_SIZE) * GRID_SIZE;
        event.subject.fy = Math.round(event.y / GRID_SIZE) * GRID_SIZE;
      } else {
        event.subject.fx = event.x;
        event.subject.fy = event.y;
      }
      event.subject.x = event.subject.fx!;
      event.subject.y = event.subject.fy!;

      // For geographic mode, persist position immediately
      if (layoutMode === 'geographic') {
        saveGeoPosition(event.subject);
      }

      // Update position directly for static layouts
      if (layoutMode !== 'force') {
        d3.select(event.sourceEvent.target.parentNode)
          .attr('transform', `translate(${event.subject.x},${event.subject.y})`);

        // Update connected links
        d3.select(container).selectAll<SVGLineElement, SimLink>('.links line')
          .attr('x1', d => (d.source as SimNode).x!)
          .attr('y1', d => (d.source as SimNode).y!)
          .attr('x2', d => (d.target as SimNode).x!)
          .attr('y2', d => (d.target as SimNode).y!);
      }
    }

    function dragended(event: d3.D3DragEvent<SVGGElement, SimNode, SimNode>) {
      if (layoutMode === 'force') {
        if (!event.active) simulation.alphaTarget(0);
        event.subject.fx = null;
        event.subject.fy = null;
      } else if (layoutMode === 'geographic') {
        // Keep position fixed and save
        saveGeoPosition(event.subject);
      }
      // Schematic keeps fx/fy set
    }

    return d3.drag<SVGGElement, SimNode>()
      .on('start', dragstarted)
      .on('drag', dragged)
      .on('end', dragended);
  }

  // Particle animation for power flow
  function startParticleAnimation(
    layer: d3.Selection<SVGGElement, unknown, null, undefined>,
    links: SimLink[]
  ) {
    const particles: Array<{
      link: SimLink;
      progress: number;
      speed: number;
    }> = [];

    // Create initial particles
    for (const link of links) {
      const numParticles = Math.max(1, Math.ceil(Math.abs(link.branch.p_flow) / 20));
      for (let i = 0; i < Math.min(numParticles, 3); i++) {
        particles.push({
          link,
          progress: Math.random(),
          speed: 0.005 + Math.random() * 0.01,
        });
      }
    }

    // Animation loop
    particleInterval = setInterval(() => {
      // Update particle positions
      for (const p of particles) {
        p.progress += p.speed;
        if (p.progress > 1) p.progress = 0;
      }

      // Render particles
      const circles = layer.selectAll<SVGCircleElement, typeof particles[0]>('circle')
        .data(particles);

      circles.enter()
        .append('circle')
        .attr('r', 2)
        .attr('fill', '#0066ff')
        .attr('opacity', 0.8)
        .merge(circles)
        .attr('cx', d => {
          const source = d.link.source as SimNode;
          const target = d.link.target as SimNode;
          return source.x! + (target.x! - source.x!) * d.progress;
        })
        .attr('cy', d => {
          const source = d.link.source as SimNode;
          const target = d.link.target as SimNode;
          return source.y! + (target.y! - source.y!) * d.progress;
        });

      circles.exit().remove();
    }, 50) as unknown as number;
  }

  // Zoom controls
  function zoomIn() {
    if (!svg || !zoomBehavior) return;
    svg.transition().duration(300).call(zoomBehavior.scaleBy, 1.5);
  }

  function zoomOut() {
    if (!svg || !zoomBehavior) return;
    svg.transition().duration(300).call(zoomBehavior.scaleBy, 0.67);
  }

  function zoomReset() {
    if (!svg || !zoomBehavior) return;
    svg.transition().duration(300).call(zoomBehavior.transform, d3.zoomIdentity);
  }

  function zoomFit() {
    if (!container || !network) return;
    // Get current node data from D3 selection
    const nodeData = d3.select(container).selectAll<SVGGElement, SimNode>('.nodes g').data();
    if (nodeData.length > 0) {
      fitToView(nodeData, container.clientWidth, container.clientHeight, true);
    }
  }

  // React to network or layout mode changes
  $effect(() => {
    // Track layoutMode to trigger reinit
    const mode = layoutMode;
    if (network && container) {
      initVisualization();
    }
  });

  // Layout mode change handler
  function setLayoutMode(mode: LayoutMode) {
    layoutMode = mode;
  }

  // Highlight selected bus
  $effect(() => {
    if (!container || !svg) return;

    const nodes = d3.select(container).selectAll<SVGGElement, SimNode>('.nodes g');

    // Reset all nodes
    nodes.select('circle')
      .attr('stroke-width', (d: SimNode) => d.isGenerator ? 3 : 1.5)
      .attr('stroke', (d: SimNode) => d.isGenerator ? '#0066ff' : '#52525b');

    // Highlight selected
    if (selectedBusId !== null) {
      nodes.filter((d: SimNode) => d.id === selectedBusId)
        .select('circle')
        .attr('stroke', '#fde047')
        .attr('stroke-width', 4);
    }
  });

  // Keyboard shortcuts handler
  function handleKeydown(event: KeyboardEvent) {
    // Ignore if focus is in an input/textarea
    const target = event.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;

    switch (event.key) {
      case ' ': // Space = solve
        event.preventDefault();
        if (event.shiftKey && onSolveDc && !solvingDc && !solvingAc) {
          onSolveDc();
        } else if (onSolveAc && !solvingAc && !solvingDc) {
          onSolveAc();
        }
        break;
      case 'f':
      case 'F':
        event.preventDefault();
        zoomFit();
        break;
      case '1':
        event.preventDefault();
        setLayoutMode('force');
        break;
      case '2':
        event.preventDefault();
        setLayoutMode('schematic');
        break;
      case '3':
        event.preventDefault();
        setLayoutMode('geographic');
        break;
      case '+':
      case '=':
        event.preventDefault();
        zoomIn();
        break;
      case '-':
      case '_':
        event.preventDefault();
        zoomOut();
        break;
      case '0':
        event.preventDefault();
        zoomReset();
        break;
    }
  }

  onMount(() => {
    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      if (network) initVisualization();
    });
    resizeObserver.observe(container);

    // Add keyboard handler
    document.addEventListener('keydown', handleKeydown);

    return () => {
      resizeObserver.disconnect();
      document.removeEventListener('keydown', handleKeydown);
      if (simulation) simulation.stop();
      if (particleInterval) clearInterval(particleInterval);
    };
  });

  onDestroy(() => {
    if (simulation) simulation.stop();
    if (particleInterval) clearInterval(particleInterval);
  });
</script>

<div class="grid-view-wrapper">
  <div class="grid-view" bind:this={container}>
    {#if !network}
      <div class="empty">No network loaded</div>
    {/if}
  </div>

  {#if network}
    <!-- Rich tooltip -->
    {#if hoveredNode}
      <div
        class="tooltip"
        style="left: {tooltipPos.x + 16}px; top: {tooltipPos.y - 10}px;"
      >
        <div class="tooltip-header">
          <span class="tooltip-id">Bus {hoveredNode.bus.id}</span>
          <span class="tooltip-type" class:generator={hoveredNode.isGenerator} class:slack={hoveredNode.bus.type === 'slack'}>
            {hoveredNode.bus.type === 'slack' ? 'Slack' : hoveredNode.isGenerator ? 'PV' : 'PQ'}
          </span>
        </div>

        {#if hoveredNode.bus.name && hoveredNode.bus.name !== `Bus${hoveredNode.bus.id}`}
          <div class="tooltip-name">{hoveredNode.bus.name}</div>
        {/if}

        <div class="tooltip-section">
          <div class="tooltip-row">
            <span class="tooltip-label">Voltage</span>
            <span class="tooltip-value" class:warning={hoveredNode.bus.vm < 0.95 || hoveredNode.bus.vm > 1.05} class:critical={hoveredNode.bus.vm < 0.9 || hoveredNode.bus.vm > 1.1}>
              {hoveredNode.bus.vm.toFixed(4)} pu
            </span>
          </div>
          <div class="tooltip-row">
            <span class="tooltip-label">Angle</span>
            <span class="tooltip-value">{hoveredNode.bus.va.toFixed(2)}°</span>
          </div>
          {#if hoveredNode.bus.voltage_kv > 0}
            <div class="tooltip-row">
              <span class="tooltip-label">Base kV</span>
              <span class="tooltip-value">{hoveredNode.bus.voltage_kv.toFixed(1)} kV</span>
            </div>
          {/if}
        </div>

        {#if hoveredNode.bus.p_load > 0 || hoveredNode.bus.q_load !== 0}
          <div class="tooltip-section">
            <div class="tooltip-section-title">Load</div>
            <div class="tooltip-row">
              <span class="tooltip-label">P</span>
              <span class="tooltip-value">{hoveredNode.bus.p_load.toFixed(2)} MW</span>
            </div>
            <div class="tooltip-row">
              <span class="tooltip-label">Q</span>
              <span class="tooltip-value">{hoveredNode.bus.q_load.toFixed(2)} MVAr</span>
            </div>
          </div>
        {/if}

        {#if hoveredNode.isGenerator}
          <div class="tooltip-section">
            <div class="tooltip-section-title">Generation</div>
            <div class="tooltip-row">
              <span class="tooltip-label">P</span>
              <span class="tooltip-value gen">{hoveredNode.genPower.toFixed(2)} MW</span>
            </div>
          </div>
        {/if}

        <div class="tooltip-section">
          <div class="tooltip-row">
            <span class="tooltip-label">Connections</span>
            <span class="tooltip-value">{connectedBranches} branch{connectedBranches !== 1 ? 'es' : ''}</span>
          </div>
        </div>
      </div>
    {/if}

    <!-- Layout mode selector -->
    <div class="layout-controls">
      <span class="layout-label">Layout <span class="kbd-hint">[1-3]</span></span>
      <div class="layout-buttons">
        <button
          class="layout-btn"
          class:active={layoutMode === 'force'}
          onclick={() => setLayoutMode('force')}
          title="Force-directed layout - physics simulation (press 1)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <circle cx="5" cy="5" r="2"/>
            <circle cx="19" cy="5" r="2"/>
            <circle cx="5" cy="19" r="2"/>
            <circle cx="19" cy="19" r="2"/>
            <line x1="7" y1="7" x2="10" y2="10"/>
            <line x1="17" y1="7" x2="14" y2="10"/>
            <line x1="7" y1="17" x2="10" y2="14"/>
            <line x1="17" y1="17" x2="14" y2="14"/>
          </svg>
          Force
        </button>
        <button
          class="layout-btn"
          class:active={layoutMode === 'schematic'}
          onclick={() => setLayoutMode('schematic')}
          title="Schematic layout - engineering grid (press 2)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="1"/>
            <line x1="3" y1="9" x2="21" y2="9"/>
            <line x1="3" y1="15" x2="21" y2="15"/>
            <line x1="9" y1="3" x2="9" y2="21"/>
            <line x1="15" y1="3" x2="15" y2="21"/>
          </svg>
          Schematic
        </button>
        <button
          class="layout-btn"
          class:active={layoutMode === 'geographic'}
          onclick={() => setLayoutMode('geographic')}
          title="Geographic layout - manual positioning (press 3)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="10" r="3"/>
            <path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7z"/>
          </svg>
          Geographic
        </button>
      </div>
    </div>

    <div class="zoom-controls">
      <button class="zoom-btn" onclick={zoomIn} title="Zoom in (+)">+</button>
      <button class="zoom-btn zoom-level" onclick={zoomReset} title="Reset zoom (0)">
        {Math.round(currentZoom * 100)}%
      </button>
      <button class="zoom-btn" onclick={zoomOut} title="Zoom out (-)">−</button>
      <button class="zoom-btn zoom-fit" onclick={zoomFit} title="Fit to view (F)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
        </svg>
      </button>
    </div>

    <div class="legend">
      <div class="legend-section">
        <span class="legend-title">Voltage (pu)</span>
        <div class="voltage-scale">
          <div class="voltage-gradient"></div>
          <div class="voltage-labels">
            <span>0.9</span>
            <span>1.0</span>
            <span>1.1</span>
          </div>
        </div>
      </div>

      <div class="legend-section">
        <span class="legend-title">Node Type</span>
        <div class="legend-item">
          <svg width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="6" fill="#22c55e" stroke="#52525b" stroke-width="1.5"/>
          </svg>
          <span>Load Bus (PQ)</span>
        </div>
        <div class="legend-item">
          <svg width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="6" fill="#22c55e" stroke="#0066ff" stroke-width="3"/>
            <path d="M12,2 L20,18 L4,18 Z" fill="none" stroke="#0066ff" stroke-width="1.5" opacity="0.7"/>
          </svg>
          <span>Generator (PV)</span>
        </div>
        <div class="legend-item">
          <svg width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="6" fill="#22c55e" stroke="#fde047" stroke-width="3"/>
          </svg>
          <span>Selected</span>
        </div>
      </div>

      <div class="legend-section">
        <span class="legend-title">Node Size</span>
        <div class="legend-item">
          <svg width="24" height="16" viewBox="0 0 24 16">
            <circle cx="6" cy="8" r="4" fill="#71717a"/>
            <circle cx="18" cy="8" r="7" fill="#71717a"/>
          </svg>
          <span>Power (MW)</span>
        </div>
      </div>

      <div class="legend-section">
        <span class="legend-title">Branch Loading</span>
        <div class="loading-scale">
          <div class="loading-gradient"></div>
          <div class="loading-labels">
            <span>0%</span>
            <span>50%</span>
            <span>100%</span>
          </div>
        </div>
        <div class="legend-item">
          <svg width="24" height="16" viewBox="0 0 24 16">
            <circle cx="8" cy="8" r="3" fill="#0066ff" opacity="0.8"/>
            <circle cx="16" cy="8" r="3" fill="#0066ff" opacity="0.8"/>
          </svg>
          <span>Power flow</span>
        </div>
      </div>
    </div>

    <!-- Network Info HUD -->
    <div class="network-hud">
      <div class="network-name">{network.name}</div>
      <div class="network-stats">
        <span class="stat"><strong>{network.buses.length}</strong> buses</span>
        <span class="stat"><strong>{network.branches.length}</strong> branches</span>
        <span class="stat"><strong>{network.generators.length}</strong> gens</span>
      </div>
      <div class="solve-buttons">
        {#if onSolveDc}
          <button class="solve-btn dc" onclick={onSolveDc} disabled={solvingDc || solvingAc || runningN1} title="DC Power Flow - fast linearized (Shift+Space)">
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
          <button class="solve-btn ac" onclick={onSolveAc} disabled={solvingAc || solvingDc || runningN1} title="AC Power Flow - Newton-Raphson (Space)">
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
        {#if onRunN1}
          <button class="solve-btn n1" onclick={onRunN1} disabled={runningN1 || solvingAc || solvingDc} title="N-1 Contingency Analysis - security screening">
            {#if runningN1}
              <span class="loading-spinner"></span>
              N-1...
            {:else}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 9v4"/>
                <path d="M12 17h.01"/>
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
              </svg>
              N-1
            {/if}
          </button>
        {/if}
      </div>
      {#if n1Result}
        <button class="n1-toggle" onclick={() => n1PanelOpen = !n1PanelOpen} class:has-violations={n1Result.contingencies_with_violations > 0}>
          <span class="n1-badge" class:secure={n1Result.contingencies_with_violations === 0} class:violations={n1Result.contingencies_with_violations > 0}>
            {n1Result.contingencies_with_violations === 0 ? 'SECURE' : `${n1Result.contingencies_with_violations} VIOLATIONS`}
          </span>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class:open={n1PanelOpen}>
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>
      {/if}
    </div>

    <!-- N-1 Contingency Results Panel -->
    {#if n1PanelOpen && n1Result}
      <div class="n1-panel">
        <div class="n1-panel-header">
          <h3>N-1 Security Analysis</h3>
          <button class="n1-close" onclick={() => n1PanelOpen = false}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <div class="n1-summary">
          <div class="n1-stat">
            <span class="n1-stat-value">{n1Result.total_contingencies}</span>
            <span class="n1-stat-label">Contingencies</span>
          </div>
          <div class="n1-stat" class:warning={n1Result.contingencies_with_violations > 0}>
            <span class="n1-stat-value">{n1Result.contingencies_with_violations}</span>
            <span class="n1-stat-label">Violations</span>
          </div>
          <div class="n1-stat" class:error={n1Result.contingencies_failed > 0}>
            <span class="n1-stat-value">{n1Result.contingencies_failed}</span>
            <span class="n1-stat-label">Failed</span>
          </div>
          <div class="n1-stat">
            <span class="n1-stat-value">{n1Result.solve_time_ms.toFixed(1)}ms</span>
            <span class="n1-stat-label">Time</span>
          </div>
        </div>

        {#if n1Result.worst_contingency}
          <div class="n1-worst">
            <div class="n1-worst-title">Worst Contingency</div>
            <div class="n1-worst-branch">
              Branch {n1Result.worst_contingency.outage_from} → {n1Result.worst_contingency.outage_to}
            </div>
            <div class="n1-worst-loading" class:critical={n1Result.worst_contingency.max_loading_pct > 100}>
              Max Loading: {n1Result.worst_contingency.max_loading_pct.toFixed(1)}%
            </div>
            {#if n1Result.worst_contingency.overloaded_branches.length > 0}
              <div class="n1-overloaded-list">
                {#each n1Result.worst_contingency.overloaded_branches.slice(0, 3) as ob}
                  <div class="n1-overloaded-item">
                    <span class="ob-branch">{ob.from}→{ob.to}</span>
                    <span class="ob-loading" class:critical={ob.loading_pct > 100}>{ob.loading_pct.toFixed(1)}%</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        {#if sortedN1Results().length > 0}
          <div class="n1-list-header">
            <span>Contingencies with Issues ({sortedN1Results().length})</span>
            <div class="n1-sort">
              <button class:active={n1SortBy === 'severity'} onclick={() => n1SortBy = 'severity'}>Severity</button>
              <button class:active={n1SortBy === 'branch'} onclick={() => n1SortBy = 'branch'}>Branch</button>
            </div>
          </div>
          <div class="n1-list">
            {#each sortedN1Results().slice(0, 20) as cont}
              <div class="n1-item" class:failed={!cont.solved} class:violation={cont.has_violations}>
                <div class="n1-item-branch">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="5" y1="12" x2="19" y2="12"/>
                    <line x1="5" y1="12" x2="9" y2="8"/>
                    <line x1="5" y1="12" x2="9" y2="16"/>
                  </svg>
                  {cont.outage_from} → {cont.outage_to}
                </div>
                <div class="n1-item-status">
                  {#if !cont.solved}
                    <span class="n1-island">ISLAND</span>
                  {:else if cont.has_violations}
                    <span class="n1-loading" class:critical={cont.max_loading_pct > 100}>
                      {cont.max_loading_pct.toFixed(1)}%
                    </span>
                  {/if}
                </div>
              </div>
            {/each}
            {#if sortedN1Results().length > 20}
              <div class="n1-more">+{sortedN1Results().length - 20} more...</div>
            {/if}
          </div>
        {:else if n1Result.contingencies_with_violations === 0 && n1Result.contingencies_failed === 0}
          <div class="n1-secure">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
              <polyline points="22 4 12 14.01 9 11.01"/>
            </svg>
            <span>System is N-1 Secure</span>
            <span class="n1-secure-sub">No overloads detected for any single branch outage</span>
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .grid-view-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .grid-view {
    width: 100%;
    height: 100%;
    min-height: 400px;
    background: var(--bg-tertiary);
    border-radius: 8px;
    overflow: hidden;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  /* Rich tooltip */
  .tooltip {
    position: absolute;
    z-index: 100;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    min-width: 180px;
    max-width: 240px;
    backdrop-filter: blur(12px);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    pointer-events: none;
    animation: tooltipFadeIn 0.15s ease-out;
  }

  @keyframes tooltipFadeIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .tooltip-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .tooltip-id {
    font-weight: 600;
    font-size: 14px;
    color: var(--text-primary);
  }

  .tooltip-type {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tooltip-type.generator {
    background: rgba(0, 102, 255, 0.2);
    color: #0066ff;
  }

  .tooltip-type.slack {
    background: rgba(139, 92, 246, 0.2);
    color: #8b5cf6;
  }

  .tooltip-name {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 8px;
    font-style: italic;
  }

  .tooltip-section {
    margin-bottom: 8px;
  }

  .tooltip-section:last-child {
    margin-bottom: 0;
  }

  .tooltip-section-title {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  .tooltip-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    padding: 2px 0;
  }

  .tooltip-label {
    color: var(--text-muted);
  }

  .tooltip-value {
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-primary);
    font-size: 11px;
  }

  .tooltip-value.warning {
    color: var(--warning);
  }

  .tooltip-value.critical {
    color: var(--error);
  }

  .tooltip-value.gen {
    color: #0066ff;
  }

  /* Layout mode controls */
  .layout-controls {
    position: absolute;
    top: 16px;
    left: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    backdrop-filter: blur(8px);
    z-index: 10;
  }

  .layout-label {
    display: block;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .kbd-hint {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-weight: 400;
    opacity: 0.6;
    font-size: 9px;
  }

  .layout-buttons {
    display: flex;
    gap: 4px;
  }

  .layout-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .layout-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .layout-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .layout-btn svg {
    opacity: 0.8;
  }

  .layout-btn.active svg {
    opacity: 1;
  }

  .zoom-controls {
    position: absolute;
    top: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    z-index: 10;
  }

  .zoom-btn {
    width: 36px;
    height: 36px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 18px;
    font-weight: 500;
    cursor: pointer;
    backdrop-filter: blur(8px);
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .zoom-btn:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .zoom-btn.zoom-level {
    font-size: 11px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    width: auto;
    padding: 0 8px;
  }

  .legend {
    position: absolute;
    bottom: 16px;
    left: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    font-size: 11px;
    backdrop-filter: blur(8px);
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 180px;
  }

  .legend-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .legend-title {
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-size: 10px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }

  .legend-item svg {
    flex-shrink: 0;
  }

  .voltage-scale {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .voltage-gradient {
    height: 8px;
    border-radius: 4px;
    background: linear-gradient(to right, #ef4444, #f59e0b, #22c55e, #22c55e, #f59e0b);
  }

  .voltage-labels {
    display: flex;
    justify-content: space-between;
    color: var(--text-muted);
    font-size: 10px;
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .loading-scale {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .loading-gradient {
    height: 8px;
    border-radius: 4px;
    background: linear-gradient(to right, #22c55e, #22c55e 50%, #f59e0b 80%, #ef4444);
  }

  .loading-labels {
    display: flex;
    justify-content: space-between;
    color: var(--text-muted);
    font-size: 10px;
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  :global(.grid-view svg) {
    display: block;
  }

  :global(.grid-view .nodes circle) {
    cursor: grab;
    transition: stroke-width 0.15s ease;
  }

  :global(.grid-view .nodes circle:hover) {
    stroke-width: 3px;
  }

  :global(.grid-view .nodes g:active circle) {
    cursor: grabbing;
  }

  /* Network Info HUD */
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

  .network-hud .solve-btn.n1 {
    background: #f59e0b; /* Amber/warning color for security analysis */
  }

  .network-hud .solve-btn.n1:hover:not(:disabled) {
    background: #d97706;
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

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* N-1 Toggle Button */
  .n1-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    margin-top: 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
    width: 100%;
    justify-content: space-between;
  }

  .n1-toggle:hover {
    background: var(--bg-secondary);
    border-color: var(--text-muted);
  }

  .n1-toggle svg {
    transition: transform 0.2s ease;
  }

  .n1-toggle svg.open {
    transform: rotate(180deg);
  }

  .n1-badge {
    font-weight: 600;
    font-size: 10px;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .n1-badge.secure {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .n1-badge.violations {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  /* N-1 Panel */
  .n1-panel {
    position: absolute;
    top: 70px;
    right: 16px;
    width: 320px;
    max-height: calc(100% - 180px);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    backdrop-filter: blur(12px);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideIn 0.2s ease-out;
    z-index: 20;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateX(20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .n1-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }

  .n1-panel-header h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .n1-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .n1-close:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  /* N-1 Summary Stats */
  .n1-summary {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }

  .n1-stat {
    text-align: center;
  }

  .n1-stat-value {
    display: block;
    font-size: 18px;
    font-weight: 600;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-primary);
  }

  .n1-stat-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .n1-stat.warning .n1-stat-value {
    color: #f59e0b;
  }

  .n1-stat.error .n1-stat-value {
    color: #ef4444;
  }

  /* Worst Contingency */
  .n1-worst {
    padding: 12px 14px;
    background: rgba(239, 68, 68, 0.1);
    border-bottom: 1px solid var(--border);
  }

  .n1-worst-title {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #ef4444;
    margin-bottom: 6px;
  }

  .n1-worst-branch {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: 'SF Mono', 'Fira Code', monospace;
    margin-bottom: 4px;
  }

  .n1-worst-loading {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .n1-worst-loading.critical {
    color: #ef4444;
    font-weight: 600;
  }

  .n1-overloaded-list {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid rgba(239, 68, 68, 0.2);
  }

  .n1-overloaded-item {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    padding: 2px 0;
  }

  .ob-branch {
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-muted);
  }

  .ob-loading {
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-secondary);
  }

  .ob-loading.critical {
    color: #ef4444;
  }

  /* N-1 List */
  .n1-list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }

  .n1-sort {
    display: flex;
    gap: 4px;
  }

  .n1-sort button {
    padding: 3px 8px;
    font-size: 10px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .n1-sort button:hover {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .n1-sort button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .n1-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .n1-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: var(--bg-tertiary);
    border-radius: 6px;
    margin-bottom: 4px;
    transition: all 0.15s ease;
  }

  .n1-item:hover {
    background: var(--bg-primary);
  }

  .n1-item.violation {
    border-left: 3px solid #f59e0b;
  }

  .n1-item.failed {
    border-left: 3px solid #ef4444;
    opacity: 0.8;
  }

  .n1-item-branch {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-secondary);
  }

  .n1-item-branch svg {
    color: var(--text-muted);
  }

  .n1-item-status {
    font-size: 11px;
    font-weight: 600;
  }

  .n1-island {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .n1-loading {
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: #f59e0b;
  }

  .n1-loading.critical {
    color: #ef4444;
  }

  .n1-more {
    text-align: center;
    font-size: 11px;
    color: var(--text-muted);
    padding: 8px;
    font-style: italic;
  }

  /* N-1 Secure state */
  .n1-secure {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px;
    text-align: center;
    color: #22c55e;
  }

  .n1-secure svg {
    margin-bottom: 12px;
    opacity: 0.8;
  }

  .n1-secure span:first-of-type {
    font-size: 14px;
    font-weight: 600;
  }

  .n1-secure-sub {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }
</style>
