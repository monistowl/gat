---

> **Implementation Plan:** See [`docs/plans/2025-12-06-paper-experiments-implementation.md`](../../docs/plans/2025-12-06-paper-experiments-implementation.md) for detailed tasks covering DSS², GNN Benchmarks, DPLib, and SoCal 28-Bus.

---

## 1. PFΔ — Benchmark Dataset for Power Flow (MIT MOSSLab, 2025) ✅ IMPLEMENTED

**Paper**: “PFΔ: A Benchmark Dataset for Power Flow under Load, Generation, and Topology Variations” ([arXiv][1])

**What it is**

* ~860k solved AC power-flow instances across multiple system sizes, with variations in load, generation, and topology; includes contingency scenarios and near-infeasible cases. ([arXiv][1])
* Dataset hosted on Hugging Face; code and generation scripts on GitHub. ([arXiv][1])

**Why it’s a perfect GAT demo**

* GAT has `gat pf ac`, `gat nminus1 ac`, and batch/scenario tooling; PFΔ is literally “lots of PF solves under contingencies/topology changes.”
* Under the hood PFΔ uses standard test grids (PGLib/MATPOWER style), which GAT already imports (`gat import matpower`). ([GitHub][2])

**GAT tutorial skeleton**

1. **Ingest base grid(s)**

   ```bash
   gat import matpower case118.m  --out grids/case118.arrow
   ```

   (Use the same base case PFΔ uses for a given subset, e.g. IEEE-118.)

2. **Rebuild a PFΔ-like scenario set in Arrow/Parquet**

   * Parse a slice of their HuggingFace JSON / CSV into Parquet with columns: load multipliers, generator status, line outages, etc.
   * Use `gat scenarios` (or just your own driver script) to generate per-scenario Arrow grids with the same perturbations.

3. **Batch PF runs**

   ```bash
   gat batch pf \
     --manifest runs/pfdelta_manifest.json \
     --out runs/pfdelta_pf \
     --max-jobs 8
   ```

4. **Validation + speed comparison**

   * Compare bus voltages and line flows against PFΔ’s stored solutions; compute MSE/relative error; ensure it matches their baseline NR solver accuracy. ([arXiv][1])
   * Benchmark “PF solves per second” vs their reported timings / reference implementation.

5. **Tutorial story**

   * “Reproducing PFΔ on a laptop with Rust PF + Arrow pipelines.”
   * Show how to go from raw PFΔ JSON → GAT grid → AC PF → Parquet outputs → Polars/DuckDB analysis.

---

## 2. OPFData — Large-Scale AC-OPF Datasets with Topology Perturbations (2024) ✅ IMPLEMENTED

**Paper**: “OPFData: Large-scale datasets for AC optimal power flow with topological perturbations” ([arXiv][3])

**What it is**

* 20 datasets of **300k solved AC-OPF instances each**, built on PGLib-OPF cases from 14-bus up to ~13k-bus, with both FullTop (load perturbations) and N-1 (single generator/line/transformer outages) variants. ([arXiv][3])
* Distributed as JSON; each example contains a complete OPF problem (grid + solution). Solved with PowerModels.jl + Ipopt/MUMPS. ([arXiv][3])

**Why it’s a perfect GAT demo**

* GAT already has `gat opf ac` and imports PGLib/MATPOWER topologies.
* You can treat each OPFData record as a “scenario” and ask: *does GAT’s AC-OPF reproduce their cost/constraints* and *how much faster is it* for batches?
* Great showcase for `gat batch opf` + scenarios + `runs` manifest system.

**GAT tutorial skeleton**

1. **Import base PGLib case**

   ```bash
   gat import matpower pglib_opf_case118_ieee.m --out grids/case118.arrow
   ```

2. **Translate a subset of OPFData**

   * Write a small CLI / Python helper that:

     * Reads OPFData JSON examples for a chosen case.
     * For each example, overwrites loads, generator limits, switches out components as in the record.
     * Emits a `scenarios_manifest.json` in GAT’s format.

