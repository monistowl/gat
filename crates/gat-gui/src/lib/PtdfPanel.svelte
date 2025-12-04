<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  // Props
  let { isOpen, onClose }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  // State
  let networkPath = $state<string>('');
  let injectionBus = $state<number | null>(null);
  let withdrawalBus = $state<number | null>(null);
  let busOptions = $state<{ id: number; name: string }[]>([]);
  let results = $state<PtdfResult[] | null>(null);
  let computing = $state(false);
  let error = $state<string | null>(null);
  let computeTime = $state<number | null>(null);

  interface PtdfResult {
    branch_id: number;
    from_bus: number;
    to_bus: number;
    branch_name: string;
    ptdf_factor: number;
    flow_change_mw: number;
  }

  // Computed
  let isConfigured = $derived(networkPath && injectionBus !== null && withdrawalBus !== null && injectionBus !== withdrawalBus);
  let topResults = $derived(results?.slice(0, 15) || []);
  let significantCount = $derived(results?.filter(r => Math.abs(r.ptdf_factor) > 0.1).length || 0);

  // Load network and populate bus list
  async function loadNetwork() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Grid Files', extensions: ['m', 'arrow', 'json', 'raw'] }
        ],
        title: 'Select Network File',
      });
      if (selected) {
        networkPath = selected as string;

        // Load network to get bus list
        const network = await invoke<{
          buses: { id: number; name: string }[];
        }>('load_case', { path: networkPath });

        busOptions = network.buses.map(b => ({ id: b.id, name: b.name }));
        injectionBus = null;
        withdrawalBus = null;
        results = null;
      }
    } catch (e) {
      error = String(e);
    }
  }

  // Compute PTDF
  async function computePtdf() {
    if (!isConfigured) return;

    computing = true;
    error = null;

    try {
      const response = await invoke<{
        injection_bus: number;
        withdrawal_bus: number;
        transfer_mw: number;
        branches: PtdfResult[];
        compute_time_ms: number;
      }>('compute_ptdf', {
        request: {
          network_path: networkPath,
          injection_bus: injectionBus,
          withdrawal_bus: withdrawalBus,
        }
      });

      results = response.branches;
      computeTime = response.compute_time_ms;
    } catch (e) {
      error = String(e);
    } finally {
      computing = false;
    }
  }

  // Format PTDF for display
  function formatPtdf(value: number): string {
    if (Math.abs(value) < 0.001) return '~0';
    return (value * 100).toFixed(1) + '%';
  }

  // Get color class for PTDF value
  function getPtdfClass(value: number): string {
    if (value > 0.1) return 'positive-high';
    if (value > 0) return 'positive';
    if (value < -0.1) return 'negative-high';
    if (value < 0) return 'negative';
    return 'neutral';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="ptdf-overlay" class:open={isOpen} onclick={onClose} role="presentation"></div>

<aside class="ptdf-pane" class:open={isOpen}>
  <header>
    <h2>📊 PTDF Analysis</h2>
    <button class="close-btn" onclick={onClose} aria-label="Close">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </header>

  <div class="pane-content">
    <!-- Network Selection -->
    <section class="config-section">
      <h3>🔌 Network</h3>

      <button class="load-btn" onclick={loadNetwork}>
        {#if networkPath}
          <span class="path-text">{networkPath.split('/').pop()}</span>
          <span class="change-hint">Click to change</span>
        {:else}
          📂 Select Network File
        {/if}
      </button>
    </section>

    <!-- Transfer Configuration -->
    {#if busOptions.length > 0}
      <section class="config-section">
        <h3>🔄 Transfer</h3>

        <div class="bus-selector">
          <label>
            <span class="label-text">Injection Bus</span>
            <select bind:value={injectionBus}>
              <option value={null}>Select bus...</option>
              {#each busOptions as bus}
                <option value={bus.id} disabled={bus.id === withdrawalBus}>
                  {bus.id}: {bus.name}
                </option>
              {/each}
            </select>
          </label>

          <div class="arrow">→</div>

          <label>
            <span class="label-text">Withdrawal Bus</span>
            <select bind:value={withdrawalBus}>
              <option value={null}>Select bus...</option>
              {#each busOptions as bus}
                <option value={bus.id} disabled={bus.id === injectionBus}>
                  {bus.id}: {bus.name}
                </option>
              {/each}
            </select>
          </label>
        </div>

        <button
          class="compute-btn"
          disabled={!isConfigured || computing}
          onclick={computePtdf}
        >
          {#if computing}
            ⏳ Computing...
          {:else}
            ⚡ Compute PTDF
          {/if}
        </button>
      </section>
    {/if}

    <!-- Results -->
    {#if results}
      <section class="results-section">
        <h3>📈 Results</h3>

        <div class="results-summary">
          <div class="stat">
            <span class="stat-value">{results.length}</span>
            <span class="stat-label">Branches</span>
          </div>
          <div class="stat">
            <span class="stat-value">{significantCount}</span>
            <span class="stat-label">Significant (>10%)</span>
          </div>
          <div class="stat">
            <span class="stat-value">{computeTime?.toFixed(1)}ms</span>
            <span class="stat-label">Compute Time</span>
          </div>
        </div>

        <div class="results-table">
          <table>
            <thead>
              <tr>
                <th>Branch</th>
                <th>From → To</th>
                <th>PTDF</th>
                <th>ΔFlow (100MW)</th>
              </tr>
            </thead>
            <tbody>
              {#each topResults as row}
                <tr class={getPtdfClass(row.ptdf_factor)}>
                  <td class="branch-name">{row.branch_name || `Br ${row.branch_id}`}</td>
                  <td class="branch-buses">{row.from_bus} → {row.to_bus}</td>
                  <td class="ptdf-value">{formatPtdf(row.ptdf_factor)}</td>
                  <td class="flow-change">{row.flow_change_mw.toFixed(1)} MW</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if results.length > 15}
            <div class="table-footer">Showing top 15 of {results.length} branches</div>
          {/if}
        </div>
      </section>
    {/if}

    {#if error}
      <section class="error-section">
        <p class="error-message">{error}</p>
      </section>
    {/if}
  </div>
</aside>

<style>
  .ptdf-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.3s ease, visibility 0.3s ease;
    z-index: 99;
  }

  .ptdf-overlay.open {
    opacity: 1;
    visibility: visible;
  }

  .ptdf-pane {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 420px;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform 0.3s ease;
    z-index: 100;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 20px rgba(0, 0, 0, 0.3);
  }

  .ptdf-pane.open {
    transform: translateX(0);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-muted);
    border-radius: 4px;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .pane-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  section {
    margin-bottom: 24px;
  }

  section h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 14px;
  }

  .load-btn {
    width: 100%;
    padding: 16px;
    background: var(--bg-secondary);
    border: 2px dashed var(--border);
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    transition: all 0.2s ease;
    color: var(--text-secondary);
  }

  .load-btn:hover {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .path-text {
    font-family: 'SF Mono', monospace;
    color: var(--text-primary);
    font-size: 13px;
  }

  .change-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .bus-selector {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    margin-bottom: 16px;
  }

  .bus-selector label {
    flex: 1;
  }

  .label-text {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .bus-selector select {
    width: 100%;
    padding: 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
  }

  .arrow {
    padding-bottom: 10px;
    color: var(--text-muted);
    font-size: 18px;
  }

  .compute-btn {
    width: 100%;
    padding: 12px;
    background: var(--accent);
    border: none;
    border-radius: 6px;
    color: white;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  .compute-btn:hover:not(:disabled) {
    background: #0052cc;
  }

  .compute-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .results-summary {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
  }

  .stat {
    flex: 1;
    text-align: center;
    padding: 10px;
    background: var(--bg-secondary);
    border-radius: 6px;
  }

  .stat-value {
    display: block;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .stat-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .results-table {
    max-height: 400px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .results-table table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .results-table th, .results-table td {
    padding: 8px 10px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .results-table th {
    background: var(--bg-secondary);
    font-weight: 600;
    color: var(--text-secondary);
    position: sticky;
    top: 0;
  }

  .branch-name {
    font-family: 'SF Mono', monospace;
    color: var(--text-primary);
  }

  .branch-buses {
    color: var(--text-muted);
  }

  .ptdf-value, .flow-change {
    font-family: 'SF Mono', monospace;
    text-align: right;
  }

  tr.positive-high { background: rgba(34, 197, 94, 0.15); }
  tr.positive-high .ptdf-value, tr.positive-high .flow-change { color: #22c55e; }

  tr.negative-high { background: rgba(239, 68, 68, 0.15); }
  tr.negative-high .ptdf-value, tr.negative-high .flow-change { color: #ef4444; }

  tr.positive .ptdf-value, tr.positive .flow-change { color: #86efac; }
  tr.negative .ptdf-value, tr.negative .flow-change { color: #fca5a5; }
  tr.neutral .ptdf-value { color: var(--text-muted); }

  .table-footer {
    padding: 8px;
    text-align: center;
    color: var(--text-muted);
    font-size: 10px;
    background: var(--bg-secondary);
  }

  .error-section {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid #ef4444;
    border-radius: 8px;
    padding: 12px;
  }

  .error-message {
    color: #ef4444;
    font-size: 12px;
    margin: 0;
  }
</style>
