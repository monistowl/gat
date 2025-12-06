<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    /** Currently loaded network data */
    network: { buses: unknown[]; branches: unknown[] } | null;
    /** Current case path */
    casePath: string | null;
    /** Callback when action completes */
    onActionComplete?: (action: string, result: unknown) => void;
    /** Callback for status updates */
    onStatus?: (message: string) => void;
  }

  let {
    network,
    casePath,
    onActionComplete,
    onStatus,
  }: Props = $props();

  // Quick action definitions
  interface QuickAction {
    id: string;
    label: string;
    icon: string;
    shortcut?: string;
    description: string;
    requiresNetwork: boolean;
    category: "solve" | "analyze" | "export";
  }

  const QUICK_ACTIONS: QuickAction[] = [
    // Solve actions
    {
      id: "solve_dc",
      label: "DC PF",
      icon: "⚡",
      shortcut: "D",
      description: "Run DC power flow (fast, linear)",
      requiresNetwork: true,
      category: "solve",
    },
    {
      id: "solve_ac",
      label: "AC PF",
      icon: "⚡",
      shortcut: "A",
      description: "Run AC power flow (Newton-Raphson)",
      requiresNetwork: true,
      category: "solve",
    },
    {
      id: "solve_opf",
      label: "DC-OPF",
      icon: "🎯",
      shortcut: "O",
      description: "Run DC optimal power flow",
      requiresNetwork: true,
      category: "solve",
    },
    // Analyze actions
    {
      id: "run_n1",
      label: "N-1",
      icon: "🔍",
      shortcut: "N",
      description: "Run N-1 contingency screening",
      requiresNetwork: true,
      category: "analyze",
    },
    {
      id: "calc_ptdf",
      label: "PTDF",
      icon: "📊",
      description: "Calculate PTDF matrix",
      requiresNetwork: true,
      category: "analyze",
    },
    {
      id: "check_voltage",
      label: "V Check",
      icon: "📏",
      description: "Check for voltage violations",
      requiresNetwork: true,
      category: "analyze",
    },
    // Export actions
    {
      id: "export_json",
      label: "→ JSON",
      icon: "📄",
      description: "Export results to JSON",
      requiresNetwork: true,
      category: "export",
    },
    {
      id: "export_csv",
      label: "→ CSV",
      icon: "📋",
      description: "Export to CSV files",
      requiresNetwork: true,
      category: "export",
    },
  ];

  // State
  let runningAction = $state<string | null>(null);
  let lastResult = $state<{ action: string; success: boolean; time: number } | null>(null);

  // Execute quick action
  async function executeAction(action: QuickAction) {
    if (!casePath || runningAction) return;

    runningAction = action.id;
    onStatus?.(`Running ${action.label}...`);

    const startTime = performance.now();

    try {
      let result: unknown;

      switch (action.id) {
        case "solve_dc":
          result = await invoke("solve_dc_power_flow", { path: casePath });
          break;
        case "solve_ac":
          result = await invoke("solve_power_flow", { path: casePath });
          break;
        case "solve_opf":
          result = await invoke("solve_dc_opf", { path: casePath });
          break;
        case "run_n1":
          result = await invoke("run_n1_contingency", { path: casePath });
          break;
        case "calc_ptdf":
          result = await invoke("calculate_ptdf", { path: casePath });
          break;
        case "check_voltage":
          result = checkVoltageViolations();
          break;
        case "export_json":
          result = await invoke("export_to_json", { path: casePath });
          break;
        case "export_csv":
          result = await invoke("export_to_csv", { path: casePath });
          break;
        default:
          throw new Error(`Unknown action: ${action.id}`);
      }

      const elapsed = Math.round(performance.now() - startTime);
      lastResult = { action: action.id, success: true, time: elapsed };
      onStatus?.(`${action.label} completed in ${elapsed}ms`);
      onActionComplete?.(action.id, result);
    } catch (e) {
      const elapsed = Math.round(performance.now() - startTime);
      lastResult = { action: action.id, success: false, time: elapsed };
      onStatus?.(`${action.label} failed: ${e}`);
    } finally {
      runningAction = null;
    }
  }

  // Local voltage check (doesn't need invoke)
  function checkVoltageViolations(): { violations: number; buses: number[] } {
    if (!network) return { violations: 0, buses: [] };

    const violations: number[] = [];
    for (const bus of network.buses as { id: number; vm: number }[]) {
      if (bus.vm < 0.95 || bus.vm > 1.05) {
        violations.push(bus.id);
      }
    }

    return { violations: violations.length, buses: violations };
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (!network || runningAction) return;

    const action = QUICK_ACTIONS.find(
      (a) => a.shortcut?.toLowerCase() === e.key.toLowerCase()
    );
    if (action) {
      e.preventDefault();
      executeAction(action);
    }
  }

  // Group actions by category
  let solveActions = $derived(QUICK_ACTIONS.filter((a) => a.category === "solve"));
  let analyzeActions = $derived(QUICK_ACTIONS.filter((a) => a.category === "analyze"));
  let exportActions = $derived(QUICK_ACTIONS.filter((a) => a.category === "export"));
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="quick-actions">
  <div class="action-group">
    <span class="group-label">Solve</span>
    {#each solveActions as action}
      <button
        class="action-btn"
        class:running={runningAction === action.id}
        class:success={lastResult?.action === action.id && lastResult.success}
        class:error={lastResult?.action === action.id && !lastResult.success}
        disabled={!network || runningAction !== null}
        onclick={() => executeAction(action)}
        title="{action.description}{action.shortcut ? ` (${action.shortcut})` : ''}"
      >
        <span class="action-icon">{action.icon}</span>
        <span class="action-label">{action.label}</span>
        {#if action.shortcut}
          <kbd class="shortcut">{action.shortcut}</kbd>
        {/if}
      </button>
    {/each}
  </div>

  <div class="divider"></div>

  <div class="action-group">
    <span class="group-label">Analyze</span>
    {#each analyzeActions as action}
      <button
        class="action-btn"
        class:running={runningAction === action.id}
        disabled={!network || runningAction !== null}
        onclick={() => executeAction(action)}
        title={action.description}
      >
        <span class="action-icon">{action.icon}</span>
        <span class="action-label">{action.label}</span>
      </button>
    {/each}
  </div>

  <div class="divider"></div>

  <div class="action-group">
    <span class="group-label">Export</span>
    {#each exportActions as action}
      <button
        class="action-btn"
        class:running={runningAction === action.id}
        disabled={!network || runningAction !== null}
        onclick={() => executeAction(action)}
        title={action.description}
      >
        <span class="action-icon">{action.icon}</span>
        <span class="action-label">{action.label}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .quick-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
  }

  .action-group {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .group-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-right: var(--space-1);
  }

  .divider {
    width: 1px;
    height: 24px;
    background: var(--border);
    margin: 0 var(--space-2);
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    transition: all var(--transition-base);
    position: relative;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .action-btn.running {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .action-btn.running::after {
    content: "";
    position: absolute;
    inset: -2px;
    border: 2px solid var(--accent);
    border-radius: var(--radius-md);
    animation: pulse-ring 1s ease-out infinite;
  }

  @keyframes pulse-ring {
    0% {
      opacity: 1;
      transform: scale(1);
    }
    100% {
      opacity: 0;
      transform: scale(1.2);
    }
  }

  .action-btn.success {
    border-color: var(--success);
  }

  .action-btn.error {
    border-color: var(--error);
  }

  .action-icon {
    font-size: 14px;
  }

  .action-label {
    font-weight: 500;
  }

  .shortcut {
    font-size: 9px;
    padding: 1px 4px;
    background: var(--bg-primary);
    border-radius: 2px;
    color: var(--text-muted);
    font-family: "SF Mono", monospace;
    margin-left: 2px;
  }

  .action-btn:hover:not(:disabled) .shortcut {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }
</style>
