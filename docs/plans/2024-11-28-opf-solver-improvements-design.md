# OPF Solver Improvements Design

**Date:** 2024-11-28
**Status:** Approved
**Author:** GAT Development Team

## Executive Summary

This document details improvements to GAT's OPF solvers targeting significant reductions in objective gaps and solve times across DC-OPF, SOCP, and AC-OPF methods.

### Current Baseline (PGLib-OPF, 68 cases)

| Method | Gap | Time | Convergence |
|--------|-----|------|-------------|
| DC-OPF | 6.16% | 898ms | 100% |
| SOCP | 4.21% | 39s | 100% |
| AC-OPF | 32%+ | varies | poor |

### Target Improvements

| Method | Current | Target | Improvement |
|--------|---------|--------|-------------|
| DC-OPF | 6.16% gap | ~4% gap | Loss factors |
| SOCP | 4.21% gap | ~2% gap | Bound tightening |
| SOCP | 39s | <10s | Warm-starting |
| AC-OPF | 32% gap | <1% gap | SOCP initialization |

## Architecture: The Convexity Cascade

The core insight is that solutions cascade down the convexity ladder, with each method warm-starting the next:

```
DC-OPF (LP, fast) ──warm-start──> SOCP (convex cone) ──warm-start──> AC-OPF (NLP)
     │                                  │                                  │
     └── θ, Pg ────────────────────────>└── Vm, Va, Pg, Qg ──────────────>└── Full AC solution
```

This approach:
1. Avoids cold-start convergence issues
2. Provides progressively better bounds
3. Enables fast screening with optional refinement

---

## Phase 1: DC-OPF Loss Factors

### Problem
DC-OPF ignores transmission losses (2-5% of generation), systematically underestimating costs.

### Solution: Loss-Inclusive DC-OPF (LIDC)

Add loss penalty factors to the objective function:

```rust
// Standard DC-OPF:
minimize: Σ (c₀ᵢ + c₁ᵢ · Pᵢ)

// Loss-inclusive:
minimize: Σ (c₀ᵢ + c₁ᵢ · Pᵢ · λᵢ)
// where λᵢ = 1 + marginal loss contribution at bus i
```

### Implementation

**File: `crates/gat-algo/src/opf/dc_opf.rs`**

1. `compute_loss_factors(network, dc_solution) -> Vec<f64>` (~100 lines)
   - Compute branch losses: `P_loss = r · (Pij/x)²`
   - Distribute to buses using PTDF sensitivity
   - Return vector of λᵢ factors

2. `build_lp_with_losses(network, loss_factors)` (~30 lines)
   - Modify cost coefficients: `c₁_adj = c₁ · λ`

3. `solve_with_loss_iterations(network, max_iter=3)` (~50 lines)
   - Iterative refinement: solve → compute losses → update factors → repeat

### Validation
```bash
gat benchmark pglib --method dc --pglib-dir data/pglib-opf -o results.csv
# Expect: 6.16% → ~4% average gap
```

---

## Phase 2: Warm-Start Infrastructure

### Problem
Each solver starts from scratch, missing optimization from previous stages.

### Solution: Unified Warm-Start Types

**File: `crates/gat-algo/src/opf/types.rs`**

```rust
/// Warm-start data for SOCP from DC-OPF
pub struct DcWarmStart {
    pub bus_angles: HashMap<String, f64>,
    pub generator_p: HashMap<String, f64>,
}

/// Warm-start data for AC-OPF from SOCP
pub struct SocpWarmStart {
    pub bus_voltage_mag: HashMap<String, f64>,
    pub bus_voltage_angle: HashMap<String, f64>,
    pub generator_p: HashMap<String, f64>,
    pub generator_q: HashMap<String, f64>,
    pub branch_p_flow: HashMap<String, f64>,
    pub branch_q_flow: HashMap<String, f64>,
}

/// Conversion functions
impl From<&OpfSolution> for DcWarmStart { ... }
impl From<&OpfSolution> for SocpWarmStart { ... }
```

