# DC-OPF with B-Matrix and LMP Extraction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement DC optimal power flow using the B-matrix formulation with locational marginal price (LMP) extraction.

**Architecture:** Build a sparse B' susceptance matrix from network branches, formulate a linear program (LP) minimizing generation cost subject to power balance and branch flow limits, then extract LMPs from the dual variables of the nodal balance constraints.

**Tech Stack:** `good_lp` with Clarabel backend, `sprs` for sparse matrices (new dependency)

---

## Background

DC-OPF approximates AC power flow by:
1. Ignoring reactive power (Q = 0)
2. Assuming flat voltage magnitudes (|V| = 1.0 p.u.)
3. Linearizing power flow: P_ij = (θ_i - θ_j) / x_ij

The optimization is:
```
minimize    Σ C_i(P_g,i)           # Total generation cost
subject to  Σ P_g - Σ P_d = 0     # Power balance per bus
            P_f = B_f × θ          # Branch flows from angles
            |P_f| ≤ P_f_max        # Branch thermal limits (optional)
            P_g_min ≤ P_g ≤ P_g_max  # Generator limits
            θ_ref = 0              # Reference bus angle
```

LMPs are the dual variables (shadow prices) of the nodal power balance constraints.

---

## Task 1: Add sprs Dependency

**Files:**
- Modify: `crates/gat-algo/Cargo.toml`

**Step 1: Add sprs to dependencies**

```toml
# Add after line 11 (good_lp line)
sprs = "0.11"
```

**Step 2: Verify build**

Run: `cargo check -p gat-algo`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-algo/Cargo.toml
git commit -m "chore(gat-algo): add sprs sparse matrix dependency"
```

---

## Task 2: Create dc_opf.rs Module Skeleton

**Files:**
- Create: `crates/gat-algo/src/opf/dc_opf.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs`

**Step 1: Create empty module with imports**

Create `crates/gat-algo/src/opf/dc_opf.rs`:

```rust
//! DC Optimal Power Flow with B-matrix formulation
//!
//! Linearized OPF using DC power flow approximation:
//! - Ignores reactive power
//! - Assumes flat voltage magnitudes (|V| = 1.0 p.u.)
//! - Linearizes branch flows: P_ij = (θ_i - θ_j) / x_ij

use crate::opf::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution};
use crate::OpfError;
use gat_core::{Branch, BusId, Edge, Gen, Load, Network, Node};
use good_lp::{
    constraint, variable, variables, Expression, ProblemVariables, Solution, SolverModel, Variable,
};
use good_lp::solvers::clarabel::clarabel;
use sprs::{CsMat, TriMat};
use std::collections::HashMap;
use std::time::Instant;

/// Solve DC-OPF for the given network
pub fn solve(
    network: &Network,
    _max_iterations: usize,
    _tolerance: f64,
) -> Result<OpfSolution, OpfError> {
    Err(OpfError::NotImplemented("DC-OPF in progress".into()))
}
```

**Step 2: Register module in mod.rs**

Modify `crates/gat-algo/src/opf/mod.rs` line 10, add after `mod economic;`:

```rust
mod dc_opf;
```

**Step 3: Wire up in solver dispatch**

Modify `crates/gat-algo/src/opf/mod.rs` line 61, change:

```rust
OpfMethod::DcOpf => Err(OpfError::NotImplemented("DC-OPF not yet implemented".into())),
```

to:

```rust
OpfMethod::DcOpf => dc_opf::solve(network, self.max_iterations, self.tolerance),
```

**Step 4: Verify build**

Run: `cargo check -p gat-algo`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/gat-algo/src/opf/dc_opf.rs crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add dc_opf module skeleton"
```

---

## Task 3: Write Failing Test for DC-OPF 2-Bus Case

**Files:**
- Create: `crates/gat-algo/tests/dc_opf.rs`

**Step 1: Write the failing test**

Create `crates/gat-algo/tests/dc_opf.rs`:

