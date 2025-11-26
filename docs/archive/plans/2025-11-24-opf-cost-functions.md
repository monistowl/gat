# OPF Cost Function Implementation Plan

**Goal**: Enable GAT's AC OPF solver to minimize generator costs, allowing meaningful comparison with reference solutions from PGLib-OPF, OPFData, and other benchmark datasets.

**Current State**: The `AcOpfSolver` returns feasible operating points but uses hardcoded costs (10 $/MWh uniform), resulting in 100% objective gap on benchmarks.

---

## Phase 1: Extend Core Data Structures

### Task 1.1: Add generator limits and costs to `Gen` struct

**File**: `crates/gat-core/src/lib.rs`

```rust
#[derive(Debug, Clone)]
pub struct Gen {
    pub id: GenId,
    pub name: String,
    pub bus: BusId,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
    // NEW: Operating limits
    pub pmin_mw: f64,           // Minimum active power (MW)
    pub pmax_mw: f64,           // Maximum active power (MW)
    pub qmin_mvar: f64,         // Minimum reactive power (MVAr)
    pub qmax_mvar: f64,         // Maximum reactive power (MVAr)
    // NEW: Cost function (polynomial form: cost = c0 + c1*P + c2*P^2)
    pub cost_model: CostModel,
}

#[derive(Debug, Clone, Default)]
pub enum CostModel {
    #[default]
    NoCost,
    /// Polynomial cost: sum(coeffs[i] * P^i) where coeffs[0] is constant term
    Polynomial(Vec<f64>),
    /// Piecewise linear: Vec<(mw, $/hr)> breakpoints
    PiecewiseLinear(Vec<(f64, f64)>),
}

impl CostModel {
    /// Create quadratic cost: c0 + c1*P + c2*P^2
    pub fn quadratic(c0: f64, c1: f64, c2: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1, c2])
    }

    /// Create linear cost: c0 + c1*P (marginal cost c1 in $/MWh)
    pub fn linear(c0: f64, c1: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1])
    }

    /// Evaluate cost at given power output
    pub fn evaluate(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => {
                coeffs.iter().enumerate()
                    .map(|(i, c)| c * p_mw.powi(i as i32))
                    .sum()
            }
            CostModel::PiecewiseLinear(points) => {
                // Linear interpolation
                if points.is_empty() { return 0.0; }
                if p_mw <= points[0].0 { return points[0].1; }
                if p_mw >= points.last().unwrap().0 { return points.last().unwrap().1; }
                for i in 0..points.len()-1 {
                    if p_mw >= points[i].0 && p_mw <= points[i+1].0 {
                        let t = (p_mw - points[i].0) / (points[i+1].0 - points[i].0);
                        return points[i].1 + t * (points[i+1].1 - points[i].1);
                    }
                }
                0.0
            }
        }
    }

    /// Get marginal cost at given power (derivative of cost function)
    pub fn marginal_cost(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => {
                // d/dP[sum(c_i * P^i)] = sum(i * c_i * P^(i-1))
                coeffs.iter().enumerate().skip(1)
                    .map(|(i, c)| (i as f64) * c * p_mw.powi(i as i32 - 1))
                    .sum()
            }
            CostModel::PiecewiseLinear(points) => {
                // Slope of current segment
                if points.len() < 2 { return 0.0; }
                for i in 0..points.len()-1 {
                    if p_mw >= points[i].0 && p_mw <= points[i+1].0 {
                        return (points[i+1].1 - points[i].1) / (points[i+1].0 - points[i].0);
                    }
                }
                0.0
            }
        }
    }
}
```

**Verification**: `cargo check -p gat-core`

### Task 1.2: Update Gen constructors with defaults

Add backwards-compatible defaults:

```rust
impl Gen {
    pub fn new(id: GenId, name: String, bus: BusId) -> Self {
        Self {
            id,
            name,
            bus,
            active_power_mw: 0.0,
            reactive_power_mvar: 0.0,
            pmin_mw: 0.0,
            pmax_mw: f64::INFINITY,
            qmin_mvar: f64::NEG_INFINITY,
            qmax_mvar: f64::INFINITY,
            cost_model: CostModel::NoCost,
        }
    }
}
```

---

## Phase 2: Update Data Loaders

### Task 2.1: Update MATPOWER parser to propagate costs

**File**: `crates/gat-io/src/importers/matpower.rs`

The `MatpowerGenCost` is already parsed. We need to:
1. Pass gencost data to network builder
2. Convert MATPOWER cost format to `CostModel`

MATPOWER gencost format:
- `model=1`: Piecewise linear (ncost points)
- `model=2`: Polynomial (ncost coefficients, highest order first)

