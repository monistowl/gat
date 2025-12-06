<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  // Props
  let {
    isOpen,
    onClose,
  }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  // Workflow definitions
  interface WorkflowStep {
    id: string;
    name: string;
    description: string;
    command: string;
    optional?: boolean;
  }

  interface Workflow {
    id: string;
    name: string;
    icon: string;
    description: string;
    category: "analysis" | "distribution" | "reliability" | "data";
    steps: WorkflowStep[];
    estimatedTime: string;
    outputs: string[];
  }

  const WORKFLOWS: Workflow[] = [
    {
      id: "full_analysis",
      name: "Full Grid Analysis",
      icon: "🔬",
      description: "Complete analysis pipeline: power flow, OPF, contingency screening, and sensitivity analysis",
      category: "analysis",
      estimatedTime: "2-5 min",
      steps: [
        { id: "pf_dc", name: "DC Power Flow", description: "Fast linear approximation", command: "gat pf dc {input} -o {output}/pf_dc.json" },
        { id: "pf_ac", name: "AC Power Flow", description: "Full Newton-Raphson solution", command: "gat pf ac {input} -o {output}/pf_ac.json" },
        { id: "opf_dc", name: "DC-OPF", description: "Economic dispatch optimization", command: "gat opf dc {input} -o {output}/opf_dc.json" },
        { id: "n1", name: "N-1 Contingency", description: "Security screening", command: "gat nminus1 dc {input} -o {output}/n1.json" },
        { id: "ptdf", name: "PTDF Analysis", description: "Transfer sensitivity factors", command: "gat analytics ptdf {input} -o {output}/ptdf.csv", optional: true },
      ],
      outputs: ["Power flow results", "Optimal dispatch", "Contingency violations", "Sensitivity matrix"],
    },
    {
      id: "quick_check",
      name: "Quick Validation",
      icon: "✅",
      description: "Fast validation: import, validate, and run DC power flow",
      category: "analysis",
      estimatedTime: "< 30 sec",
      steps: [
        { id: "validate", name: "Validate Network", description: "Check data integrity", command: "gat validate {input}" },
        { id: "islands", name: "Check Islands", description: "Detect disconnected components", command: "gat graph islands {input}" },
        { id: "pf_dc", name: "DC Power Flow", description: "Quick solvability test", command: "gat pf dc {input} -o {output}/quick_pf.json" },
      ],
      outputs: ["Validation report", "Island detection", "Solvability status"],
    },
    {
      id: "derms_analysis",
      name: "DER Flexibility Study",
      icon: "🔋",
      description: "Analyze distributed energy resources: flexibility envelopes, hosting capacity, and stress tests",
      category: "distribution",
      estimatedTime: "3-8 min",
      steps: [
        { id: "base_pf", name: "Base Case PF", description: "Establish baseline voltages", command: "gat dist pf {input} -o {output}/base_pf.json" },
        { id: "envelope", name: "DER Envelopes", description: "Calculate P-Q flexibility bounds", command: "gat derms envelope {input} -o {output}/envelopes.json" },
        { id: "hostcap", name: "Hosting Capacity", description: "Maximum DER penetration analysis", command: "gat dist hostcap {input} -o {output}/hosting.json" },
        { id: "stress", name: "Stress Test", description: "Max export/import scenarios", command: "gat derms stress-test {input} --scenario max-export -o {output}/stress.json", optional: true },
      ],
      outputs: ["Flexibility envelopes", "Hosting capacity by bus", "Stress test results"],
    },
    {
      id: "reliability_study",
      name: "Resource Adequacy",
      icon: "📊",
      description: "Reliability assessment: LOLE, EUE, and ELCC calculations for resource planning",
      category: "reliability",
      estimatedTime: "5-15 min",
      steps: [
        { id: "gen_summary", name: "Generation Fleet", description: "Analyze installed capacity", command: "gat inspect generators {input} --format json -o {output}/generators.json" },
        { id: "reliability", name: "LOLE/EUE Calculation", description: "Monte Carlo reliability simulation", command: "gat analytics reliability {input} -o {output}/reliability.json" },
        { id: "elcc", name: "ELCC Estimation", description: "Capacity credit for renewables", command: "gat analytics elcc {input} -o {output}/elcc.json", optional: true },
      ],
      outputs: ["LOLE (days/year)", "EUE (MWh/year)", "ELCC by resource"],
    },
    {
      id: "data_pipeline",
      name: "Data Export Pipeline",
      icon: "📤",
      description: "Export grid data to multiple formats for external tools and ML workflows",
      category: "data",
      estimatedTime: "1-2 min",
      steps: [
        { id: "export_csv", name: "Export to CSV", description: "Spreadsheet-friendly format", command: "gat inspect buses {input} --format csv -o {output}/buses.csv && gat inspect branches {input} --format csv -o {output}/branches.csv && gat inspect generators {input} --format csv -o {output}/generators.csv" },
        { id: "export_pm", name: "Export to PowerModels", description: "Julia ecosystem format", command: "gat convert {input} -f powermodels -o {output}/network.json" },
        { id: "export_matpower", name: "Export to MATPOWER", description: "MATLAB format", command: "gat convert {input} -f matpower -o {output}/network.m", optional: true },
      ],
      outputs: ["CSV files (buses, branches, generators)", "PowerModels JSON", "MATPOWER .m file"],
    },
    {
      id: "benchmark_solvers",
      name: "Solver Comparison",
      icon: "⏱️",
      description: "Benchmark different OPF solution methods for performance comparison",
      category: "analysis",
      estimatedTime: "2-5 min",
      steps: [
        { id: "dc_opf", name: "DC-OPF (Linear)", description: "Fastest, approximate", command: "gat opf dc {input} -o {output}/bench_dc.json" },
        { id: "socp", name: "SOCP Relaxation", description: "Convex relaxation", command: "gat opf dc {input} --method socp -o {output}/bench_socp.json" },
        { id: "ac_fd", name: "AC Fast-Decoupled", description: "Iterative approximation", command: "gat opf ac {input} -o {output}/bench_ac_fd.json" },
        { id: "ac_nlp", name: "AC-NLP (Full)", description: "Nonlinear optimization", command: "gat opf ac-nlp {input} -o {output}/bench_ac_nlp.json", optional: true },
      ],
      outputs: ["Solve times", "Objective values", "Convergence status"],
    },
  ];

  // State
  let selectedWorkflow = $state<Workflow | null>(null);
  let inputFile = $state<string>("");
  let outputDir = $state<string>("");
  let enabledSteps = $state<Set<string>>(new Set());
  let currentStep = $state<number>(-1);
  let stepResults = $state<Map<string, StepResult>>(new Map());
  let isRunning = $state(false);
  let error = $state<string | null>(null);

  interface StepResult {
    status: "pending" | "running" | "success" | "error" | "skipped";
    duration_ms?: number;
    error?: string;
  }

  // Category labels
  const categoryLabels: Record<string, { label: string; color: string }> = {
    analysis: { label: "Analysis", color: "var(--accent)" },
    distribution: { label: "Distribution", color: "var(--success)" },
    reliability: { label: "Reliability", color: "var(--warning)" },
    data: { label: "Data", color: "var(--text-muted)" },
  };

  // Initialize enabled steps when workflow changes
  $effect(() => {
    if (selectedWorkflow) {
      const newEnabled = new Set<string>();
      selectedWorkflow.steps.forEach((step) => {
        if (!step.optional) {
          newEnabled.add(step.id);
        }
      });
      enabledSteps = newEnabled;
      stepResults = new Map();
      currentStep = -1;
    }
  });

  // File/directory pickers
  async function pickInputFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: "Grid Files", extensions: ["arrow", "parquet", "m", "raw", "json"] },
        ],
        title: "Select Grid File",
      });
      if (selected) {
        inputFile = selected as string;
      }
    } catch (e) {
      console.error("Failed to open file picker:", e);
    }
  }

  async function pickOutputDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Output Directory",
      });
      if (selected) {
        outputDir = selected as string;
      }
    } catch (e) {
      console.error("Failed to open directory picker:", e);
    }
  }

  // Toggle optional step
  function toggleStep(stepId: string) {
    const newEnabled = new Set(enabledSteps);
    if (newEnabled.has(stepId)) {
      newEnabled.delete(stepId);
    } else {
      newEnabled.add(stepId);
    }
    enabledSteps = newEnabled;
  }

  // Run workflow
  async function runWorkflow() {
    if (!selectedWorkflow || !inputFile || !outputDir) return;

    isRunning = true;
    error = null;
    stepResults = new Map();

    const stepsToRun = selectedWorkflow.steps.filter((s) => enabledSteps.has(s.id));

    for (let i = 0; i < stepsToRun.length; i++) {
      const step = stepsToRun[i];
      currentStep = i;

      // Update status to running
      stepResults = new Map(stepResults).set(step.id, { status: "running" });

      const startTime = performance.now();

      try {
        // Build command with substitutions
        const command = step.command
          .replace("{input}", inputFile)
          .replace("{output}", outputDir);

        // Execute via Tauri backend
        await invoke("run_cli_command", { command });

        const duration = Math.round(performance.now() - startTime);
        stepResults = new Map(stepResults).set(step.id, {
          status: "success",
          duration_ms: duration,
        });
      } catch (e) {
        const duration = Math.round(performance.now() - startTime);
        stepResults = new Map(stepResults).set(step.id, {
          status: "error",
          duration_ms: duration,
          error: String(e),
        });
        error = `Step "${step.name}" failed: ${e}`;
        break;
      }
    }

    // Mark skipped steps
    selectedWorkflow.steps.forEach((step) => {
      if (!enabledSteps.has(step.id) && !stepResults.has(step.id)) {
        stepResults = new Map(stepResults).set(step.id, { status: "skipped" });
      }
    });

    currentStep = -1;
    isRunning = false;
  }

  // Get status icon
  function getStatusIcon(status: StepResult["status"]): string {
    switch (status) {
      case "pending":
        return "○";
      case "running":
        return "◐";
      case "success":
        return "✓";
      case "error":
        return "✗";
      case "skipped":
        return "–";
    }
  }

  // Get status color
  function getStatusColor(status: StepResult["status"]): string {
    switch (status) {
      case "pending":
        return "var(--text-muted)";
      case "running":
        return "var(--accent)";
      case "success":
        return "var(--success)";
      case "error":
        return "var(--error)";
      case "skipped":
        return "var(--text-muted)";
    }
  }

  // Computed
  let canRun = $derived(inputFile && outputDir && enabledSteps.size > 0);
  let completedCount = $derived(
    Array.from(stepResults.values()).filter((r) => r.status === "success").length
  );
  let totalEnabled = $derived(enabledSteps.size);
