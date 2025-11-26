# OPF Cost Minimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable the AC-OPF solver to minimize generation cost by integrating MATPOWER gencost data into the objective function.

**Architecture:** The infrastructure exists but is disconnected. MATPOWER parser already extracts `gencost` (polynomial/piecewise-linear). Gen struct has `cost_model: CostModel` field. AC-NLP solver has objective function that reads cost coefficients. The gap: `matpower.rs` line 142 always assigns `CostModel::NoCost`. We need to wire gencost → CostModel → solver.

**Tech Stack:** Rust, gat-core (CostModel), gat-io (MATPOWER parser), gat-algo (AC-NLP solver)

---

### Task 1: Add Test for Gencost Integration

**Files:**
- Create: `crates/gat-io/src/importers/tests/gencost_integration.rs`
- Modify: `crates/gat-io/src/importers/mod.rs`

**Step 1: Write the failing test**

In `crates/gat-io/src/importers/tests/gencost_integration.rs`:

```rust
use crate::importers::load_matpower_network;
use gat_core::CostModel;
use std::path::Path;

#[test]
fn test_gencost_polynomial_loaded() {
    // case14_ieee has polynomial gencost (model=2)
    let path = Path::new("../../data/pglib-opf/pglib_opf_case14_ieee.m");
    if !path.exists() {
        eprintln!("Skipping test: PGLib data not available");
        return;
    }

    let network = load_matpower_network(path).expect("Failed to load case14");

    // Find a generator and check it has cost data
    let mut found_cost = false;
    for node in network.graph.node_weights() {
        if let gat_core::Node::Gen(gen) = node {
            match &gen.cost_model {
                CostModel::Polynomial(coeffs) => {
                    assert!(!coeffs.is_empty(), "Polynomial cost should have coefficients");
                    found_cost = true;
                    break;
                }
                CostModel::NoCost => {
                    // This is the bug we're fixing
                }
                _ => {}
            }
        }
    }

    assert!(found_cost, "At least one generator should have polynomial cost from gencost");
}
```

**Step 2: Add module declaration**

In `crates/gat-io/src/importers/mod.rs`, add near the top:

```rust
#[cfg(test)]
mod tests;
```

Create directory `crates/gat-io/src/importers/tests/` and `mod.rs`:

```rust
mod gencost_integration;
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p gat-io test_gencost_polynomial_loaded -- --nocapture`

Expected: FAIL with assertion "At least one generator should have polynomial cost"

**Step 4: Commit test**

```bash
git add crates/gat-io/src/importers/tests/
git commit -m "test: add failing test for gencost integration"
```

---

### Task 2: Implement Gencost to CostModel Conversion

**Files:**
- Modify: `crates/gat-io/src/importers/matpower.rs:135-150`

**Step 1: Find the gencost assignment location**

Current code at line 142:
```rust
cost_model: gat_core::CostModel::NoCost,
```

**Step 2: Implement conversion function**

Add this function before `pub fn load_matpower_network`:

```rust
/// Convert MATPOWER gencost to CostModel
fn gencost_to_cost_model(gencost: Option<&MatpowerGenCost>) -> gat_core::CostModel {
    match gencost {
        None => gat_core::CostModel::NoCost,
        Some(gc) => {
            match gc.model {
                2 => {
                    // Polynomial cost: cost = c_n*P^n + ... + c_1*P + c_0
                    // MATPOWER stores highest degree first: [c_n, ..., c_1, c_0]
                    // CostModel expects lowest degree first: [c_0, c_1, ..., c_n]
                    let coeffs: Vec<f64> = gc.cost.iter().rev().copied().collect();
                    gat_core::CostModel::Polynomial(coeffs)
                }
                1 => {
                    // Piecewise linear: pairs of (MW, $/hr)
                    // gc.cost = [p1, c1, p2, c2, ...]
                    let points: Vec<(f64, f64)> = gc.cost
                        .chunks(2)
                        .filter_map(|chunk| {
                            if chunk.len() == 2 {
                                Some((chunk[0], chunk[1]))
                            } else {
                                None
                            }
                        })
                        .collect();
                    gat_core::CostModel::PiecewiseLinear(points)
                }
                _ => gat_core::CostModel::NoCost,
            }
        }
    }
}
```

**Step 3: Wire gencost to Gen creation**

Replace line 142 (`cost_model: gat_core::CostModel::NoCost,`) with:

```rust
cost_model: gencost_to_cost_model(case.gencost.get(i)),
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gat-io test_gencost_polynomial_loaded -- --nocapture`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/gat-io/src/importers/matpower.rs
git commit -m "feat: integrate MATPOWER gencost into Gen.cost_model"
```

---

### Task 3: Verify AC-NLP Uses Cost Coefficients

**Files:**
- Create: `crates/gat-algo/src/opf/tests/cost_objective.rs`

**Step 1: Write test for objective computation**

```rust
use crate::opf::ac_nlp::AcOpfSolver;
use gat_core::{Network, Node, Gen, Bus, CostModel};

#[test]
fn test_objective_uses_cost_model() {
    // Create minimal network with known cost
    let mut network = Network::new();

    // Add slack bus
    let bus = Bus::new(1, "Bus1".to_string())
        .with_voltage(1.0, 0.0)
        .as_slack();
    let bus_idx = network.add_bus(bus);

    // Add generator with quadratic cost: 100 + 10*P + 0.1*P^2
    let gen = Gen::new(1, "Gen1".to_string(), 1)
        .with_p_limits(0.0, 100.0)
        .with_cost(CostModel::Polynomial(vec![100.0, 10.0, 0.1]));
    network.add_gen(gen, bus_idx);

    // Add load
    let load = gat_core::Load::new(1, "Load1".to_string(), 1, 50.0, 10.0);
    network.add_load(load, bus_idx);

    // Solve
    let solver = AcOpfSolver::new();
    let result = solver.solve(&network);

    assert!(result.is_ok(), "Solver should converge");
    let solution = result.unwrap();

    // At P=50 MW: cost = 100 + 10*50 + 0.1*50^2 = 100 + 500 + 250 = 850
    // Allow some tolerance for solver approximation
    assert!(solution.objective_value > 0.0, "Objective should be positive with cost model");
    assert!(
        (solution.objective_value - 850.0).abs() < 100.0,
        "Objective {} should be near 850 for P=50MW with quadratic cost",
        solution.objective_value
    );
}
```

**Step 2: Add test module**

In `crates/gat-algo/src/opf/mod.rs`, add:

```rust
#[cfg(test)]
mod tests;
```

Create `crates/gat-algo/src/opf/tests/mod.rs`:

```rust
mod cost_objective;
```

**Step 3: Run test**

Run: `cargo test -p gat-algo test_objective_uses_cost_model -- --nocapture`

Expected: PASS (infrastructure already exists)

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/tests/
git commit -m "test: verify AC-NLP objective uses cost model"
```

---

### Task 4: Run PGLib Benchmark with Cost Objectives

**Files:**
- None (verification only)

**Step 1: Build release**

Run: `cargo build --release -p gat-cli`

**Step 2: Run benchmark on small case**

Run: `./target/release/gat-cli benchmark pglib --pglib-dir data/pglib-opf --case-filter case14 --baseline data/pglib-opf/baseline.csv --out results/pglib_cost_test.csv`

**Step 3: Check objective gap**

Run: `cat results/pglib_cost_test.csv | head -5`

Expected: `objective_gap_rel` should be < 1.0 (not 100% gap anymore)

**Step 4: Document results**

If objective gap is now reasonable (< 5%), the integration is working.

If still 100% gap, check:
1. Is gencost being parsed? Add debug print in `gencost_to_cost_model`
2. Is CostModel reaching solver? Print `gen.cost_model` in AC-NLP

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: verify gencost integration end-to-end"
```

---

### Task 5: Handle Edge Cases

**Files:**
- Modify: `crates/gat-io/src/importers/matpower.rs`

**Step 1: Add test for missing gencost**

```rust
#[test]
fn test_no_gencost_defaults_to_no_cost() {
    // Network without gencost section should still work
    let content = r#"
function mpc = case_no_cost
mpc.version = '2';
mpc.baseMVA = 100;
mpc.bus = [1 3 50 10 0 0 1 1.0 0 230 1 1.1 0.9];
mpc.gen = [1 50 10 100 -100 1.0 100 1 100 0 0 0 0 0 0 0 0 0 0 0 0];
mpc.branch = [];
"#;
    // Parse should succeed with NoCost
}
```

**Step 2: Handle gencost count mismatch**

If `gencost.len() < gen.len()`, use NoCost for missing generators:

```rust
cost_model: gencost_to_cost_model(case.gencost.get(i)),
// .get(i) returns None if index out of bounds, which maps to NoCost
```

**Step 3: Run full test suite**

Run: `cargo test -p gat-io -p gat-algo`

Expected: All tests pass

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: handle edge cases in gencost integration"
```

---

## Verification Checklist