**File: `crates/gat-algo/src/opf/mod.rs`**

```rust
/// Cascaded solver with automatic warm-starting
pub fn solve_cascaded(
    network: &Network,
    target: OpfMethod,  // Stop at this level
    config: &OpfConfig,
) -> Result<CascadedResult, OpfError> {
    let dc = solve_dc_opf(network)?;
    if target == OpfMethod::DcApprox { return Ok(dc.into()); }

    let socp = solve_socp_warm(network, &dc.into())?;
    if target == OpfMethod::SocpRelaxation { return Ok(socp.into()); }

    let ac = solve_ac_warm(network, &socp.into())?;
    Ok(ac.into())
}
```

### CLI Integration

```bash
# Explicit warm-start
gat opf socp grid.arrow --warm-start dc

# Cascaded solve (auto warm-start)
gat opf ac grid.arrow --cascaded
```

---

## Phase 3: SOCP Speed Optimization

### Problem
SOCP averages 39 seconds per case, limiting practical use.

### Solution A: Warm-Starting from DC

**File: `crates/gat-algo/src/opf/socp.rs`**

```rust
pub fn warm_start_from_dc(dc: &DcWarmStart, network: &Network) -> SocpInitialPoint {
    SocpInitialPoint {
        vm: vec![1.0; n_bus],  // flat voltage magnitudes
        va: dc.bus_angles.values().copied().collect(),
        pg: dc.generator_p.values().copied().collect(),
        qg: vec![0.0; n_gen],  // estimate later from power factor
        ell: compute_current_sq_from_flows(&dc, network),
    }
}
```

### Solution B: Solver Tuning

**File: `crates/gat-algo/src/opf/socp.rs`**

```rust
pub struct SocpSolverConfig {
    pub max_iter: usize,        // 200 → 100
    pub tol_feas: f64,          // 1e-8 → 1e-6
    pub tol_gap: f64,           // 1e-8 → 1e-6
    pub equilibrate: bool,      // true (helps conditioning)
}

impl Default for SocpSolverConfig {
    fn default() -> Self {
        Self {
            max_iter: 100,
            tol_feas: 1e-6,
            tol_gap: 1e-6,
            equilibrate: true,
        }
    }
}
```

### Validation
```bash
gat benchmark pglib --method socp --warm-start dc -o results.csv
# Expect: 39s → ~10s average time
```

---

## Phase 4: SOCP Gap Tightening

### Problem
Standard SOCP relaxation has 4.21% gap due to loose bounds.

### Solution A: OBBT (Optimization-Based Bound Tightening)

**File: `crates/gat-algo/src/opf/socp.rs`**

```rust
/// Tighten variable bounds by solving min/max LPs
pub fn tighten_bounds(network: &Network, bounds: &mut VariableBounds) -> TighteningStats {
    let mut improved = 0;

    for var_idx in 0..n_vars {
        // Minimize this variable subject to relaxed constraints
        let lb = solve_bound_lp(network, var_idx, Minimize)?;
        if lb > bounds.lower[var_idx] + 1e-6 {
            bounds.lower[var_idx] = lb;
            improved += 1;
        }

        // Maximize
        let ub = solve_bound_lp(network, var_idx, Maximize)?;
        if ub < bounds.upper[var_idx] - 1e-6 {
            bounds.upper[var_idx] = ub;
            improved += 1;
        }
    }

    TighteningStats { vars_tightened: improved }
}
```

### Solution B: QC Envelopes (McCormick relaxation for cos(θ))

```rust
/// Add quadratic convex envelope constraints for voltage angle products
pub fn add_qc_envelopes(model: &mut SocpModel, network: &Network) {
    for branch in network.branches() {
        let (i, j) = (branch.from_bus, branch.to_bus);
        let theta_max = bounds.angle_diff_max(i, j);
        let theta_min = bounds.angle_diff_min(i, j);

        // cos(θ) ≥ tangent at θ_max
        // cos(θ) ≥ tangent at θ_min
        // cos(θ) ≤ secant between θ_min and θ_max
        model.add_cos_envelope(i, j, theta_min, theta_max);
    }
}
```

