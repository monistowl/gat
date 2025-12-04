<script lang="ts">
  import { onMount } from 'svelte';
  import katex from 'katex';

  // Props
  let {
    isOpen = false,
    activeView = 'grid',
    onClose
  }: {
    isOpen: boolean;
    activeView: 'grid' | 'ybus' | 'arch';
    onClose: () => void;
  } = $props();

  // Content sections for each view - expanded from GAT reference documentation
  const content: Record<string, { title: string; sections: Array<{ heading: string; body: string }> }> = {
    grid: {
      title: "Power Flow Analysis",
      sections: [
        {
          heading: "What is Power Flow?",
          body: `Power flow analysis answers the fundamental question: *Given generation and load at each bus, what are the voltages and line flows throughout the network?*

Every other grid analysis builds on power flow:
- **OPF** optimizes generation subject to power flow equations
- **Contingency analysis** runs power flow for each outage scenario
- **State estimation** reconciles measurements with power flow physics
- **Planning studies** evaluate future scenarios via power flow

Understanding power flow deeply is essential for interpreting results and debugging convergence issues.`
        },
        {
          heading: "Physical Intuition",
          body: `Power flow is governed by two physical principles:

1. **Conservation of charge** (Kirchhoff's Current Law): Current entering a node equals current leaving
2. **Conservation of energy** (Kirchhoff's Voltage Law): Voltage drops around any closed loop sum to zero

Power flows from higher to lower voltage angle. For two buses connected by a line:

$$P_{12} \\approx \\frac{V_1 V_2}{X} \\sin(\\theta_1 - \\theta_2)$$

Key insights:
- **Angle difference** drives real power flow — larger $\\theta_1 - \\theta_2$ means more MW flowing
- **Reactance limits** how much power can flow for a given angle difference
- **Voltage magnitudes** affect transfer capability`
        },
        {
          heading: "Complex Power",
          body: `In AC circuits, power comes in three types:

| Type | Symbol | Unit | What It Does |
|------|--------|------|--------------|
| **Real Power** | $P$ | Watts (W) | Does useful work |
| **Reactive Power** | $Q$ | VAR | Sustains magnetic/electric fields |
| **Apparent Power** | $S$ | VA | Total power equipment must deliver |

These form the **power triangle**: $S = \\sqrt{P^2 + Q^2}$

Complex power elegantly captures both: $\\mathbf{S} = P + jQ = \\mathbf{V} \\cdot \\mathbf{I}^*$

**Power factor** $\\cos(\\theta) = P/S$ measures how effectively current delivers real power. Poor power factor wastes current capacity.`
        },
        {
          heading: "Bus Types",
          body: `At each bus, there are four electrical quantities: $P$, $Q$, $|V|$, $\\theta$. Bus types determine which are specified vs. solved:

| Type | Known | Unknown | Purpose |
|------|-------|---------|---------|
| **Slack (Ref)** | $|V|, \\theta$ | $P, Q$ | Reference angle, absorbs mismatch |
| **PV (Generator)** | $P, |V|$ | $Q, \\theta$ | Voltage-controlled generation |
| **PQ (Load)** | $P, Q$ | $|V|, \\theta$ | Fixed demand |

**Slack bus**: Sets the angle reference ($\\theta = 0$) and absorbs power mismatch (losses + imbalance). Every synchronous island needs exactly one.

**PV→PQ switching**: If a generator's $Q$ exceeds limits, the bus converts to PQ at that limit — this is handled automatically.`
        },
        {
          heading: "Newton-Raphson",
          body: `Newton-Raphson iteratively solves nonlinear equations with **quadratic convergence** — each iteration roughly doubles the number of correct digits:

$$\\mathbf{x}_{k+1} = \\mathbf{x}_k - \\mathbf{J}^{-1} \\mathbf{f}(\\mathbf{x}_k)$$

The **Jacobian** $\\mathbf{J}$ has a natural 2×2 block structure:

$$\\mathbf{J} = \\begin{pmatrix} \\partial P/\\partial\\theta & \\partial P/\\partial|V| \\\\ \\partial Q/\\partial\\theta & \\partial Q/\\partial|V| \\end{pmatrix}$$

**Algorithm:**
1. Initialize: $|V| = 1.0$ p.u., $\\theta = 0$ (flat start)
2. Compute mismatches: $\\Delta P$, $\\Delta Q$
3. Check convergence: if $\\max(|\\Delta P|, |\\Delta Q|) < \\epsilon$, stop
4. Build Jacobian, solve linear system
5. Update: $\\theta \\leftarrow \\theta + \\Delta\\theta$, $|V| \\leftarrow |V| + \\Delta|V|$

Typical convergence: **3-7 iterations** from flat start.`
        },
        {
          heading: "DC Power Flow",
          body: `DC power flow is a **linear approximation** enabling much faster solutions:

**Assumptions:**
- Voltage magnitudes $\\approx 1.0$ p.u.
- Small angle differences: $\\sin(\\theta) \\approx \\theta$
- Lossless lines: $R \\ll X$

This yields a linear system: $\\mathbf{P} = \\mathbf{B} \\cdot \\boldsymbol{\\theta}$

**No iteration needed** — just solve the linear system. But DC power flow ignores reactive power and losses.

| Use DC | Use AC |
|--------|--------|
| Screening studies | Final verification |
| Contingency ranking | Voltage analysis |
| Market clearing (LMPs) | Loss calculation |
| Large-scale studies | Distribution networks |`
        },
        {
          heading: "Convergence Issues",
          body: `Power flow doesn't always converge. Common causes:

**Heavy Loading**: System may have no solution if load exceeds transfer capability.

**Reactive Power Limits**: Generators hitting Q limits switch from PV to PQ, potentially causing voltage collapse.

**Bad Initial Guess**: Flat start may be far from solution for stressed systems. Try warm start from similar case.

**Data Errors**: Incorrect impedances, missing buses, or topology errors cause immediate divergence.

**Debugging tips:**
- Check for negative resistance or zero impedance branches
- Look for isolated buses (islands)
- Verify generation equals load plus reasonable losses
- Try reducing load to find a feasible point`
        },
        {
          heading: "Voltage Colors",
          body: `The visualization colors buses by voltage magnitude:

- 🟢 **Green** ($\\approx 1.0$ p.u.): Nominal voltage — optimal operation
- 🟡 **Yellow** (0.95–1.05 p.u.): Acceptable range — minor deviation
- 🔴 **Red** (<0.9 or >1.1 p.u.): Voltage violation — requires attention

**Physical interpretation:**
- **Low voltages** indicate heavy loading or insufficient reactive support
- **High voltages** may occur on lightly loaded lines (line charging effect)
- **Voltage collapse** happens when reactive support is exhausted — prevented by maintaining adequate reactive reserves`
        }
      ]
    },
    ybus: {
      title: "The Y-Bus Matrix",
      sections: [
        {
          heading: "Why a Matrix?",
          body: `The **bus admittance matrix** (Y-bus) encodes the entire network topology and parameters in a single sparse matrix:

$$\\mathbf{I} = \\mathbf{Y} \\cdot \\mathbf{V}$$

At each bus, Kirchhoff's Current Law says: current injected = sum of currents flowing out on branches. The Y-bus captures all branch admittances and how buses connect.

Every power flow solver builds the Y-bus as its first step — it's the fundamental data structure of power flow analysis.`
        },
        {
          heading: "Building Y-Bus",
          body: `For a network with $n$ buses, the Y-bus is an $n \\times n$ complex matrix.

**Off-diagonal elements** ($i \\neq j$):
$$Y_{ij} = -y_{ij} = -\\frac{1}{R_{ij} + jX_{ij}}$$

If buses $i$ and $j$ are not directly connected, $Y_{ij} = 0$.

**Diagonal elements**:
$$Y_{ii} = \\sum_{j \\neq i} y_{ij} + y_i^{\\text{shunt}}$$

The diagonal equals the sum of all admittances connected to that bus, plus any shunt elements (capacitors, reactors, line charging).

**Why negative off-diagonal?** Current from bus $i$ to $j$ is $I_{i \\to j} = y_{ij}(V_i - V_j)$. The negative sign captures this relationship.`
        },
        {
          heading: "Impedance & Admittance",
          body: `**Impedance** $\\mathbf{Z} = R + jX$ combines resistance (losses) and reactance (energy storage):

- **$R$ (resistance)**: Dissipates energy as heat — causes $I^2R$ losses
- **$X$ (reactance)**: Stores energy in magnetic fields (inductors) — transmission lines are primarily inductive

**Admittance** $\\mathbf{Y} = 1/\\mathbf{Z} = G + jB$ is the inverse:

- **$G$ (conductance)**: $G = R/(R^2+X^2)$ — represents resistive losses
- **$B$ (susceptance)**: $B = -X/(R^2+X^2)$ — represents reactive power flow

For typical transmission lines, $|X| >> |R|$ (3-10× larger), so $|B| >> |G|$. This is why DC power flow ignores resistance.`
        },
        {
          heading: "The π-Model",
          body: `Real transmission lines have distributed capacitance to ground, modeled as shunt elements at each end (the **π-model**):

\`\`\`
         y_series
    ──┬───/\\/\\/\\───┬──
      │           │
    jB/2        jB/2
      │           │
     ─┴─         ─┴─
\`\`\`

This **line charging** adds $jB/2$ to the diagonal elements at each end.

**Physical intuition**: Line charging is why lightly-loaded transmission lines can cause overvoltage — they're pumping reactive power into the system. Significant for long high-voltage lines.`
        },
        {
          heading: "Transformers",
          body: `Transformers with tap ratio $t$ require special treatment. The Y-bus contributions are **asymmetric**:

$$Y_{ii} \\mathrel{+}= \\frac{y}{t^2}$$

$$Y_{jj} \\mathrel{+}= y$$

$$Y_{ij} = Y_{ji} = -\\frac{y}{t}$$

Off-nominal taps ($t \\neq 1$) make the Y-bus asymmetric.

**Phase shifters** add a complex tap ratio $t \\angle \\phi$, enabling active power flow control:

$$Y_{ij} = -\\frac{y}{t e^{-j\\phi}}, \\quad Y_{ji} = -\\frac{y}{t e^{j\\phi}}$$`
        },
        {
          heading: "Sparsity",
          body: `Real power networks are sparse — each bus connects to only a few others:

- A **1,000-bus** network has 1,000,000 matrix elements
- But only **~3,000** are non-zero (0.3% fill rate)
- A **10,000-bus** network: ~30,000 non-zeros out of 100,000,000

GAT stores Y-bus in **CSR format** (Compressed Sparse Row) for:
- O(nnz) storage instead of O(n²)
- Efficient matrix-vector multiplication
- Fast row access for power calculations

A 10,000-bus Y-bus uses ~300KB instead of ~800MB.`
        },
        {
          heading: "Y-Bus Properties",
          body: `**Symmetry**: For networks with only lines and transformers (no phase shifters), $Y_{ij} = Y_{ji}$ — allows efficient storage.

**Row sums**: For a network without shunts, each row sums to zero: $\\sum_j Y_{ij} = 0$. With shunts, row sums equal total shunt admittance at that bus.

**Singularity**: Y-bus is singular (det = 0) for any connected network — reflects that we can add any constant to all voltages without changing currents. Power flow handles this by fixing the slack bus angle.

**Z-Bus**: The inverse $\\mathbf{Z} = \\mathbf{Y}^{-1}$ is used for fault analysis but is **fully dense**, so rarely computed for large networks.`
        },
        {
          heading: "To Power Flow",
          body: `Combining $\\mathbf{I} = \\mathbf{Y} \\cdot \\mathbf{V}$ with complex power $S = V \\cdot I^*$:

$$S_i = V_i \\sum_{k=1}^{n} Y_{ik}^* V_k^*$$

Separating into real and imaginary parts:

$$P_i = \\sum_{k=1}^{n} |V_i||V_k|(G_{ik}\\cos\\theta_{ik} + B_{ik}\\sin\\theta_{ik})$$

$$Q_i = \\sum_{k=1}^{n} |V_i||V_k|(G_{ik}\\sin\\theta_{ik} - B_{ik}\\cos\\theta_{ik})$$

where $\\theta_{ik} = \\theta_i - \\theta_k$ is the angle difference.

These are the **nonlinear equations** solved by Newton-Raphson.`
        }
      ]
    },
    arch: {
      title: "GAT Architecture",
      sections: [
        {
          heading: "Layered Design",
          body: `GAT follows a clean layered architecture where each crate has a single responsibility:

- **gat-core**: Data model (Bus, Branch, Gen, Load) and network graph
- **gat-io**: File format parsers (MATPOWER, PSS/E, CIM, pandapower)
- **gat-algo**: Algorithms (power flow, OPF, state estimation)
- **gat-cli**: Command-line interface
- **gat-demo**: Visualization (this application)

This separation enables:
- Independent testing of each layer
- Easy addition of new file formats
- Algorithm development without I/O concerns`
        },
        {
          heading: "Graph Model",
          body: `The network is stored as a **petgraph** directed graph:

- **Nodes**: Buses, Generators, Loads, Shunts
- **Edges**: Branches, Transformers

This enables efficient traversal, island detection, and topological analysis with $O(V + E)$ complexity.

**Key algorithms:**
- DFS/BFS for island detection
- Topological sorting for radial networks
- Shortest path for electrical distance`
        },
        {
          heading: "Type-Safe IDs",
          body: `GAT uses newtype wrappers for IDs to prevent mixing different entity types:

\`\`\`rust
pub struct BusId(usize);
pub struct GenId(usize);
pub struct BranchId(usize);
\`\`\`

This catches bugs at **compile time** — you can't accidentally pass a GenId where a BusId is expected.

The type system enforces correctness without runtime overhead (zero-cost abstraction).`
        },
        {
          heading: "Sparse Matrices",
          body: `Large-scale computation uses sparse matrices via the **sprs** crate:

- **CSR format**: Compressed Sparse Row for efficient row access
- **COO format**: Coordinate format for matrix construction
- **CSC format**: Compressed Sparse Column for column access

**Storage comparison** for 10,000-bus Y-bus:
- Dense: ~800 MB (n² complex numbers)
- Sparse: ~300 KB (only non-zeros)

**LU factorization** uses AMD (Approximate Minimum Degree) ordering to minimize fill-in.`
        },
        {
          heading: "Power Flow",
          body: `AC power flow uses Newton-Raphson with sparse LU factorization:

1. Build sparse Y-bus from network
2. Compute power mismatches
3. Form sparse Jacobian
4. LU factorize with AMD ordering
5. Solve, update, repeat

**Performance:**
- 10,000 buses: <100ms
- 100,000 buses: ~1 second
- Memory: O(n) for sparse structures`
        },
        {
          heading: "OPF Methods",
          body: `Optimal Power Flow minimizes generation cost subject to constraints:

| Method | Accuracy | Speed | Guarantees |
|--------|----------|-------|------------|
| Economic Dispatch | ~20% gap | Fastest | Global (convex) |
| DC-OPF | 3-5% gap | Fast | Global (LP/QP) |
| SOCP | 1-3% gap | Moderate | Global (convex) |
| AC-OPF | Exact | Slowest | Local only |

**DC-OPF**: Linear program, ignores reactive power
**SOCP**: Second-Order Cone relaxation, often exact for radial networks
**AC-OPF**: Full nonlinear program via L-BFGS or IPOPT`
        },
        {
          heading: "LMP Pricing",
          body: `**Locational Marginal Price** (LMP) is the marginal cost of serving one additional MW at a specific bus:

$$\\text{LMP}_i = \\lambda + \\text{Congestion}_i + \\text{Losses}_i$$

Components:
- **Energy** ($\\lambda$): System marginal cost
- **Congestion**: Shadow price of binding flow limits
- **Losses**: Marginal loss contribution

LMPs come from the **dual variables** of power balance constraints in OPF. Price separation between buses indicates congestion.`
        },
        {
          heading: "File Formats",
          body: `GAT supports multiple industry-standard formats:

**Input formats:**
- **MATPOWER** (.m): MATLAB case files
- **PSS/E RAW**: Siemens PTI format
- **CIM/CGMES**: IEC standard XML
- **pandapower**: Python JSON format

**Native format:**
- **Apache Arrow**: Columnar format for high-performance data interchange
- Separate tables: buses.arrow, branches.arrow, generators.arrow, loads.arrow
- manifest.json for metadata and provenance`
        }
      ]
    }
  };

  let activeSection = $state(0);
  let renderedContent = $state<string[]>([]);

  // Render markdown with KaTeX
  function renderContent(text: string): string {
    // First, render display math ($$...$$)
    let result = text.replace(/\$\$([\s\S]*?)\$\$/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: true, throwOnError: false });
      } catch {
        return `<pre class="math-error">${math}</pre>`;
      }
    });

    // Then render inline math ($...$)
    result = result.replace(/\$([^\$\n]+?)\$/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: false, throwOnError: false });
      } catch {
        return `<code class="math-error">${math}</code>`;
      }
    });

    // Convert markdown-style formatting
    result = result
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\n\n/g, '</p><p>')
      .replace(/\n- /g, '</p><ul><li>')
      .replace(/<\/li>\n/g, '</li>')
      .replace(/^- /gm, '<li>')
      .replace(/(<li>.*?)(<\/p>|$)/g, '$1</li></ul>');

    // Handle tables (simple conversion)
    if (result.includes('|')) {
      const lines = result.split('\n');
      let inTable = false;
      let tableHtml = '';
      const processedLines: string[] = [];

      for (const line of lines) {
        if (line.startsWith('|') && line.endsWith('|')) {
          if (!inTable) {
            inTable = true;
            tableHtml = '<table>';
          }
          if (line.includes('---')) {
            continue; // Skip separator row
          }
          const cells = line.slice(1, -1).split('|').map(c => c.trim());
          const tag = tableHtml === '<table>' ? 'th' : 'td';
          tableHtml += `<tr>${cells.map(c => `<${tag}>${c}</${tag}>`).join('')}</tr>`;
        } else {
          if (inTable) {
            tableHtml += '</table>';
            processedLines.push(tableHtml);
            inTable = false;
            tableHtml = '';
          }
          processedLines.push(line);
        }
      }
      if (inTable) {
        tableHtml += '</table>';
        processedLines.push(tableHtml);
      }
      result = processedLines.join('\n');
    }

    return `<p>${result}</p>`;
  }

  // Re-render when view changes
  $effect(() => {
    const viewContent = content[activeView];
    if (viewContent) {
      renderedContent = viewContent.sections.map(s => renderContent(s.body));
      activeSection = 0;
    }
  });