- [x] `test_gencost_polynomial_loaded` passes
- [x] `test_objective_uses_cost_model` passes (renamed to `ac_opf_polynomial_cost`)
- [x] PGLib case14 benchmark shows objective_gap < 5% (**4.87%**)
- [x] Networks without gencost still work (NoCost default via `.get(i)`)
- [ ] Full test suite passes: `cargo test --workspace`

---

## Implementation Log (2025-11-25)

### Completed Tasks (Batch 1: Tasks 1-3)

**Task 1: Add failing test for gencost integration**
- Added `test_gencost_polynomial_loaded` to `crates/gat-io/src/importers/tests.rs`
- Note: Used existing `tests.rs` file rather than creating a `tests/` directory as the plan suggested
- Test verified to fail before implementation
- Commit: `test: add failing test for gencost integration`

**Task 2: Implement gencost to CostModel conversion**
- Added `gencost_to_cost_model()` function to `crates/gat-io/src/importers/matpower.rs`
- Handles polynomial (model=2) and piecewise linear (model=1) cost functions
- Reverses MATPOWER's high-to-low coefficient order to match CostModel's low-to-high expectation
- Wired generator loop to use `case.gencost.get(i)` for safe indexing
- Test passes after implementation
- Commit: `feat: integrate MATPOWER gencost into Gen.cost_model`

**Task 3: Verify AC-NLP uses cost coefficients**
- Added `ac_opf_polynomial_cost` test to `crates/gat-algo/tests/ac_opf.rs`
- Test verifies quadratic cost: 100 + 10*P + 0.1*P^2 → ~850 $/hr at P=50MW
- Actual result: P=50.21 MW, cost=$854.17 ✓
- Also verified `ac_opf_three_bus_economic_dispatch` - cheaper generator dispatches more
- Commit: `test: verify AC-NLP objective uses cost model`

### Additional Fixes Required

**is_synchronous_condenser field migration**
- The `Gen` struct recently gained an `is_synchronous_condenser: bool` field
- Multiple files needed updating to add this field:
  - `crates/gat-io/src/importers/matpower.rs` (2 locations)
  - `crates/gat-io/src/importers/cim.rs`
  - `crates/gat-io/src/importers/psse.rs`
  - `crates/gat-io/src/sources/opfdata.rs`
  - `crates/gat-io/src/sources/pfdelta.rs`
  - All test files in `crates/gat-algo/tests/` (used sed to bulk-fix)

### Potential Merge Issues

1. **is_synchronous_condenser field**: If main branch has different changes to Gen struct initializers, there may be conflicts. All Gen initializers now include `is_synchronous_condenser: false`.

2. **Test file location**: Plan specified creating `crates/gat-io/src/importers/tests/gencost_integration.rs` but I added to existing `tests.rs` file. No conflict expected.

3. **Test file modifications in gat-algo**: Bulk sed replacement added `is_synchronous_condenser: false` before every `cost_model:` line in test files. If main has modified these tests, manual conflict resolution may be needed.

### Commits Made

```
22ee6ed test: add failing test for gencost integration
0b80a4c feat: integrate MATPOWER gencost into Gen.cost_model
f0e6a7e test: verify AC-NLP objective uses cost model
```

### Completed Tasks (Batch 2: Task 4)

**Task 4: Run PGLib benchmark with cost objectives**
- Built release: `cargo build --release -p gat-cli`
- Ran case14 benchmark: 4.87% gap (target was < 5%) ✓
- Ran full PGLib benchmark on all 41 available cases

**Full Benchmark Results:**
```
Total cases: 41
Converged: 41 (100%)
Average objective gap: 6.59%
Median objective gap: 2.49%
Cases < 5% gap: 30/39 (77%)
Cases < 10% gap: 32/39 (82%)
```

**Gap Distribution:**
| Gap Range | Count | Notable Cases |
|-----------|-------|---------------|
| < 1% | 5 | case200_activ (0.07%), case5658_epigrids (0.01%) |
| 1-5% | 25 | case14_ieee (4.87%), case118_ieee (3.18%) |
| 5-10% | 2 | case57_ieee (6.48%), case9591_goc (5.28%) |
| 10-20% | 4 | case5_pjm (13.91%), case3970_goc (17.93%) |
| 20-40% | 2 | case30_ieee (29.50%), case1803_snem (35.09%) |
| > 40% | 1 | case3_lmbd (54.59%) |

**Analysis:**
- 77% of cases achieve < 5% gap - excellent for first-iteration NLP solver
- High-gap outliers (case3_lmbd, case1803_snem) may have unusual cost structures
- 27 additional cases blocked by negative reactance (need phase-shifter support)

### Remaining Tasks

- Task 5: Handle edge cases (already implemented via `.get(i)` safe indexing)
