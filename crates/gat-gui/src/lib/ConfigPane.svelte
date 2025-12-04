<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";

  // Types matching Rust structs
  interface SolverConfig {
    native_enabled: boolean;
    default_lp: string;
    default_nlp: string;
    timeout_seconds: number;
    max_iterations: number;
  }

  interface LoggingConfig {
    level: string;
  }

  interface DataConfig {
    grid_cache: string;
    results_dir: string;
  }

  interface UiConfig {
    theme: string;
    enable_animations: boolean;
  }

  interface AppConfig {
    solvers: SolverConfig;
    logging: LoggingConfig;
    data: DataConfig;
    ui: UiConfig;
  }

  // Props
  let { isOpen, onClose }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  // State
  let config = $state<AppConfig | null>(null);
  let configPath = $state<string>('');
  let loading = $state(false);
  let saving = $state(false);
  let saveMessage = $state<{ type: 'success' | 'error', text: string } | null>(null);
  let hasChanges = $state(false);

  // Available options
  const lpSolvers = ['clarabel', 'highs', 'cbc'];
  const nlpSolvers = ['lbfgs', 'ipopt', 'slsqp'];
  const logLevels = ['trace', 'debug', 'info', 'warn', 'error'];
  const themes = ['dark', 'light', 'system'];

  // Load config on mount
  onMount(async () => {
    await loadConfig();
    try {
      configPath = await invoke<string>('get_config_path');
    } catch (e) {
      console.error('Failed to get config path:', e);
    }
  });

  async function loadConfig() {
    loading = true;
    try {
      config = await invoke<AppConfig>('get_config');
      hasChanges = false;
    } catch (e) {
      console.error('Failed to load config:', e);
    } finally {
      loading = false;
    }
  }

  async function saveConfig() {
    if (!config) return;

    saving = true;
    saveMessage = null;
    try {
      await invoke('save_config', { config });
      saveMessage = { type: 'success', text: 'Configuration saved successfully!' };
      hasChanges = false;
      setTimeout(() => saveMessage = null, 3000);
    } catch (e) {
      saveMessage = { type: 'error', text: `Failed to save: ${e}` };
    } finally {
      saving = false;
    }
  }

  function resetToDefaults() {
    config = {
      solvers: {
        native_enabled: true,
        default_lp: 'clarabel',
        default_nlp: 'lbfgs',
        timeout_seconds: 300,
        max_iterations: 1000,
      },
      logging: {
        level: 'info',
      },
      data: {
        grid_cache: '~/.gat/cache/grids',
        results_dir: '~/.gat/results',
      },
      ui: {
        theme: 'dark',
        enable_animations: true,
      },
    };
    hasChanges = true;
  }

  function markChanged() {
    hasChanges = true;
  }

  // Close on escape
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="config-overlay" class:open={isOpen} onclick={onClose} role="presentation"></div>

