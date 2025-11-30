# GRID ANALYSIS TOOLKIT (GAT)

## v0.5.0 Status (Current)

**Completed Features:**

✅ **Unified OPF Architecture** (OpfSolver with OpfMethod enum: EconomicDispatch, DcOpf, SocpRelaxation, AcOpf)
✅ **Generator Cost Models** (CostModel enum with polynomial and piecewise-linear support)
✅ **Economic Dispatch** (Merit-order optimization with generator limits and cost functions)
✅ **Monte Carlo Reliability Analysis** (LOLE, EUE, Deliverability Score)
✅ **Multi-Area Coordination** (CANOS framework with zone-to-zone metrics)
✅ **FLISR Integration** (Fault Location, Isolation, Service Restoration with reliability tracking)
✅ **VVO with Reliability Constraints** (Volt-Var Optimization respecting min deliverability scores)
✅ **Maintenance Scheduling** (Multi-area outage coordination with LOLE thresholds)
✅ **PFDelta Integration** (Data loader for 859,800 test cases)
✅ **Benchmark Command** (AC OPF benchmarking against IEEE test cases)
✅ **Comprehensive Test Suite** (51+ reliability tests, 14+ integration tests)
✅ **SOCP Relaxation** (Branch-flow model with Clarabel backend)
✅ **Full Nonlinear AC-OPF** (L-BFGS penalty method, pure Rust)
✅ **IPOPT Backend for AC-OPF** (Analytical Jacobian & Hessian, <0.01% gap validated)
✅ **Bus Shunt Support** (Fixed capacitors/reactors in Y-bus and AC-OPF)

**Recent Highlights (November 2024):**
- ✅ **IPOPT AC-OPF validated** against PGLib reference values:
  - IEEE 14-bus: $2,178.08/hr (ref: $2,178.10) — **Gap: -0.00%**
  - IEEE 118-bus: $97,213.61/hr (ref: $97,214.00) — **Gap: -0.00%**
- ✅ Analytical thermal constraint Jacobian with correct chain-rule derivatives
- ✅ Diagnostics module for AC-OPF introspection and debugging
- ✅ Comprehensive preprint (draft5) with full mathematical formulations

**Documentation:**
- `docs/guide/opf.md` — Unified OPF solver architecture with IPOPT validation results
- `docs/guide/pf.md` — Power flow with shunt support
- `docs/guide/reliability.md` — Monte Carlo algorithms and LOLE/EUE metrics
- `docs/guide/adms.md` — Updated with reliability integration details
- `docs/guide/benchmark.md` — PFDelta integration and benchmarking workflow
- `docs/guide/overview.md` — Cross-references for new capabilities
- `docs/papers/README.md` — Research preprints including comprehensive draft5

---

## Roadmap Overview

This roadmap documents the CLI-first workspace, the milestone plan, and the surrounding tooling/infrastructure needed to keep the docs, schema exports, and `bd` issue graph honest.

## 0) Workspace layout
```
gat/
  crates/
    gat-core/        # types, math, graph model, units
    gat-io/          # file formats & adapters
    gat-algo/        # PF/OPF/state-estimation/contingency
    gat-ts/          # time-series utils & stores
    gat-viz/         # plotting primitives shared by CLI + GUI
    gat-cli/         # `gat` binary (subcommands)
    gat-gui/         # `egui` dashboard (eframe)
  scripts/           # packaging/install helpers
  test_data/         # MATPOWER/CSV/RDF fixtures + telemetry
  docs/              # authoritative Markdown + generated assets
  site/              # mdBook/preview bundle built from docs/
  ROADMAP.md         # this plan with acceptance criteria
```

This structure mirrors toolkits like ELF: shared libs feed both CLI and GUI, packaging scripts build tarballs, and docs/ contains the canonical Markdown tree consumed by `cargo xtask doc all`, `gat-mcp-docs`, and whatever doc-serving pipeline we layer on later.

## Cross-cutting foundations (Milestone M0)

**Targets**

* Error handling: `anyhow`, `thiserror`, `tracing` + `tracing-subscriber`.
* CLI UX: `clap` derives, shell completions, man pages, `indicatif` progress bars.
* Columnar I/O: `polars` (lazy) + Arrow/Parquet with `default-features = false` where appropriate.
* Numerics: `ndarray`, `faer`, and `sprs`; add BLAS/LAPACK features only when needed.
* Graph algorithms: topology via `petgraph`.

**Deliverables**

* `gat-core` with typed units, IDs, and the neutral network schema.
* `gat-cli` scaffolding with logged `gat --help`, global `--log-level`, and structured profiling flags.

## Milestone 1 — File formats & ingestion (M1)

**Scope**

* PSS®E RAW ingestion via `power_flow_data`.
* MATPOWER via `caseformat`.
* CIM RDF/XML streaming through `quick-xml` + `sophia`.
* Validation command `gat validate dataset --spec <schema.json>`.

**CLI**

* `gat import psse --raw` / `gat import matpower --m` / `gat import cim --rdf` (all → `grid.arrow`).
* `gat validate dataset` ensures fixtures match the schema.

**Acceptance**

* Round-trip MATPOWER ↔ Arrow ↔ Parquet.
* Ingest 5+ public RAW files plus a CIM excerpt that retains topology + nameplate.

## Milestone 2 — Topology & graphs (M2)

* Build/clean graphs, compute stats, identify islands and exports.
* CLI: `gat graph stats`, `gat graph islands`, `gat graph export`.
* Export command supports `--format graphviz|dot` and `--out <file>` while `--emit` prints the node→island map.
* CLI: `gat graph visualize` uses `fdg-sim` to layout nodes and emits JSON positions.

