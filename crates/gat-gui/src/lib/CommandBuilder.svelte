<script lang="ts">
  // Types
  interface CommandParam {
    id: string;
    label: string;
    type: 'text' | 'number' | 'select' | 'checkbox';
    required?: boolean;
    flag?: string;
    default?: string;
    options?: string[];
    help?: string;
  }

  interface Command {
    name: string;
    icon: string;
    description: string;
    template: string;
    params: CommandParam[];
  }

  // Props
  let { isOpen, onClose }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  // Command definitions (ported from website)
  const COMMANDS: Record<string, Command> = {
    pf_dc: {
      name: "DC Power Flow",
      icon: "⚡",
      description: "Fast linearized power flow analysis",
      template: "gat pf dc",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow", help: "Arrow/Parquet file with network data" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "flows.parquet", help: "Where to save results" },
        { id: "solver", label: "Solver", type: "select", flag: "--solver", options: ["gauss", "faer"], help: "Linear solver backend" },
        { id: "threads", label: "Threads", type: "number", flag: "--threads", help: "Number of parallel threads (auto by default)" }
      ]
    },
    pf_ac: {
      name: "AC Power Flow",
      icon: "⚡",
      description: "Full nonlinear AC power flow with Newton-Raphson",
      template: "gat pf ac",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "flows.parquet" },
        { id: "tol", label: "Tolerance", type: "number", flag: "--tol", default: "1e-6", help: "Convergence tolerance" },
        { id: "max-iter", label: "Max Iterations", type: "number", flag: "--max-iter", default: "20", help: "Maximum Newton iterations" },
        { id: "solver", label: "Solver", type: "select", flag: "--solver", options: ["gauss", "faer"] }
      ]
    },
    opf_dc: {
      name: "DC Optimal Power Flow",
      icon: "🎯",
      description: "Economic dispatch with linearized power flow (LP)",
      template: "gat opf dc",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow" },
        { id: "cost", label: "Cost File", type: "text", required: true, flag: "--cost", default: "costs.csv", help: "CSV with gen_id,cost_per_mw" },
        { id: "limits", label: "Limits File", type: "text", flag: "--limits", default: "limits.csv", help: "CSV with gen_id,p_min,p_max" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "dispatch.parquet" },
        { id: "solver", label: "Solver", type: "select", flag: "--solver", options: ["clarabel", "highs", "cbc"] }
      ]
    },
    opf_socp: {
      name: "SOCP Relaxation OPF",
      icon: "🔮",
      description: "Second-order cone relaxation for radial networks",
      template: "gat opf socp",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow" },
        { id: "cost", label: "Cost File", type: "text", required: true, flag: "--cost", default: "costs.csv" },
        { id: "limits", label: "Limits File", type: "text", flag: "--limits", default: "limits.csv" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "dispatch.parquet" },
        { id: "tol", label: "Tolerance", type: "number", flag: "--tol", default: "1e-8", help: "Solver tolerance" },
        { id: "max-iter", label: "Max Iterations", type: "number", flag: "--max-iter", default: "100" }
      ]
    },
    opf_ac: {
      name: "AC Optimal Power Flow",
      icon: "🎯",
      description: "Full nonlinear AC OPF with L-BFGS solver",
      template: "gat opf ac",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow" },
        { id: "cost", label: "Cost File", type: "text", required: true, flag: "--cost", default: "costs.csv" },
        { id: "limits", label: "Limits File", type: "text", flag: "--limits", default: "limits.csv" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "dispatch.parquet" },
        { id: "tol", label: "Tolerance", type: "number", flag: "--tol", default: "1e-6" },
        { id: "max-iter", label: "Max Iterations", type: "number", flag: "--max-iter", default: "100" }
      ]
    },
    contingency: {
      name: "N-1 Contingency",
      icon: "🔍",
      description: "Screen system for reliability violations",
      template: "gat contingency n1",
      params: [
        { id: "input", label: "Input Grid File", type: "text", required: true, default: "grid.arrow" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "violations.parquet" },
        { id: "parallel", label: "Parallel Execution", type: "checkbox", flag: "--parallel", help: "Run contingencies in parallel" },
        { id: "limit-violations", label: "Violation Threshold", type: "number", flag: "--limit-violations", default: "0.95", help: "Flag violations above this p.u." }
      ]
    },
    ts_resample: {
      name: "Time Series Resample",
      icon: "📈",
      description: "Resample time series data to different frequency",
      template: "gat ts resample",
      params: [
        { id: "input", label: "Input Time Series", type: "text", required: true, default: "timeseries.parquet" },
        { id: "freq", label: "Frequency", type: "select", required: true, flag: "--freq", options: ["1min", "5min", "15min", "1h", "1d"], help: "Target sampling frequency" },
        { id: "agg", label: "Aggregation", type: "select", required: true, flag: "--agg", options: ["mean", "sum", "min", "max", "first", "last"] },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "resampled.parquet" }
      ]
    },
    batch_pf: {
      name: "Batch Power Flow",
      icon: "🔄",
      description: "Run power flow on multiple scenarios",
      template: "gat batch pf",
      params: [
        { id: "manifest", label: "Manifest File", type: "text", required: true, flag: "--manifest", default: "scenarios.json", help: "JSON with scenario list" },
        { id: "out", label: "Output Directory", type: "text", required: true, flag: "--out", default: "results/" },
        { id: "parallel", label: "Parallel Jobs", type: "number", flag: "--parallel", default: "4", help: "Number of concurrent scenarios" }
      ]
    },
    derms_aggregate: {
      name: "DER Aggregation",
      icon: "🔋",
      description: "Aggregate distributed energy resources",
      template: "gat derms aggregate",
      params: [
        { id: "input", label: "DER Data File", type: "text", required: true, default: "ders.parquet" },
        { id: "method", label: "Method", type: "select", required: true, flag: "--method", options: ["envelope", "centroid"], help: "Aggregation methodology" },
        { id: "out", label: "Output File", type: "text", required: true, flag: "--out", default: "aggregated.parquet" }
      ]
    }
  };

  // State
  let selectedCommand = $state<string | null>(null);
  let paramValues = $state<Record<string, string | boolean>>({});
  let copyMessage = $state<string | null>(null);
  let optionalExpanded = $state(true);

  // Computed: current command
  let currentCmd = $derived(selectedCommand ? COMMANDS[selectedCommand] : null);

  // Computed: built command string
  let commandString = $derived.by(() => {
    if (!currentCmd || !selectedCommand) return '';

    let parts = [currentCmd.template];
    let flagParts: string[] = [];

    currentCmd.params.forEach(param => {
      const value = paramValues[param.id];

      if (param.type === 'checkbox') {
        if (value === true && param.flag) {
          flagParts.push(param.flag);
        }
      } else {
        const strValue = String(value || '').trim();
        if (strValue) {
          if (param.flag) {
            flagParts.push(`${param.flag} ${strValue}`);
          } else {
            parts.push(strValue);
          }
        }
      }
    });

    return [...parts, ...flagParts].join(' \\\n    ');
  });

  // Computed: single-line command for copying
  let commandOneLine = $derived(commandString.replace(/\s*\\\n\s*/g, ' '));

  // Initialize param values when command changes
  $effect(() => {
    if (currentCmd) {
      const values: Record<string, string | boolean> = {};
      currentCmd.params.forEach(param => {
        if (param.type === 'checkbox') {
          values[param.id] = false;
        } else {
          values[param.id] = param.default || '';
        }
      });
      paramValues = values;
    }
  });

  function selectCommand(cmdKey: string) {
    selectedCommand = cmdKey;
  }

  function goBack() {
    selectedCommand = null;
  }

  async function copyCommand(multiline: boolean = false) {
    const text = multiline ? commandString : commandOneLine;
    await navigator.clipboard.writeText(text);
    copyMessage = multiline ? 'Multi-line copied!' : 'Copied!';
    setTimeout(() => copyMessage = null, 2000);
  }

  // Required/optional params
  let requiredParams = $derived(currentCmd?.params.filter(p => p.required) || []);
  let optionalParams = $derived(currentCmd?.params.filter(p => !p.required) || []);

  // Close on escape
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      if (selectedCommand) {
        goBack();
      } else {
        onClose();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="command-overlay" class:open={isOpen} onclick={onClose} role="presentation"></div>

<aside class="command-pane" class:open={isOpen}>
  <header>
    <div class="header-left">
      {#if selectedCommand}
        <button class="back-btn" onclick={goBack} aria-label="Go back">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7"/>
          </svg>
        </button>
      {/if}
      <h2>
        {#if selectedCommand && currentCmd}
          {currentCmd.icon} {currentCmd.name}
        {:else}
          Command Builder
        {/if}
      </h2>
    </div>
    <button class="close-btn" onclick={onClose} aria-label="Close">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </header>

  <div class="pane-content">
    {#if !selectedCommand}
      <!-- Command Selection Grid -->
      <div class="command-intro">
        <p>Build GAT CLI commands visually. Select an analysis type to configure.</p>
      </div>

      <div class="command-grid">
        {#each Object.entries(COMMANDS) as [key, cmd]}
          <button class="command-card" onclick={() => selectCommand(key)}>
            <span class="card-icon">{cmd.icon}</span>
            <span class="card-name">{cmd.name}</span>
            <span class="card-desc">{cmd.description}</span>
          </button>
        {/each}
      </div>
    {:else if currentCmd}
      <!-- Command Builder Form -->
      <p class="cmd-description">{currentCmd.description}</p>

      <!-- Required Parameters -->
      {#if requiredParams.length > 0}
        <section class="param-section">
          <h3><span class="required-marker">*</span> Required Parameters</h3>
          <div class="param-list">
            {#each requiredParams as param}
              <div class="param-field">
                <label for={param.id}>
                  {param.label}
                  <span class="required-marker">*</span>
                </label>

                {#if param.type === 'text' || param.type === 'number'}
                  <input
                    type={param.type}
                    id={param.id}
                    bind:value={paramValues[param.id]}
                    placeholder={param.default || ''}
                  />
                {:else if param.type === 'select'}
                  <select id={param.id} bind:value={paramValues[param.id]}>
                    <option value="">-- Select --</option>
                    {#each param.options || [] as opt}
                      <option value={opt}>{opt}</option>
                    {/each}
                  </select>
                {:else if param.type === 'checkbox'}
                  <label class="checkbox-field">
                    <input
                      type="checkbox"
                      checked={paramValues[param.id] === true}
                      onchange={(e) => paramValues[param.id] = e.currentTarget.checked}
                    />
                    <span>{param.label}</span>
                  </label>
                {/if}

                {#if param.help}
                  <p class="param-help">{param.help}</p>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Optional Parameters -->
      {#if optionalParams.length > 0}
        <section class="param-section">
          <button class="section-toggle" onclick={() => optionalExpanded = !optionalExpanded}>
            <h3>Optional Parameters</h3>
            <span class="toggle-icon">{optionalExpanded ? '▼' : '▶'}</span>
          </button>

          {#if optionalExpanded}
            <div class="param-list">
              {#each optionalParams as param}
                <div class="param-field">
                  <label for={param.id}>{param.label}</label>

                  {#if param.type === 'text' || param.type === 'number'}
                    <input
                      type={param.type}
                      id={param.id}
                      bind:value={paramValues[param.id]}
                      placeholder={param.default || ''}
                    />
                  {:else if param.type === 'select'}
                    <select id={param.id} bind:value={paramValues[param.id]}>
                      <option value="">-- Select --</option>
                      {#each param.options || [] as opt}
                        <option value={opt}>{opt}</option>
                      {/each}
                    </select>
                  {:else if param.type === 'checkbox'}
                    <label class="checkbox-field">
                      <input
                        type="checkbox"
                        checked={paramValues[param.id] === true}
                        onchange={(e) => paramValues[param.id] = e.currentTarget.checked}
                      />
                      <span>{param.label}</span>
                    </label>
                  {/if}

                  {#if param.help}
                    <p class="param-help">{param.help}</p>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      <!-- Command Preview -->
      <section class="preview-section">
        <div class="preview-header">
          <h3>Command Preview</h3>
          <span class="preview-hint">with line continuation</span>
        </div>
        <div class="preview-box">
          <pre class="command-preview">{commandString || currentCmd.template}</pre>
        </div>

        <div class="copy-buttons">
          <button class="copy-btn primary" onclick={() => copyCommand(false)}>
            {copyMessage === 'Copied!' ? '✅ Copied!' : '📋 Copy (1-line)'}
          </button>
          <button class="copy-btn secondary" onclick={() => copyCommand(true)}>
            {copyMessage === 'Multi-line copied!' ? '✅ Copied!' : '📋 Copy (multi)'}
          </button>
        </div>

        <div class="usage-hint">
          <p><strong>Tip:</strong> The multi-line format with <code>\</code> works in bash/zsh. Use 1-line for Windows CMD.</p>
        </div>
      </section>
    {/if}
  </div>
</aside>

<style>
  .command-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.3s ease, visibility 0.3s ease;
    z-index: 99;
  }

  .command-overlay.open {
    opacity: 1;
    visibility: visible;
  }

  .command-pane {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 520px;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform 0.3s ease;
    z-index: 100;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 20px rgba(0, 0, 0, 0.3);
  }

  .command-pane.open {
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

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .back-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 6px;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s ease;
  }

  .back-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .close-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-muted);
    border-radius: 4px;
    transition: all 0.15s ease;
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

  .command-intro {
    margin-bottom: 20px;
  }

  .command-intro p {
    color: var(--text-secondary);
    font-size: 14px;
  }

  /* Command Grid */
  .command-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .command-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .command-card:hover {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .card-icon {
    font-size: 24px;
    margin-bottom: 8px;
  }

  .card-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .card-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* Command Builder */
  .cmd-description {
    color: var(--text-secondary);
    font-size: 13px;
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }

  .param-section {
    margin-bottom: 20px;
  }

  .param-section h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 12px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .section-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    margin-bottom: 12px;
  }

  .section-toggle h3 {
    margin: 0;
  }

  .toggle-icon {
    color: var(--text-muted);
    font-size: 12px;
  }

  .required-marker {
    color: var(--accent);
  }

  .param-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .param-field label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .param-field input[type="text"],
  .param-field input[type="number"],
  .param-field select {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    transition: border-color 0.15s ease;
  }

  .param-field input:focus,
  .param-field select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .checkbox-field {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-field input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

  .checkbox-field span {
    font-size: 13px;
    color: var(--text-primary);
  }

  .param-help {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  /* Preview */
  .preview-section {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }

  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .preview-header h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .preview-hint {
    font-size: 10px;
    color: var(--text-muted);
  }

  .preview-box {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    overflow-x: auto;
    margin-bottom: 12px;
  }

  .command-preview {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 12px;
    color: #22c55e;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
  }

  .copy-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;
  }

  .copy-btn {
    padding: 10px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .copy-btn.primary {
    background: var(--accent);
    border: 1px solid var(--accent);
    color: white;
  }

  .copy-btn.primary:hover {
    background: #0052cc;
  }

  .copy-btn.secondary {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .copy-btn.secondary:hover {
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .usage-hint {
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 10px 12px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .usage-hint strong {
    color: var(--text-secondary);
  }

  .usage-hint code {
    color: #22c55e;
    background: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
  }
</style>