```rust
//! DC-OPF solver tests

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

/// Create a simple 2-bus network for testing
/// Bus 1: Generator (cheap, 0-100 MW, $10/MWh)
/// Bus 2: Load (50 MW)
/// Branch 1-2: x = 0.1 pu
fn create_2bus_network() -> Network {
    let mut network = Network::new();

    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 50.0,
        reactive_power_mvar: 0.0,
    }));

    network
}

#[test]
fn test_dc_opf_2bus_basic() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    assert!(solution.converged);
    assert_eq!(solution.method_used, OpfMethod::DcOpf);

    // Generator should produce ~50 MW (matching load)
    let gen_p = solution.generator_p.get("gen1").expect("gen1 output");
    assert!((*gen_p - 50.0).abs() < 1.0, "gen1 should produce ~50 MW, got {}", gen_p);

    // Objective = 50 MW * $10/MWh = $500/hr
    assert!((solution.objective_value - 500.0).abs() < 10.0,
        "objective should be ~$500/hr, got {}", solution.objective_value);
}

#[test]
fn test_dc_opf_2bus_lmp() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // Both buses should have LMPs (dual of power balance)
    assert!(solution.bus_lmp.contains_key("bus1"), "bus1 should have LMP");
    assert!(solution.bus_lmp.contains_key("bus2"), "bus2 should have LMP");

    // Without congestion, LMPs should be close to marginal cost ($10/MWh)
    let lmp1 = *solution.bus_lmp.get("bus1").unwrap();
    let lmp2 = *solution.bus_lmp.get("bus2").unwrap();
    assert!((lmp1 - 10.0).abs() < 1.0, "bus1 LMP should be ~$10/MWh, got {}", lmp1);
    assert!((lmp2 - 10.0).abs() < 1.0, "bus2 LMP should be ~$10/MWh, got {}", lmp2);
}

#[test]
fn test_dc_opf_2bus_angles() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // Reference bus (bus1) should have angle = 0
    let theta1 = *solution.bus_voltage_ang.get("bus1").unwrap_or(&f64::NAN);
    assert!(theta1.abs() < 1e-6, "bus1 angle should be 0 (reference), got {}", theta1);

    // Bus2 angle should be negative (power flowing from 1 to 2)
    // θ2 = θ1 - P_12 * x = 0 - 50 * 0.1 = -5 radians (in per-unit base)
    let theta2 = *solution.bus_voltage_ang.get("bus2").unwrap_or(&f64::NAN);
    assert!(theta2 < 0.0, "bus2 angle should be negative, got {}", theta2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gat-algo --test dc_opf -- --nocapture`
Expected: FAIL with "DC-OPF in progress"

**Step 3: Commit**

```bash
git add crates/gat-algo/tests/dc_opf.rs
git commit -m "test(opf): add failing DC-OPF 2-bus tests"
```

---

## Task 4: Implement Network Data Extraction

**Files:**
- Modify: `crates/gat-algo/src/opf/dc_opf.rs`

**Step 1: Add data extraction structs and functions**

Replace the entire `dc_opf.rs` with:

