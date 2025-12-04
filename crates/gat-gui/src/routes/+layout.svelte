<script lang="ts">
  import type { Snippet } from 'svelte';
  import { themeState } from '$lib/stores/theme.svelte';

  let { children }: { children: Snippet } = $props();

  // Apply theme class to document
  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-theme', themeState.resolved);
    }
  });
</script>

<div class="app">
  {@render children()}
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(html, body) {
    height: 100%;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 14px;
    line-height: 1.5;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    transition: background-color 0.2s ease, color 0.2s ease;
  }

  /* Dark theme (default) */
  :global(:root),
  :global([data-theme="dark"]) {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --bg-primary: #0a0a0f;
    --bg-secondary: #111118;
    --bg-tertiary: #1a1a24;
    --border: #27272a;
    --text-primary: #e4e4e7;
    --text-secondary: #a1a1aa;
    --text-muted: #71717a;
    --success: #22c55e;
    --warning: #f59e0b;
    --error: #ef4444;

    /* Component-specific */
    --code-bg: #1e1e2e;
    --card-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    --overlay-bg: rgba(0, 0, 0, 0.5);
  }

  /* Light theme */
  :global([data-theme="light"]) {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --bg-primary: #ffffff;
    --bg-secondary: #f8f9fa;
    --bg-tertiary: #f1f3f5;
    --border: #dee2e6;
    --text-primary: #1a1a2e;
    --text-secondary: #495057;
    --text-muted: #868e96;
    --success: #22c55e;
    --warning: #f59e0b;
    --error: #ef4444;

    /* Component-specific */
    --code-bg: #f8f9fa;
    --card-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    --overlay-bg: rgba(0, 0, 0, 0.3);
  }

  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
  }
</style>
