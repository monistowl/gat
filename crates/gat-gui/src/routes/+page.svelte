<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import GridView from "$lib/GridView.svelte";
  import YbusExplorer from "$lib/YbusExplorer.svelte";
  import ArchitectureDiagram from "$lib/ArchitectureDiagram.svelte";
  import EducationDrawer from "$lib/EducationDrawer.svelte";
  import ConfigPane from "$lib/ConfigPane.svelte";
  import CommandBuilder from "$lib/CommandBuilder.svelte";
  import BatchJobPane from "$lib/BatchJobPane.svelte";
  import NotebookPane from "$lib/NotebookPane.svelte";
  import PtdfPanel from "$lib/PtdfPanel.svelte";
  import { themeState } from "$lib/stores/theme.svelte";

  // Types for our data
  interface CaseInfo {
    name: string;
    path: string;
    buses: number | null;
  }

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

  interface NetworkJson {
    name: string;
    buses: BusJson[];
    branches: BranchJson[];
    generators: GeneratorJson[];
    base_mva: number;
  }

  interface PowerFlowResult {
    buses: BusJson[];
    branches: BranchJson[];
    converged: boolean;
    iterations: number;
    max_mismatch: number;
    solve_time_ms: number;
  }

  interface DcPowerFlowResult {
    buses: BusJson[];
    branches: BranchJson[];
    converged: boolean;
    solve_time_ms: number;
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

  // State
  let cases = $state<CaseInfo[]>([]);
  let selectedCase = $state<CaseInfo | null>(null);
  let recentCases = $state<CaseInfo[]>([]);

  // Recent cases storage
  const RECENT_CASES_KEY = 'gat-recent-cases';
  const MAX_RECENT_CASES = 5;

  function loadRecentCases(): CaseInfo[] {
    try {
      const stored = localStorage.getItem(RECENT_CASES_KEY);
      if (stored) {
        return JSON.parse(stored) as CaseInfo[];
      }
    } catch (e) {
      console.warn('Failed to load recent cases:', e);
    }
    return [];
  }

  function saveRecentCase(caseInfo: CaseInfo) {
    // Remove if already in list, then add to front
    const filtered = recentCases.filter(c => c.path !== caseInfo.path);
    recentCases = [caseInfo, ...filtered].slice(0, MAX_RECENT_CASES);
    try {
      localStorage.setItem(RECENT_CASES_KEY, JSON.stringify(recentCases));
    } catch (e) {
      console.warn('Failed to save recent cases:', e);
    }
  }
  let network = $state<NetworkJson | null>(null);
  let pfResult = $state<PowerFlowResult | null>(null);
  let dcResult = $state<DcPowerFlowResult | null>(null);
  let n1Result = $state<N1ContingencyResult | null>(null);
  let activeView = $state<'grid' | 'ybus' | 'arch'>('grid');
  let status = $state<string>('Ready');
  let loading = $state<boolean>(false);
  let solving = $state<boolean>(false);
  let solvingDc = $state<boolean>(false);
  let runningN1 = $state<boolean>(false);
  let selectedBusId = $state<number | null>(null);
  let drawerOpen = $state<boolean>(false);
  let configOpen = $state<boolean>(false);
  let commandBuilderOpen = $state<boolean>(false);
  let batchJobOpen = $state<boolean>(false);
  let notebookOpen = $state<boolean>(false);
  let ptdfOpen = $state<boolean>(false);

  // Handle bus selection from Y-bus explorer
  function handleSelectBus(busId: number) {
    selectedBusId = busId;
    // Switch to grid view to see the selected bus
    activeView = 'grid';
    status = `Selected bus ${busId}`;
  }

  // Hero case: load and auto-solve
  async function runHeroCase(caseInfo: CaseInfo) {
    await loadCase(caseInfo);
    // Small delay for visual feedback before solving
    setTimeout(() => {
      solvePowerFlow();
    }, 500);
  }

  // Load cases on mount
  onMount(async () => {
    // Load recent cases from localStorage
    recentCases = loadRecentCases();

    try {
      cases = await invoke<CaseInfo[]>("list_cases");
      status = `Loaded ${cases.length} cases`;
    } catch (e) {
      status = `Error: ${e}`;
    }
  });

  // Load selected case
  async function loadCase(caseInfo: CaseInfo) {
    selectedCase = caseInfo;
    loading = true;
    pfResult = null;
    dcResult = null;
    n1Result = null;
    status = `Loading ${caseInfo.name}...`;

    try {
      const start = performance.now();
      network = await invoke<NetworkJson>("load_case", { path: caseInfo.path });
      const elapsed = Math.round(performance.now() - start);
      status = `Loaded ${network.buses.length} buses, ${network.branches.length} branches in ${elapsed}ms`;

      // Save to recent cases on successful load
      saveRecentCase(caseInfo);
    } catch (e) {
      status = `Error: ${e}`;
      network = null;
    } finally {
      loading = false;
    }
  }

  // Solve power flow
  async function solvePowerFlow() {
    if (!selectedCase) return;

    solving = true;
    status = `Solving power flow...`;

    try {
      pfResult = await invoke<PowerFlowResult>("solve_power_flow", { path: selectedCase.path });

      // Update network with solved voltages
      if (network && pfResult) {
        const busMap = new Map(pfResult.buses.map(b => [b.id, b]));
        network = {
          ...network,
          buses: network.buses.map(bus => {
            const solved = busMap.get(bus.id);
            return solved ? { ...bus, vm: solved.vm, va: solved.va, type: solved.type } : bus;
          }),
        };
      }

      status = pfResult.converged
        ? `AC: Converged in ${pfResult.iterations} iter, ${pfResult.solve_time_ms.toFixed(1)}ms`
        : `AC: Did not converge (${pfResult.iterations} iter)`;
    } catch (e) {
      status = `AC solve error: ${e}`;
      pfResult = null;
    } finally {
      solving = false;
    }
  }

  // Solve DC power flow (linearized approximation)
  async function solveDcPowerFlow() {
    if (!selectedCase) return;

    solvingDc = true;
    status = `Solving DC power flow...`;

    try {
      dcResult = await invoke<DcPowerFlowResult>("solve_dc_power_flow", { path: selectedCase.path });

      // Update network with solved angles (DC assumes flat voltage)
      if (network && dcResult) {
        const busMap = new Map(dcResult.buses.map(b => [b.id, b]));
        const branchMap = new Map(dcResult.branches.map(br => [`${br.from}-${br.to}`, br]));

        network = {
          ...network,
          buses: network.buses.map(bus => {
            const solved = busMap.get(bus.id);
            return solved ? { ...bus, vm: solved.vm, va: solved.va } : bus;
          }),
          branches: network.branches.map(br => {
            const solved = branchMap.get(`${br.from}-${br.to}`);
            return solved ? { ...br, p_flow: solved.p_flow, loading_pct: solved.loading_pct } : br;
          }),
        };
      }

      status = `DC: Solved in ${dcResult.solve_time_ms.toFixed(2)}ms (flat voltage)`;
    } catch (e) {
      status = `DC solve error: ${e}`;
      dcResult = null;
    } finally {
      solvingDc = false;
    }
  }

  // Run N-1 contingency analysis
  async function runN1Contingency() {
    if (!selectedCase) return;

    runningN1 = true;
    status = `Running N-1 security analysis...`;

    try {
      n1Result = await invoke<N1ContingencyResult>("run_n1_contingency", { path: selectedCase.path });

      const violationText = n1Result.contingencies_with_violations > 0
        ? `${n1Result.contingencies_with_violations} violations`
        : 'SECURE';
      const failedText = n1Result.contingencies_failed > 0
        ? `, ${n1Result.contingencies_failed} islands`
        : '';
      status = `N-1: ${n1Result.total_contingencies} contingencies, ${violationText}${failedText} (${n1Result.solve_time_ms.toFixed(1)}ms)`;
    } catch (e) {
      status = `N-1 error: ${e}`;
      n1Result = null;
    } finally {
      runningN1 = false;
    }
  }

  // Format bus count for display
  function formatBusCount(buses: number | null): string {
    if (buses === null) return '?';
    if (buses >= 1000) return `${(buses / 1000).toFixed(1)}k`;
    return buses.toString();
  }

  // Extract short name from full case name
  function shortName(name: string): string {
    return name.replace('pglib_opf_', '').replace(/_/g, ' ');
  }
</script>

<div class="shell">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1 class="logo">GAT</h1>
    </div>

    <div class="sidebar-section">
      <h2 class="section-title">Cases</h2>
      <div class="case-list">
        {#each cases.slice(0, 15) as caseInfo}
          <button
            class="case-item"
            class:selected={selectedCase?.name === caseInfo.name}
            onclick={() => loadCase(caseInfo)}
          >
            <span class="case-name">{shortName(caseInfo.name)}</span>
            <span class="case-buses">{formatBusCount(caseInfo.buses)}</span>
          </button>
        {/each}
      </div>
    </div>

    {#if recentCases.length > 0}
      <div class="sidebar-section recent-section">
        <h2 class="section-title">Recent</h2>
        <div class="case-list">
          {#each recentCases as caseInfo}
            <button
              class="case-item recent"
              class:selected={selectedCase?.name === caseInfo.name}
              onclick={() => loadCase(caseInfo)}
            >
              <span class="recent-icon">↻</span>
              <span class="case-name">{shortName(caseInfo.name)}</span>
              <span class="case-buses">{formatBusCount(caseInfo.buses)}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="sidebar-section">
      <h2 class="section-title">Hero Cases</h2>
      <p class="section-hint">Auto-load & solve</p>
      <div class="case-list">
        {#each cases.filter(c => c.buses && [14, 118, 9241].includes(c.buses)) as caseInfo}
          <button
            class="case-item hero"
            class:selected={selectedCase?.name === caseInfo.name}
            onclick={() => runHeroCase(caseInfo)}
            disabled={loading || solving}
          >
            <span class="play-icon">▶</span>
            <span class="case-name">{shortName(caseInfo.name)}</span>
            <span class="case-buses">{formatBusCount(caseInfo.buses)}</span>
          </button>
        {/each}
      </div>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="main">
    <div class="content">
      {#if activeView === 'arch'}
        <!-- Architecture view is always accessible -->
        <div class="arch-container full view-transition">
          <ArchitectureDiagram />
        </div>
      {:else if !network}
        <div class="placeholder">
          <div class="placeholder-icon">⚡</div>
          <h2>Select a case to visualize</h2>
          <p>Choose from the sidebar to load a power grid network</p>
        </div>
      {:else}
        {#if activeView === 'grid'}
          <div class="view-container view-transition">
            <GridView
              {network}
              {selectedBusId}
              onSolveAc={solvePowerFlow}
              onSolveDc={solveDcPowerFlow}
              onRunN1={runN1Contingency}
              solvingAc={solving}
              {solvingDc}
              {runningN1}
              {n1Result}
            />
          </div>
        {:else if activeView === 'ybus'}
          <div class="view-container view-transition">
            <YbusExplorer
              casePath={selectedCase?.path ?? null}
              onSelectBus={handleSelectBus}
              {network}
              onSolveAc={solvePowerFlow}
              onSolveDc={solveDcPowerFlow}
              solvingAc={solving}
              {solvingDc}
            />
          </div>
        {/if}
      {/if}
    </div>

    <!-- View Tabs + Status Bar -->
    <div class="footer">
      <div class="view-tabs">
        <button
          class="tab"
          class:active={activeView === 'grid'}
          onclick={() => activeView = 'grid'}
        >Grid</button>
        <button
          class="tab"
          class:active={activeView === 'ybus'}
          onclick={() => activeView = 'ybus'}
        >Y-bus</button>
      </div>

      <div class="footer-btns">
        <button class="footer-btn" onclick={() => commandBuilderOpen = !commandBuilderOpen} title="CLI Command Builder">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="4 17 10 11 4 5"/>
            <line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
          CLI
        </button>
        <button class="footer-btn" onclick={() => batchJobOpen = !batchJobOpen} title="Batch Job Runner">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
            <polyline points="7.5 4.21 12 6.81 16.5 4.21"/>
            <polyline points="7.5 19.79 7.5 14.6 3 12"/>
            <polyline points="21 12 16.5 14.6 16.5 19.79"/>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
            <line x1="12" y1="22.08" x2="12" y2="12"/>
          </svg>
          Batch
        </button>
        <button class="footer-btn" onclick={() => notebookOpen = !notebookOpen} title="Research Notebook">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
          </svg>
          Notebook
        </button>
        <button class="footer-btn" onclick={() => ptdfOpen = !ptdfOpen} title="PTDF Analysis">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 3v18h18"/>
            <path d="M18 9l-5 5-4-4-3 3"/>
          </svg>
          PTDF
        </button>
        <button
          class="footer-btn theme-toggle"
          onclick={() => themeState.toggle()}
          title="Theme: {themeState.preference} ({themeState.resolved})"
        >
          {#if themeState.preference === 'system'}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
              <line x1="8" y1="21" x2="16" y2="21"/>
              <line x1="12" y1="17" x2="12" y2="21"/>
            </svg>
          {:else if themeState.resolved === 'dark'}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
            </svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="5"/>
              <line x1="12" y1="1" x2="12" y2="3"/>
              <line x1="12" y1="21" x2="12" y2="23"/>
              <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
              <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
              <line x1="1" y1="12" x2="3" y2="12"/>
              <line x1="21" y1="12" x2="23" y2="12"/>
              <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
              <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
            </svg>
          {/if}
          {#if themeState.preference === 'system'}
            Auto
          {:else if themeState.resolved === 'dark'}
            Dark
          {:else}
            Light
          {/if}
        </button>
      </div>

      <div class="footer-right">
        <button
          class="tab"
          class:active={activeView === 'arch'}
          onclick={() => activeView = 'arch'}
        >Architecture</button>
        <button class="icon-btn" onclick={() => drawerOpen = !drawerOpen} title="Learn">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
            <path d="M12 17h.01"/>
          </svg>
        </button>
        <button class="icon-btn" onclick={() => configOpen = !configOpen} title="Settings">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
        <div class="status">
          {#if loading}
            <span class="loading-indicator"></span>
          {/if}
          <span>{status}</span>
          {#if network && !loading}
            <span class="status-ok">✓</span>
          {/if}
        </div>
      </div>
    </div>
  </main>

  <!-- Educational Content Drawer -->
  <EducationDrawer isOpen={drawerOpen} {activeView} onClose={() => drawerOpen = false} />

  <!-- Configuration Pane -->
  <ConfigPane isOpen={configOpen} onClose={() => configOpen = false} />

  <!-- CLI Command Builder -->
  <CommandBuilder isOpen={commandBuilderOpen} onClose={() => commandBuilderOpen = false} />

  <!-- Batch Job Runner -->
  <BatchJobPane isOpen={batchJobOpen} onClose={() => batchJobOpen = false} />

  <!-- Research Notebook -->
  <NotebookPane bind:isOpen={notebookOpen} />

  <!-- PTDF Analysis -->
  <PtdfPanel isOpen={ptdfOpen} onClose={() => ptdfOpen = false} />
</div>

<style>
  .shell {
    display: flex;
    height: 100%;
    background: var(--bg-primary);
  }

  /* Sidebar */
  .sidebar {
    width: 220px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .logo {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -1px;
  }

  .sidebar-section {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 0 16px 4px;
  }

  .section-hint {
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.7;
    padding: 0 16px 8px;
    font-style: italic;
  }

  .case-list {
    max-height: 300px;
    overflow-y: auto;
  }

  .case-item {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    transition: all 0.15s ease;
  }

  .case-item:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .case-item.selected {
    background: var(--accent);
    color: white;
  }

  .case-item.hero {
    color: var(--accent);
  }

  .case-item.hero:hover {
    color: var(--text-primary);
  }

  .case-buses {
    font-size: 11px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .case-item.selected .case-buses {
    color: rgba(255, 255, 255, 0.7);
  }

  .play-icon {
    font-size: 10px;
    margin-right: 6px;
  }

  .recent-icon {
    font-size: 12px;
    margin-right: 4px;
    opacity: 0.6;
  }

  .case-item.recent {
    color: var(--text-muted);
    font-size: 12px;
    padding: 6px 16px;
  }

  .case-item.recent:hover {
    color: var(--text-primary);
  }

  .case-item.recent .recent-icon {
    opacity: 1;
  }

  .recent-section {
    background: var(--bg-tertiary);
  }

  /* Main Content */
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    /* Minimal padding - views will fill edge to edge */
    padding: 0;
  }

  .placeholder {
    text-align: center;
    color: var(--text-muted);
  }

  .placeholder-icon {
    font-size: 64px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .placeholder h2 {
    color: var(--text-secondary);
    font-weight: 500;
    margin-bottom: 8px;
  }

  /* Full-bleed view container - fills all available space */
  .view-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .arch-container {
    width: 100%;
    height: 100%;
    max-width: 1400px;
    padding: 24px;
    overflow: hidden;
  }

  .arch-container.full {
    height: 100%;
  }

  /* View transitions */
  .view-transition {
    animation: fadeSlideIn 0.25s ease-out;
  }

  @keyframes fadeSlideIn {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Footer */
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    height: 40px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
  }

  .view-tabs {
    display: flex;
    gap: 4px;
  }

  .tab {
    padding: 6px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .tab:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--accent);
    color: white;
  }

  .footer-btns {
    display: flex;
    gap: 8px;
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .footer-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .footer-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .footer-btn svg {
    opacity: 0.7;
  }

  .footer-btn:hover svg {
    opacity: 1;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .status-ok {
    color: var(--success);
  }

  .loading-indicator {
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Scrollbar styling */
  :global(::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }

  :global(::-webkit-scrollbar-track) {
    background: var(--bg-secondary);
  }

  :global(::-webkit-scrollbar-thumb) {
    background: var(--border);
    border-radius: 4px;
  }

  :global(::-webkit-scrollbar-thumb:hover) {
    background: var(--text-muted);
  }
</style>
