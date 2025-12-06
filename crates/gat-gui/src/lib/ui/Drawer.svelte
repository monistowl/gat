<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    /** Whether the drawer is open */
    isOpen: boolean;
    /** Callback when drawer should close */
    onClose: () => void;
    /** Drawer title */
    title?: string;
    /** Width of the drawer */
    width?: "sm" | "md" | "lg" | "xl";
    /** Position of the drawer */
    position?: "left" | "right";
    /** Show backdrop overlay */
    showBackdrop?: boolean;
    /** Close on backdrop click */
    closeOnBackdropClick?: boolean;
    /** Close on Escape key */
    closeOnEscape?: boolean;
    /** Header slot for custom header content */
    header?: Snippet;
    /** Footer slot */
    footer?: Snippet;
    /** Main content slot */
    children?: Snippet;
  }

  let {
    isOpen,
    onClose,
    title,
    width = "md",
    position = "right",
    showBackdrop = true,
    closeOnBackdropClick = true,
    closeOnEscape = true,
    header,
    footer,
    children,
  }: Props = $props();

  const widthMap = {
    sm: "320px",
    md: "400px",
    lg: "480px",
    xl: "600px",
  };

  // Handle escape key
  function handleKeydown(e: KeyboardEvent) {
    if (closeOnEscape && e.key === "Escape" && isOpen) {
      onClose();
    }
  }

  // Handle backdrop click
  function handleBackdropClick() {
    if (closeOnBackdropClick) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if showBackdrop}
  <div
    class="drawer-backdrop"
    class:open={isOpen}
    onclick={handleBackdropClick}
    role="presentation"
  ></div>
{/if}

<aside
  class="drawer drawer-{position}"
  class:open={isOpen}
  style:--drawer-width={widthMap[width]}
  role="dialog"
  aria-modal="true"
  aria-label={title}
>
  <div class="drawer-header">
    {#if header}
      {@render header()}
    {:else if title}
      <h2 class="drawer-title">{title}</h2>
    {/if}
    <button
      class="close-btn"
      onclick={onClose}
      aria-label="Close drawer"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>

  <div class="drawer-content">
    {#if children}
      {@render children()}
    {/if}
  </div>

  {#if footer}
    <div class="drawer-footer">
      {@render footer()}
    </div>
  {/if}
</aside>

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    backdrop-filter: var(--backdrop-blur);
    -webkit-backdrop-filter: var(--backdrop-blur);
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--transition-slow), visibility var(--transition-slow);
    z-index: 998;
  }

  .drawer-backdrop.open {
    opacity: 1;
    visibility: visible;
  }

  .drawer {
    position: fixed;
    top: 0;
    bottom: 0;
    width: var(--drawer-width, 400px);
    max-width: 90vw;
    background: var(--bg-secondary);
    box-shadow: var(--drawer-shadow);
    transition: transform var(--transition-slow);
    z-index: 999;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .drawer-right {
    right: 0;
    border-left: 1px solid var(--border);
    transform: translateX(100%);
  }

  .drawer-left {
    left: 0;
    border-right: 1px solid var(--border);
    transform: translateX(-100%);
  }

  .drawer.open {
    transform: translateX(0);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
    flex-shrink: 0;
  }

  .drawer-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
    flex-shrink: 0;
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    border-color: var(--border);
    color: var(--text-primary);
  }

  .close-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .drawer-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-6);
  }

  .drawer-footer {
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border);
    background: var(--bg-tertiary);
    flex-shrink: 0;
  }

  /* Smooth scrolling and touch support */
  .drawer-content {
    -webkit-overflow-scrolling: touch;
    overscroll-behavior: contain;
  }
</style>