## Milestone 3 — Power flow solvers (M3)

**DC PF:** assemble B′, solve sparse system (sprs/Rayon).
**AC PF:** Newton–Raphson with LDL, optional `power_flow` crate reference.

CLI: `gat pf dc grid.arrow --out flows.parquet`, `gat pf ac ... --tol 1e-8`.
* Both commands persist flows under `pf-dc/` or `pf-ac/` and surface solver choice via the registry.

**Acceptance:** Converge MATPOWER cases, compare flows/angles, AC NR scales to ~10k buses.

## Milestone 4 — Optimal Power Flow (M4)

* DC OPF as LP/MILP with `good_lp` + `highs`.
* AC OPF stages: first penalty/linearization, then `argmin` or external solvers.
* CLI: `gat opf dc --cost ... --limits ... --out ...`.

## Milestone 5 — Contingency screening & state estimation (M5)

* N-1 screening: run DCPF per outage, reuse graph deltas where possible.
* WLS state estimation over flow/injection measurements.
* CLI: `gat nminus1 dc`, `gat se wls` (see `docs/guide/se.md`).

## Milestone 6 — Time-series & markets (M6)

* Resample/join/aggregate telemetry with `polars` on Arrow/Parquet tables.
* CLI commands under `gat ts` to resample, join, and aggregate (see `docs/guide/ts.md`).

## Milestone 7 — GUI (egui) dashboard (M7)

* `gat-gui` uses shared `gat-viz` primitives, `egui_dock`, multi-viewport layout, and stays tightly coupled to CLI logic.
* CLI stub `gat gui run` ensures parity until the full UI ships (see `docs/guide/gui.md`).

## Visualization & export

* `gat viz plot` is a placeholder that runs the `gat-viz` helper (see `docs/guide/viz.md`) so exporters stay wired to the CLI for future SVG/PNG/Parquet output.

## Packaging & installer (continuous)

* `scripts/package.sh` builds release tarballs `dist/gat-<ver>-<arch>.tar.gz` (requires `jq`).
* `scripts/install.sh [prefix]` installs to `~/.local` by default (see `docs/guide/packaging.md`).

## Testing strategy

* Golden fixtures in `test_data/` for MATPOWER, CIM, telemetry, and regulators.
* CLI regression tests driven by `assert_cmd` that cover `pf`, `opf`, `nminus1`, `ts`, and the GUI stub.

## Dataset adapters (M14)

Leverage publicly maintained datasets and built-in helpers to hydrate grids, telemetry, and weather:

1. `gat dataset rts-gmlc fetch --out data/rts-gmlc` → `gat import matpower` + `gat ts import dsgrid`.
2. `gat dataset hiren list|fetch` for curated MATPOWER cases.
3. `gat dataset dsgrid` to copy dsgrid Parquet fixtures.
4. `gat dataset sup3rcc-fetch|sample` to tie weather to buses.
5. `gat dataset pras` normalizes PRAS LOLE/EUE data.

(Full CLI details in `docs/guide/datasets.md`.)

## Scaling roadmap

* **Horizon 1:** Multicore on one node with `rayon`, `--threads`, partitioned Parquet, `run.json` checkpoints.
* **Horizon 2:** Fan-out executors (`Local`, `ProcessPool`), object store abstraction (`opendal`), chunk specs/results, optional `gat-worker` + manifest-watching GUI.
* **Horizon 3:** Containerized CLI, workflow templates (Argo/Flyte/Temporal), `gat-svc` gRPC API, OTLP tracing, chunk-level priority controls.
* **Horizon 4:** HPC executors (`Slurm`, `MPI`), PETSc/Trilinos solves, region-by-region decomposition.
* **Horizon 5:** Serverless bursts (`gat-map`), Arrow Flight endpoints, DataFusion/Ballista for distributed joins.

See `docs/guide/scaling.md` for the detailed horizon-by-horizon plan, chunk contracts, and solver strategy.

## Crate shortlist

* Formats & data: `power_flow_data`, `caseformat`, `quick-xml` + `sophia`, `polars`, `arrow2`, optional `netcdf`, `geo`.
* Math & graphs: `ndarray`, `faer`, `sprs`, `petgraph`.
* Solvers: `good_lp` + `highs`, `argmin`, `power_flow` reference crate.
* CLI infra: `clap`, `indicatif`, `tracing`.
* GUI: `egui`, `eframe`, `egui_dock`.

## Phasing & effort

* **M0–M1:** Core types, ingest adapters, Arrow/Parquet plumbing, CLI skeleton.
* **M2–M3:** Topology tools, DC/AC PF solvers.
* **M4–M5:** DC-OPF via HiGHS, N-1 DC, WLS SE, dataset adapters.
* **M6–M7:** Time-series helpers, GUI parity, viz/export primitives.

## Notes & nudges

* Prefer Arrow/Parquet artifacts so downstream tools (Polars/DuckDB) can reuse them without conversions.
* Keep CIM ingestion minimal at first—topology + equipment metadata.
* Use the simple tarball + `install.sh` pattern so labs can adopt without cargo.
* For large cases, bias to sparse solvers and graph-aware chunking before investing in distributed LA.

## Recent deliverables

* Time-series suite with `gat ts {resample,join,agg}` plus regression tests covering telemetry workflows.
* Graph tooling (`gat graph stats/islands/export`) now surfaces topology summaries and DOT exports for Milestone 2.
* Packaging scripts documented in `docs/guide/packaging.md` that build/install release artifacts.
* CLI tests using workspace fixtures (`test_data/ts`, `test_data/matpower`) and `assert_cmd` to guard PF/OPF progress.
