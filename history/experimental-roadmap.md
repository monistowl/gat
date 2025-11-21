Short version: GAT already looks like the “engine block” for a serious analytics module. The obvious next projects are mostly about (a) fanning out scenarios, (b) codifying reliability / RA metrics, (c) producing ML-friendly feature tables, and (d) adding distribution/DER/ADMS wrappers that match module decomposition.

I’ll organize this as a set of concrete, agent-parsable projects, each tied to the modules we hope to build.


Additional detailed planning documents:
history/gat-scenarios-plan.md
history/gat-batch-plan.md
history/gat-analytics-ds-plan.md
history/gat-featurize-gnn-plan.md


---

## 0. Align on the “module map”

The big pieces are:

* **CANOS** – fast scenario engine: AC-OPF counterfactuals, N-1/N-k, policy “what-ifs”.
* **Power-GNN** – physics-informed state/parameter estimation over the network graph.
* **KPI predictor** – probabilistic reliability KPIs + SHAP/explanations.
* **Allocation layer** – congestion surplus + residuals into stakeholder-facing cost narratives.
* **BMCL** – behavior-manifold co-simulation (geo-agents + grid feedback).

From GAT’s side, you already have:

* Importers (PSSE / MATPOWER / CIM).
* Core modeling & solvers (`gat-core`, `gat-algo`, OPF/PF, N-1, PTDF).
* Time-series tools (`gat-ts`).
* A run-manifest system (`gat runs`) that’s perfect for fan-out pipelines.
* Stubs for **`gat-dist`**, **`gat-derms`**, **`gat-adms`**, **`gat-schemas`**.

So the “obvious” projects are the ones that turn those stubs + core tools into the data/compute primitives that CANOS / Power-GNN / KPI / allocation / BMCL expect.

---

## 1. Scenario Engine & Reliability Sandbox (CANOS-adjacent)

**Goal:** Turn GAT into a CLI-first CANOS prototype: you hand it a base grid + scenario definitions, it fans out PF/OPF runs and emits nicely partitioned Parquet artifacts compatible with API mockups.

### 1.1 `gat scenarios` crate / subcommand

**Purpose:** Represent and manipulate scenarios as first-class objects.

* **Inputs**

  * Base grid snapshot (`grid.arrow`).
  * Time-series slices (`profiles.parquet` from `gat-ts`).
  * Scenario definitions (JSON/YAML): outages, load multipliers, renewable multipliers, dispatch overrides, policy toggles.
* **Core features**

  * `gat scenarios validate <scenarios.json>` – type/consistency checks.
  * `gat scenarios expand` – expand templated scenario definitions (e.g. “all N-1 branches”, “load_scale in {0.9, 1.0, 1.1}”).
  * `gat scenarios materialize` – produce per-scenario modified grid snapshots as Arrow files + a manifest.

---

### 1.2 `gat batch` (fan-out over scenarios × time)

**Purpose:** A generic fan-out runner that uses `gat runs` internally but makes “CANOS-style” job sets trivial.

* **Commands**

  * `gat batch pf` – run DC/AC PF for each (scenario, time_slice).
  * `gat batch opf` – run (D)C/AC OPF similarly.
* **Inputs**

  * Scenario manifest from `gat scenarios`.
  * Time-slice definitions (e.g. 24 representative hours, or a month of timestamp indices).
  * Solver options (backend, tolerances, relaxations).
* **Outputs**

  * Partitioned Parquet: `scenario_id=…/time=…/pf.parquet`, `opf.parquet`, etc.
  * A “job manifest” tying each run to the base `run.json` from the underlying `gat runs`.
* **Implementation**

  * Thin orchestrator crate (maybe `gat-batch`) that:

    * Reads manifests.
    * Spawns `gat pf` / `gat opf` subcommands in parallel.
    * Tracks their `run_id`/output locations.

**Why it helps** This is CANOS v0: “give me lots of OPF counterfactuals fast, in a reproducible format.”

---

### 1.3 `gat analytics reliability` – LOLE / EUE / RA metrics

**Purpose:** Convert batches of PF/OPF outputs into basic adequacy metrics and time-localized “stress” indicators.

* **Inputs**

  * Batch outputs from `gat batch` (flows, voltages, LMPs, unserved energy flags).
  * Scenario probabilities or weights (from a simple CSV/Parquet).
* **Commands**

  * `gat analytics reliability`:

    * Compute metrics like LOLE (hours with unserved load), EUE/ENS (energy not served), frequency of thermal violations, etc.
    * Aggregate by zone, bus, resource class.
* **Outputs**

  * Parquet tables keyed by `(scenario_id, time, zone, metric)` suitable for RA accreditation / KPI work.

**Why it helps:** Gives the “KPI predictor” a baseline truth set and provides the RA-like metrics your notes talk about (probabilities of stress, unserved energy, constrained hours, etc.).

---

## 2. Deliverability & RA Accreditation (DS × ELCC scaffolding)

