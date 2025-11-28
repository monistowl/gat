# Power Flow and Optimal Power Flow

GAT provides DC and AC power flow solvers, plus optimal power flow (OPF) solvers for economic dispatch. The v0.5.0 release adds vendored native solvers (CLP, IPOPT), multi-period dispatch, and shunt element support for exact agreement with external tools.

## Power Flow Commands

### DC Power Flow

```bash
gat pf dc grid.arrow --out flows.parquet
```

Performs the classical DC approximation (`B' θ = P`) with configurable solver/backends:

* `--solver gauss|faer`: selects the linear solver registered in `gat-core::solver`.
* `--threads auto|N`: hints for Rayon's thread pool.
* `--out-partitions col1,col2`: writes partitioned Parquet under `pf-dc/` while copying a canonical file to `flows.parquet`.

The command prints branch counts, min/max flow, and the `pf-dc/.run.json` manifest for `gat runs resume`.

### AC Power Flow

```bash
gat pf ac grid.arrow --out flows.parquet --tol 1e-6 --max-iter 20
```

Runs the AC Newton-Raphson driver with tolerance/iteration controls:

* `--tol 1e-6`: exit when max mismatch drops below this tolerance.
* `--max-iter 20`: stop after this many Newton steps even if not converged.
* `--solver` and `--threads` behave as in the DC command.
* The output lives under `pf-ac/` plus the canonical `flows.parquet`.

**Q-Limit Enforcement (v0.3.3+):**

AC power flow supports PV-PQ bus switching for reactive power limits:

```bash
gat pf ac grid.arrow --out flows.parquet --enforce-q-limits
```

When a generator hits its Q limit (qmin or qmax), the bus switches from PV (voltage-controlled) to PQ (load bus) and the reactive output is clamped. This produces more physically accurate solutions.

**Shunt Element Support (v0.4.0+, enhanced in v0.5.0):**

AC power flow now includes full shunt element modeling (fixed capacitors and reactors). This is essential for achieving exact agreement with external tools like MATPOWER, PowerModels.jl, and PSS/E:

```bash
gat pf ac grid.arrow --out flows.parquet --include-shunts
```

Shunts are modeled as constant-admittance injections at their connected bus:
- `Gs`: Shunt conductance (p.u.) — real power injection
- `Bs`: Shunt susceptance (p.u.) — reactive power injection (positive = capacitive)

The Y-bus construction includes both branch charging and bus shunt elements, ensuring power balance equations match the physical network.

---

## Optimal Power Flow

### DC Optimal Power Flow

```bash
gat opf dc grid.arrow \
  --cost costs.csv \
  --limits limits.csv \
  --out dispatch.parquet
```

**Inputs:**
* Cost per bus/generator (linear or piecewise)
* Dispatch limits (Pmin/Pmax)
* Branch flow limits (optional)

**Outputs:**
* Feasible dispatch
* Branch flows
* LMPs (Locational Marginal Prices)

### AC Optimal Power Flow (SOCP Relaxation)

```bash
gat opf socp grid.arrow --out dispatch.parquet
```

Second-order cone programming relaxation of AC-OPF. Fast but may produce inexact solutions for meshed networks.

### AC Optimal Power Flow (Full Nonlinear) — v0.4.0+

```bash
gat opf ac-nlp grid.arrow --out dispatch.json --tol 1e-4 --max-iter 200
```

Full nonlinear AC-OPF using penalty method with L-BFGS optimizer (argmin crate).

**Mathematical Formulation:**

Variables: V_i (voltage magnitude), θ_i (angle), P_g, Q_g (generator dispatch)

Minimize: Σ (c₀ + c₁·P_g + c₂·P_g²)

Subject to:
- Power balance: P_inj = P_gen - P_load, Q_inj = Q_gen - Q_load
- AC power flow: P_i = Σ V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
- Voltage limits: V_min ≤ V ≤ V_max
- Generator limits: P_min ≤ P_g ≤ P_max, Q_min ≤ Q_g ≤ Q_max
- Thermal limits: P_ij² + Q_ij² ≤ S_max²

**Solver Algorithm:**
1. Convert constrained OPF to unconstrained penalty formulation
2. Solve using L-BFGS with More-Thuente line search
3. Iteratively increase penalty until constraints satisfied
4. Project solution onto bounds
5. Return generator dispatch, bus voltages, and LMPs

**Performance (PGLib v0.4.0):**
- 65/68 cases converge (95.6%)
- Median objective gap: 2.91% vs. baseline
- 48 cases under 5% gap (76%)
- Handles networks from 14 to 13,659 buses

---

## Benchmarking

### PGLib Benchmark

Run AC-OPF against the industry-standard PGLib benchmark suite:

```bash
gat benchmark pglib \
  --pglib-dir /path/to/pglib-opf \
  --baseline baseline.csv \
  --out results.csv
```

See `docs/guide/benchmark.md` for detailed benchmark documentation.

### PFDelta Benchmark

Run against the PFDelta contingency dataset (859,800 instances):

```bash
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --contingency n-1 \
  --max-cases 1000 \
  --out results.csv
```

---

## API Usage (Rust)

### Using OpfSolver

```rust
use gat_algo::{OpfMethod, OpfSolver};

let solver = OpfSolver::new()
    .with_method(OpfMethod::AcOpf)
    .with_max_iterations(200)
    .with_tolerance(1e-4);

let solution = solver.solve(&network)?;

println!("Converged: {}", solution.converged);
println!("Objective: ${:.2}", solution.objective_value);
for (gen, p) in &solution.generator_p {
    println!("  {}: {:.2} MW", gen, p);
}
```

### Available Methods

- `OpfMethod::DcOpf` — Linear DC approximation
- `OpfMethod::SocpRelaxation` — Second-order cone relaxation
- `OpfMethod::AcOpf` — Full nonlinear AC-OPF (v0.4.0+)

---

## References

- **AC-OPF Module**: `crates/gat-algo/src/opf/ac_nlp/`
- **Power Flow**: `crates/gat-algo/src/power_flow/`
- **Tests**: `crates/gat-algo/tests/ac_opf.rs`
- **PGLib**: https://github.com/power-grid-lib/pglib-opf
