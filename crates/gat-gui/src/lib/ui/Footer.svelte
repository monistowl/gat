<script lang="ts">
  import type { Snippet } from "svelte";
  import { themeState } from "$lib/stores/theme.svelte";
  import Spinner from "./Spinner.svelte";

  type View = "grid" | "ybus" | "arch";

  interface Props {
    /** Current active view */
    activeView: View;
    /** Callback when view changes */
    onViewChange: (view: View) => void;
    /** Status message to display */
    status: string;
    /** Whether network is loaded */
    hasNetwork?: boolean;
    /** Whether something is loading */
    loading?: boolean;
    /** Toggle command builder drawer */
    onToggleCommandBuilder?: () => void;
    /** Toggle batch jobs drawer */
    onToggleBatch?: () => void;
    /** Toggle notebook drawer */
    onToggleNotebook?: () => void;
    /** Toggle PTDF panel */
    onTogglePtdf?: () => void;
    /** Toggle education drawer */
    onToggleEducation?: () => void;
    /** Toggle config panel */
    onToggleConfig?: () => void;
    /** Optional center slot for custom controls */
    center?: Snippet;
  }

  let {
    activeView,
    onViewChange,
    status,
    hasNetwork = false,
    loading = false,
    onToggleCommandBuilder,
    onToggleBatch,
    onToggleNotebook,
    onTogglePtdf,
    onToggleEducation,
    onToggleConfig,
    center,
  }: Props = $props();

  const views: { id: View; label: string }[] = [
    { id: "grid", label: "Grid" },
    { id: "ybus", label: "Y-bus" },
  ];
</script>

<footer class="footer">
  <!-- Left: View tabs -->
  <div class="footer-left">
    <div class="view-tabs">
      {#each views as view}
        <button
          class="tab"
          class:active={activeView === view.id}
          onclick={() => onViewChange(view.id)}
        >
          {view.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Center: Tool buttons -->
  <div class="footer-center">
    {#if center}
      {@render center()}
    {:else}
      {#if onToggleCommandBuilder}
        <button class="tool-btn" onclick={onToggleCommandBuilder} title="CLI Command Builder">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="4 17 10 11 4 5"/>
            <line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
          <span class="tool-label">CLI</span>
        </button>
      {/if}
      {#if onToggleBatch}
        <button class="tool-btn" onclick={onToggleBatch} title="Batch Job Runner">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
            <polyline points="7.5 4.21 12 6.81 16.5 4.21"/>
            <polyline points="7.5 19.79 7.5 14.6 3 12"/>
            <polyline points="21 12 16.5 14.6 16.5 19.79"/>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
            <line x1="12" y1="22.08" x2="12" y2="12"/>
          </svg>
          <span class="tool-label">Batch</span>
        </button>
      {/if}
      {#if onToggleNotebook}
        <button class="tool-btn" onclick={onToggleNotebook} title="Research Notebook">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
          </svg>
          <span class="tool-label">Notebook</span>
        </button>
      {/if}
      {#if onTogglePtdf}
        <button class="tool-btn" onclick={onTogglePtdf} title="PTDF Analysis">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 3v18h18"/>
            <path d="M18 9l-5 5-4-4-3 3"/>
          </svg>
          <span class="tool-label">PTDF</span>
        </button>
      {/if}
      <button
        class="tool-btn theme-toggle"
        onclick={() => themeState.toggle()}
        title="Theme: {themeState.preference} ({themeState.resolved})"
      >
        {#if themeState.preference === "system"}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
            <line x1="8" y1="21" x2="16" y2="21"/>
            <line x1="12" y1="17" x2="12" y2="21"/>
          </svg>
          <span class="tool-label">Auto</span>
        {:else if themeState.resolved === "dark"}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
          </svg>
          <span class="tool-label">Dark</span>
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
          <span class="tool-label">Light</span>
        {/if}
      </button>
    {/if}
  </div>

  <!-- Right: Additional controls + status -->
  <div class="footer-right">
    <button
      class="tab"
      class:active={activeView === "arch"}
      onclick={() => onViewChange("arch")}
    >
      Architecture
    </button>

    {#if onToggleEducation}
      <button class="icon-btn" onclick={onToggleEducation} title="Learn">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
          <path d="M12 17h.01"/>
        </svg>
      </button>
    {/if}

    {#if onToggleConfig}
      <button class="icon-btn" onclick={onToggleConfig} title="Settings">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
      </button>
    {/if}

    <div class="status">
      {#if loading}
        <Spinner size="sm" />
      {/if}
      <span class="status-text">{status}</span>
      {#if hasNetwork && !loading}
        <span class="status-ok">✓</span>
      {/if}
    </div>
  </div>
</footer>

<style>
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 0 var(--space-4);
    height: var(--footer-height, 44px);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .footer-left,
  .footer-center,
  .footer-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .footer-center {
    gap: var(--space-2);
  }

  .footer-right {
    margin-left: auto;
  }

  .view-tabs {
    display: flex;
    gap: 2px;
    background: var(--bg-tertiary);
    padding: 2px;
    border-radius: var(--radius-md);
  }

  .tab {
    padding: 6px 14px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    border-radius: var(--radius-sm);
    transition: all var(--transition-base);
    font-weight: 500;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--accent);
    color: white;
  }

  .tool-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
  }

  .tool-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .tool-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .tool-btn svg {
    opacity: 0.8;
  }

  .tool-btn:hover svg {
    opacity: 1;
  }

  .tool-label {
    display: none;
  }

  @media (min-width: 800px) {
    .tool-label {
      display: inline;
    }
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
  }

  .icon-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .icon-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--text-muted);
    font-family: "SF Mono", "Fira Code", monospace;
    max-width: 300px;
    overflow: hidden;
  }

  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-ok {
    color: var(--success);
    flex-shrink: 0;
  }
</style>
