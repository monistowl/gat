<script lang="ts">
  import type { Snippet } from "svelte";

  interface CaseInfo {
    name: string;
    path: string;
    buses: number | null;
  }

  interface Props {
    /** List of available cases */
    cases: CaseInfo[];
    /** Currently selected case */
    selectedCase: CaseInfo | null;
    /** Recently accessed cases */
    recentCases?: CaseInfo[];
    /** Whether a case is currently loading */
    loading?: boolean;
    /** Whether a solver is running */
    solving?: boolean;
    /** Callback when a case is selected */
    onSelectCase: (caseInfo: CaseInfo) => void;
    /** Callback for hero case (auto-solve) */
    onHeroCase?: (caseInfo: CaseInfo) => void;
    /** Optional header slot */
    header?: Snippet;
  }

  let {
    cases,
    selectedCase,
    recentCases = [],
    loading = false,
    solving = false,
    onSelectCase,
    onHeroCase,
    header,
  }: Props = $props();

  // Hero case bus counts
  const HERO_BUS_COUNTS = [14, 118, 9241];

  // Derived: hero cases filtered from main list
  let heroCases = $derived(cases.filter(c => c.buses && HERO_BUS_COUNTS.includes(c.buses)));

  // Format bus count for display
  function formatBusCount(buses: number | null): string {
    if (buses === null) return "?";
    if (buses >= 1000) return `${(buses / 1000).toFixed(1)}k`;
    return buses.toString();
  }

  // Extract short name from full case name
  function shortName(name: string): string {
    return name.replace("pglib_opf_", "").replace(/_/g, " ");
  }
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    {#if header}
      {@render header()}
    {:else}
      <h1 class="logo">GAT</h1>
    {/if}
  </div>

  <div class="sidebar-section">
    <h2 class="section-title">Cases</h2>
    <div class="case-list">
      {#each cases.slice(0, 15) as caseInfo}
        <button
          class="case-item"
          class:selected={selectedCase?.name === caseInfo.name}
          onclick={() => onSelectCase(caseInfo)}
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
            onclick={() => onSelectCase(caseInfo)}
          >
            <span class="recent-icon">↻</span>
            <span class="case-name">{shortName(caseInfo.name)}</span>
            <span class="case-buses">{formatBusCount(caseInfo.buses)}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if onHeroCase && heroCases.length > 0}
    <div class="sidebar-section">
      <h2 class="section-title">Hero Cases</h2>
      <p class="section-hint">Auto-load & solve</p>
      <div class="case-list">
        {#each heroCases as caseInfo}
          <button
            class="case-item hero"
            class:selected={selectedCase?.name === caseInfo.name}
            onclick={() => onHeroCase(caseInfo)}
            disabled={loading || solving}
          >
            <span class="play-icon">▶</span>
            <span class="case-name">{shortName(caseInfo.name)}</span>
            <span class="case-buses">{formatBusCount(caseInfo.buses)}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="sidebar-footer">
    <span class="version">v0.5.4</span>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width, 220px);
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: var(--space-4);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .logo {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -1px;
  }

  .sidebar-section {
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--border);
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 0 var(--space-4) var(--space-1);
  }

  .section-hint {
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.7;
    padding: 0 var(--space-4) var(--space-2);
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
    padding: var(--space-2) var(--space-4);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    transition: all var(--transition-base);
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

  .case-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .case-buses {
    font-size: 11px;
    color: var(--text-muted);
    font-family: "SF Mono", "Fira Code", monospace;
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
    padding: 6px var(--space-4);
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

  .sidebar-footer {
    margin-top: auto;
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--border);
  }

  .version {
    font-size: 11px;
    color: var(--text-muted);
    font-family: "SF Mono", "Fira Code", monospace;
  }
</style>
