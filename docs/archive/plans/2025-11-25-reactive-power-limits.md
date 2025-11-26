# Reactive Power Limits (Q-Limit) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enforce generator reactive power limits (Qmin, Qmax) during AC power flow, converting PV buses to PQ buses when Q limits are hit.

**Architecture:** The AC power flow solver treats generators at PV buses (voltage-controlled). When a generator hits its Q limit, it can no longer control voltage and should switch to PQ mode. This requires: (1) checking Q against limits after each iteration, (2) converting bus type when limit is hit, (3) re-running until no more conversions needed. This is the standard "Q-limit enforcement" or "PV-PQ switching" algorithm.

**Tech Stack:** Rust, gat-algo (AC power flow solver), gat-core (Bus, Gen types)

---

### Task 1: Add Q-Limit Test Case

**Files:**
- Create: `crates/gat-algo/src/power_flow/tests/q_limits.rs`

**Step 1: Write failing test**

```rust
use gat_core::{Network, Bus, Gen, Load, Branch};
use crate::power_flow::AcPowerFlowSolver;

#[test]
fn test_q_limit_enforcement() {
    // Create network where generator Q limit will be hit
    let mut network = Network::new();

    // Bus 1: Slack
    let bus1 = Bus::new(1, "Slack".to_string())
        .with_voltage(1.0, 0.0)
        .as_slack();
    let b1 = network.add_bus(bus1);

    // Bus 2: PV bus with limited Q
    let bus2 = Bus::new(2, "PV".to_string())
        .with_voltage(1.05, 0.0);  // Voltage setpoint
    let b2 = network.add_bus(bus2);

    // Generator at bus 1 (slack, unlimited)
    let gen1 = Gen::new(1, "Gen1".to_string(), 1)
        .with_p_limits(0.0, 200.0)
        .with_q_limits(-100.0, 100.0);
    network.add_gen(gen1, b1);

    // Generator at bus 2 with TIGHT Q limits
    let gen2 = Gen::new(2, "Gen2".to_string(), 2)
        .with_p_limits(50.0, 50.0)  // Fixed P
        .with_q_limits(0.0, 10.0);  // Very limited Q
    network.add_gen(gen2, b2);

    // Heavy reactive load that will exceed gen2's Q limit
    let load = Load::new(1, "Load1".to_string(), 2, 40.0, 50.0);  // 50 MVAR
    network.add_load(load, b2);

    // Connect buses
    let branch = Branch::new(1, "Line1".to_string(), 1, 2)
        .with_impedance(0.01, 0.1);
    network.add_branch(branch, b1, b2);

    // Solve power flow
    let solver = AcPowerFlowSolver::new()
        .with_q_limit_enforcement(true);  // NEW OPTION
    let result = solver.solve(&network);

    assert!(result.is_ok(), "Power flow should converge");
    let solution = result.unwrap();

    // Gen2's Q should be at its limit (10 MVAR), not higher
    let gen2_q = solution.generator_q_mvar.get(&2).copied().unwrap_or(0.0);
    assert!(
        gen2_q <= 10.0 + 0.1,  // Allow small tolerance
        "Gen2 Q ({}) should be at or below limit (10 MVAR)",
        gen2_q
    );

    // Bus 2 voltage should have dropped below setpoint (can't hold it)
    let bus2_vm = solution.bus_voltage_magnitude.get(&2).copied().unwrap_or(1.0);
    assert!(
        bus2_vm < 1.05,
        "Bus 2 voltage ({}) should drop below setpoint when Q-limited",
        bus2_vm
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gat-algo test_q_limit_enforcement`

Expected: FAIL - method `with_q_limit_enforcement` not found

**Step 3: Commit test**

```bash
git add crates/gat-algo/src/power_flow/tests/
git commit -m "test: add failing test for Q-limit enforcement"
```

---

### Task 2: Add Q-Limit Enforcement Option to Solver

**Files:**
- Modify: `crates/gat-algo/src/power_flow/mod.rs` (or ac_pf.rs)

**Step 1: Add configuration field**

```rust
pub struct AcPowerFlowSolver {
    pub tolerance: f64,
    pub max_iterations: usize,
    pub enforce_q_limits: bool,  // NEW
}

impl AcPowerFlowSolver {
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 20,
            enforce_q_limits: false,  // Default: off for backward compatibility
        }
    }

    pub fn with_q_limit_enforcement(mut self, enable: bool) -> Self {
        self.enforce_q_limits = enable;
        self
    }
}
```

**Step 2: Run test**

Run: `cargo test -p gat-algo test_q_limit_enforcement`

Expected: FAIL - enforcement not implemented yet, but compiles

**Step 3: Commit**

```bash
git add crates/gat-algo/src/power_flow/
git commit -m "feat: add Q-limit enforcement option to AcPowerFlowSolver"
```

---

### Task 3: Implement PV-PQ Bus Switching

**Files:**
- Modify: `crates/gat-algo/src/power_flow/ac_pf.rs` (main solve loop)

**Step 1: Add bus type tracking**

After the Newton-Raphson converges, check Q limits:

```rust
/// Check generator Q against limits and switch bus types if needed
fn check_q_limits(
    &self,
    network: &Network,
    bus_types: &mut HashMap<BusId, BusType>,
    gen_q: &HashMap<GenId, f64>,
) -> bool {
    let mut switched = false;

    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            let q = gen_q.get(&gen.id).copied().unwrap_or(0.0);

            // Check if Q is outside limits
            if q > gen.qmax_mvar {
                // Hit upper limit - fix Q at Qmax, release voltage
                if bus_types.get(&gen.bus) == Some(&BusType::PV) {
                    bus_types.insert(gen.bus, BusType::PQ);
                    switched = true;
                    eprintln!(
                        "Bus {} switched PV->PQ: Q={:.2} > Qmax={:.2}",
                        gen.bus, q, gen.qmax_mvar
                    );
                }
            } else if q < gen.qmin_mvar {
                // Hit lower limit - fix Q at Qmin, release voltage
                if bus_types.get(&gen.bus) == Some(&BusType::PV) {
                    bus_types.insert(gen.bus, BusType::PQ);
                    switched = true;
                    eprintln!(
                        "Bus {} switched PV->PQ: Q={:.2} < Qmin={:.2}",
                        gen.bus, q, gen.qmin_mvar
                    );
                }
            }
        }
    }

    switched
}
```

**Step 2: Add outer loop for Q-limit iterations**

```rust
pub fn solve(&self, network: &Network) -> Result<PowerFlowSolution> {
    // Initialize bus types from network
    let mut bus_types = self.classify_buses(network);

    let max_q_iterations = 10;
    for q_iter in 0..max_q_iterations {
        // Run Newton-Raphson with current bus types
        let solution = self.newton_raphson(network, &bus_types)?;

        if !self.enforce_q_limits {
            return Ok(solution);
        }

        // Check Q limits and switch if needed
        let gen_q = self.compute_generator_q(network, &solution);
        let switched = self.check_q_limits(network, &mut bus_types, &gen_q);

        if !switched {
            // No more switches needed - converged
            return Ok(solution);
        }

        eprintln!("Q-limit iteration {}: buses switched, re-solving", q_iter + 1);
    }

    Err(anyhow!("Q-limit enforcement did not converge in {} iterations", max_q_iterations))
}
```

**Step 3: Implement compute_generator_q**

```rust
fn compute_generator_q(
    &self,
    network: &Network,
    solution: &PowerFlowSolution,
) -> HashMap<GenId, f64> {
    let mut gen_q = HashMap::new();

    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            // Q_gen = Q_injected + Q_load at bus
            // From power balance: Q_gen = Q_calc - Q_load
            let bus_q_calc = solution.bus_q_injection.get(&gen.bus).copied().unwrap_or(0.0);
            // For simplicity, assume gen provides all Q at its bus
            gen_q.insert(gen.id, bus_q_calc);
        }
    }

    gen_q
}
```

**Step 4: Run test**