</script>

<svelte:head>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
</svelte:head>

<div class="drawer-backdrop" class:open={isOpen} onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose()} role="button" tabindex="-1"></div>

<aside class="drawer" class:open={isOpen}>
  <div class="drawer-header">
    <h2>{content[activeView]?.title || 'Learn'}</h2>
    <button class="close-btn" onclick={onClose} aria-label="Close drawer">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  </div>

  <nav class="section-nav">
    {#each content[activeView]?.sections || [] as section, i}
      <button
        class="nav-item"
        class:active={activeSection === i}
        onclick={() => activeSection = i}
      >
        {section.heading}
      </button>
    {/each}
  </nav>

  <div class="drawer-content">
    {#each content[activeView]?.sections || [] as section, i}
      {#if activeSection === i}
        <article class="section">
          <h3>{section.heading}</h3>
          <div class="body">
            {@html renderedContent[i] || ''}
          </div>
        </article>
      {/if}
    {/each}
  </div>

  <div class="drawer-footer">
    <span class="source-hint">Content from GAT documentation</span>
  </div>
</aside>

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.3s ease, visibility 0.3s ease;
    z-index: 998;
  }

  .drawer-backdrop.open {
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
    transform: translateX(100%);
    transition: transform 0.3s ease;
    z-index: 999;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .drawer.open {
    transform: translateX(0);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .drawer-header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .section-nav {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    overflow-y: auto;
    max-height: 120px;
    flex-shrink: 0;
  }

  .nav-item {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 16px;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .nav-item:hover {
    border-color: var(--text-muted);
    color: var(--text-secondary);
  }

  .nav-item.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .drawer-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .section h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .body {
    font-size: 14px;
    line-height: 1.7;
    color: var(--text-secondary);
  }

  .body :global(p) {
    margin-bottom: 12px;
  }

  .body :global(strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  .body :global(em) {
    font-style: italic;
  }

  .body :global(code) {
    background: var(--bg-tertiary);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 13px;
    color: var(--accent);
  }

  .body :global(ul) {
    margin: 12px 0;
    padding-left: 20px;
  }

  .body :global(li) {
    margin-bottom: 6px;
  }

  .body :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 16px 0;
    font-size: 13px;
  }

  .body :global(th),
  .body :global(td) {
    padding: 8px 12px;
    border: 1px solid var(--border);
    text-align: left;
  }

  .body :global(th) {
    background: var(--bg-tertiary);
    font-weight: 600;
    color: var(--text-primary);
  }

  /* KaTeX styling */
  .body :global(.katex-display) {
    margin: 16px 0;
    padding: 16px;
    background: var(--bg-tertiary);
    border-radius: 8px;
    overflow-x: auto;
  }

  .body :global(.katex) {
    font-size: 1.1em;
  }

  .drawer-footer {
    padding: 12px 24px;
    border-top: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .source-hint {
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
