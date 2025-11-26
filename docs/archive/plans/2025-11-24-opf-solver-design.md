# OPF Solver Architecture Design

**Date:** 2025-11-24
**Status:** Phase 2 Complete (v0.3.3)
**Author:** Claude (brainstorming session)

> **Implementation Status:**
> - ✅ Phase 1: Module restructure, OpfMethod enum, unified OpfSolver, economic dispatch
> - ✅ Phase 2: DC-OPF with B-matrix and LMP extraction (v0.3.3)
> - ⏳ Phase 3: SOCP relaxation (planned)
> - ⏳ Phase 4: CLI integration with --method flag (planned)

## Overview

Design for a unified OPF (Optimal Power Flow) solver supporting multiple solution methods with increasing accuracy: economic dispatch, DC-OPF, SOCP relaxation, and full AC-OPF.

### Goals

- Match published AC-OPF benchmark results for research validation
- Provide multiple solver methods with clear accuracy/speed tradeoffs
- Clean CLI interface with sensible defaults
- Leverage existing `good_lp`/Clarabel infrastructure

### Non-Goals

- Real-time operation (this is a research tool)
- Security-constrained OPF (future work)
- Stochastic/robust OPF (future work)

## Solution Methods

| Method | CLI Flag | Typical Gap | Speed | Use Case |
|--------|----------|-------------|-------|----------|
| Economic Dispatch | `economic` | ~20% | Fastest | Quick estimates, screening |
| DC-OPF | `dc` | ~3-5% | Fast | Planning studies |
| SOCP Relaxation | `socp` | ~1-3% | Moderate | Research benchmarking |
| AC-OPF (IPM) | `ac` | <1% | Slowest | High-fidelity analysis |

Default: `socp` (best accuracy with guaranteed convergence)

## Architecture

### Core Types

```rust
/// OPF solution method
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpfMethod {
    /// Merit-order economic dispatch (no network constraints)
    EconomicDispatch,
    /// DC optimal power flow (LP with B-matrix)
    DcOpf,
    /// Second-order cone relaxation of AC-OPF
    #[default]
    SocpRelaxation,
    /// Full nonlinear AC-OPF (interior point) - future
    AcOpf,
}

/// Unified OPF solver
pub struct OpfSolver {
    method: OpfMethod,
    max_iterations: usize,
    tolerance: f64,
    lp_solver: LpSolverKind,
}

impl OpfSolver {
    pub fn new() -> Self;
    pub fn with_method(self, method: OpfMethod) -> Self;
    pub fn with_tolerance(self, tol: f64) -> Self;
    pub fn with_max_iterations(self, max_iter: usize) -> Self;
    pub fn with_lp_solver(self, solver: LpSolverKind) -> Self;

    pub fn solve(&self, network: &Network) -> Result<OpfSolution, OpfError>;
}

// Backward compatibility
pub type AcOpfSolver = OpfSolver;
```

### Solution Output

```rust
pub struct OpfSolution {
    // === Status ===
    pub converged: bool,
    pub method_used: OpfMethod,
    pub iterations: usize,
    pub solve_time_ms: u128,

    // === Objective ===
    pub objective_value: f64,  // Total cost ($/hr)

    // === Primal Variables ===
    pub generator_p: HashMap<String, f64>,      // Active power (MW)
    pub generator_q: HashMap<String, f64>,      // Reactive power (MVAr)
    pub bus_voltage_mag: HashMap<String, f64>,  // |V| in p.u.
    pub bus_voltage_ang: HashMap<String, f64>,  // θ in radians
    pub branch_p_flow: HashMap<String, f64>,    // MW flow (from end)
    pub branch_q_flow: HashMap<String, f64>,    // MVAr flow (from end)

    // === Dual Variables ===
    pub bus_lmp: HashMap<String, f64>,          // $/MWh at each bus

    // === Constraint Info ===
    pub binding_constraints: Vec<ConstraintInfo>,
    pub total_losses_mw: f64,
}

pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub value: f64,
    pub limit: f64,
    pub shadow_price: f64,
}

pub enum ConstraintType {
    GeneratorPMax,
    GeneratorPMin,
    GeneratorQMax,
    GeneratorQMin,
    BranchFlowLimit,
    VoltageMax,
    VoltageMin,
    PowerBalance,
}
```

Note: Not all fields are populated by all methods. Economic dispatch won't have LMPs or voltage angles. Documentation will specify which fields are valid per method.

## Method Implementations

### Economic Dispatch (existing)

Merit-order dispatch sorting generators by marginal cost. No network model.