Run: `cargo test -p gat-algo test_q_limit_enforcement`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/gat-algo/src/power_flow/
git commit -m "feat: implement PV-PQ switching for Q-limit enforcement"
```

---

### Task 4: Handle Q at Limit Correctly

**Files:**
- Modify: `crates/gat-algo/src/power_flow/ac_pf.rs`

**Step 1: Add test for Q clamping**

```rust
#[test]
fn test_q_clamped_at_limit() {
    // When Q limit is hit, the generator's Q should be exactly at the limit
    // ... similar setup as above ...

    let solution = solver.solve(&network).unwrap();
    let gen2_q = solution.generator_q_mvar.get(&2).copied().unwrap_or(0.0);

    // Should be clamped to exactly Qmax
    assert!(
        (gen2_q - 10.0).abs() < 0.01,
        "Gen2 Q ({}) should be clamped to Qmax (10)",
        gen2_q
    );
}
```

**Step 2: Clamp Q in solution**

When a bus is in PQ mode due to Q limit, set Q to the limit value:

```rust
fn finalize_solution(
    &self,
    solution: &mut PowerFlowSolution,
    bus_types: &HashMap<BusId, BusType>,
    gen_q_limits: &HashMap<GenId, (f64, f64)>,  // (qmin, qmax)
) {
    for (gen_id, q) in solution.generator_q_mvar.iter_mut() {
        if let Some(&(qmin, qmax)) = gen_q_limits.get(gen_id) {
            if *q > qmax {
                *q = qmax;
            } else if *q < qmin {
                *q = qmin;
            }
        }
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p gat-algo q_limit`

Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: clamp generator Q at limits in final solution"
```

---

### Task 5: Add CLI Flag for Q-Limit Enforcement

**Files:**
- Modify: `crates/gat-cli/src/commands/pf.rs`

**Step 1: Add CLI argument**

```rust
#[derive(Parser)]
pub struct PfCommand {
    // ... existing args ...

    /// Enforce generator reactive power limits (PV-PQ switching)
    #[arg(long, default_value = "false")]
    pub enforce_q_limits: bool,
}
```

**Step 2: Pass to solver**

```rust
let solver = AcPowerFlowSolver::new()
    .with_tolerance(args.tol)
    .with_max_iterations(args.max_iter)
    .with_q_limit_enforcement(args.enforce_q_limits);
```

**Step 3: Test CLI**

Run: `./target/release/gat-cli pf ac grid.arrow --enforce-q-limits --out result.parquet`

Expected: No errors, output file created

**Step 4: Commit**

```bash
git add crates/gat-cli/src/commands/pf.rs
git commit -m "feat: add --enforce-q-limits flag to pf command"
```

---

### Task 6: Integration Test with PGLib Case

**Files:**
- None (verification)

**Step 1: Run on case14**

```bash
./target/release/gat-cli pf ac data/pglib-opf/pglib_opf_case14_ieee.m \
    --enforce-q-limits \
    --out results/case14_qlim.parquet
```

**Step 2: Verify Q values are within limits**

```bash
# Use Python/polars to check
python3 -c "
import polars as pl
df = pl.read_parquet('results/case14_qlim.parquet')
print(df.select(['gen_id', 'q_mvar', 'qmin_mvar', 'qmax_mvar']))
# All q_mvar should be between qmin and qmax
"
```

**Step 3: Commit results**

```bash
git add results/
git commit -m "test: verify Q-limit enforcement on PGLib case14"
```

---

## Verification Checklist

- [x] `test_q_limit_enforcement` passes
- [x] `test_q_clamped_at_limit` passes
- [x] `test_q_limit_not_enforced_by_default` passes
- [x] `test_pv_to_pq_switching` passes
- [ ] CLI flag `--enforce-q-limits` works
- [ ] PGLib case14 runs with Q-limits enforced
- [ ] All generator Q values in solution are within [Qmin, Qmax]
- [x] PV buses correctly switch to PQ when limited
- [ ] Full test suite: `cargo test --workspace`

---

## Implementation Log (2025-11-25)

### Batch 1: Tasks 1-4 Completed

**Files Created:**
- `crates/gat-algo/src/power_flow/mod.rs` - Module definition with re-exports
- `crates/gat-algo/src/power_flow/ac_pf.rs` - Full Newton-Raphson AC power flow solver
- `crates/gat-algo/src/power_flow/q_limits.rs` - Q-limit test cases

**Files Modified:**
- `crates/gat-algo/src/power_flow.rs` → moved to `crates/gat-algo/src/power_flow/legacy.rs`

**Test Results:**
```
running 4 tests
test power_flow::q_limits::tests::test_q_limit_enforcement ... ok
test power_flow::q_limits::tests::test_q_clamped_at_limit ... ok
test power_flow::q_limits::tests::test_pv_to_pq_switching ... ok
test power_flow::q_limits::tests::test_q_limit_not_enforced_by_default ... ok

test result: ok. 4 passed; 0 failed
```

### Implementation Details

The new `AcPowerFlowSolver` implements:

1. **Newton-Raphson in polar coordinates** - Full Jacobian computation with ∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V
2. **Bus type classification** - Slack, PV, PQ with proper variable assignment
3. **Y-bus construction** - Admittance matrix from branch impedances
4. **Q-limit enforcement outer loop** - Checks Q against limits, switches PV→PQ, re-solves
5. **Q-spec updating** - When a generator hits its Q limit, `q_spec` is updated to fix Q at the limit

### Deviations from Plan

1. **API differences**: The plan assumed `Bus::new().with_voltage().as_slack()` builder pattern which doesn't exist. Tests use the actual `gat_core` types (`BusId`, `GenId`, etc.) with `Network.graph.add_node(Node::Bus(...))`.

2. **Module structure**: Created `power_flow/` directory module instead of single file. Moved existing `power_flow.rs` to `power_flow/legacy.rs` to preserve backward compatibility.

3. **Additional test**: Added `test_q_limit_not_enforced_by_default` to verify default behavior doesn't enforce limits.

### Known Issues / Potential Concerns for Merging

1. **Compiler warnings**: 6 warnings about unused variables/fields (can be cleaned up):
   - `solution` assigned but overwritten
   - `bus_idx_map` unused in `build_solution`
   - `pmin_mw`, `pmax_mw` unused in `GeneratorData`
   - `shift` unused in `BranchData`

2. **Pre-existing test failures**: Other test files in `crates/gat-algo/tests/` fail with `missing field is_synchronous_condenser`. This is unrelated to this change - those tests use struct literal syntax directly instead of builder patterns.

3. **Simple linear solver**: Uses Gaussian elimination with partial pivoting. For large networks, may want to use a sparse solver (e.g., from `nalgebra-sparse` or `faer`).

4. **Single generator per bus assumption**: The current implementation assumes one generator per bus for Q calculation. Multiple generators at the same bus would need proportional Q allocation.

5. **No PQ→PV switching**: Once a bus switches to PQ, it stays PQ. Full implementation would check if Q moves back within limits and switch back to PV.

### Remaining Tasks

- Task 5: Add `--enforce-q-limits` CLI flag to `pf ac` command
- Task 6: Integration test with PGLib case14
