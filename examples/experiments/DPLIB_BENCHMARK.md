---

## DPLib — Distributed OPF Benchmark Library

**Paper**: "DPLib: A Standard Benchmark Library for Distributed Power System Analysis and Optimization" ([arXiv](https://arxiv.org/html/2506.20819v2))

**Reference Implementation**: [DPLib GitHub](https://github.com/DPLib-Benchmark)

---

### What This Provides

GAT implements distributed OPF using the **Alternating Direction Method of Multipliers (ADMM)**, enabling scalable optimization across partitioned networks:

1. **Graph Partitioning** — METIS-based network decomposition
2. **Parallel Subproblem Solving** — Independent OPF for each partition
3. **Consensus Coordination** — Boundary voltage agreement via ADMM
4. **Tie-Line Flow Reporting** — Power exchanges between regions

---

### Quick Start

```bash
# Run distributed OPF on a partitioned network
gat opf run network.arrow \
  --method dc \
  --distributed \
  --partitions 4 \
  --out results/distributed_opf.parquet

# Benchmark against DPLib centralized results
gat benchmark dplib \
  --pglib-dir data/pglib-opf \
  --out results/dplib_benchmark.csv
```

---

### ADMM Algorithm Overview

The ADMM-based distributed OPF follows this structure:

**1. Partition Phase**
- Partition the network into K regions using METIS spectral partitioning
- Identify tie-lines (boundary branches) between partitions
- Extract boundary buses that require consensus

**2. ADMM Iteration Loop**

For each iteration k:

```
x-update:  Solve local OPF for each partition in parallel
z-update:  Average boundary voltages across partitions (consensus)
λ-update:  Update dual variables (Lagrange multipliers)
Check:     Compute primal/dual residuals, check convergence
```

**3. Solution Merge**
- Combine partition solutions
- Compute tie-line power flows
- Report total objective and convergence metrics

---

### Configuration Options

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--partitions` | 4 | Number of network partitions |
| `--penalty` | 1.0 | ADMM penalty parameter (ρ) |
| `--max-iter` | 100 | Maximum ADMM iterations |
| `--primal-tol` | 1e-4 | Primal residual tolerance |
| `--dual-tol` | 1e-4 | Dual residual tolerance |
| `--adaptive-penalty` | true | Enable penalty adaptation |
| `--inner-method` | dc | Inner OPF solver (dc, socp, ac) |

---

### API Usage

```rust
use gat_algo::opf::admm::{AdmmOpfSolver, AdmmConfig};
use gat_algo::opf::OpfMethod;

// Configure ADMM solver
let solver = AdmmOpfSolver::new(AdmmConfig {
    num_partitions: 4,
    penalty: 1.0,
    max_iter: 100,
    primal_tol: 1e-4,
    dual_tol: 1e-4,
    inner_method: OpfMethod::DcOpf,
    adaptive_penalty: true,
    ..Default::default()
});

// Solve distributed OPF
let result = solver.solve(&network)?;

// Access results
println!("Converged: {}", result.converged);
println!("Iterations: {}", result.iterations);
println!("Objective: {:.2}", result.objective);
println!("Tie-lines: {}", result.num_tie_lines);

// Examine tie-line flows
for (branch_id, (p_mw, q_mvar, from_part, to_part)) in &result.tie_line_flows {
    println!("  {}: P={:.2} MW, Q={:.2} MVAr ({} -> {})",
             branch_id, p_mw, q_mvar, from_part, to_part);
}

// Access branch flows and losses
println!("Total losses: {:.2} MW", result.total_losses_mw);
for (branch_id, p_mw) in &result.branch_p_flow {
    let q_mvar = result.branch_q_flow.get(branch_id).unwrap_or(&0.0);
    println!("  {}: P={:.2} MW, Q={:.2} MVAr", branch_id, p_mw, q_mvar);
}
```

---

### Solution Structure

The `AdmmSolution` contains:

```rust
pub struct AdmmSolution {
    // Core results
    pub objective: f64,                          // Total cost
    pub bus_voltage_mag: HashMap<String, f64>,   // Per-bus Vm
    pub bus_voltage_ang: HashMap<String, f64>,   // Per-bus Va
    pub generator_p: HashMap<String, f64>,       // Generator dispatch
    pub generator_q: HashMap<String, f64>,       // Reactive dispatch

    // Convergence info
    pub converged: bool,
    pub iterations: usize,
    pub primal_residual: f64,                    // ||x - z||
    pub dual_residual: f64,                      // ||z^k - z^{k-1}||

    // Partition info
    pub partition_objectives: Vec<f64>,          // Per-partition cost
    pub partition_sizes: Vec<usize>,             // Buses per partition
    pub num_tie_lines: usize,                    // Boundary branches

    // Tie-line flows (partition boundary)
    pub tie_line_flows: HashMap<String, (f64, f64, usize, usize)>,

    // Branch flows (all branches)
    pub branch_p_flow: HashMap<String, f64>,     // Active power (MW)
    pub branch_q_flow: HashMap<String, f64>,     // Reactive power (MVAr)
    pub total_losses_mw: f64,                    // System losses (MW)

    // Timing
    pub solve_time_ms: u128,
    pub phase_times_ms: AdmmPhaseTimes,
}
```

---

### Comparison with DPLib Paper

The DPLib paper provides ADMM-based DC and AC OPF results on partitioned PGLib cases. GAT's implementation:

| Feature | DPLib (MATLAB) | GAT (Rust) |
|---------|---------------|------------|
| Partitioning | Manual/METIS | METIS-based |
| DC-OPF | YALMIP + Gurobi | Native DC solver |
| AC-OPF | IPOPT | IPOPT (optional) |
| Parallelism | parfor | rayon |
| Data format | MATPOWER | Arrow/Parquet |

**Validation approach:**
1. Load the same PGLib case used in DPLib
2. Apply equivalent partitioning
3. Run ADMM with same parameters
4. Compare objective values and convergence

---

### Implementation Notes

**Subnetwork Extraction:**
Each partition receives:
- All internal buses and branches
- Boundary buses (shared with neighbors)
- Generators and loads assigned to partition
- Virtual boundary loads representing tie-line power injections

**Branch Power Flow Calculation:**
Power flow on all branches (including tie-lines) is computed using the standard AC power flow equations with transformer tap ratio (τ) and phase shift (φ) support:

```
P_from = (V_from² × G / τ²) - (V_from × V_to / τ) × (G cos(θ_from - θ_to - φ) + B sin(θ_from - θ_to - φ))
Q_from = -(V_from² × (B + B_charging/2) / τ²) - (V_from × V_to / τ) × (G sin(θ_from - θ_to - φ) - B cos(θ_from - θ_to - φ))
```

Where:
- `G + jB = 1/(R + jX)` is the series admittance
- `B_charging` is the line charging susceptance
- `τ` is the transformer tap ratio (1.0 for lines)
- `φ` is the phase shift angle (0 for lines)

**Total Losses Calculation:**
System losses are computed as the sum of active power flows into each branch from both ends:
```
Total Losses = Σ (P_from + P_to) for all branches
```

**Consensus Variables:**
Boundary buses have voltage magnitude and angle as consensus variables:
```rust
z_vm = average of V_m across partitions sharing this bus
z_va = average of V_a across partitions sharing this bus
```

---

### Current Status

| Component | Status |
|-----------|--------|
| METIS partitioning | ✅ Implemented |
| DC-OPF subproblems | ✅ Implemented |
| Parallel x-update | ✅ Implemented (rayon) |
| Consensus z-update | ✅ Implemented |
| Dual λ-update | ✅ Implemented |
| Tie-line flow calculation | ✅ Implemented |
| Branch flow calculation | ✅ Implemented |
| Total losses calculation | ✅ Implemented |
| Adaptive penalty | ✅ Implemented |
| SOCP inner solver | 🟡 Partial |
| AC-OPF inner solver | 🟡 Requires IPOPT |
| Full DPLib validation | 🟡 Pending |

---

### References

1. Chen, Y., et al. (2025). DPLib: A Standard Benchmark Library for Distributed Power System Analysis and Optimization. *arXiv:2506.20819*.

2. Boyd, S., et al. (2011). Distributed Optimization and Statistical Learning via the Alternating Direction Method of Multipliers. *Foundations and Trends in Machine Learning*.

3. Karypis, G., & Kumar, V. (1998). A Fast and High Quality Multilevel Scheme for Partitioning Irregular Graphs. *SIAM J. Scientific Computing*.
