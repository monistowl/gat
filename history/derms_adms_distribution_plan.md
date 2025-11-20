# DERMS / ADMS / distribution workflow explansion roadmap for GAT

---

## 1. What DERMS / ADMS / distribution workflows should mean *for GAT*

Rather than trying to re-implement a realtime utility control room, aim GAT at **offline / planning / “digital twin” workflows** that real ADMS/DERMS platforms implement internally:

* **Distribution-oriented analytics**

  * Radial / weakly meshed feeder PF/OPF
  * Volt/VAR optimization & loss minimization
  * Hosting-capacity & locational benefit metrics
  * Reliability / outage / FLISR simulations

* **DERMS-style portfolio analytics**

  * Aggregated P–Q flexibility envelopes for DER fleets
  * Multi-period DER dispatch given tariffs / constraints
  * “What-if” curtailment and constraint scenarios at feeder or substation level

Canonical ADMS functions in the wild include FLISR, Volt/VAR optimization, conservation voltage reduction, peak demand management, outage management, and DER integration. ([The Department of Energy's Energy.gov][1])

Canonical DERMS responsibilities are hierarchical monitoring and control of DERs (solar, storage, EVs, flexible loads) to maintain reliability and optimize cost/constraints at distribution level, often with transactive / market-like interfaces. ([NREL][2])

So: **GAT = “the math & workflows underneath ADMS/DERMS”, surfaced as CLI + Parquet.**

---

## 2. Fit with existing GAT semantics

Current GAT primitives:

* **Network core:** graph imports (MATPOWER/PSSE/CIM), PF/OPF (DC/AC), PTDF. 
* **Scenario & reliability:** N-1 screening, basic SE. 
* **Time-series plumbing:** resample/join/agg, plus dataset fetcher (OPS data, etc.). 
* **Execution model:** each command is a thin CLI wrapping library code, outputs Parquet + `run.json` manifests, fan-out friendly. 

For ADMS/DERMS:

* Re-use **PF/OPF/SE/TS** as the inner kernels.
* Layer **named workflows** that:

  * Take in *config + asset tables + scenarios*
  * Launch *one or many PF/OPF/SE/TS runs under the hood*
  * Emit *aggregated metrics + detailed Parquets* for later slicing.

Concretely, we add **three new CLI namespaces**:

```bash
gat dist   # distribution-network-specific modeling & OPF
gat derms  # DER portfolio / flexibility / scheduling workflows
gat adms   # reliability, FLISR, Volt/VAR, outage & restoration simulations
```

Each should follow the same pattern as `gat ts` or `gat analytics ptdf`: small, sharp subcommands with Arrow/Parquet schemas and resumable `run.json`.

---

## 3. Building blocks: crates & theory you can lean on

### 3.1 Rust crates

These are not mandatory, but they’re useful references / potential dependencies:

* **powers / powers-pf** – Rust power-flow/OPF toolkit translated from MATPOWER; Newton, fast-decoupled, Gauss–Seidel, DC PF. ([Crates][3])

  * You can either:

    * Keep GAT’s own solvers and read their design for cross-validation, or
    * Factor out some algorithms into a shared crate if you want convergence with the wider Rust ecosystem.

* Existing GAT stack already uses:

  * **Clarabel + good_lp** as LP/QP backends for OPF. 
  * **Polars/Arrow** for IO and TS.
  * **petgraph-style** graph modeling (inferred from the PF/graph tooling).

For distribution OPF and Volt/VAR, adding or tightening:

* **Clarabel’s conic interface** (for SOCP relaxations of radial OPF if you go that route).
* A small **unbalanced three-phase modeling crate** (potentially homegrown first).

### 3.2 Canonical OPF & distribution references

Good theory targets for the distribution layer:

* **“Optimal Power Flow in Distribution Networks”** – convex branch-flow OPF formulation for radial networks (Baran/Wu line of work summarized). ([nali.seas.harvard.edu][4])
* **Recent OPF survey** – up-to-date formulations and solution methods, including ML-aided OPF. ([MDPI][5])
* **SE-driven distribution OPF** – NREL/UTD work on solving distribution OPF on top of state estimation for partially observed feeders. ([The University of Texas at Dallas][6])
* **Real-time distribution network OPF for volt/VAR & congestion** – lab demo of AC-OPF-based control of wind/tap-changers. ([knowledge.rtds.com][7])

For ADMS/DERMS function lists and scope:

* DOE / Argonne **ADMS guidelines & function inventories** (FLISR, CVR, Volt/VAR, load forecasting, outage management). ([The Department of Energy's Energy.gov][1])
* NREL + DOE **FAST-DERMS architecture** – hierarchical, federated DERMS design you can crib for your config/aggregation semantics. ([gmlc.doe.gov][8])

Those give you enough grounding to write docs that will pass the sniff test with utility folks.

---

## 4. Proposed CLI surface & semantics

### 4.1 `gat dist` – distribution modeling & OPF

**New data models (Parquet/Arrow tables)**

* `dist_nodes.parquet` – node-level attributes:

  * `node_id`, `phase` (A/B/C/ABC), `type` (load, source, slack, DER_agg)
  * `v_min`, `v_max` (per unit), `load_p`, `load_q` (nominal), `area/feeder_id`
* `dist_branches.parquet` – line/transformer attributes:

  * `branch_id`, `from_node`, `to_node`, `r`, `x`, `b`, `tap`, `status`, `thermal_limit`
* Optional:

  * `dist_caps.parquet` – capacitor banks and regulators.
  * `dist_switches.parquet` – for later FLISR logic.

**Core commands**

1. `gat dist import`

   * Adapters from MATPOWER/CIM/PSSE into the distribution schemas (subsets of existing importers, possibly with `--feeder-only` filters).

2. `gat dist pf`

   * AC PF specialized for radial/weakly meshed feeders. Options:

     * `--method {nr, fast-decoupled, backward-forward}`
     * `--phases {balanced, per-phase}`

3. `gat dist opf`

   * Distribution OPF built on:

     * Existing AC/DC OPF kernel, plus
     * Extra constraints for nodal voltage bounds, tap positions, shunt devices, and possibly per-phase constraints.
   * Objectives toggled by flag:

     * `--objective loss`  – minimize losses
     * `--objective vvo`   – Volt/VAR optimization (minimize voltage deviations + losses)
     * `--objective host`  – hosting capacity: “maximize additional DER injection at {node(s)} under constraints.”

4. `gat dist hostcap`

   * Wrapper workflow that:

     * Sweeps increasing DER injection at specified locations
     * Calls `dist opf` for each
     * Emits:

       * `hostcap_summary.parquet` (max MW per node/feeder)
       * `hostcap_detail.parquet` (per-scenario PF/OPF results)

---

### 4.2 `gat derms` – DER portfolio & flexibility analytics

**DER asset model (Parquet)**

`der_assets.parquet`:

* `asset_id`, `bus_id`, `phase`, `asset_type` (PV, battery, EV, flexible_load, etc.)
* `p_min`, `p_max`, `q_min`, `q_max` (per time step)
* `ramp_up`, `ramp_down`, `energy_cap`, `soc_min`, `soc_max`, `efficiency`
* `owner_id`, `agg_id`, `priority`, `cost_curve_id`
* Optional: `telemetry_id` to join with `gat ts` outputs.

**Core commands**

1. `gat derms envelope`

   * Compute **P–Q flexibility envelopes** for:

     * Individual assets, aggregations (`--group-by agg_id`), or feeder/substation regions.
   * Implementation:

     * For each region:

       * Solve a set of OPFs / feasibility checks at extremal P/Q combinations.
       * Approximate a convex polytope (or convex hull of samples).
   * Outputs:

     * `der_envelopes.parquet` – rows: (region, vertex_id, P, Q, label)
     * Optional coarse “box” summarization.

2. `gat derms schedule`

   * Multi-period DER dispatch optimizer:

     * Inputs:

       * `grid.arrow` or `dist_*` tables
       * `der_assets.parquet`
       * `price_series.parquet` (TS of energy/ancillary prices or grid signals)
       * `load_forecast.parquet` (can be produced via `gat ts` or external)
     * Objective:

       * Minimize cost or maximize revenue (`--objective {cost,min-curtailment}`).
     * Constraints:

       * Per-period P/Q limits, energy balance, SOC dynamics, network constraints via distribution OPF kernel.
   * Output:

     * `der_schedule.parquet` – asset/time setpoints (P, Q, SOC)
     * `der_schedule_summary.parquet` – per-feeder/agg totals.

3. `gat derms stress-test`

   * Loop: pick random or scripted contingencies and price/forecast scenarios; run `derms schedule` repeatedly to test:

     * Curtailment rates
     * Voltage violations
     * Feeder overloads
   * Output: per-scenario metrics and violation distributions.

This closely mirrors the FAST-DERMS idea of local aggregations exposing flexibility to higher-level controllers, but keeps everything as offline analytics. ([gmlc.doe.gov][8])

---

### 4.3 `gat adms` – reliability, FLISR, Volt/VAR & outage workflows

**Extra data tables**

* `reliability.parquet` – per-branch/node failure and repair stats:

  * `element_id`, `type` (line, transformer, switch), `lambda` (failures/yr), `r` (repair hours), `switchable` bool.
* `switching_devices.parquet` – reclosers, sectionalizers, switches, breakers.
* `outage_scenarios.parquet` – optional user-defined contingencies beyond N-1.

**Core commands**

1. `gat adms flisr-sim`

   * Simulate **Fault Location, Isolation, and Service Restoration**:

     * Input: feeder model + reliability data + switching topology.
     * For each fault scenario:

       * Use `nminus1 dc` / `dist pf` to identify overloaded/failed branches.
       * Apply a simple rule-based or MILP restoration algorithm (reconfigure switches, respect radial constraints).
     * Outputs:

       * `flisr_runs.parquet` – for each scenario: #customers interrupted, duration, switching sequence.
       * `reliability_indices.parquet` – SAIDI/SAIFI/CAIDI per feeder/area.

2. `gat adms vvo-plan`

   * Offline **Volt/VAR & CVR planning**:

     * Use `dist opf --objective vvo` on:

       * Typical days (low load / high PV / peak).
     * Optimize:

       * Tap changer positions, cap bank statuses, DER reactive setpoints.
     * Outputs:

       * `vvo_settings.parquet` – recommended settings as a function of load/PV level.
       * `vvo_performance.parquet` – predicted loss reduction, voltage deviation statistics.

3. `gat adms outage-mc`

   * Reliability Monte Carlo:

     * Sample outages over a long horizon from `reliability.parquet`.
     * For each sample:

       * Apply FLISR or a simplified restoration,
       * Run `dist pf` to check feasibility,
       * Compute unserved energy & interruption metrics.
     * Outputs:

       * `outage_samples.parquet`, `outage_stats.parquet`.

4. `gat adms state-estimation` (wrapper)

   * Glue existing **WLS SE** to distribution use cases:

     * `gat se wls` already exists; this command would:

       * Map distribution measurement configs,
       * Run SE,
       * Optionally feed the estimated state into `dist opf` (closing the loop per the NREL/UTD distribution OPF-SE works). ([Research Hub][9])

---

## 5. Implementation roadmap (agent-parsable-ish)

Here’s a linearized plan that small coding agents could follow.

### Phase 0 – Repo plumbing & naming

* [ ] Add new crates:

  * `crates/gat-dist`
  * `crates/gat-derms`
  * `crates/gat-adms`
* [ ] Wire them into `gat-cli`:

  * Add `dist`, `derms`, `adms` subcommands under the same Clap structure as `ts`, `pf`, `opf`.
* [ ] Define shared Arrow schemas in a `gat-schemas` module (if you don’t already have one) for:

  * Dist network tables
  * DER asset tables
  * Reliability tables

### Phase 1 – Minimal `gat dist` (balanced distribution PF/OPF)

* [ ] Implement `dist_nodes` / `dist_branches` schemas.
* [ ] Add `gat dist import matpower`:

  * Reuse existing import path; filter for distribution-like networks or just re-label them.
* [ ] Implement `gat dist pf`:

  * Start with **balanced AC PF** for radial networks (reuse AC PF engine with minor constraints).
* [ ] Implement `gat dist opf`:

  * Start with DC OPF for distribution as a wrapper over existing `opf dc`.
  * Add voltage and thermal constraints where possible.
* [ ] Implement `gat dist hostcap`:

  * Simple heuristic sweep: incremental DER injection at chosen nodes, call `dist opf` until infeasible.
* [ ] Add docs: `docs/guide/dist.md` with examples.

### Phase 2 – `gat derms` core (assets + envelopes + scheduling)

* [ ] Define `der_assets.parquet` schema & validation CLI:

  * `gat derms validate-assets der_assets.parquet`.
* [ ] Implement `gat derms envelope`:

  * For each group (bus, agg_id, feeder):

    * Generate a small set of OPF problems:

      * (`P_min`, `Q_min`), (`P_min`, `Q_max`), (`P_max`, `Q_min`), (`P_max`, `Q_max`)
    * Solve via `dist opf`, collect feasible points.
    * Optionally compute convex hull in P–Q.
* [ ] Implement `gat derms schedule`:

  * Multi-period LP/QP:

    * Variables: P/Q/SOC for each asset/time.
    * Constraints: asset bounds + network constraints via linearized OPF or PTDF.
  * Integrate with `gat ts` for price/forecast joins.
* [ ] Implement `gat derms stress-test`:

  * Generate random price/forecast scenarios & run `schedule` in a loop; store metrics.
* [ ] Add docs: `docs/guide/derms.md`, plus small test datasets under `test_data/derms/`.

### Phase 3 – `gat adms` reliability, FLISR & VVO planning

* [ ] Define `reliability.parquet`, `switching_devices.parquet`, `outage_scenarios.parquet` schemas.
* [ ] Implement `gat adms flisr-sim`:

  * Start with *very simple* FLISR:

    * Single fault at a time; isolate nearest sectionalizing devices; reclose alternate feeds if capacity allows.
* [ ] Implement `gat adms outage-mc`:

  * Poisson sampling of failures using `lambda` & repair times from `reliability.parquet`.
  * For each sample, call `flisr-sim` and compute SAIDI/SAIFI.
* [ ] Implement `gat adms vvo-plan` on top of `dist opf` with `--objective vvo`:

  * Static day-types: low, medium, high load.
  * Optimize taps/caps/DER reactive setpoints.
* [ ] Implement `gat adms se-loop` (optional):

  * Glue script that:

    * Reads raw measurements,
    * Calls `gat se wls`,
    * Writes estimated state,
    * Optionally solves a small corrective `dist opf`.

### Phase 4 – TUI/GUI integration & datasets

* [ ] Extend `gat-tui` with:

  * A **feeder view** panel using `dist_nodes/dist_branches`.
  * A **DERMS dashboard** panel showing flexibility envelopes, schedule metrics.
  * An **ADMS reliability panel** showing SAIDI/SAIFI and outage maps.
* [ ] Add `gat dataset public` entries for:

  * IEEE 13-node / 34-node feeders with synthetic DERs.
* [ ] Expand docs:

  * Tutorials: “From transmission OPF to DERMS envelopes in 30 minutes.”
  * Cookbook recipes for VVO, hosting capacity, reliability simulations.

---



[1]: https://www.energy.gov/sites/default/files/2024-02/11-02-2015_doe-voe-insights-into-advanced-distribution-management-systems-report_508.pdf?utm_source=chatgpt.com "Insights into Advanced Distribution Management Systems"
[2]: https://www.nrel.gov/grid/distributed-energy-resource-management-systems?utm_source=chatgpt.com "Distributed Energy Resource Management Systems"
[3]: https://crates.io/crates/powers?utm_source=chatgpt.com "powers - crates.io: Rust Package Registry"
[4]: https://nali.seas.harvard.edu/file_url/122?utm_source=chatgpt.com "Optimal Power Flow in Distribution Networks"
[5]: https://www.mdpi.com/1996-1073/16/16/5974?utm_source=chatgpt.com "Optimal Power Flow in Distribution Network: A Review on ..."
[6]: https://www.utdallas.edu/~ths150130/papers/GuoEtAl_ACC2020.pdf?utm_source=chatgpt.com "Solving Optimal Power Flow for Distribution Networks with ..."
[7]: https://knowledge.rtds.com/hc/en-us/articles/360037559653-Real-Time-Optimisation-of-Distribution-Networks-using-Optimal-Power-Flow?utm_source=chatgpt.com "Real-Time Optimisation of Distribution Networks using ..."
[8]: https://gmlc.doe.gov/projects/federated-architecture-secure-and-transactive-distributed-energy-resource-management?utm_source=chatgpt.com "Federated Architecture for Secure and Transactive Distributed ..."
[9]: https://research-hub.nrel.gov/en/publications/solving-optimal-power-flow-for-distribution-networks-with-state-e-2?utm_source=chatgpt.com "Solving Optimal Power Flow for Distribution Networks with ..."