- Input: Generators with costs and limits, total load
- Output: Generator dispatch minimizing cost
- Limitations: Ignores network constraints, losses, voltage

### DC-OPF

Linearized power flow with LP optimization.

**Formulation:**
```
minimize    Σ C_i(P_g,i)
subject to  Σ P_g - Σ P_d = P_loss
            P_f = B_f × θ
            |P_f| ≤ P_f_max
            P_g_min ≤ P_g ≤ P_g_max
            θ_ref = 0
```

**Loss Iteration:**
1. Solve DC-OPF (lossless)
2. Estimate losses: `P_loss ≈ Σ r_ij × (P_f,ij / V_nom)²`
3. Re-solve with loss term added to balance
4. Repeat until convergence (typically 2-3 iterations)

**LMP Extraction:** Dual variables of power balance constraints.

### SOCP Relaxation

Second-order cone relaxation using lifted variables.

**Lifted Variables:**
```
c_ij = |V_i||V_j|cos(θ_i - θ_j)
s_ij = |V_i||V_j|sin(θ_i - θ_j)
w_i  = |V_i|²
```

**SOC Constraint:**
```
c_ij² + s_ij² ≤ w_i × w_j
```

**Power Flow (lifted):**
```
P_ij = g_ij×w_i - g_ij×c_ij - b_ij×s_ij
Q_ij = -b_ij×w_i + b_ij×c_ij - g_ij×s_ij
```

**Properties:**
- Convex → guaranteed global optimum of relaxation
- Exact for radial networks
- Typically <3% gap for meshed networks
- Clarabel handles SOC cones natively

**Voltage Recovery:** Extract |V|, θ from w, c, s variables.

### AC-OPF (Future)

Full nonlinear interior point method. To be implemented after SOCP is validated.

## File Structure

```
crates/gat-algo/src/
├── opf/
│   ├── mod.rs              # Re-exports, OpfSolver, OpfMethod, OpfSolution
│   ├── economic.rs         # Merit-order dispatch
│   ├── dc_opf.rs           # DC-OPF with B-matrix
│   ├── socp_opf.rs         # SOCP relaxation
│   └── ac_opf.rs           # Future: full nonlinear IPM
├── ac_opf.rs               # Thin wrapper, deprecated alias
└── ...
```

## CLI Interface

### Arguments

```rust
#[arg(long, default_value = "socp", value_parser = parse_opf_method)]
method: OpfMethod,

#[arg(long, default_value = "clarabel")]
lp_solver: LpSolverKind,
```

### Help Text

```
OPTIONS:
    --method <METHOD>    OPF solution method [default: socp]
                         economic - Merit-order dispatch (fastest, ~20% gap)
                         dc       - DC optimal power flow (~5% gap)
                         socp     - SOCP relaxation (~2% gap)
                         ac       - Full AC-OPF (most accurate)

                         Aliases: fast=economic, balanced=dc, accurate=socp
```

### Output Format

Add `method` column to benchmark CSV:

```csv
sample_id,method,converged,objective_value,objective_gap_rel,...
```

## Implementation Plan

### Phase 1: Restructure (~1 day)

- Create `opf/` module structure
- Move economic dispatch to `opf/economic.rs`
- Implement `OpfMethod` enum and unified `OpfSolver`
- Keep `AcOpfSolver` as type alias for backward compatibility
- Update lib.rs exports

### Phase 2: DC-OPF (~2 days)

- Implement B-matrix construction from network
- LP formulation using `good_lp`
- Loss iteration loop
- LMP extraction from dual variables
- Unit tests against known solutions

### Phase 3: SOCP (~3 days)

- SOCP model with lifted variables (w, c, s)
- Clarabel SOC constraint formulation
- Voltage magnitude and angle recovery
- Validate against OPFData benchmarks (target: <3% gap)

### Phase 4: CLI + Integration (~1 day)

- Add `--method` flag to benchmark commands
- Update benchmark result structs with method column
- Integration tests comparing all methods
- Documentation updates

## Testing Strategy

### Unit Tests

- B-matrix construction correctness
- SOCP constraint formulation
- Voltage recovery from lifted variables
- Cost function evaluation

### Integration Tests

- IEEE test cases (14, 30, 118 bus)
- Compare method results against each other
- Validate LMPs sum to system lambda

### Benchmark Validation

- OPFData: Target <3% gap with SOCP
- PGLib: Compare against published results
- Track regression in objective gaps

## Future Extensions

- **Security-Constrained OPF:** N-1 contingency constraints
- **AC-OPF (IPM):** Full nonlinear interior point
- **Unit Commitment:** Binary on/off decisions
- **Stochastic OPF:** Uncertainty in renewables/load
