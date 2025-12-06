<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Visual style variant */
    variant?: "default" | "success" | "warning" | "error" | "info";
    /** Size preset */
    size?: "sm" | "md";
    /** Pulsing animation for active states */
    pulse?: boolean;
    /** Badge content */
    children?: Snippet;
  }

  let {
    variant = "default",
    size = "md",
    pulse = false,
    children,
  }: Props = $props();
</script>

<span class="badge badge-{variant} badge-{size}" class:pulse>
  {#if children}
    {@render children()}
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border-radius: 999px;
    font-weight: 500;
    font-family: 'SF Mono', 'Fira Code', monospace;
    white-space: nowrap;
  }

  /* Sizes */
  .badge-sm {
    padding: 2px 8px;
    font-size: 10px;
  }

  .badge-md {
    padding: 4px 12px;
    font-size: 11px;
  }

  /* Variants */
  .badge-default {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .badge-success {
    background: color-mix(in srgb, var(--success), transparent 85%);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success), transparent 70%);
  }

  .badge-warning {
    background: color-mix(in srgb, var(--warning), transparent 85%);
    color: var(--warning);
    border: 1px solid color-mix(in srgb, var(--warning), transparent 70%);
  }

  .badge-error {
    background: color-mix(in srgb, var(--error), transparent 85%);
    color: var(--error);
    border: 1px solid color-mix(in srgb, var(--error), transparent 70%);
  }

  .badge-info {
    background: color-mix(in srgb, var(--accent), transparent 85%);
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent), transparent 70%);
  }

  /* Pulsing animation for active indicators */
  .pulse {
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