3. **Run AC-OPF in batch**

   ```bash
   gat batch opf ac \
     --manifest runs/opfdata/case118_manifest.json \
     --out runs/opfdata/case118_results \
     --max-jobs 8
   ```

4. **Compare to OPFData solutions**

   * Cost gap, KKT residuals, constraint violations vs their stored solution fields. ([arXiv][3])
   * Performance comparison vs their Julia+Ipopt solver on the same hardware (or approximated via per-instance timing).

5. **Tutorial story**

   * “From OPFData JSON to reproducible AC-OPF experiments in Rust.”
   * Nice place to show `gat runs describe`, reliability analytics on N-1 scenarios, etc.

---

## 3. DPLib — Distributed OPF Benchmark Library (2025) 🟡 PLANNED

**Paper**: “DPLib: A Standard Benchmark Library for Distributed Power System Analysis and Optimization” ([arXiv][4])

**What it is**

* A set of **partitioned PGLib-OPF MATPOWER cases** plus ADMM-based DC and AC OPF solvers implemented in MATLAB + YALMIP/IPOPT. ([arXiv][4])
* All partitioned cases and solver outputs (residuals, costs) are publicly available via GitHub. ([arXiv][4])

**Why it’s a good GAT demo**

* Even if GAT doesn’t yet do *distributed* ADMM, you can:

  * Import the *centralized* PGLib cases and reproduce their centralized OPF results.
  * Use DPLib’s reported centralized costs and residuals as ground truth.
* Very natural for showcasing:

  * `gat opf {dc,ac}` scalability on large PGLib cases (200 buses up to ~9k buses). ([arXiv][4])
  * `gat analytics reliability` on those same topologies.

**GAT tutorial skeleton**

1. **Reproduce centralized baseline**

   ```bash
   gat import matpower pglib_opf_case2000_goc.m --out grids/case2000.arrow
   gat opf ac grids/case2000.arrow --out results/case2000_opf.parquet
   ```

2. **Compare costs**

   * Use DPLib’s reported centralized OPF cost for the same case as a reference; compute gap. ([arXiv][4])

3. **Scale up**

   * Repeat for pglib_opf_case4661_sdet, 6468_rte, 9241_pegase etc. ([arXiv][4])
   * Wrap in `gat batch opf` to show multi-core scaling.

4. **Optional “future work” note in tutorial**

   * Sketch how a future `gat opf ac --admm` mode could be benchmarked directly against DPLib’s ADMM solvers.

---

## 4. PFΔ + GAT Reliability / N-1 (same dataset, different angle) ✅ IMPLEMENTED

This is technically still PFΔ, but worth calling out as a second **distinct** tutorial:

**Angle**: Use PFΔ’s load/topology variation space as a sandbox for **N-1 screening and reliability metrics**.

* PFΔ explicitly covers contingencies and near-infeasible conditions; they discuss PF as the bottleneck for contingency analysis and topology optimization. ([arXiv][1])

**GAT tutorial skeleton**

1. From PFΔ’s metadata, construct a contingency list (outaged lines/gens).
2. Import the base grid(s) → Arrow.
3. Use:

   ```bash
   gat nminus1 ac grids/case118.arrow \
     --outages outages_case118.yaml \
     --out runs/case118_nminus1.parquet
   gat analytics reliability \
     --grid grids/case118.arrow \
     --outages outages_case118.yaml \
     --out runs/case118_reliability.parquet
   ```
4. Show how PFΔ’s pre-solved PF solutions let you:

   * Check GAT’s PF correctness under each contingency.
   * Add Monte-Carlo outage frequency assumptions on top using `gat analytics reliability` + `reliability_monte_carlo`.

---

## 5. DSS² — Deep Statistical Solver for Distribution System State Estimation (2023) 🟡 PLANNED

**Paper**: “Deep Statistical Solver for Distribution System State Estimation” (DSS²) ([arXiv][5])

**What it is**

