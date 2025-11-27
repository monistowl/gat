+++
title = "Power Flow"
description = "Power flow commands"
weight = 10
+++

# Power Flow Commands

`gat pf` runs the DC or AC power-flow solvers built on `gat-core` graphs plus the shared solver registry (`gat-core::solver::SolverKind`). The CLI exposes the same options for both solvers so you can switch backends, control threading, and stage the output for downstream tools.

{% callout(type="tip") %}
**Quick start:** For most use cases, start with DC power flow for fast linearized analysis, then move to AC for precise nonlinear results.
{% end %}

## `gat pf dc <grid.arrow> --out flows.parquet`

Performs the classical DC approximation (`B' θ = P`) with configurable solver/backends:

* `--solver gauss|faer`: selects the linear solver registered in `gat-core::solver`.
* `--threads auto|N`: hints for Rayon's thread pool.
* `--out-partitions col1,col2`: writes partitioned Parquet under `pf-dc/` while copying a canonical file to `flows.parquet`.

The command prints branch counts, min/max flow, and the `pf-dc/.run.json` manifest for `gat runs resume`.

**Example:**

{% code(lang="bash") %}
# Run DC power flow on IEEE 14-bus system
gat pf dc test_data/matpower/ieee14.arrow --out flows.parquet

# Use specific solver and thread count
gat pf dc grid.arrow --solver faer --threads 4 --out flows.parquet
{% end %}

{% callout(type="info") %}
**Performance tip:** DC power flow typically completes in milliseconds, even for large systems (12k+ buses).
{% end %}

## `gat pf ac <grid.arrow> --out flows.parquet`

Runs the AC Newton–Raphson driver with tolerance/iteration controls:

* `--tol 1e-6`: exit when max mismatch drops below this tolerance.
* `--max-iter 20`: stop after this many Newton steps even if not converged.
* `--solver` and `--threads` behave as in the DC command.
* The output lives under `pf-ac/` plus the canonical `flows.parquet`.

### Q-Limit Enforcement

AC power flow supports PV-PQ bus switching for reactive power limits:

{% code(lang="bash") %}
gat pf ac grid.arrow --out flows.parquet --enforce-q-limits
{% end %}

When a generator hits its Q limit (qmin or qmax), the bus switches from PV (voltage-controlled) to PQ (load bus) and the reactive output is clamped.

### Shunt Element Support (v0.4.0)

AC power flow includes full shunt element modeling (fixed capacitors and reactors). This is essential for achieving exact agreement with external tools like MATPOWER, PowerModels.jl, and PSS/E:

{% code(lang="bash") %}
gat pf ac grid.arrow --out flows.parquet --include-shunts
{% end %}

Shunts are modeled as constant-admittance injections:
- **Gs**: Shunt conductance (p.u.) — real power injection
- **Bs**: Shunt susceptance (p.u.) — reactive power injection (positive = capacitive)

## Related Commands

- [OPF](/guide/opf/) — Optimal power flow analysis
- [Inspect](/guide/inspect/) — Network inspection and validation
- [Batch](/guide/batch/) — Multi-scenario power flow