<aside class="config-pane" class:open={isOpen}>
  <header>
    <h2>Configuration</h2>
    <button class="close-btn" onclick={onClose} aria-label="Close">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </header>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <span>Loading configuration...</span>
    </div>
  {:else if config}
    <div class="config-content">
      <!-- Solvers Section -->
      <section class="config-section">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 3L2 12h3v9h6v-6h2v6h6v-9h3L12 3z"/>
          </svg>
          Solver Settings
        </h3>

        <div class="form-group">
          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={config.solvers.native_enabled}
              onchange={markChanged}
            />
            <span class="checkmark"></span>
            <span class="label-text">Enable native solvers</span>
          </label>
          <p class="hint">Use compiled Rust solvers for better performance</p>
        </div>

        <div class="form-group">
          <label class="form-label">Default LP Solver</label>
          <div class="radio-group">
            {#each lpSolvers as solver}
              <label class="radio-label">
                <input
                  type="radio"
                  name="default_lp"
                  value={solver}
                  bind:group={config.solvers.default_lp}
                  onchange={markChanged}
                />
                <span class="radio-mark"></span>
                <span class="label-text">{solver}</span>
              </label>
            {/each}
          </div>
        </div>

        <div class="form-group">
          <label class="form-label">Default NLP Solver</label>
          <div class="radio-group">
            {#each nlpSolvers as solver}
              <label class="radio-label">
                <input
                  type="radio"
                  name="default_nlp"
                  value={solver}
                  bind:group={config.solvers.default_nlp}
                  onchange={markChanged}
                />
                <span class="radio-mark"></span>
                <span class="label-text">{solver}</span>
              </label>
            {/each}
          </div>
        </div>

        <div class="form-row">
          <div class="form-group half">
            <label class="form-label" for="timeout">Timeout (seconds)</label>
            <input
              type="number"
              id="timeout"
              bind:value={config.solvers.timeout_seconds}
              oninput={markChanged}
              min="1"
              max="3600"
            />
          </div>
          <div class="form-group half">
            <label class="form-label" for="iterations">Max Iterations</label>
            <input
              type="number"
              id="iterations"
              bind:value={config.solvers.max_iterations}
              oninput={markChanged}
              min="1"
              max="100000"
            />
          </div>
        </div>
      </section>

      <!-- Logging Section -->
      <section class="config-section">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <path d="M14 2v6h6"/>
            <path d="M16 13H8M16 17H8M10 9H8"/>
          </svg>
          Logging
        </h3>

        <div class="form-group">
          <label class="form-label">Log Level</label>
          <div class="radio-group horizontal">
            {#each logLevels as level}
              <label class="radio-label">
                <input
                  type="radio"
                  name="log_level"
                  value={level}
                  bind:group={config.logging.level}
                  onchange={markChanged}
                />
                <span class="radio-mark"></span>
                <span class="label-text">{level}</span>
              </label>
            {/each}
          </div>
        </div>
      </section>

      <!-- Data Paths Section -->
      <section class="config-section">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
          </svg>
          Data Paths
        </h3>

        <div class="form-group">
          <label class="form-label" for="grid_cache">Grid Cache Directory</label>
          <input
            type="text"
            id="grid_cache"
            bind:value={config.data.grid_cache}
            oninput={markChanged}
            placeholder="~/.gat/cache/grids"
          />
        </div>

        <div class="form-group">
          <label class="form-label" for="results_dir">Results Directory</label>
          <input
            type="text"
            id="results_dir"
            bind:value={config.data.results_dir}
            oninput={markChanged}
            placeholder="~/.gat/results"
          />
        </div>
      </section>

      <!-- UI Section -->
      <section class="config-section">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
          </svg>
          User Interface
        </h3>

        <div class="form-group">
          <label class="form-label">Theme</label>
          <div class="radio-group horizontal">
            {#each themes as theme}
              <label class="radio-label">
                <input
                  type="radio"
                  name="theme"
                  value={theme}
                  bind:group={config.ui.theme}
                  onchange={markChanged}
                />
                <span class="radio-mark"></span>
                <span class="label-text">{theme}</span>
              </label>
            {/each}
          </div>
        </div>

        <div class="form-group">
          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={config.ui.enable_animations}
              onchange={markChanged}
            />
            <span class="checkmark"></span>
            <span class="label-text">Enable animations</span>
          </label>
          <p class="hint">Smooth transitions and visual effects</p>
        </div>
      </section>

      <!-- Config Path Info -->
      <section class="config-section config-path">
        <p class="path-label">Config file:</p>
        <code>{configPath}</code>
      </section>
    </div>

    <!-- Footer with actions -->
    <footer>
      {#if saveMessage}
        <div class="message" class:success={saveMessage.type === 'success'} class:error={saveMessage.type === 'error'}>
          {saveMessage.text}
        </div>
      {/if}
      <div class="actions">
        <button class="btn secondary" onclick={resetToDefaults}>
          Reset to Defaults
        </button>
        <button
          class="btn primary"
          onclick={saveConfig}
          disabled={saving || !hasChanges}
        >
          {saving ? 'Saving...' : 'Save Changes'}
        </button>
      </div>
    </footer>
  {/if}
</aside>

<style>
  .config-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.3s ease, visibility 0.3s ease;
    z-index: 99;
  }

  .config-overlay.open {
    opacity: 1;
    visibility: visible;
  }

  .config-pane {
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

  .config-pane.open {
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

  .loading {
    flex: 1;
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

  .config-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
  }

  .config-section {
    margin-bottom: 24px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border);
  }

  .config-section:last-of-type {
    border-bottom: none;
    margin-bottom: 0;
  }

  .config-section h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .config-section h3 svg {
    color: var(--accent);
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-row {
    display: flex;
    gap: 12px;
  }

  .form-group.half {
    flex: 1;
  }

  .form-label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    margin: 4px 0 0 28px;
  }

  /* Text/Number inputs */
  input[type="text"],
  input[type="number"] {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  input[type="text"]:focus,
  input[type="number"]:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(0, 102, 255, 0.1);
  }

  /* Radio buttons */
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .radio-group.horizontal {
    flex-direction: row;
    flex-wrap: wrap;
    gap: 12px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .radio-label input[type="radio"] {
    position: absolute;
    opacity: 0;
    cursor: pointer;
  }

  .radio-mark {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border);
    border-radius: 50%;
    background: var(--bg-tertiary);
    position: relative;
    transition: all 0.15s ease;
  }

  .radio-label:hover .radio-mark {
    border-color: var(--accent);
  }

  .radio-label input[type="radio"]:checked + .radio-mark {
    border-color: var(--accent);
    background: var(--accent);
  }

  .radio-label input[type="radio"]:checked + .radio-mark::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: white;
  }

  /* Checkboxes */
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    cursor: pointer;
  }

  .checkmark {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border);
    border-radius: 4px;
    background: var(--bg-tertiary);
    position: relative;
    transition: all 0.15s ease;
  }

  .checkbox-label:hover .checkmark {
    border-color: var(--accent);
  }

  .checkbox-label input[type="checkbox"]:checked + .checkmark {
    border-color: var(--accent);
    background: var(--accent);
  }

  .checkbox-label input[type="checkbox"]:checked + .checkmark::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 5px;
    width: 4px;
    height: 8px;
    border: solid white;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  .label-text {
    font-size: 13px;
    color: var(--text-primary);
  }

  /* Config path */
  .config-path {
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 12px !important;
    margin-top: 16px;
  }

  .config-path .path-label {
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .config-path code {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: 'SF Mono', 'Fira Code', monospace;
    word-break: break-all;
  }

  /* Footer */
  footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .message {
    padding: 10px 12px;
    border-radius: 6px;
    font-size: 13px;
    margin-bottom: 12px;
  }

  .message.success {
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .message.error {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .actions {
    display: flex;
    gap: 12px;
  }

  .btn {
    flex: 1;
    padding: 10px 16px;
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
</style>