Stress **Deliverability Score (DS)** and **ELCC***. You don’t need the full auction to be useful: GAT can compute network-centric DS and hand that to a higher-level RA solver.

### 2.1 `gat analytics ds` – Deliverability Score engine

**Purpose:** For each resource, quantify how “deliverable” its capacity is under stress scenarios.

* **Inputs**

  * Base grid + contingencies (from `gat scenarios`).
  * PTDF/OTDF matrices from `gat analytics ptdf`.
  * RA stress cases: representative load/renewable patterns.
* **Logic (v0)**

  * For each resource, inject a small test increment in stress cases.
  * Use PTDF/OTDF to estimate incremental flows vs. branch limits.
  * Compute DS as a function of headroom utilization (e.g. fraction of stress cases where the increment is feasible).
* **Outputs**

  * `resource_id, scenario_id, ds` Parquet table.
  * Optional zone-aggregated DS surfaces.

**Why it helps:** You can plug these DS numbers into the ELCC×DS accreditation formulas from the notes without building all the RA market scaffolding yet.

---

### 2.2 `gat analytics elcc` – simple ELCC sandbox

Even a toy ELCC estimator is useful for pipelines and examples.

* **Inputs**

  * Resource profiles (availability / output distributions).
  * Reliability metrics from §1.3.
* **Functionality (v0)**

  * For each class (solar, wind, storage, etc.), estimate marginal ELCC from:

    * Running reliability metrics with and without incremental capacity from that class.
* **Outputs**

  * `class_id, elcc_mean, elcc_ci_lo, elcc_ci_hi`.

This gives you a concrete place to hook in fancier ELCC / RA research later while keeping the interfaces stable now.

---

## 3. Feature Fabric for Power-GNN & KPI Models

**Goal:** Make GAT the place where raw grid data becomes clean, ML-ready graph/tabular features; leave the actual training to Python/Julia/whatever.

### 3.1 `gat featurize gnn` – graph export for Power-GNN

**Purpose:** Export a grid + time window as a graph dataset that a GNN can consume.

* **Inputs**

  * Grid snapshot(s), maybe plus recent PF/OPF results.
  * Time-series slices (physical measurements, loads, injections).
* **Outputs**

  * Node table: `bus_id`, static features (type, shunt, area), dynamic features (P/Q injections, voltages, past few lags).
  * Edge table: `branch_id`, from/to buses, x/r, limits, flows, outage flags.
  * Optionally: `graph_meta` with adjacency indices for PyTorch Geometric, DGL, etc.
* **CLI**

  * `gat featurize gnn <grid.arrow> <profiles.parquet> --window 2024-07-01T13:00Z/2024-07-01T19:00Z --out gnn_features.parquet`

This is the “Power-GNN input pack.” The state-estimation module can be developed in a notebook against these features without worrying about power flow details.

---

### 3.2 `gat featurize kpi` – KPI training/eval tables

**Purpose:** Feed the probabilistic KPI predictor (TabNet/NGBoost, etc.) with rich yet standardized features.

* **Inputs**

  * Outputs from `gat batch` and `gat analytics reliability` (flows, LMPs, violations).
  * Scenario metadata: which outages/policies were in play.
* **Outputs**

  * Wide tables keyed by `(scenario_id, time, zone)` with:

    * Aggregated system stress metrics (peak utilization, number of violations, RA margins).
    * Policy flags (DR programs, DER controls).
    * Weather/seasonal indices (passed in via join).
* **CLI**

  * `gat featurize kpi <batch_root> --join reliability.parquet --out kpi_features.parquet`

Gives the KPI predictor all the “X” features; the “y” labels come from the reliability tables.

---

## 4. Allocation & Settlement Sandbox

You have a whole section in the notes about congestion-surplus + residual allocation, Shapley-style narratives, etc. GAT can compute the raw quantities now and leave the more complex game theory to later.

### 4.1 `gat alloc rents` – congestion & surplus decomposition

**Purpose:** Compute congestion rents and other surplus quantities across scenarios and time.

* **Inputs**

  * OPF results (LMPs, nodal injections, line flows).
  * Tariff/margin parameters (basic CSV/Parquet).
* **Outputs**

  * Tables like:

    * `time, zone, congestion_rent`, `time, branch_id, flow_rent`.
    * `resource_id, time, congestion_charge_credit`.
* **CLI**

  * `gat alloc rents <opf_results.parquet> --topology grid.arrow --out rents.parquet`

This is the numerical backbone for the “allocation layer.” You can later plug these into a Shapley-style or BMCL-aware allocator.

---

### 4.2 `gat alloc kpi` – simple contribution analysis

Don’t do full SHAP yet; start with linear or gradient-based sensitivity proxies.

* **Inputs**

  * KPI outputs (e.g. probability of shortfall / EUE).
  * Scenario meta (which outages/controls applied).
* **Outputs**

  * For each KPI and scenario, approximate contribution of each control/portfolio to improvement/worsening.
* **CLI**

  * `gat alloc kpi <kpi_results.parquet> --meta scenarios.parquet --out kpi_contrib.parquet`