### Validation
```bash
gat benchmark pglib --method socp --tighten-bounds -o results.csv
# Expect: 4.21% → ~2% average gap
```

---

## Phase 5: AC-OPF Convergence

### Problem
AC-OPF (penalty + L-BFGS) converges to poor solutions (32% gap) due to bad initialization.

### Solution: SOCP-Initialized IPOPT

**File: `crates/gat-algo/src/opf/ac_nlp/ipopt_solver.rs`**

```rust
/// Initialize IPOPT from SOCP solution
pub fn warm_start_from_socp(socp: &SocpWarmStart, problem: &AcOpfProblem) -> Vec<f64> {
    let mut x = vec![0.0; problem.n_var];

    // Voltage magnitudes (direct from SOCP)
    for (i, bus) in problem.buses.iter().enumerate() {
        x[problem.v_offset + i] = socp.bus_voltage_mag[&bus.name];
    }

    // Voltage angles (recovered from branch-flow model)
    for (i, bus) in problem.buses.iter().enumerate() {
        x[problem.theta_offset + i] = socp.bus_voltage_angle[&bus.name];
    }

    // Generator dispatch
    for (i, gen) in problem.generators.iter().enumerate() {
        x[problem.pg_offset + i] = socp.generator_p[&gen.name];
        x[problem.qg_offset + i] = socp.generator_q[&gen.name];
    }

    x
}

/// Configure IPOPT for warm-start
pub fn configure_warm_start(ipopt: &mut Ipopt) {
    ipopt.set_option("warm_start_init_point", "yes");
    ipopt.set_option("warm_start_bound_push", 1e-9);
    ipopt.set_option("warm_start_mult_bound_push", 1e-9);
    ipopt.set_option("mu_init", 1e-5);  // small barrier (near solution)
    ipopt.set_option("max_iter", 100);
    ipopt.set_option("tol", 1e-6);
}
```

### Fallback Path

If IPOPT feature not enabled, use L-BFGS with SOCP warm-start:

```rust
#[cfg(not(feature = "solver-ipopt"))]
pub fn solve_ac_warm(network: &Network, warm: &SocpWarmStart) -> Result<OpfSolution> {
    let mut solver = PenaltySolver::new(network);
    solver.set_initial_point(warm.to_vec());
    solver.solve()
}
```

### Validation
```bash
gat benchmark pglib --method ac --cascaded -o results.csv
# Expect: 32% → <1% average gap, >90% convergence
```

---

## Implementation Schedule

| Phase | Tasks | Est. Lines | Dependencies | Duration |
|-------|-------|------------|--------------|----------|
| 1 | DC loss factors | ~180 | None | 1 day |
| 2 | Warm-start types | ~150 | None | 1 day |
| 3 | SOCP speed | ~100 | Phase 2 | 0.5 day |
| 4 | SOCP tightening | ~300 | Phase 2 | 2 days |
| 5 | AC convergence | ~230 | Phases 2-4 | 1.5 days |
| **Total** | | ~960 | | ~6 days |

## Success Criteria

The plan succeeds when PGLib validation shows:

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| DC-OPF gap | 6.16% | <5% | Pending |
| SOCP gap | 4.21% | <3% | Pending |
| SOCP time | 39s | <15s | Pending |
| AC-OPF gap | 32% | <1% | Pending |
| AC convergence | ~0% | >90% | Pending |

## References

1. Baran & Wu (1989). Network reconfiguration in distribution systems. IEEE TPWRD.
2. Farivar & Low (2013). Branch Flow Model: Relaxations and Convexification. IEEE TPWRS.
3. Coffrin et al. (2015). The QC Relaxation: A Theoretical and Computational Study. IEEE TPWRS.
4. Wächter & Biegler (2006). On the implementation of an interior-point filter line-search algorithm for large-scale nonlinear programming. Math. Prog.
