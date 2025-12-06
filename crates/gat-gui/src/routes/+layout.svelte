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

  /* Spacing scale */
  :global(:root) {
    --space-1: 4px;
    --space-2: 8px;
    --space-3: 12px;
    --space-4: 16px;
    --space-5: 20px;
    --space-6: 24px;
    --space-8: 32px;
    --space-10: 40px;

    /* Border radii */
    --radius-sm: 4px;
    --radius-md: 6px;
    --radius-lg: 8px;
    --radius-xl: 12px;
    --radius-full: 999px;

    /* Transitions */
    --transition-fast: 0.1s ease;
    --transition-base: 0.15s ease;
    --transition-slow: 0.25s ease;

    /* Sidebar and layout */
    --sidebar-width: 220px;
    --footer-height: 44px;
    --drawer-width: 380px;

    /* Focus ring for accessibility */
    --focus-ring: 0 0 0 2px var(--accent);

    /* Backdrop blur for overlays */
    --backdrop-blur: blur(8px);
  }

  /* Dark theme (default) */
  :global(:root),
  :global([data-theme="dark"]) {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --accent-subtle: rgba(0, 102, 255, 0.15);
    --bg-primary: #0a0a0f;
    --bg-secondary: #111118;
    --bg-tertiary: #1a1a24;
    --bg-elevated: #1f1f2a;
    --border: #27272a;
    --border-subtle: #1e1e24;
    --text-primary: #e4e4e7;
    --text-secondary: #a1a1aa;
    --text-muted: #71717a;
    --success: #22c55e;
    --success-subtle: rgba(34, 197, 94, 0.15);
    --warning: #f59e0b;
    --warning-subtle: rgba(245, 158, 11, 0.15);
    --error: #ef4444;
    --error-subtle: rgba(239, 68, 68, 0.15);

    /* Component-specific */
    --code-bg: #1e1e2e;
    --card-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    --card-shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.5);
    --overlay-bg: rgba(0, 0, 0, 0.6);
    --drawer-shadow: -4px 0 24px rgba(0, 0, 0, 0.4);
  }

  /* Light theme */
  :global([data-theme="light"]) {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --accent-subtle: rgba(0, 102, 255, 0.1);
    --bg-primary: #ffffff;
    --bg-secondary: #f8f9fa;
    --bg-tertiary: #f1f3f5;
    --bg-elevated: #ffffff;
    --border: #dee2e6;
    --border-subtle: #e9ecef;
    --text-primary: #1a1a2e;
    --text-secondary: #495057;
    --text-muted: #868e96;
    --success: #22c55e;
    --success-subtle: rgba(34, 197, 94, 0.1);
    --warning: #f59e0b;
    --warning-subtle: rgba(245, 158, 11, 0.1);
    --error: #ef4444;
    --error-subtle: rgba(239, 68, 68, 0.1);

    /* Component-specific */
    --code-bg: #f8f9fa;
    --card-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    --card-shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.12);
    --overlay-bg: rgba(0, 0, 0, 0.3);
    --drawer-shadow: -4px 0 24px rgba(0, 0, 0, 0.15);
  }

  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
  }
</style>