A stepping stone towards the “Partition SHAP” explainability layer in the notes.

---

## 5. Distribution, DERMS, and ADMS stubs that actually do something

You already have workspace members:

* `gat-dist` – distribution-domain tools.
* `gat-derms` – DER envelope/pricing.
* `gat-adms` – FLISR / VVO / outage helpers.

The obvious GAT-side v0s:

### 5.1 `gat-dist` v0 – MATPOWER+hosting capacity

**Goal:** Make `gat-dist` useful without going full OpenDSS.

* **Features**

  * `gat dist import matpower` – specialize importer to distribution-style MATPOWER cases (phases, low-kV, etc.).
  * `gat dist pf ac` – run full AC PF for feeders.
  * `gat dist hosting` – simple hosting capacity sweeps:

    * Step up DER injection at candidate buses until voltage/thermal limits violated.
* **Outputs**

  * Hosting curves per bus/feeder as Parquet, for later use by DERMS.

This is exactly the data you want for “hosting capacity vs. upgrades” tradeoffs in DER-heavy regions.

---

### 5.2 `gat-derms` v0 – DER envelopes & price response

**Goal:** Given granular DER assets and prices, compute aggregated envelopes and response curves.

* **Inputs**

  * DER asset table: device type, kW/kWh, constraints, location (bus/feeder).
  * Tariff/price signals (time-series).
* **Features**

  * `gat derms envelope` – compute per-bus/feeder aggregated P/Q capability regions vs. price.
  * `gat derms schedule` – offline scheduling: given a price trajectory, output dispatch of DER fleet obeying constraints.
* **Outputs**

  * Envelope polygons or sampled envelopes (Pmin/Pmax, Qmin/Qmax) per node/time.
  * Schedules as time-series (for feeding into PF/OPF).

These outputs can be passed into CANOS scenarios as controllable injections, and used in BMCL as control knobs.

---

### 5.3 `gat-adms` v0 – reliability & control building blocks

**Goal:** Provide inputs for SAIDI/SAIFI/etc. and simple control strategies.

* **Inputs**

  * Distribution topology (from `gat-dist`).
  * Failure rate tables, repair times.
  * Control rules (switching / FLISR heuristics).
* **Features**

  * `gat adms reliability` – Monte Carlo outage sampling to compute SAIDI/SAIFI/CAIDI at feeder/zone level.
  * `gat adms flisr` – simple rule-based restoration strategies to benchmark more complex approaches later.
* **Outputs**

  * Reliability metrics per zone/customer class.
  * Event logs useful for BMCL (what customers saw during outages).

Feeds both the RA/KPI side (distribution reliability) and BMCL’s focus on “localized, behavior-driven grid stress.”

---

## 6. BMCL-adjacent scaffolding (without doing all of BMCL yet)

The full BMCL is a whole research program. But some GAT-side primitives are cheap and useful:

### 6.1 `gat geo join` – GIS + grid joiner

**Purpose:** Build the “feature fabric” that BMCL wants.

* **Inputs**

  * Grid topology (buses/branches/lines).
  * GIS layers (shapefiles/GeoParquet for feeders, census tracts, etc.).
  * Load/customer tables.
* **Features**

  * Map buses/feeders to polygons (tracts, neighborhoods).
  * Aggregate loads/DER per polygon.
* **Outputs**

  * `polygon_id` ↔ `bus_id` mapping tables.
  * Polygon feature tables ready for BMCL’s “geo-agents.”

---

### 6.2 `gat featurize geo` – time-series feature fabric

**Purpose:** Produce the multi-modal feature store described in the BMCL section (weather, AMI, mobility, etc.), but as plain Parquet.

* **Inputs**

  * Time-series from AMI/SCADA/weather/mobility proxies.
  * Polygon mapping from `gat geo join`.
* **Outputs**

  * Feature tables keyed by `(polygon_id, time)` with lags, rolling stats, event flags.
* **CLI**

  * `gat featurize geo <ts_root> --join polygon_map.parquet --out geo_features.parquet`

Power-GNN and KPI models can be extended with these “behavioral” features; BMCL can run in a separate stack using these tables.

---

## 7. Meta: make everything MCP-/agent-friendly

Since you’re using coding agents and MCP:

* Ensure each new subcommand:

  * Has a **stable, explicit Parquet schema** (documented in `gat-schemas`).
  * Writes `run.json` manifests with all inputs/resolved files.
* Add or extend **`gat-mcp-docs`** so that:

  * The new commands and schemas show up in auto-generated docs.
  * An MCP client can introspect “what is a `gat featurize gnn` run and what does it produce?”

This makes it trivial for a prototype agent to orchestrate: import → scenario → batch → analytics → featurize → hand off to ML code.

---

If you want, next step could be: pick 2–3 of these (my vote: `gat scenarios`, `gat batch`, `gat analytics ds`, and `gat featurize gnn`) and I can lay out a more rigid roadmap.md with concrete tasks and file/crate touchpoints.