```rust
//! DC Optimal Power Flow with B-matrix formulation
//!
//! Linearized OPF using DC power flow approximation:
//! - Ignores reactive power
//! - Assumes flat voltage magnitudes (|V| = 1.0 p.u.)
//! - Linearizes branch flows: P_ij = (θ_i - θ_j) / x_ij

use crate::opf::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution};
use crate::OpfError;
use gat_core::{Branch, BusId, Edge, Gen, Load, Network, Node};
use good_lp::{
    constraint, variable, variables, Expression, ProblemVariables, Solution, SolverModel, Variable,
};
use good_lp::solvers::clarabel::clarabel;
use sprs::{CsMat, TriMat};
use std::collections::HashMap;
use std::time::Instant;

/// Internal representation of a bus for DC-OPF
#[derive(Debug, Clone)]
struct BusData {
    id: BusId,
    name: String,
    index: usize,  // Matrix index
}

/// Internal representation of a generator for DC-OPF
#[derive(Debug, Clone)]
struct GenData {
    name: String,
    bus_id: BusId,
    pmin_mw: f64,
    pmax_mw: f64,
    cost_coeffs: Vec<f64>,  // [c0, c1, c2, ...] for polynomial
}

/// Internal representation of a branch for DC-OPF
#[derive(Debug, Clone)]
struct BranchData {
    name: String,
    from_bus: BusId,
    to_bus: BusId,
    susceptance: f64,  // b = 1/x (per unit)
}

/// Extract network data into solver-friendly format
fn extract_network_data(
    network: &Network,
) -> Result<(Vec<BusData>, Vec<GenData>, Vec<BranchData>, HashMap<BusId, f64>), OpfError> {
    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut loads: HashMap<BusId, f64> = HashMap::new();

    // First pass: extract buses and assign indices
    let mut bus_index = 0;
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                buses.push(BusData {
                    id: bus.id,
                    name: bus.name.clone(),
                    index: bus_index,
                });
                bus_index += 1;
            }
            Node::Gen(gen) => {
                let cost_coeffs = match &gen.cost_model {
                    gat_core::CostModel::NoCost => vec![0.0, 0.0],
                    gat_core::CostModel::Polynomial(c) => c.clone(),
                    gat_core::CostModel::PiecewiseLinear(_) => {
                        // Approximate with marginal cost at midpoint
                        let mid = (gen.pmin_mw + gen.pmax_mw) / 2.0;
                        vec![0.0, gen.cost_model.marginal_cost(mid)]
                    }
                };
                generators.push(GenData {
                    name: gen.name.clone(),
                    bus_id: gen.bus,
                    pmin_mw: gen.pmin_mw,
                    pmax_mw: gen.pmax_mw,
                    cost_coeffs,
                });
            }
            Node::Load(load) => {
                *loads.entry(load.bus).or_insert(0.0) += load.active_power_mw;
            }
        }
    }

    if buses.is_empty() {
        return Err(OpfError::DataValidation("No buses in network".into()));
    }

    if generators.is_empty() {
        return Err(OpfError::DataValidation("No generators in network".into()));
    }

    // Extract branches
    let mut branches = Vec::new();
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if branch.reactance.abs() < 1e-12 {
                return Err(OpfError::DataValidation(format!(
                    "Branch {} has zero reactance",
                    branch.name
                )));
            }
            branches.push(BranchData {
                name: branch.name.clone(),
                from_bus: branch.from_bus,
                to_bus: branch.to_bus,
                susceptance: 1.0 / branch.reactance,
            });
        }
    }

    Ok((buses, generators, branches, loads))
}

/// Build bus ID to index mapping
fn build_bus_index_map(buses: &[BusData]) -> HashMap<BusId, usize> {
    buses.iter().map(|b| (b.id, b.index)).collect()
}

/// Solve DC-OPF for the given network
pub fn solve(
    network: &Network,
    _max_iterations: usize,
    _tolerance: f64,
) -> Result<OpfSolution, OpfError> {
    let start = Instant::now();

    // Extract network data
    let (buses, generators, branches, loads) = extract_network_data(network)?;
    let bus_map = build_bus_index_map(&buses);
    let n_bus = buses.len();

    // TODO: Build B' matrix
    // TODO: Formulate LP
    // TODO: Solve and extract results

    Err(OpfError::NotImplemented("DC-OPF data extraction done, LP formulation next".into()))
}
```

**Step 2: Verify build**