```rust
fn convert_matpower_gencost(gencost: &MatpowerGenCost) -> CostModel {
    match gencost.model {
        1 => {
            // Piecewise linear: pairs of (mw, cost)
            let points: Vec<(f64, f64)> = gencost.cost
                .chunks(2)
                .map(|chunk| (chunk[0], chunk[1]))
                .collect();
            CostModel::PiecewiseLinear(points)
        }
        2 => {
            // Polynomial: coefficients in descending order, we need ascending
            let mut coeffs: Vec<f64> = gencost.cost.clone();
            coeffs.reverse();
            CostModel::Polynomial(coeffs)
        }
        _ => CostModel::NoCost,
    }
}
```

### Task 2.2: Update OPFData parser to extract costs

**File**: `crates/gat-io/src/sources/opfdata.rs`

OPFData generator format includes cost coefficients at indices 9 and 10 (a1, a2):
- `a1`: Linear cost coefficient ($/MWh)
- `a2`: Quadratic cost coefficient ($/MW²h)

```rust
// In build_network_from_opfdata(), when parsing generators:
let a1 = gen_array.get(9).and_then(|v| v.as_f64()).unwrap_or(0.0);
let a2 = gen_array.get(10).and_then(|v| v.as_f64()).unwrap_or(0.0);

let cost_model = if a1 != 0.0 || a2 != 0.0 {
    // OPFData uses: cost = a1*P + a2*P^2 (no constant term)
    CostModel::Polynomial(vec![0.0, a1, a2])
} else {
    CostModel::NoCost
};
```

---

## Phase 3: Update AC OPF Solver

### Task 3.1: Add cost-aware solve method

**File**: `crates/gat-algo/src/ac_opf.rs`

```rust
impl AcOpfSolver {
    /// Solve AC OPF minimizing generation cost
    pub fn solve(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        let start = std::time::Instant::now();

        // Validate network
        self.validate_network(network)?;

        // Extract generators with costs and limits
        let generators: Vec<GenData> = self.extract_generators(network);
        let total_load = self.calculate_total_load(network);

        // Check feasibility
        let total_pmax: f64 = generators.iter().map(|g| g.pmax).sum();
        if total_pmax < total_load {
            return Err(AcOpfError::Infeasible(format!(
                "Generator capacity insufficient: need {} MW, max {} MW",
                total_load, total_pmax
            )));
        }

        // Solve economic dispatch (simplified DC OPF for now)
        let dispatch = self.solve_economic_dispatch(&generators, total_load)?;

        // Calculate objective value
        let objective_value: f64 = generators.iter()
            .zip(dispatch.iter())
            .map(|(gen, &p)| gen.cost_model.evaluate(p))
            .sum();

        Ok(AcOpfSolution {
            converged: true,
            objective_value,
            generator_outputs: generators.iter()
                .zip(dispatch.iter())
                .map(|(g, &p)| (g.name.clone(), p))
                .collect(),
            bus_voltages: HashMap::new(), // TODO: AC power flow
            branch_flows: HashMap::new(), // TODO: AC power flow
            iterations: 1,
            solve_time_ms: start.elapsed().as_millis(),
        })
    }

    /// Economic dispatch: minimize cost subject to power balance and gen limits
    fn solve_economic_dispatch(
        &self,
        generators: &[GenData],
        total_load: f64,
    ) -> Result<Vec<f64>, AcOpfError> {
        // For linear/quadratic costs, use merit order or QP
        // Start simple: merit order dispatch (sort by marginal cost)

        let mut dispatch = vec![0.0; generators.len()];
        let mut remaining_load = total_load * 1.01; // 1% loss estimate

        // Sort generators by marginal cost at Pmin
        let mut gen_order: Vec<usize> = (0..generators.len()).collect();
        gen_order.sort_by(|&a, &b| {
            let mc_a = generators[a].cost_model.marginal_cost(generators[a].pmin);
            let mc_b = generators[b].cost_model.marginal_cost(generators[b].pmin);
            mc_a.partial_cmp(&mc_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Dispatch in merit order
        for &idx in &gen_order {
            let gen = &generators[idx];
            let available = (gen.pmax - gen.pmin).max(0.0);
            let needed = remaining_load.min(available);
            dispatch[idx] = gen.pmin + needed;
            remaining_load -= needed;

            if remaining_load <= 0.0 {
                break;
            }
        }

        if remaining_load > 0.001 {
            return Err(AcOpfError::Infeasible(format!(
                "Cannot meet load: {} MW remaining after dispatch",
                remaining_load
            )));
        }

        Ok(dispatch)
    }
}

struct GenData {
    name: String,
    pmin: f64,
    pmax: f64,
    cost_model: CostModel,
}
```

### Task 3.2: Add proper QP solver for quadratic costs (future)

For quadratic costs, merit order gives suboptimal results. A proper implementation would use good_lp with Clarabel:

