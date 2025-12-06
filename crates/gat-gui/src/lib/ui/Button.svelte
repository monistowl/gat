<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Visual style variant */
    variant?: "primary" | "secondary" | "ghost" | "danger";
    /** Size preset */
    size?: "sm" | "md" | "lg";
    /** Icon-only button (square shape) */
    iconOnly?: boolean;
    /** Active/selected state */
    active?: boolean;
    /** Disabled state */
    disabled?: boolean;
    /** Full width button */
    fullWidth?: boolean;
    /** Optional title attribute for tooltips */
    title?: string;
    /** Click handler */
    onclick?: (e: MouseEvent) => void;
    /** Button content */
    children?: Snippet;
  }

  let {
    variant = "secondary",
    size = "md",
    iconOnly = false,
    active = false,
    disabled = false,
    fullWidth = false,
    title,
    onclick,
    children,
  }: Props = $props();
</script>

<button
  class="btn btn-{variant} btn-{size}"
  class:icon-only={iconOnly}
  class:active
  class:full-width={fullWidth}
  {disabled}
  {title}
  {onclick}
>
  {#if children}
    {@render children()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
    font-weight: 500;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Sizes */
  .btn-sm {
    padding: 4px 10px;
    font-size: 12px;
    border-radius: 4px;
  }

  .btn-md {
    padding: 6px 14px;
    font-size: 13px;
  }

  .btn-lg {
    padding: 10px 20px;
    font-size: 14px;
  }

  /* Icon-only variants */
  .icon-only.btn-sm {
    width: 24px;
    height: 24px;
    padding: 0;
  }

  .icon-only.btn-md {
    width: 32px;
    height: 32px;
    padding: 0;
  }

  .icon-only.btn-lg {
    width: 40px;
    height: 40px;
    padding: 0;
  }

  /* Variants */
  .btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover, color-mix(in srgb, var(--accent), black 15%));
    border-color: var(--accent-hover, color-mix(in srgb, var(--accent), black 15%));
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    border-color: var(--border);
    color: var(--text-secondary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn-ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-danger {
    background: transparent;
    border-color: var(--error);
    color: var(--error);
  }

  .btn-danger:hover:not(:disabled) {
    background: var(--error);
    color: white;
  }

  /* Active state - used for tab-like selection */
  .active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .active:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
  }

  /* Full width */
  .full-width {
    width: 100%;
  }

  /* SVG icon styling */
  .btn :global(svg) {
    flex-shrink: 0;
    opacity: 0.8;
  }

  .btn:hover:not(:disabled) :global(svg) {
    opacity: 1;
  }
</style>