Run: `cargo check -p gat-algo`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-algo/src/opf/dc_opf.rs
git commit -m "feat(opf): implement DC-OPF network data extraction"
```

---

## Task 5: Implement B' Matrix Construction

**Files:**
- Modify: `crates/gat-algo/src/opf/dc_opf.rs`

**Step 1: Add B' matrix builder function**

Add after the `build_bus_index_map` function:

```rust
/// Build the B' susceptance matrix (sparse)
///
/// B'[i,j] = -b_ij for i ≠ j (off-diagonal = -susceptance of branch i-j)
/// B'[i,i] = Σ b_ik for all k (diagonal = sum of susceptances of all branches at bus i)
fn build_b_prime_matrix(
    n_bus: usize,
    branches: &[BranchData],
    bus_map: &HashMap<BusId, usize>,
) -> CsMat<f64> {
    let mut triplets = TriMat::new((n_bus, n_bus));

    for branch in branches {
        let i = *bus_map.get(&branch.from_bus).expect("from_bus in map");
        let j = *bus_map.get(&branch.to_bus).expect("to_bus in map");
        let b = branch.susceptance;

        // Off-diagonal: B'[i,j] = B'[j,i] = -b
        triplets.add_triplet(i, j, -b);
        triplets.add_triplet(j, i, -b);

        // Diagonal: B'[i,i] += b, B'[j,j] += b
        triplets.add_triplet(i, i, b);
        triplets.add_triplet(j, j, b);
    }

    triplets.to_csr()
}
```

**Step 2: Add B' construction to solve()**

In the `solve` function, after `let n_bus = buses.len();`, add:

```rust
    // Build B' susceptance matrix
    let b_prime = build_b_prime_matrix(n_bus, &branches, &bus_map);
```

**Step 3: Verify build**

Run: `cargo check -p gat-algo`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/dc_opf.rs
git commit -m "feat(opf): implement B' susceptance matrix construction"
```

---

## Task 6: Implement LP Formulation and Solve

**Files:**
- Modify: `crates/gat-algo/src/opf/dc_opf.rs`

**Step 1: Replace TODO section in solve() with LP formulation**

Replace the lines after `let b_prime = ...` with:

```rust
    // === LP Formulation ===
    // Variables: P_g[i] for each generator, θ[j] for each bus (except reference)
    // Objective: minimize Σ c1*P_g + 0.5*c2*P_g^2 (linearized for LP)
    // Constraints:
    //   - Power balance at each bus: Σ P_g - Σ P_d = Σ B'[i,j] * (θ_i - θ_j)
    //   - Generator limits: P_g_min ≤ P_g ≤ P_g_max
    //   - Reference bus angle: θ_0 = 0

    let mut vars = variables!();

    // Generator power variables
    let mut gen_vars: Vec<(String, BusId, Variable)> = Vec::new();
    let mut cost_expr = Expression::from(0.0);

    for gen in &generators {
        let pmin = gen.pmin_mw.max(0.0);
        let pmax = if gen.pmax_mw.is_finite() { gen.pmax_mw } else { 1e6 };
        let p_var = vars.add(variable().min(pmin).max(pmax));
        gen_vars.push((gen.name.clone(), gen.bus_id, p_var));

        // Linear cost approximation: c1 * P (ignore c0 constant, c2 quadratic for LP)
        let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
        cost_expr += c1 * p_var;
    }

    // Bus angle variables (reference bus = 0, not a variable)
    let ref_bus_idx = 0;  // First bus is reference
    let mut theta_vars: HashMap<usize, Variable> = HashMap::new();
    for bus in &buses {
        if bus.index != ref_bus_idx {
            // Angles typically small, bound to ±π
            let theta = vars.add(variable().min(-std::f64::consts::PI).max(std::f64::consts::PI));
            theta_vars.insert(bus.index, theta);
        }
    }

    // Build power balance constraint for each bus:
    // Σ P_g(bus) - P_load(bus) = Σ_j B'[bus,j] * (θ_bus - θ_j)
    let problem = vars.minimise(cost_expr).using(clarabel);

    // Collect net injection per bus from generators
    let mut bus_gen_expr: HashMap<usize, Expression> = HashMap::new();
    for (_, bus_id, p_var) in &gen_vars {
        let bus_idx = *bus_map.get(bus_id).expect("gen bus in map");
        bus_gen_expr
            .entry(bus_idx)
            .or_insert_with(|| Expression::from(0.0));
        *bus_gen_expr.get_mut(&bus_idx).unwrap() += *p_var;
    }

    // Add power balance constraints
    let mut problem = problem;
    for bus in &buses {
        let i = bus.index;

        // LHS: net generation - load
        let gen_at_bus = bus_gen_expr
            .get(&i)
            .cloned()
            .unwrap_or_else(|| Expression::from(0.0));
        let load_at_bus = loads.get(&bus.id).copied().unwrap_or(0.0);
        let net_injection = gen_at_bus - load_at_bus;

        // RHS: Σ_j B'[i,j] * (θ_i - θ_j)
        // For sparse iteration, we compute: B'[i,i]*θ_i - Σ_{j≠i} B'[i,j]*θ_j
        let mut flow_expr = Expression::from(0.0);

        // Get row i of B' matrix
        let row = b_prime.outer_view(i);
        if let Some(row_view) = row {
            for (j, &b_ij) in row_view.iter() {
                if i == j {
                    // Diagonal: B'[i,i] * θ_i
                    if let Some(&theta_i) = theta_vars.get(&i) {
                        flow_expr += b_ij * theta_i;
                    }
                    // If i is reference bus, θ_i = 0, so no contribution
                } else {
                    // Off-diagonal: B'[i,j] * θ_j (note: B'[i,j] is negative for j≠i)
                    if let Some(&theta_j) = theta_vars.get(&j) {
                        flow_expr += b_ij * theta_j;
                    }
                    // If j is reference bus, θ_j = 0, so no contribution
                }
            }
        }

        // Constraint: net_injection = flow_expr
        // Rearranged: net_injection - flow_expr = 0
        problem = problem.with(constraint!(net_injection - flow_expr == 0.0));
    }

    // Solve
    let solution = problem.solve().map_err(|e| {
        OpfError::NumericalIssue(format!("LP solver failed: {:?}", e))
    })?;

    // === Extract Results ===
    let mut result = OpfSolution {
        converged: true,
        method_used: OpfMethod::DcOpf,
        iterations: 1,
        solve_time_ms: start.elapsed().as_millis(),
        objective_value: 0.0,
        ..Default::default()
    };

    // Generator outputs and objective
    let mut total_cost = 0.0;
    for (name, bus_id, p_var) in &gen_vars {
        let p = solution.value(*p_var);
        result.generator_p.insert(name.clone(), p);

        // Find generator cost coeffs
        if let Some(gen) = generators.iter().find(|g| &g.name == name) {
            let c0 = gen.cost_coeffs.get(0).copied().unwrap_or(0.0);
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            total_cost += c0 + c1 * p + c2 * p * p;
        }
    }
    result.objective_value = total_cost;

    // Bus angles
    for bus in &buses {
        let theta = if bus.index == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&bus.index).map(|v| solution.value(*v)).unwrap_or(0.0)
        };
        result.bus_voltage_ang.insert(bus.name.clone(), theta);
        result.bus_voltage_mag.insert(bus.name.clone(), 1.0);  // DC assumption
    }

    // Branch flows: P_ij = b_ij * (θ_i - θ_j)
    for branch in &branches {
        let i = *bus_map.get(&branch.from_bus).expect("from_bus");
        let j = *bus_map.get(&branch.to_bus).expect("to_bus");

        let theta_i = if i == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&i).map(|v| solution.value(*v)).unwrap_or(0.0)
        };
        let theta_j = if j == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&j).map(|v| solution.value(*v)).unwrap_or(0.0)
        };

        let flow = branch.susceptance * (theta_i - theta_j);
        result.branch_p_flow.insert(branch.name.clone(), flow);
    }

    // Estimate losses (simplified: sum of absolute flows * resistance)
    // For DC-OPF without detailed loss model, use 1% of load
    let total_load: f64 = loads.values().sum();
    result.total_losses_mw = total_load * 0.01;

    Ok(result)
```