* A deep-learning based DSSE method, plus classic WLS baselines, on distribution feeders (pandapower-style). ([arXiv][5])
* GitHub repo with full code and case setups. ([GitHub][6])

**Why it’s good for GAT**

* GAT exposes `gat se wls` for state estimation and has AC PF tools.
* You don’t need to re-implement their neural net; you just:

  * Reproduce the **WLS baseline** results on the same networks with GAT.
  * Compare runtime and residuals.
* Nice “bridge tutorial” from traditional DSSE → ML DSSE.

**GAT tutorial skeleton**

1. **Convert one of their feeders** to MATPOWER/CIM and import:

   ```bash
   gat import matpower feeder_uk_lv.m --out grids/feeder_uk_lv.arrow
   ```

2. **Build measurement sets** from their scripts:

   * Export pseudo-measurement + SCADA snapshots → CSV/Parquet.

3. **Run WLS SE**

   ```bash
   gat se wls \
     --grid grids/feeder_uk_lv.arrow \
     --measurements data/feeder_uk_lv_measurements.parquet \
     --out runs/feeder_uk_lv_se.parquet
   ```

4. **Compare**

   * State vectors, residuals, and RMSE vs their WLS baseline and (optionally) the DSS² model’s performance. ([arXiv][5])

5. **Tutorial story**

   * “Reproducing DSSE benchmarks and wiring them into a Rust/Arrow workflow; ML model remains in Python, but data plumbing and classical solver are GAT.”

---

## 6. SoCal 28-Bus Digital Twin — Real Distribution Grid + PMU Data (2025) ✅ IMPLEMENTED

**Paper**: “A Digital Twin of an Electrical Distribution Grid: SoCal 28-Bus Dataset” ([arXiv][7])

**What it is**

* Real 28-bus distribution grid with:

  * Synchro-waveforms (time-domain),
  * Synchro-phasors (steady-state),
  * Circuit parameters (topology, line models, etc.). ([ResearchGate][8])
* They demonstrate phasor and time-domain state estimation, plus list many potential applications. ([arXiv][7])

**Why it’s a good (slightly more work) GAT demo**

* Real-world dataset; very persuasive for users.
* GAT can:

  * Ingest the circuit (once converted) and run AC PF.
  * Take phasor snapshots and perform WLS SE (`gat se wls`).
  * Use `gat ts` to handle the time series aspects of PMU streams.
* You probably won’t replicate their full time-domain least-squares waveform SE right away, but you can do a “per-time-step phasor SE” tutorial.

**GAT tutorial skeleton**

1. Convert their line/transformer model into MATPOWER and import with `gat import matpower`.
2. Parse phasor snapshots → Parquet (`timestamp, bus, Vm, Va, I, etc.`).
3. Use `gat ts resample/join` to prepare measurement tables.
4. Run `gat se wls` snapshot-by-snapshot; compare residual statistics vs those in the paper. ([arXiv][7])

---

## 7. GNN Benchmarks — For Future `gat featurize gnn` Demos 🟡 PLANNED

If you want a GNN-centric tutorial to showcase `gat featurize gnn`, a couple of nice targets:

* “A power grid benchmark dataset for graph neural networks” (NeurIPS Datasets & Benchmarks 2024) — curated dataset and GitHub org for GNN models on power grids. ([NeurIPS Proceedings][9])
* Classic “Graph Neural Solver for Power Systems” and successors with GitHub code/datasets. ([GitHub][10])

You can:

1. Import their grids into GAT → Arrow.
2. Use `gat featurize gnn` to generate node/edge feature tables matching their model inputs.
3. Use their code as-is for the model, but drive all data creation via GAT for speed + reproducibility.

---

## Implementation Status

