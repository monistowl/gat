<script lang="ts">
  interface Props {
    /** Size of the spinner */
    size?: "sm" | "md" | "lg";
    /** Optional label text */
    label?: string;
  }

  let {
    size = "md",
    label,
  }: Props = $props();

  const sizeMap = {
    sm: 12,
    md: 16,
    lg: 24,
  };
</script>

<span class="spinner-container spinner-{size}">
  <span
    class="spinner"
    style:width="{sizeMap[size]}px"
    style:height="{sizeMap[size]}px"
  ></span>
  {#if label}
    <span class="spinner-label">{label}</span>
  {/if}
</span>

<style>
  .spinner-container {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .spinner {
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner-sm .spinner {
    border-width: 1.5px;
  }

  .spinner-lg .spinner {
    border-width: 3px;
  }

  .spinner-label {
    font-size: 12px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .spinner-lg .spinner-label {
    font-size: 14px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