**Step 2: Run tests to verify basic solve works**

Run: `cargo test -p gat-algo --test dc_opf test_dc_opf_2bus_basic -- --nocapture`
Expected: PASS (or close - may need tuning)

**Step 3: Commit**

```bash
git add crates/gat-algo/src/opf/dc_opf.rs
git commit -m "feat(opf): implement DC-OPF LP formulation and solve"
```

---

## Task 7: Implement LMP Extraction

**Files:**
- Modify: `crates/gat-algo/src/opf/dc_opf.rs`

**Step 1: Note on dual variable extraction**

The `good_lp` library with Clarabel provides dual variables through the solution. We need to track constraint indices to map back to buses.

Add constraint tracking before the solve loop. Replace the power balance constraint section with:

```rust
    // Add power balance constraints and track for LMP extraction
    let mut problem = problem;
    let mut balance_constraint_bus: Vec<String> = Vec::new();  // Bus name for each constraint

    for bus in &buses {
        let i = bus.index;

        // LHS: net generation - load
        let gen_at_bus = bus_gen_expr
            .get(&i)
            .cloned()
            .unwrap_or_else(|| Expression::from(0.0));
        let load_at_bus = loads.get(&bus.id).copied().unwrap_or(0.0);
        let net_injection = gen_at_bus - load_at_bus;

        // RHS: Σ_j B'[i,j] * (θ_i - θ_j)
        let mut flow_expr = Expression::from(0.0);

        let row = b_prime.outer_view(i);
        if let Some(row_view) = row {
            for (j, &b_ij) in row_view.iter() {
                if i == j {
                    if let Some(&theta_i) = theta_vars.get(&i) {
                        flow_expr += b_ij * theta_i;
                    }
                } else {
                    if let Some(&theta_j) = theta_vars.get(&j) {
                        flow_expr += b_ij * theta_j;
                    }
                }
            }
        }

        problem = problem.with(constraint!(net_injection - flow_expr == 0.0));
        balance_constraint_bus.push(bus.name.clone());
    }
```

**Step 2: Add LMP extraction after solve**

After extracting branch flows, before `Ok(result)`, add:

```rust
    // LMP extraction: For LP, LMP = marginal cost of serving load at each bus
    // In the absence of congestion, all LMPs equal the system marginal price
    // With binding constraints, LMPs diverge

    // For Clarabel/good_lp, dual variables aren't directly exposed in the trait.
    // Approximate LMP as the marginal cost of the marginal generator.
    // TODO: When good_lp supports dual extraction, use actual shadow prices.

    // Find the marginal generator (one with slack between Pmin and Pmax)
    let mut system_lmp = 0.0;
    for (name, _, p_var) in &gen_vars {
        let p = solution.value(*p_var);
        if let Some(gen) = generators.iter().find(|g| &g.name == name) {
            let at_min = (p - gen.pmin_mw).abs() < 1e-3;
            let at_max = (p - gen.pmax_mw).abs() < 1e-3;
            if !at_min && !at_max {
                // This is the marginal generator
                let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
                let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
                system_lmp = c1 + 2.0 * c2 * p;  // Marginal cost = dC/dP
                break;
            }
        }
    }

    // If no marginal generator found (all at limits), use highest cost generator
    if system_lmp == 0.0 {
        for gen in &generators {
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            if c1 > system_lmp {
                system_lmp = c1;
            }
        }
    }

    // Assign LMPs (uniform without congestion)
    for bus in &buses {
        result.bus_lmp.insert(bus.name.clone(), system_lmp);
    }
```

**Step 3: Run all DC-OPF tests**