</script>

<!-- Backdrop -->
<div
  class="backdrop"
  class:open={isOpen}
  onclick={onClose}
  role="presentation"
></div>

<!-- Drawer -->
<aside class="drawer" class:open={isOpen}>
  <div class="drawer-header">
    <h2>Workflows</h2>
    <button class="close-btn" onclick={onClose} aria-label="Close workflows panel">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>

  <div class="drawer-content">
    {#if !selectedWorkflow}
      <!-- Workflow Selection -->
      <div class="workflow-grid">
        {#each WORKFLOWS as workflow}
          <button
            class="workflow-card"
            onclick={() => (selectedWorkflow = workflow)}
          >
            <div class="workflow-icon">{workflow.icon}</div>
            <div class="workflow-info">
              <h3>{workflow.name}</h3>
              <p>{workflow.description}</p>
              <div class="workflow-meta">
                <span
                  class="category-badge"
                  style:--badge-color={categoryLabels[workflow.category].color}
                >
                  {categoryLabels[workflow.category].label}
                </span>
                <span class="time-estimate">~{workflow.estimatedTime}</span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    {:else}
      <!-- Workflow Configuration -->
      <div class="workflow-config">
        <button class="back-btn" onclick={() => (selectedWorkflow = null)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
          Back to workflows
        </button>

        <div class="workflow-header">
          <span class="workflow-icon-lg">{selectedWorkflow.icon}</span>
          <div>
            <h3>{selectedWorkflow.name}</h3>
            <p>{selectedWorkflow.description}</p>
          </div>
        </div>

        <!-- Input/Output Selection -->
        <div class="config-section">
          <h4>Files</h4>
          <div class="file-row">
            <label for="input-grid">Input Grid</label>
            <div class="file-input">
              <input
                id="input-grid"
                type="text"
                bind:value={inputFile}
                placeholder="Select grid file..."
                readonly
              />
              <button onclick={pickInputFile}>Browse</button>
            </div>
          </div>
          <div class="file-row">
            <label for="output-dir">Output Directory</label>
            <div class="file-input">
              <input
                id="output-dir"
                type="text"
                bind:value={outputDir}
                placeholder="Select output directory..."
                readonly
              />
              <button onclick={pickOutputDir}>Browse</button>
            </div>
          </div>
        </div>

        <!-- Steps -->
        <div class="config-section">
          <h4>Steps ({totalEnabled} enabled)</h4>
          <div class="steps-list">
            {#each selectedWorkflow.steps as step, i}
              {@const result = stepResults.get(step.id)}
              {@const isEnabled = enabledSteps.has(step.id)}
              <div
                class="step-item"
                class:optional={step.optional}
                class:disabled={!isEnabled}
                class:running={result?.status === "running"}
              >
                <div class="step-number">
                  {#if result}
                    <span style:color={getStatusColor(result.status)}>
                      {getStatusIcon(result.status)}
                    </span>
                  {:else}
                    <span class="step-index">{i + 1}</span>
                  {/if}
                </div>
                <div class="step-info">
                  <div class="step-name">
                    {step.name}
                    {#if step.optional}
                      <span class="optional-badge">optional</span>
                    {/if}
                  </div>
                  <div class="step-desc">{step.description}</div>
                  {#if result?.duration_ms}
                    <div class="step-time">{result.duration_ms}ms</div>
                  {/if}
                  {#if result?.error}
                    <div class="step-error">{result.error}</div>
                  {/if}
                </div>
                {#if step.optional}
                  <button
                    class="toggle-btn"
                    class:enabled={isEnabled}
                    onclick={() => toggleStep(step.id)}
                    disabled={isRunning}
                  >
                    {isEnabled ? "On" : "Off"}
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <!-- Outputs -->
        <div class="config-section">
          <h4>Outputs</h4>
          <ul class="output-list">
            {#each selectedWorkflow.outputs as output}
              <li>{output}</li>
            {/each}
          </ul>
        </div>

        <!-- Error message -->
        {#if error}
          <div class="error-banner">
            <strong>Error:</strong> {error}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Footer with Run button -->
  {#if selectedWorkflow}
    <div class="drawer-footer">
      {#if isRunning}
        <div class="progress-bar">
          <div
            class="progress-fill"
            style:width="{(completedCount / totalEnabled) * 100}%"
          ></div>
        </div>
        <span class="progress-text">
          Running step {currentStep + 1} of {totalEnabled}...
        </span>
      {:else}
        <button class="run-btn" onclick={runWorkflow} disabled={!canRun}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          Run Workflow
        </button>
        {#if completedCount > 0}
          <span class="complete-text">
            {completedCount}/{totalEnabled} steps complete
          </span>
        {/if}
      {/if}
    </div>
  {/if}
</aside>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    backdrop-filter: var(--backdrop-blur);
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--transition-slow), visibility var(--transition-slow);
    z-index: 998;
  }

  .backdrop.open {
    opacity: 1;
    visibility: visible;
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: 480px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    box-shadow: var(--drawer-shadow);
    transform: translateX(100%);
    transition: transform var(--transition-slow);
    z-index: 999;
    display: flex;
    flex-direction: column;
  }

  .drawer.open {
    transform: translateX(0);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-5) var(--space-6);
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .drawer-header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .drawer-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
  }

  .drawer-footer {
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border);
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  /* Workflow Grid */
  .workflow-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .workflow-card {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: all var(--transition-base);
  }

  .workflow-card:hover {
    border-color: var(--accent);
    background: var(--bg-elevated);
  }

  .workflow-icon {
    font-size: 28px;
    flex-shrink: 0;
  }

  .workflow-info {
    flex: 1;
    min-width: 0;
  }

  .workflow-info h3 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .workflow-info p {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 8px;
    line-height: 1.4;
  }

  .workflow-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .category-badge {
    font-size: 11px;
    padding: 2px 8px;
    background: color-mix(in srgb, var(--badge-color), transparent 85%);
    color: var(--badge-color);
    border-radius: var(--radius-full);
    font-weight: 500;
  }

  .time-estimate {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Workflow Config */
  .workflow-config {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
    align-self: flex-start;
  }

  .back-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .workflow-header {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border-radius: var(--radius-lg);
  }

  .workflow-icon-lg {
    font-size: 36px;
  }

  .workflow-header h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .workflow-header p {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .config-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .config-section h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .file-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .file-row label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .file-input {
    display: flex;
    gap: var(--space-2);
  }

  .file-input input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 13px;
  }

  .file-input button {
    padding: var(--space-2) var(--space-4);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    transition: all var(--transition-base);
  }

  .file-input button:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  /* Steps List */
  .steps-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .step-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: all var(--transition-base);
  }

  .step-item.disabled {
    opacity: 0.5;
  }

  .step-item.running {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }

  .step-number {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
    border-radius: var(--radius-full);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .step-info {
    flex: 1;
    min-width: 0;
  }

  .step-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .optional-badge {
    font-size: 10px;
    padding: 1px 6px;
    background: var(--bg-secondary);
    color: var(--text-muted);
    border-radius: var(--radius-full);
    font-weight: 400;
  }

  .step-desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .step-time {
    font-size: 11px;
    color: var(--text-muted);
    font-family: "SF Mono", monospace;
    margin-top: 4px;
  }

  .step-error {
    font-size: 11px;
    color: var(--error);
    margin-top: 4px;
  }

  .toggle-btn {
    padding: 4px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .toggle-btn.enabled {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .toggle-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Output list */
  .output-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .output-list li {
    font-size: 13px;
    color: var(--text-secondary);
    padding-left: var(--space-4);
    position: relative;
  }

  .output-list li::before {
    content: "→";
    position: absolute;
    left: 0;
    color: var(--text-muted);
  }

  /* Error banner */
  .error-banner {
    padding: var(--space-3) var(--space-4);
    background: var(--error-subtle);
    border: 1px solid var(--error);
    border-radius: var(--radius-md);
    color: var(--error);
    font-size: 13px;
  }

  /* Run button */
  .run-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .run-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .run-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .complete-text {
    font-size: 13px;
    color: var(--success);
  }

  /* Progress */
  .progress-bar {
    flex: 1;
    height: 6px;
    background: var(--bg-primary);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width var(--transition-base);
  }

  .progress-text {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }
</style>
