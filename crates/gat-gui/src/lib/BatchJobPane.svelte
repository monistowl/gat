<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  // Props
  let { isOpen, onClose }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  // State - Directories
  let inputDir = $state<string>('');
  let outputDir = $state<string>('');
  let inputDragOver = $state(false);
  let outputDragOver = $state(false);

  // State - Run Settings
  let analysisType = $state<string>('pf_dc');
  let parallelJobs = $state<number>(4);
  let filePattern = $state<string>('*.arrow');

  // State - Solver Configuration
  let solverMode = $state<'simple' | 'advanced'>('simple');
  let primaryLpSolver = $state<string>('clarabel');
  let primaryNlpSolver = $state<string>('lbfgs');
  let fallbackLpSolver = $state<string>('');
  let fallbackNlpSolver = $state<string>('');
  let useFallback = $state<boolean>(false);

  // State - Solver Options
  let tolerance = $state<string>('1e-6');
  let maxIterations = $state<number>(100);
  let warmStart = $state<boolean>(true);
  let presolve = $state<boolean>(true);
  let crossover = $state<boolean>(false);
  let verboseSolver = $state<boolean>(false);

  // State - Reporting
  let outputFormat = $state<string>('parquet');
  let logLevel = $state<string>('info');
  let saveLogs = $state<boolean>(true);
  let generateSummary = $state<boolean>(true);
  let generateViolations = $state<boolean>(false);

  // State - Execution
  let runId = $state<string | null>(null);
  let isRunning = $state(false);
  let progress = $state({ completed: 0, total: 0 });
  let results = $state<JobResult[] | null>(null);
  let runError = $state<string | null>(null);

  interface JobResult {
    job_id: string;
    status: string;
    duration_ms: number | null;
    error: string | null;
  }

  // Analysis types
  const analysisTypes = [
    { id: 'pf_dc', name: 'DC Power Flow', icon: '⚡', needsLp: false, needsNlp: false },
    { id: 'pf_ac', name: 'AC Power Flow', icon: '⚡', needsLp: false, needsNlp: false },
    { id: 'opf_dc', name: 'DC OPF', icon: '🎯', needsLp: true, needsNlp: false },
    { id: 'opf_ac', name: 'AC OPF', icon: '🎯', needsLp: true, needsNlp: true },
    { id: 'contingency', name: 'N-1 Contingency', icon: '🔍', needsLp: false, needsNlp: false },
  ];

  // Available solvers by category
  const lpSolvers = [
    { id: 'clarabel', name: 'Clarabel', desc: 'Interior-point conic (pure Rust)', native: false },
    { id: 'highs', name: 'HiGHS', desc: 'LP/MIP simplex & IPM', native: true },
    { id: 'cbc', name: 'CBC', desc: 'COIN-OR branch & cut', native: true },
  ];

  const nlpSolvers = [
    { id: 'lbfgs', name: 'L-BFGS', desc: 'Quasi-Newton (pure Rust)', native: false },
    { id: 'ipopt', name: 'IPOPT', desc: 'Interior-point NLP', native: true },
  ];

  const pfSolvers = [
    { id: 'nr', name: 'Newton-Raphson', desc: 'Classic iterative method' },
    { id: 'gs', name: 'Gauss-Seidel', desc: 'Simple iterative' },
    { id: 'fdxb', name: 'Fast Decoupled XB', desc: 'Decoupled P-Q iteration' },
    { id: 'fdbx', name: 'Fast Decoupled BX', desc: 'Alternate decoupling' },
  ];

  // Solver presets
  const solverPresets = [
    { id: 'fast', name: 'Fast', desc: 'Prioritize speed', lp: 'clarabel', nlp: 'lbfgs', fallbackLp: '', fallbackNlp: '' },
    { id: 'robust', name: 'Robust', desc: 'Fallback chain', lp: 'clarabel', nlp: 'lbfgs', fallbackLp: 'highs', fallbackNlp: 'ipopt' },
    { id: 'accurate', name: 'Accurate', desc: 'Best convergence', lp: 'highs', nlp: 'ipopt', fallbackLp: '', fallbackNlp: '' },
    { id: 'native', name: 'Native Only', desc: 'Use optimized native solvers', lp: 'highs', nlp: 'ipopt', fallbackLp: 'cbc', fallbackNlp: '' },
  ];

  // Output formats
  const outputFormats = ['parquet', 'arrow', 'csv', 'json'];
  const logLevels = ['error', 'warn', 'info', 'debug', 'trace'];

  // Computed
  let currentAnalysis = $derived(analysisTypes.find(t => t.id === analysisType));
  let needsLpSolver = $derived(currentAnalysis?.needsLp ?? false);
  let needsNlpSolver = $derived(currentAnalysis?.needsNlp ?? false);
  let needsPfSolver = $derived(!needsLpSolver && !needsNlpSolver);
  let isConfigured = $derived(inputDir && outputDir);

  // Apply preset
  function applyPreset(preset: typeof solverPresets[0]) {
    primaryLpSolver = preset.lp;
    primaryNlpSolver = preset.nlp;
    fallbackLpSolver = preset.fallbackLp;
    fallbackNlpSolver = preset.fallbackNlp;
    useFallback = !!(preset.fallbackLp || preset.fallbackNlp);
  }

  // Get solver summary for display
  let solverSummary = $derived.by(() => {
    if (needsPfSolver) {
      return pfSolvers.find(s => s.id === primaryLpSolver)?.name || 'Newton-Raphson';
    }
    const parts: string[] = [];
    if (needsLpSolver) {
      parts.push(`LP: ${lpSolvers.find(s => s.id === primaryLpSolver)?.name || primaryLpSolver}`);
    }
    if (needsNlpSolver) {
      parts.push(`NLP: ${nlpSolvers.find(s => s.id === primaryNlpSolver)?.name || primaryNlpSolver}`);
    }
    if (useFallback && (fallbackLpSolver || fallbackNlpSolver)) {
      parts.push('+ fallback');
    }
    return parts.join(', ') || 'Default';
  });

  // Directory pickers
  async function pickInputDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Input Directory',
      });
      if (selected) {
        inputDir = selected as string;
      }
    } catch (e) {
      console.error('Failed to open directory picker:', e);
    }
  }

  async function pickOutputDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Output Directory',
      });
      if (selected) {
        outputDir = selected as string;
      }
    } catch (e) {
      console.error('Failed to open directory picker:', e);
    }
  }

  // Drag and drop handlers
  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
  }

  function handleInputDragEnter(e: DragEvent) {
    e.preventDefault();
    inputDragOver = true;
  }

  function handleInputDragLeave(e: DragEvent) {
    e.preventDefault();
    inputDragOver = false;
  }

  function handleOutputDragEnter(e: DragEvent) {
    e.preventDefault();
    outputDragOver = true;
  }

  function handleOutputDragLeave(e: DragEvent) {
    e.preventDefault();
    outputDragOver = false;
  }

  function handleInputDrop(e: DragEvent) {
    e.preventDefault();
    inputDragOver = false;

    const items = e.dataTransfer?.items;
    if (items && items.length > 0) {
      // In a real implementation, we'd extract the path from the dropped item
      // For now, show a message since browser security limits direct path access
      const item = items[0];
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file) {
          // Show the file name as feedback (full path not available in browser)
          inputDir = `[Dropped: ${file.name}]`;
        }
      }
    }
  }

  function handleOutputDrop(e: DragEvent) {
    e.preventDefault();
    outputDragOver = false;

    const items = e.dataTransfer?.items;
    if (items && items.length > 0) {
      const item = items[0];
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file) {
          outputDir = `[Dropped: ${file.name}]`;
        }
      }
    }
  }

  function clearInput() {
    inputDir = '';
  }

  function clearOutput() {
    outputDir = '';
  }

  // Close on escape
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }

  // Estimate job count (mock)
  let estimatedJobs = $derived.by(() => {
    if (!inputDir) return 0;
    // Mock: return a random estimate based on file pattern
    return filePattern === '*.arrow' ? 12 : filePattern === '*.m' ? 8 : 5;
  });

  // Execution functions
  async function runBatch() {
    if (!inputDir || !outputDir) return;

    isRunning = true;
    runError = null;
    results = null;

    try {
      const response = await invoke<{ run_id: string; total_jobs: number }>('run_batch_job', {
        request: {
          input_dir: inputDir,
          output_dir: outputDir,
          file_pattern: filePattern,
          analysis_type: analysisType,
          parallel_jobs: parallelJobs,
          tolerance: parseFloat(tolerance),
          max_iterations: maxIterations,
        }
      });

      runId = response.run_id;
      progress = { completed: 0, total: response.total_jobs };

      // Poll for status
      pollStatus();
    } catch (e) {
      runError = String(e);
      isRunning = false;
    }
  }

  async function pollStatus() {
    if (!runId) return;

    try {
      const status = await invoke<{
        status: string;
        completed: number;
        total: number;
        results: JobResult[] | null;
        error: string | null;
      }>('get_batch_status', { runId });

      progress = { completed: status.completed, total: status.total };

      if (status.status === 'completed') {
        isRunning = false;
        results = status.results;
      } else if (status.status === 'failed') {
        isRunning = false;
        runError = status.error;
      } else {
        // Still running, poll again
        setTimeout(pollStatus, 500);
      }
    } catch (e) {
      runError = String(e);
      isRunning = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="batch-overlay" class:open={isOpen} onclick={onClose} role="presentation"></div>

<aside class="batch-pane" class:open={isOpen}>
  <header>
    <h2>🔄 Batch Job</h2>
    <button class="close-btn" onclick={onClose} aria-label="Close">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </header>

  <div class="pane-content">
    <!-- Directory Selection -->
    <section class="dir-section">
      <h3>📁 Directories</h3>

      <!-- Input Directory -->
      <div class="dir-field">
        <label>Input Directory</label>
        <div
          class="drop-zone"
          class:drag-over={inputDragOver}
          class:has-value={!!inputDir}
          ondragover={handleDragOver}
          ondragenter={handleInputDragEnter}
          ondragleave={handleInputDragLeave}
          ondrop={handleInputDrop}
          role="button"
          tabindex="0"
          onclick={pickInputDir}
          onkeydown={(e) => e.key === 'Enter' && pickInputDir()}
        >
          {#if inputDir}
            <div class="drop-zone-content has-path">
              <span class="path-icon">📂</span>
              <span class="path-text">{inputDir}</span>
              <button class="clear-btn" onclick={(e) => { e.stopPropagation(); clearInput(); }} aria-label="Clear">×</button>
            </div>
          {:else}
            <div class="drop-zone-content empty">
              <span class="drop-icon">📥</span>
              <span class="drop-text">Drop folder here or click to browse</span>
              <span class="drop-hint">Contains grid files ({filePattern})</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Output Directory -->
      <div class="dir-field">
        <label>Output Directory</label>
        <div
          class="drop-zone"
          class:drag-over={outputDragOver}
          class:has-value={!!outputDir}
          ondragover={handleDragOver}
          ondragenter={handleOutputDragEnter}
          ondragleave={handleOutputDragLeave}
          ondrop={handleOutputDrop}
          role="button"
          tabindex="0"
          onclick={pickOutputDir}
          onkeydown={(e) => e.key === 'Enter' && pickOutputDir()}
        >
          {#if outputDir}
            <div class="drop-zone-content has-path">
              <span class="path-icon">📂</span>
              <span class="path-text">{outputDir}</span>
              <button class="clear-btn" onclick={(e) => { e.stopPropagation(); clearOutput(); }} aria-label="Clear">×</button>
            </div>
          {:else}
            <div class="drop-zone-content empty">
              <span class="drop-icon">📤</span>
              <span class="drop-text">Drop folder here or click to browse</span>
              <span class="drop-hint">Results will be saved here</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- File Pattern -->
      <div class="pattern-field">
        <label for="file-pattern">File Pattern</label>
        <select id="file-pattern" bind:value={filePattern}>
          <option value="*.arrow">*.arrow (Arrow IPC)</option>
          <option value="*.parquet">*.parquet (Parquet)</option>
          <option value="*.m">*.m (MATPOWER)</option>
          <option value="*.raw">*.raw (PSS/E RAW)</option>
        </select>
      </div>
    </section>

    <!-- Run Settings -->
    <section class="settings-section">
      <h3>⚙️ Run Settings</h3>

      <!-- Analysis Type -->
      <div class="setting-field">
        <label>Analysis Type</label>
        <div class="analysis-grid">
          {#each analysisTypes as type}
            <button
              class="analysis-btn"
              class:selected={analysisType === type.id}
              onclick={() => analysisType = type.id}
            >
              <span class="analysis-icon">{type.icon}</span>
              <span class="analysis-name">{type.name}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Parallel Jobs -->
      <div class="setting-row">
        <div class="setting-field half">
          <label for="parallel">Parallel Jobs</label>
          <input type="number" id="parallel" bind:value={parallelJobs} min="1" max="32" />
        </div>
        <div class="setting-field half">
          <label for="tolerance">Tolerance</label>
          <input type="text" id="tolerance" bind:value={tolerance} placeholder="1e-6" />
        </div>
      </div>
    </section>

    <!-- Solver Configuration -->
    <section class="solver-section">
      <div class="section-header">
        <h3>🔧 Solver Configuration</h3>
        <div class="mode-toggle">
          <button
            class="mode-btn"
            class:active={solverMode === 'simple'}
            onclick={() => solverMode = 'simple'}
          >Simple</button>
          <button
            class="mode-btn"
            class:active={solverMode === 'advanced'}
            onclick={() => solverMode = 'advanced'}
          >Advanced</button>
        </div>
      </div>

      {#if solverMode === 'simple'}
        <!-- Preset Buttons -->
        <div class="preset-grid">
          {#each solverPresets as preset}
            <button
              class="preset-btn"
              class:selected={primaryLpSolver === preset.lp && primaryNlpSolver === preset.nlp}
              onclick={() => applyPreset(preset)}
            >
              <span class="preset-name">{preset.name}</span>
              <span class="preset-desc">{preset.desc}</span>
            </button>
          {/each}
        </div>
      {:else}
        <!-- Advanced Solver Selection -->
        <div class="solver-config">
          {#if needsPfSolver}
            <!-- Power Flow Solver -->
            <div class="solver-group">
              <label>Power Flow Method</label>
              <div class="solver-options">
                {#each pfSolvers as solver}
                  <button
                    class="solver-option"
                    class:selected={primaryLpSolver === solver.id}
                    onclick={() => primaryLpSolver = solver.id}
                  >
                    <span class="solver-name">{solver.name}</span>
                    <span class="solver-desc">{solver.desc}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          {#if needsLpSolver}
            <!-- LP Solver -->
            <div class="solver-group">
              <label>LP Solver (Primary)</label>
              <div class="solver-options">
                {#each lpSolvers as solver}
                  <button
                    class="solver-option"
                    class:selected={primaryLpSolver === solver.id}
                    onclick={() => primaryLpSolver = solver.id}
                  >
                    <div class="solver-header">
                      <span class="solver-name">{solver.name}</span>
                      {#if solver.native}
                        <span class="native-badge">Native</span>
                      {/if}
                    </div>
                    <span class="solver-desc">{solver.desc}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          {#if needsNlpSolver}
            <!-- NLP Solver -->
            <div class="solver-group">
              <label>NLP Solver (Primary)</label>
              <div class="solver-options">
                {#each nlpSolvers as solver}
                  <button
                    class="solver-option"
                    class:selected={primaryNlpSolver === solver.id}
                    onclick={() => primaryNlpSolver = solver.id}
                  >
                    <div class="solver-header">
                      <span class="solver-name">{solver.name}</span>
                      {#if solver.native}
                        <span class="native-badge">Native</span>
                      {/if}
                    </div>
                    <span class="solver-desc">{solver.desc}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Fallback Configuration -->
          {#if needsLpSolver || needsNlpSolver}
            <div class="fallback-section">
              <label class="checkbox-field">
                <input type="checkbox" bind:checked={useFallback} />
                <span>Enable fallback solvers</span>
              </label>

              {#if useFallback}
                <div class="fallback-grid">
                  {#if needsLpSolver}
                    <div class="setting-field">
                      <label for="fallback-lp">LP Fallback</label>
                      <select id="fallback-lp" bind:value={fallbackLpSolver}>
                        <option value="">None</option>
                        {#each lpSolvers.filter(s => s.id !== primaryLpSolver) as solver}
                          <option value={solver.id}>{solver.name}</option>
                        {/each}
                      </select>
                    </div>
                  {/if}
                  {#if needsNlpSolver}
                    <div class="setting-field">
                      <label for="fallback-nlp">NLP Fallback</label>
                      <select id="fallback-nlp" bind:value={fallbackNlpSolver}>
                        <option value="">None</option>
                        {#each nlpSolvers.filter(s => s.id !== primaryNlpSolver) as solver}
                          <option value={solver.id}>{solver.name}</option>
                        {/each}
                      </select>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Solver Options -->
          <div class="solver-options-group">
            <label>Solver Options</label>
            <div class="options-grid">
              <label class="checkbox-field">
                <input type="checkbox" bind:checked={warmStart} />
                <span>Warm start</span>
              </label>
              <label class="checkbox-field">
                <input type="checkbox" bind:checked={presolve} />
                <span>Presolve</span>
              </label>
              {#if needsLpSolver}
                <label class="checkbox-field">
                  <input type="checkbox" bind:checked={crossover} />
                  <span>Crossover (simplex)</span>
                </label>
              {/if}
              <label class="checkbox-field">
                <input type="checkbox" bind:checked={verboseSolver} />
                <span>Verbose output</span>
              </label>
            </div>

            <div class="setting-row">
              <div class="setting-field half">
                <label for="max-iter">Max Iterations</label>
                <input type="number" id="max-iter" bind:value={maxIterations} min="1" max="10000" />
              </div>
            </div>
          </div>
        </div>
      {/if}
    </section>

    <!-- Reporting Options -->
    <section class="reporting-section">
      <h3>📊 Reporting</h3>

      <div class="setting-row">
        <div class="setting-field half">
          <label for="output-format">Output Format</label>
          <select id="output-format" bind:value={outputFormat}>
            {#each outputFormats as fmt}
              <option value={fmt}>{fmt.toUpperCase()}</option>
            {/each}
          </select>
        </div>

        <div class="setting-field half">
          <label for="log-level">Log Level</label>
          <select id="log-level" bind:value={logLevel}>
            {#each logLevels as level}
              <option value={level}>{level}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="checkbox-group">
        <label class="checkbox-field">
          <input type="checkbox" bind:checked={saveLogs} />
          <span>Save execution logs</span>
        </label>

        <label class="checkbox-field">
          <input type="checkbox" bind:checked={generateSummary} />
          <span>Generate summary report</span>
        </label>

        <label class="checkbox-field">
          <input type="checkbox" bind:checked={generateViolations} />
          <span>Flag constraint violations</span>
        </label>
      </div>
    </section>

    <!-- Job Summary -->
    <section class="summary-section">
      <h3>📋 Job Summary</h3>
      <div class="summary-grid">
        <div class="summary-item">
          <span class="summary-label">Input Files</span>
          <span class="summary-value">{inputDir ? `~${estimatedJobs} files` : '—'}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">Analysis</span>
          <span class="summary-value">{analysisTypes.find(t => t.id === analysisType)?.name || '—'}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">Solver</span>
          <span class="summary-value">{solverSummary}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">Parallelism</span>
          <span class="summary-value">{parallelJobs} jobs</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">Tolerance</span>
          <span class="summary-value">{tolerance}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">Output</span>
          <span class="summary-value">{outputFormat.toUpperCase()}</span>
        </div>
      </div>
    </section>

    <!-- Results (shown after completion) -->
    {#if results}
      <section class="results-section">
        <h3>📊 Results</h3>

        <!-- Summary Stats -->
        <div class="results-summary">
          <div class="stat success">
            <span class="stat-value">{results.filter(r => r.status === 'ok').length}</span>
            <span class="stat-label">Passed</span>
          </div>
          <div class="stat error">
            <span class="stat-value">{results.filter(r => r.status === 'error').length}</span>
            <span class="stat-label">Failed</span>
          </div>
          <div class="stat">
            <span class="stat-value">{(results.reduce((sum, r) => sum + (r.duration_ms || 0), 0) / 1000).toFixed(2)}s</span>
            <span class="stat-label">Total Time</span>
          </div>
        </div>

        <!-- Job Table -->
        <div class="results-table">
          <table>
            <thead>
              <tr>
                <th>Job</th>
                <th>Status</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {#each results.slice(0, 10) as job}
                <tr class:error={job.status === 'error'}>
                  <td class="job-name">{job.job_id}</td>
                  <td class="job-status">
                    {#if job.status === 'ok'}
                      <span class="status-ok">✓</span>
                    {:else}
                      <span class="status-error" title={job.error || ''}>✗</span>
                    {/if}
                  </td>
                  <td class="job-time">{job.duration_ms?.toFixed(1) || '—'}ms</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if results.length > 10}
            <div class="table-footer">+ {results.length - 10} more jobs</div>
          {/if}
        </div>
      </section>
    {/if}

    {#if runError}
      <section class="error-section">
        <h3>❌ Error</h3>
        <p class="error-message">{runError}</p>
      </section>
    {/if}
  </div>

  <!-- Footer Actions -->
  <footer>
    <div class="footer-info">
      {#if !isConfigured}
        <span class="info-warning">⚠️ Select input and output directories</span>
      {:else}
        <span class="info-ready">✓ Ready to run</span>
      {/if}
    </div>
    <div class="footer-actions">
      <button class="btn secondary" onclick={onClose}>Cancel</button>
      <button
        class="btn primary"
        disabled={!isConfigured || isRunning}
        onclick={runBatch}
      >
        {#if isRunning}
          ⏳ Running... ({progress.completed}/{progress.total})
        {:else}
          ▶ Run Batch Job
        {/if}
      </button>
    </div>
  </footer>
</aside>

<style>
  .batch-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.3s ease, visibility 0.3s ease;
    z-index: 99;
  }

  .batch-overlay.open {
    opacity: 1;
    visibility: visible;
  }

  .batch-pane {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 480px;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform 0.3s ease;
    z-index: 100;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 20px rgba(0, 0, 0, 0.3);
  }

  .batch-pane.open {
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

  section {
    margin-bottom: 24px;
  }

  section h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 14px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Directory Fields */
  .dir-field {
    margin-bottom: 14px;
  }

  .dir-field label,
  .pattern-field label,
  .setting-field label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .drop-zone {
    border: 2px dashed var(--border);
    border-radius: 8px;
    background: var(--bg-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    min-height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .drop-zone:hover {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .drop-zone.drag-over {
    border-color: var(--accent);
    background: rgba(0, 102, 255, 0.1);
    border-style: solid;
  }

  .drop-zone.has-value {
    border-style: solid;
    border-color: var(--accent);
  }

  .drop-zone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px;
    width: 100%;
  }

  .drop-zone-content.has-path {
    flex-direction: row;
    justify-content: space-between;
    gap: 12px;
  }

  .drop-icon, .path-icon {
    font-size: 24px;
  }

  .drop-text {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .drop-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .path-text {
    flex: 1;
    font-size: 12px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clear-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    width: 24px;
    height: 24px;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    transition: all 0.15s ease;
  }

  .clear-btn:hover {
    background: #ef4444;
    border-color: #ef4444;
    color: white;
  }

  .pattern-field select,
  .setting-field select,
  .setting-field input {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    transition: border-color 0.15s ease;
  }

  .setting-field input {
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .pattern-field select:focus,
  .setting-field select:focus,
  .setting-field input:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* Analysis Type Grid */
  .analysis-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .analysis-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .analysis-btn:hover {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .analysis-btn.selected {
    border-color: var(--accent);
    background: rgba(0, 102, 255, 0.1);
  }

  .analysis-icon {
    font-size: 20px;
  }

  .analysis-name {
    font-size: 10px;
    color: var(--text-secondary);
    text-align: center;
  }

  .analysis-btn.selected .analysis-name {
    color: var(--accent);
    font-weight: 500;
  }

  /* Settings Row */
  .setting-row {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
  }

  .setting-field.half {
    flex: 1;
  }

  /* Checkbox Group */
  .checkbox-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .checkbox-field {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .checkbox-field input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

  .checkbox-field span {
    font-size: 13px;
    color: var(--text-secondary);
  }

  /* Summary Section */
  .summary-section {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 0;
  }

  .summary-section h3 {
    margin-bottom: 12px;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .summary-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .summary-value {
    font-size: 14px;
    color: var(--text-primary);
    font-weight: 500;
  }

  /* Footer */
  footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .footer-info {
    font-size: 12px;
  }

  .info-warning {
    color: #f59e0b;
  }

  .info-ready {
    color: #22c55e;
  }

  .footer-actions {
    display: flex;
    gap: 10px;
  }

  .btn {
    padding: 10px 20px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn.secondary {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .btn.secondary:hover {
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .btn.primary {
    background: var(--accent);
    border: 1px solid var(--accent);
    color: white;
  }

  .btn.primary:hover:not(:disabled) {
    background: #0052cc;
  }

  .btn.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Solver Section */
  .solver-section {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    background: var(--bg-secondary);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .section-header h3 {
    margin-bottom: 0;
  }

  .mode-toggle {
    display: flex;
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 3px;
    gap: 2px;
  }

  .mode-btn {
    padding: 6px 12px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    color: var(--text-secondary);
  }

  .mode-btn.active {
    background: var(--bg-primary);
    color: var(--accent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  /* Preset Grid */
  .preset-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }

  .preset-btn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
  }

  .preset-btn:hover {
    border-color: var(--accent);
    background: var(--bg-primary);
  }

  .preset-btn.selected {
    border-color: var(--accent);
    background: rgba(0, 102, 255, 0.1);
  }

  .preset-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .preset-desc {
    font-size: 11px;
    color: var(--text-muted);
  }

  .preset-btn.selected .preset-name {
    color: var(--accent);
  }

  /* Advanced Solver Config */
  .solver-config {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .solver-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .solver-group > label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .solver-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .solver-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    width: 100%;
  }

  .solver-option:hover {
    border-color: var(--accent);
    background: var(--bg-primary);
  }

  .solver-option.selected {
    border-color: var(--accent);
    background: rgba(0, 102, 255, 0.1);
  }

  .solver-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .solver-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .solver-option.selected .solver-name {
    color: var(--accent);
  }

  .solver-desc {
    font-size: 11px;
    color: var(--text-muted);
  }

  .native-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    background: #22c55e20;
    color: #22c55e;
    border-radius: 4px;
  }

  /* Fallback Section */
  .fallback-section {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  .fallback-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    margin-top: 10px;
  }

  /* Solver Options Group */
  .solver-options-group {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  .solver-options-group > label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 10px;
  }

  .options-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
    margin-bottom: 12px;
  }

  /* Results Section */
  .results-section {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .results-summary {
    display: flex;
    gap: 16px;
    margin-bottom: 16px;
  }

  .stat {
    flex: 1;
    text-align: center;
    padding: 12px;
    background: var(--bg-tertiary);
    border-radius: 6px;
  }

  .stat.success .stat-value { color: #22c55e; }
  .stat.error .stat-value { color: #ef4444; }

  .stat-value {
    display: block;
    font-size: 24px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .stat-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .results-table {
    max-height: 300px;
    overflow-y: auto;
  }

  .results-table table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .results-table th, .results-table td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .results-table th {
    background: var(--bg-tertiary);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .results-table tr.error { background: rgba(239, 68, 68, 0.1); }

  .job-name {
    font-family: 'SF Mono', monospace;
    color: var(--text-primary);
  }

  .job-time {
    color: var(--text-muted);
    font-family: 'SF Mono', monospace;
  }

  .status-ok { color: #22c55e; }
  .status-error { color: #ef4444; cursor: help; }

  .table-footer {
    padding: 8px;
    text-align: center;
    color: var(--text-muted);
    font-size: 11px;
  }

  /* Error Section */
  .error-section {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid #ef4444;
    border-radius: 8px;
    padding: 16px;
  }

  .error-message {
    color: #ef4444;
    font-family: 'SF Mono', monospace;
    font-size: 12px;
  }
</style>