| Paper | Status | GAT Features Used |
|-------|--------|-------------------|
| PFΔ | ✅ Implemented | `gat benchmark pfdelta`, `gat pf ac` |
| OPFData | ✅ Implemented | `gat benchmark pglib`, `gat opf ac` |
| PFΔ + N-1 | ✅ Implemented | `gat nminus1`, `gat analytics reliability` |
| DSS² | ✅ Implemented | `gat benchmark dss2`, CIGRE MV network, WLS SE |
| GNN Benchmarks | ✅ Implemented | `gat featurize gnn`, PyTorch Geometric export, round-trip validation |
| DPLib | ✅ Implemented | ADMM distributed OPF, graph partitioning, tie-line flows |
| SoCal 28-Bus | ✅ Implemented | PMU importer, time-series SE, 28-bus loader |

## Implementation Priority

Remaining work for partial implementations:

1. **DSS²** ✅ COMPLETE
   - ✅ CIGRE MV network builder (`cigre.rs`)
   - ✅ Measurement generator with noise
   - ✅ CLI `gat benchmark dss2` command
   - ✅ Documentation (`examples/experiments/DSS2_BENCHMARK.md`)
   - Results: MAE 0.30° ± 0.01°, 100% convergence rate

2. **GNN Benchmarks** ✅ COMPLETE
   - ✅ PowerGraph dataset loader (`powergraph.rs`)
   - ✅ GNN featurization with physics-informed features
   - ✅ Round-trip validation tests (PyTorch Geometric + NeurIPS formats)
   - ✅ Documentation (`examples/experiments/GNN_BENCHMARK.md`)

3. **DPLib** ✅ COMPLETE
   - ✅ CLI `gat benchmark dplib` command
   - ✅ Graph partitioning module (`graph/partition.rs`)
   - ✅ ADMM solver with full consensus algorithm (`opf/admm.rs`)
   - ✅ Parallel x-update with rayon
   - ✅ Tie-line flow calculation and reporting
   - ✅ Documentation (`examples/experiments/DPLIB_BENCHMARK.md`)

4. **SoCal 28-Bus** ✅ COMPLETE
   - ✅ PMU data format importer (`pmu.rs`)
   - ✅ Time-series state estimation (`state_estimation.rs`)
   - ✅ 28-bus distribution network loader (`socal28.rs`)
   - ✅ Documentation in SE guide


[1]: https://arxiv.org/html/2510.22048v1 "PFΔ: A Benchmark Dataset for Power Flow under Load, Generation, and Topology Variations"
[2]: https://github.com/power-grid-lib/pglib-opf?utm_source=chatgpt.com "power-grid-lib/pglib-opf: Benchmarks for the Optimal ..."
[3]: https://arxiv.org/html/2406.07234v1 "OPFData: Large-scale datasets for AC optimal power flow with topological perturbations"
[4]: https://arxiv.org/html/2506.20819v2 "DPLib: A Standard Benchmark Library for Distributed Power System Analysis and Optimization"
[5]: https://arxiv.org/pdf/2301.01835?utm_source=chatgpt.com "Deep Statistical Solver for Distribution System State ..."
[6]: https://github.com/TU-Delft-AI-Energy-Lab/Deep-Statistical-Solver-for-Distribution-System-State-Estimation?utm_source=chatgpt.com "Implementation of Deep Statistical Solver for Distribution ..."
[7]: https://arxiv.org/html/2504.06588v1 "A Digital Twin of an Electrical Distribution Grid: SoCal 28-Bus Dataset"
[8]: https://www.researchgate.net/publication/390639007_A_Digital_Twin_of_an_Electrical_Distribution_Grid_SoCal_28-Bus_Dataset?utm_source=chatgpt.com "A Digital Twin of an Electrical Distribution Grid: SoCal 28- ..."
[9]: https://proceedings.neurips.cc/paper_files/paper/2024/file/c7caf017cbbca1f4b368ffdc7bb8f319-Paper-Datasets_and_Benchmarks_Track.pdf?utm_source=chatgpt.com "A power grid benchmark dataset for graph neural networks"
[10]: https://github.com/bdonon/GraphNeuralSolver?utm_source=chatgpt.com "bdonon/GraphNeuralSolver"