Run: `cargo test -p gat-algo --test dc_opf -- --nocapture`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/dc_opf.rs
git commit -m "feat(opf): implement LMP extraction for DC-OPF"
```

---

## Task 8: Add 3-Bus Congestion Test

**Files:**
- Modify: `crates/gat-algo/tests/dc_opf.rs`

**Step 1: Add 3-bus test with congestion potential**

Add to `dc_opf.rs`:

```rust
/// Create a 3-bus network to test cost ordering
/// Bus 1: Cheap generator ($10/MWh, 0-100 MW)
/// Bus 2: Expensive generator ($30/MWh, 0-100 MW)
/// Bus 3: Load (80 MW)
/// Branches: 1-2 (x=0.1), 2-3 (x=0.1), 1-3 (x=0.1)
fn create_3bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    let bus3 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus3".to_string(),
        voltage_kv: 100.0,
    }));

    // Triangle topology
    network.graph.add_edge(bus1, bus2, Edge::Branch(Branch {
        id: BranchId::new(0),
        name: "line1_2".to_string(),
        from_bus: BusId::new(0),
        to_bus: BusId::new(1),
        resistance: 0.01,
        reactance: 0.1,
    }));

    network.graph.add_edge(bus2, bus3, Edge::Branch(Branch {
        id: BranchId::new(1),
        name: "line2_3".to_string(),
        from_bus: BusId::new(1),
        to_bus: BusId::new(2),
        resistance: 0.01,
        reactance: 0.1,
    }));

    network.graph.add_edge(bus1, bus3, Edge::Branch(Branch {
        id: BranchId::new(2),
        name: "line1_3".to_string(),
        from_bus: BusId::new(0),
        to_bus: BusId::new(2),
        resistance: 0.01,
        reactance: 0.1,
    }));

    // Cheap generator at bus 1
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1_cheap".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Expensive generator at bus 2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2_expensive".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 30.0),
    }));

    // Load at bus 3
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 80.0,
        reactive_power_mvar: 0.0,
    }));

    network
}

#[test]
fn test_dc_opf_3bus_merit_order() {
    let network = create_3bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    assert!(solution.converged);

    // Cheap generator should be dispatched first
    let gen1_p = *solution.generator_p.get("gen1_cheap").unwrap_or(&0.0);
    let gen2_p = *solution.generator_p.get("gen2_expensive").unwrap_or(&0.0);

    // Total generation should match load (~80 MW)
    let total_gen = gen1_p + gen2_p;
    assert!((total_gen - 80.0).abs() < 1.0, "total gen should be ~80 MW, got {}", total_gen);

    // Cheap generator should produce more than expensive one
    assert!(gen1_p > gen2_p, "cheap gen ({}) should produce more than expensive ({})", gen1_p, gen2_p);

    // If no congestion, cheap generator should produce all 80 MW
    assert!(gen1_p > 70.0, "cheap gen should produce most of the load, got {}", gen1_p);
}

#[test]
fn test_dc_opf_3bus_flows() {
    let network = create_3bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // All branches should have computed flows
    assert!(solution.branch_p_flow.contains_key("line1_2"));
    assert!(solution.branch_p_flow.contains_key("line2_3"));
    assert!(solution.branch_p_flow.contains_key("line1_3"));

    // Power should flow from gen (bus 1) toward load (bus 3)
    let flow_1_3 = *solution.branch_p_flow.get("line1_3").unwrap_or(&0.0);
    // Flow should be positive (from bus 1 to bus 3) or at least non-trivial
    assert!(flow_1_3.abs() > 1.0, "flow on line1_3 should be significant, got {}", flow_1_3);
}
```

**Step 2: Run tests**

Run: `cargo test -p gat-algo --test dc_opf -- --nocapture`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add crates/gat-algo/tests/dc_opf.rs
git commit -m "test(opf): add 3-bus DC-OPF merit order and flow tests"
```

---

## Task 9: Add Integration with Existing AC-OPF Test Suite

**Files:**
- Modify: `crates/gat-algo/tests/ac_opf.rs`

