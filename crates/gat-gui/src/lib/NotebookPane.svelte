<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface NotebookDemo {
    title: string;
    description: string;
    path: string;
  }

  interface QuickAction {
    label: string;
    command: string;
    notes: string;
  }

  interface NotebookManifest {
    app: string;
    description: string;
    workspace: string;
    port: number;
    notebooks_dir: string;
    datasets_dir: string;
    context_dir: string;
    demos: NotebookDemo[];
    quick_actions: QuickAction[];
    status_badges: string[];
  }

  let { isOpen = $bindable(false) }: { isOpen: boolean } = $props();

  let manifest = $state<NotebookManifest | null>(null);
  let selectedDemo = $state<NotebookDemo | null>(null);
  let notebookContent = $state<string>("");
  let loading = $state(false);
  let error = $state<string | null>(null);
  let copiedCommand = $state<string | null>(null);

  // Load manifest when pane opens
  $effect(() => {
    if (isOpen && !manifest) {
      loadManifest();
    }
  });

  async function loadManifest() {
    loading = true;
    error = null;
    try {
      manifest = await invoke<NotebookManifest>("get_notebook_manifest");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function selectDemo(demo: NotebookDemo) {
    selectedDemo = demo;
    loading = true;
    try {
      notebookContent = await invoke<string>("read_notebook", { path: demo.path });
    } catch (e) {
      notebookContent = `# ${demo.title}\n\n${demo.description}\n\n*Notebook file not found. Initialize workspace to create demo notebooks.*`;
    } finally {
      loading = false;
    }
  }

  function backToList() {
    selectedDemo = null;
    notebookContent = "";
  }

  async function initWorkspace() {
    loading = true;
    try {
      await invoke<string>("init_notebook_workspace", { workspacePath: null });
      await loadManifest();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function copyCommand(command: string) {
    navigator.clipboard.writeText(command);
    copiedCommand = command;
    setTimeout(() => {
      copiedCommand = null;
    }, 2000);
  }

  // Icon for each demo type
  function getDemoIcon(title: string): string {
    if (title.toLowerCase().includes("power flow")) return "zap";
    if (title.toLowerCase().includes("batch") || title.toLowerCase().includes("scenario")) return "layers";
    if (title.toLowerCase().includes("validation") || title.toLowerCase().includes("cleanup")) return "check-circle";
    if (title.toLowerCase().includes("time") || title.toLowerCase().includes("forecast")) return "clock";
    if (title.toLowerCase().includes("contingency") || title.toLowerCase().includes("reliability")) return "shield";
    if (title.toLowerCase().includes("benchmark") || title.toLowerCase().includes("solver")) return "activity";
    if (title.toLowerCase().includes("geo") || title.toLowerCase().includes("spatial")) return "map";
    if (title.toLowerCase().includes("rag") || title.toLowerCase().includes("context")) return "database";
    if (title.toLowerCase().includes("rust") || title.toLowerCase().includes("script")) return "code";
    if (title.toLowerCase().includes("ingestion") || title.toLowerCase().includes("conversion")) return "download";
    if (title.toLowerCase().includes("tracking") || title.toLowerCase().includes("experiment")) return "git-branch";
    return "file-text";
  }
</script>

<div class="pane-overlay" class:visible={isOpen} onclick={() => (isOpen = false)} role="presentation">
  <div
    class="notebook-pane"
    class:open={isOpen}
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Notebook pane"
  >
    <div class="pane-header">
      <div class="header-content">
        {#if selectedDemo}
          <button class="back-btn" onclick={backToList}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
          </button>
        {/if}
        <div class="header-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
          </svg>
        </div>
        <div class="header-text">
          <h2>{selectedDemo ? selectedDemo.title : "Research Notebook"}</h2>
          <span class="subtitle">{selectedDemo ? selectedDemo.path : "Twinsong-inspired workflows"}</span>
        </div>
      </div>
      <button class="close-btn" onclick={() => (isOpen = false)}>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="pane-content">
      {#if loading}
        <div class="loading-state">
          <div class="spinner"></div>
          <span>Loading...</span>
        </div>
      {:else if error}
        <div class="error-state">
          <span class="error-icon">!</span>
          <span>{error}</span>
          <button onclick={loadManifest}>Retry</button>
        </div>
      {:else if selectedDemo}
        <!-- Notebook content view -->
        <div class="notebook-view">
          <div class="notebook-content">
            <pre>{notebookContent}</pre>
          </div>
        </div>
      {:else if manifest}
        <!-- Main manifest view -->
        <div class="manifest-view">
          <!-- Status badges -->
          <div class="status-section">
            {#each manifest.status_badges as badge}
              <span class="status-badge">{badge}</span>
            {/each}
          </div>

          <!-- Workspace info -->
          <div class="workspace-info">
            <div class="info-row">
              <span class="info-label">Workspace</span>
              <span class="info-value">{manifest.workspace}</span>
            </div>
            <div class="info-row">
              <span class="info-label">Port</span>
              <span class="info-value">{manifest.port}</span>
            </div>
          </div>

          <!-- Quick actions -->
          <div class="section">
            <h3>Quick Actions</h3>
            <div class="quick-actions">
              {#each manifest.quick_actions as action}
                <div class="quick-action">
                  <div class="action-header">
                    <span class="action-label">{action.label}</span>
                    <button
                      class="copy-btn"
                      class:copied={copiedCommand === action.command}
                      onclick={() => copyCommand(action.command)}
                    >
                      {copiedCommand === action.command ? "Copied!" : "Copy"}
                    </button>
                  </div>
                  <code class="action-command">{action.command}</code>
                  <span class="action-notes">{action.notes}</span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Demo notebooks -->
          <div class="section">
            <h3>Demo Notebooks</h3>
            <div class="demos-grid">
              {#each manifest.demos as demo}
                <button class="demo-card" onclick={() => selectDemo(demo)}>
                  <div class="demo-icon">
                    {#if getDemoIcon(demo.title) === "zap"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "layers"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polygon points="12 2 2 7 12 12 22 7 12 2" />
                        <polyline points="2 17 12 22 22 17" />
                        <polyline points="2 12 12 17 22 12" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "check-circle"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                        <polyline points="22 4 12 14.01 9 11.01" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "clock"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10" />
                        <polyline points="12 6 12 12 16 14" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "shield"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "activity"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "map"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polygon points="1 6 1 22 8 18 16 22 23 18 23 2 16 6 8 2 1 6" />
                        <line x1="8" y1="2" x2="8" y2="18" />
                        <line x1="16" y1="6" x2="16" y2="22" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "database"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <ellipse cx="12" cy="5" rx="9" ry="3" />
                        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
                        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "code"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="16 18 22 12 16 6" />
                        <polyline points="8 6 2 12 8 18" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "download"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" y1="15" x2="12" y2="3" />
                      </svg>
                    {:else if getDemoIcon(demo.title) === "git-branch"}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="6" y1="3" x2="6" y2="15" />
                        <circle cx="18" cy="6" r="3" />
                        <circle cx="6" cy="18" r="3" />
                        <path d="M18 9a9 9 0 0 1-9 9" />
                      </svg>
                    {:else}
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                        <polyline points="14 2 14 8 20 8" />
                        <line x1="16" y1="13" x2="8" y2="13" />
                        <line x1="16" y1="17" x2="8" y2="17" />
                        <polyline points="10 9 9 9 8 9" />
                      </svg>
                    {/if}
                  </div>
                  <div class="demo-info">
                    <span class="demo-title">{demo.title}</span>
                    <span class="demo-desc">{demo.description}</span>
                  </div>
                  <div class="demo-arrow">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M9 18l6-6-6-6" />
                    </svg>
                  </div>
                </button>
              {/each}
            </div>
          </div>

          <!-- Initialize button -->
          <div class="section">
            <button class="init-btn" onclick={initWorkspace}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 5v14M5 12h14" />
              </svg>
              Initialize / Refresh Workspace
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .pane-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--overlay-bg, rgba(0, 0, 0, 0.5));
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.2s ease, visibility 0.2s ease;
    z-index: 100;
  }

  .pane-overlay.visible {
    opacity: 1;
    visibility: visible;
  }

  .notebook-pane {
    position: fixed;
    top: 0;
    right: 0;
    width: 520px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    transform: translateX(100%);
    transition: transform 0.25s ease;
    display: flex;
    flex-direction: column;
    z-index: 101;
  }

  .notebook-pane.open {
    transform: translateX(0);
  }

  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .back-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }

  .back-btn:hover {
    color: var(--accent);
    background: var(--bg-secondary);
  }

  .header-icon {
    width: 36px;
    height: 36px;
    background: var(--accent);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .header-text h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .header-text .subtitle {
    font-size: 12px;
    color: var(--text-muted);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 8px;
    border-radius: 6px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .pane-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .loading-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px;
    color: var(--text-secondary);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-state .error-icon {
    width: 32px;
    height: 32px;
    background: var(--error);
    color: white;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
  }

  .error-state button {
    padding: 8px 16px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .status-section {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 16px;
  }

  .status-badge {
    padding: 4px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 12px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .workspace-info {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 20px;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
  }

  .info-label {
    font-size: 12px;
    color: var(--text-muted);
  }

  .info-value {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: monospace;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .section {
    margin-bottom: 24px;
  }

  .section h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .quick-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .quick-action {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
  }

  .action-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .action-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .copy-btn {
    padding: 4px 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .copy-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .copy-btn.copied {
    background: var(--success);
    border-color: var(--success);
    color: white;
  }

  .action-command {
    display: block;
    font-size: 11px;
    color: var(--accent);
    background: var(--code-bg);
    padding: 8px;
    border-radius: 4px;
    margin-bottom: 6px;
    overflow-x: auto;
    white-space: nowrap;
  }

  .action-notes {
    font-size: 11px;
    color: var(--text-muted);
  }

  .demos-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .demo-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    width: 100%;
  }

  .demo-card:hover {
    border-color: var(--accent);
    background: var(--bg-secondary);
  }

  .demo-icon {
    width: 36px;
    height: 36px;
    background: var(--bg-secondary);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .demo-info {
    flex: 1;
    min-width: 0;
  }

  .demo-title {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .demo-desc {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .demo-arrow {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .init-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 12px;
    background: var(--bg-tertiary);
    border: 1px dashed var(--border);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .init-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    border-style: solid;
  }

  /* Notebook content view */
  .notebook-view {
    height: 100%;
  }

  .notebook-content {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    overflow: auto;
    max-height: calc(100vh - 200px);
  }

  .notebook-content pre {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-wrap: break-word;
    margin: 0;
  }
</style>