```rust
use good_lp::{constraint, default_solver, variable, Expression, SolverModel, Solution};

fn solve_qp_dispatch(
    generators: &[GenData],
    total_load: f64,
) -> Result<Vec<f64>, AcOpfError> {
    let mut vars = ProblemVariables::new();
    let mut p_vars = Vec::new();

    // Create variables for each generator
    for gen in generators {
        let p = vars.add(variable().min(gen.pmin).max(gen.pmax));
        p_vars.push(p);
    }

    // Build objective: sum of cost functions
    let mut objective = Expression::from(0.0);
    for (gen, &p) in generators.iter().zip(p_vars.iter()) {
        match &gen.cost_model {
            CostModel::Polynomial(coeffs) => {
                // Linear term
                if coeffs.len() > 1 {
                    objective += coeffs[1] * p;
                }
                // Note: Clarabel handles quadratic objectives
                // For now, linearize at midpoint
                if coeffs.len() > 2 {
                    let p_mid = (gen.pmin + gen.pmax) / 2.0;
                    let marginal = 2.0 * coeffs[2] * p_mid + coeffs.get(1).unwrap_or(&0.0);
                    objective += marginal * p;
                }
            }
            _ => {}
        }
    }

    // Power balance constraint
    let sum_p: Expression = p_vars.iter().copied().sum();
    let problem = vars.minimise(objective)
        .using(default_solver)
        .with(constraint!(sum_p == total_load * 1.01));

    let solution = problem.solve().map_err(|e|
        AcOpfError::NumericalIssue(format!("QP solve failed: {:?}", e))
    )?;

    Ok(p_vars.iter().map(|&p| solution.value(p)).collect())
}
```

---

## Phase 4: Update Benchmarks

### Task 4.1: Verify objective computation in benchmarks

**Files**:
- `crates/gat-cli/src/commands/benchmark/pglib.rs`
- `crates/gat-cli/src/commands/benchmark/opfdata.rs`

No changes needed - they already extract `solution.objective_value` and compare to baseline.

### Task 4.2: Add cost parsing to test fixtures

Ensure test_data/pglib cases have gencost data and it's being parsed.

---

## Phase 5: Testing & Validation

### Task 5.1: Unit tests for CostModel

```rust
#[test]
fn test_quadratic_cost() {
    // cost = 100 + 20*P + 0.01*P^2
    let cost = CostModel::quadratic(100.0, 20.0, 0.01);

    assert!((cost.evaluate(0.0) - 100.0).abs() < 1e-6);
    assert!((cost.evaluate(100.0) - (100.0 + 2000.0 + 100.0)).abs() < 1e-6);

    // Marginal cost = 20 + 0.02*P
    assert!((cost.marginal_cost(0.0) - 20.0).abs() < 1e-6);
    assert!((cost.marginal_cost(100.0) - 22.0).abs() < 1e-6);
}

#[test]
fn test_piecewise_linear_cost() {
    let cost = CostModel::PiecewiseLinear(vec![
        (0.0, 0.0),
        (50.0, 1000.0),
        (100.0, 2500.0),
    ]);

    assert!((cost.evaluate(25.0) - 500.0).abs() < 1e-6);
    assert!((cost.evaluate(75.0) - 1750.0).abs() < 1e-6);
}
```

### Task 5.2: Integration test with PGLib case14

```rust
#[test]
fn test_pglib_case14_objective() {
    let network = load_matpower_network("test_data/pglib/pglib_opf_case14_ieee/case.m")?;
    let solver = AcOpfSolver::new();
    let solution = solver.solve(&network)?;

    // PGLib baseline objective for case14
    let baseline = 2178.08; // from baseline.csv or reference
    let gap = (solution.objective_value - baseline).abs() / baseline;

    assert!(gap < 0.01, "Objective gap {} > 1%", gap);
}
```

---

## Implementation Order

1. **Phase 1** (Core): Add CostModel enum and extend Gen struct (~1 hour)
2. **Phase 2** (IO): Update MATPOWER and OPFData parsers (~1 hour)
3. **Phase 3** (Solver): Implement merit-order dispatch (~1 hour)
4. **Phase 5** (Testing): Add unit and integration tests (~30 min)
5. **Verify**: Run benchmarks, expect <10% objective gap on PGLib

Total estimated time: ~4 hours for basic implementation, with QP solver as future enhancement.

---

## Expected Results After Implementation

| Dataset | Current Gap | Expected Gap |
|---------|-------------|--------------|
| PGLib-OPF case14 | N/A (no cost) | <5% |
| PGLib-OPF case118 | N/A | <10% |
| OPFData case118 | 100% | <15%* |

*OPFData gap may be higher because our DC approximation doesn't account for AC losses and reactive power.

---

## Future Enhancements

1. **True AC OPF**: Newton-Raphson with cost minimization in inner loop
2. **SDP/SOCP Relaxations**: Convex relaxations for global optimality bounds
3. **Interior-point QP**: Use Clarabel for quadratic costs directly
4. **Branch thermal limits**: Add MVA flow constraints
5. **Voltage bounds**: Add 0.95-1.05 p.u. constraints