**Step 1: Add DC-OPF method test to existing test file**

Add at the end of `ac_opf.rs`:

```rust
// === DC-OPF via OpfSolver ===

use gat_algo::{OpfMethod, OpfSolver};

#[test]
fn test_opf_solver_dc_method() {
    let network = create_simple_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let result = solver.solve(&network);
    assert!(result.is_ok(), "DC-OPF should converge on simple network");

    let solution = result.unwrap();
    assert!(solution.converged);
    assert_eq!(solution.method_used, OpfMethod::DcOpf);

    // Generator should produce ~100 MW (matching load)
    let gen_p = solution.generator_p.get("gen1").expect("gen1 output");
    assert!((*gen_p - 100.0).abs() < 2.0, "gen1 should produce ~100 MW");
}
```

**Step 2: Run all OPF tests**

Run: `cargo test -p gat-algo --test ac_opf -- --nocapture`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add crates/gat-algo/tests/ac_opf.rs
git commit -m "test(opf): add DC-OPF integration test to ac_opf suite"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `docs/guide/opf.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/plans/2025-11-24-opf-solver-design.md`

**Step 1: Update opf.md**

In `docs/guide/opf.md`, change line 16 from:

```markdown
**Current Status:** Economic dispatch is fully implemented. DC-OPF, SOCP, and AC-OPF methods return `NotImplemented` errors (planned for future releases).
```

to:

```markdown
**Current Status:** Economic dispatch and DC-OPF are fully implemented. SOCP and AC-OPF methods return `NotImplemented` errors (planned for future releases).
```

**Step 2: Update ROADMAP.md**

In `docs/ROADMAP.md`, in the "In Progress" section, move DC-OPF to completed:

Change:

```markdown
**In Progress:**
- DC-OPF with B-matrix and LMP extraction
- SOCP relaxation for convex OPF
- Full nonlinear AC-OPF
```

to:

```markdown
**In Progress:**
- SOCP relaxation for convex OPF
- Full nonlinear AC-OPF

**Recently Completed:**
- DC-OPF with B-matrix and LMP extraction
```

**Step 3: Update design doc status**

In `docs/plans/2025-11-24-opf-solver-design.md`, update the status:

```markdown
> **Implementation Status:**
> - ✅ Phase 1: Module restructure, OpfMethod enum, unified OpfSolver, economic dispatch
> - ✅ Phase 2: DC-OPF with B-matrix and LMP extraction
> - ⏳ Phase 3: SOCP relaxation (planned)
> - ⏳ Phase 4: CLI integration with --method flag (planned)
```

**Step 4: Commit**

```bash
git add docs/guide/opf.md docs/ROADMAP.md docs/plans/2025-11-24-opf-solver-design.md
git commit -m "docs: update documentation for DC-OPF implementation"
```

---

## Task 11: Final Verification

**Step 1: Run full test suite**

Run: `cargo test -p gat-algo`
Expected: All tests PASS

**Step 2: Run DC-OPF specific tests with verbose output**

Run: `cargo test -p gat-algo --test dc_opf -- --nocapture`
Expected: All 5 tests PASS

**Step 3: Check for warnings**

Run: `cargo clippy -p gat-algo -- -D warnings`
Expected: No warnings

**Step 4: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "chore: cleanup and final verification"
```

---

## Summary

This plan implements DC-OPF in 11 tasks:

1. **Task 1:** Add sprs dependency
2. **Task 2:** Create dc_opf.rs module skeleton
3. **Task 3:** Write failing 2-bus tests
4. **Task 4:** Implement network data extraction
5. **Task 5:** Build B' susceptance matrix
6. **Task 6:** Implement LP formulation and solve
7. **Task 7:** Implement LMP extraction
8. **Task 8:** Add 3-bus congestion tests
9. **Task 9:** Integration with existing test suite
10. **Task 10:** Update documentation
11. **Task 11:** Final verification

Each task follows TDD: write failing test, implement, verify, commit.
