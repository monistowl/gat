#GAT: GRID ANALYSIS TOOLKIT
GAT is a CLI-driven suite of tools for working with power grid analytics data.

# 0) Workspace layout (monorepo)

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
  scripts/           # package/install helpers
  test_data/         # tiny RAW / MATPOWER / CIM snippets, CSVs
  ROADMAP.md         # this plan with acceptance criteria
```

Why this shape: it mirrors ELF’s split between `*-lib`, `*-cli`, and `*-gui`, plus installer scripts and fixtures, so both CLI and GUI ride one code path and you can ship self-contained tarballs with symlinked binaries. ([GitHub][1])

---

# 1) Cross-cutting foundations (Milestone M0)

**Targets**

* Error + tracing: `anyhow`/`thiserror` and `tracing` + `tracing-subscriber`. ([Docs.rs][2])
* CLI UX: `clap` (derive), shell completion, manpages; progress bars via `indicatif`. ([Docs.rs][3])
* Dataframe + columnar I/O: `polars` (lazy) + Arrow/Parquet; keep features minimal (`default-features = false` where useful). ([Docs.rs][4])
* Numerics: `ndarray`/`faer` for dense ops; `sprs` (and friends) for sparse; consider `ndarray-linalg` if you want LAPACK-backed paths. ([Docs.rs][5])
* Graphs: `petgraph` for topology (buses/branches). ([Docs.rs][6])

**Deliverables**

* `gat-core` with typed units, IDs, and a neutral network schema (Bus/Branch/Gen/Load/Transformer; attach metadata via `petgraph` node/edge weights).
* `gat-cli` scaffolding with `gat --help`, `--log-level`, and global `--profile`.

---

# 2) File formats & ingestion (M1)

**Scope**

* **PSS®E RAW**: use the `power_flow_data` crate (PSSE .raw parser) for fast ingestion. ([Crates][7])
* **MATPOWER case**: ingest/export with the `caseformat` crate. ([Crates][8])
* **CIM (IEC 61970/61968)**: stream RDF/XML via `quick-xml` and map triples with `sophia` (RDF toolkit). Keep a thin mapping layer (CIM class ↦ `gat-core` types). ([Docs.rs][9])

  * Background references to ensure correct scope of 61970/61968. ([PNNL][10])

**CLI**

* `gat import psse --raw case.raw -o grid.arrow`
* `gat import matpower --m case.m -o grid.arrow`
* `gat import cim --rdf dir_or_zip -o grid.arrow`
* `gat validate dataset --spec tests/schema.json` (ELF-style dataset checker). ([GitHub][1])

**Acceptance**

* Round-trip MATPOWER↔internal↔Parquet; ingest 5+ public RAW files; parse a small CIM excerpt and retain topology + nameplate.

---

# 3) Topology & graph utilities (M2)

**Scope**

* Build/clean graph from ingested data: connectivity, islanding, k-core, spanning trees, degree stats. `petgraph::algo` covers BFS/DFS, Dijkstra, SCC; add your own on top. ([Docs.rs][11])

**CLI**

* `gat graph stats grid.arrow`
* `gat graph islands grid.arrow --emit island_id`
* `gat graph export --format graphviz`

---

# 4) Power flow solvers (M3)

**DC PF**

* Linear DC PF: assemble B′ and solve sparse linear system; `sprs` for CSR + triangular solves; or `faer` for dense fallbacks. ([Docs.rs][12])

**AC PF**

* Start with Newton–Raphson (PQ/PV constraints, polar form). Use `sprs` + LDL where possible; consider existing crates:

  * `power_flow` (AC PF via SUNDIALS/KINSOL) if you want a reference/interop path. ([lib.rs][13])
  * Experimental power system crates (e.g., RustPower) exist but plan to own core algorithms. ([Docs.rs][14])

**CLI**

* `gat pf dc grid.arrow --out flows.parquet`
* `gat pf ac grid.arrow --tol 1e-8 --max-iter 20`

**Acceptance**

* Converge standard MATPOWER cases; compare bus angles/flows with known DC baselines; AC NR convergence on cases up to ~10k buses.

---

# 5) Optimal Power Flow (M4)

**DC-OPF**

* Model as LP/MILP with `good_lp` + `highs` backend; supports large LP/MIP and integrates cleanly. ([GitHub][15])

**(Later) AC-OPF / nonlinear**

* Stage 1: penalty/linearization approaches.
* Stage 2: prototype with `argmin` (pure Rust solvers) and/or link an external solver if needed. ([Argmin][16])

**CLI**

* `gat opf dc grid.arrow --cost cost.csv --limits limits.csv -o dispatch.parquet`

---

# 6) Contingency analysis & state estimation (M5)

**N-1 screening**

* Parallel DCPF across outages; graph-delta updates where possible.

**State Estimation (WLS)**

* Implement WLS with sparse normal equations; reuse `sprs` and `faer`.

**CLI**

* `gat nminus1 dc grid.arrow --contingencies outages.csv -o results.parquet`
* `gat se wls grid.arrow --measurements meas.parquet`

---

# 7) Time-series & markets (M6)

**Timeseries**

* Store and manipulate telemetry (SCADA/PMU) with Polars LazyFrames; resample/join/window ops. ([Docs.rs][17])
* Optional: NetCDF/NetCDF-3 adapters (`netcdf`, `netcdf3`) for weather & load profiles. ([Crates][18])

**CLI**

* `gat ts resample telemetry.parquet --rule 5s`
* `gat ts join a.parquet b.parquet --on ts`

---

# 8) GUI (egui) dashboard (M7)

**Approach**

* `eframe` app embedding shared `gat-viz` plots; tabbed layout via `egui_dock`; multiple viewports if you want multi-window workflows. ([Docs.rs][19])

**Panels**

* Loader (Arrow/Parquet/Run bundle), Network view (summary + island map), PF panel (run/inspect), OPF panel (cost/limits/solution), TS panel (curves), Contingency panel (ranked violations).

**Parity with CLI**

* Each GUI action spawns the same internal pipeline the CLI uses (exact same functions). This mirrors ELF’s “shared store/figure model” idea. ([GitHub][1])

---

# 9) Visualization & export (M7+)

* Lightweight plotting primitives (lines/markers) for CLI/GUI parity—export SVG/PNG/Parquet summaries.
* Optional geospatial overlays: `geo` + `shapefile`/`geozero-shp` to render lines on maps. ([Docs.rs][20])

---

# 10) Packaging & installers (continuous)

* ELF-style `scripts/package.sh` to produce `gat-<ver>-<arch>-<os>.tar.xz` plus `.sha256` and `scripts/install.sh` that symlinks `gat` and `gat-gui` to `~/.local/bin` (and a `current` pointer). ([GitHub][1])

---

# 11) Testing strategy

* Golden fixtures in `test_data/` (tiny RAW/MATPOWER/CIM); CLI regression suite (`cargo test` spawns subcommands).
* Compare PF/OPF outputs against well-known cases (MATPOWER docs; PowerModels/Julia PSSE parsing references). ([Matpower][21])

---

# 12) Initial CLI surface (v0.1)

```
gat import {psse|matpower|cim} …      # M1
gat validate dataset …                 # M1
gat graph {stats|islands|export} …     # M2
gat pf {dc|ac} …                       # M3
gat opf dc …                           # M4
gat nminus1 dc …                       # M5
gat se wls …                           # M5
gat ts {resample|join|agg} …           # M6
```

---

## Crate shortlist (by task)

* **Formats & data**

  * PSS®E RAW: `power_flow_data`. ([Crates][7])
  * MATPOWER: `caseformat`. ([Docs.rs][22])
  * CIM RDF/XML: `quick-xml` + `sophia`. ([Docs.rs][9])
  * DataFrames: `polars` (+ `parquet`, `arrow/arrow2`). ([Docs.rs][4])
  * NetCDF (optional): `netcdf`, `netcdf3`. ([Crates][18])
  * Geo (optional): `geo`, `shapefile`. ([Docs.rs][20])
* **Math & graphs**

  * Dense LA: `ndarray`, `faer`; Sparse: `sprs`. ([Docs.rs][5])
  * Graphs: `petgraph`. ([Docs.rs][6])
* **Solvers**

  * LP/MILP (DC-OPF): `good_lp` + `highs`. ([GitHub][15])
  * Nonlinear: `argmin` (prototype). ([Argmin][16])
  * (Reference AC PF crate): `power_flow`. ([lib.rs][13])
* **CLI/infra**

  * `clap`, `indicatif`, `tracing` & `tracing-subscriber`. ([Docs.rs][3])
* **GUI**

  * `egui`/`eframe`, `egui_dock`, multi-viewport guidance. ([Docs.rs][19])

---

## Phasing & rough effort

* **M0–M1 (ingest & skeleton)**: weeks — core types, parsers, Arrow/Parquet, basic CLI. ([Crates][7])
* **M2–M3 (graphs + PF)**: weeks — topology tools, DC/AC PF. ([Docs.rs][6])
* **M4–M5 (OPF + N-1/SE)**: weeks — DC-OPF via HiGHS; N-1 DC; WLS SE. ([GitHub][15])
* **M6–M7 (TS + GUI)**:  — Polars TS, egui tabs, parity with CLI. ([Docs.rs][4])

---

## Notes & nudges

* Prefer **Arrow/Parquet** for all intermediate artifacts (fast, columnar, zero-copy friendly). ([Apache Arrow][23])
* Keep **CIM** minimal at first (topology + basic equipment classes); it’s big—lean on streaming XML and selective RDF term mapping. ([Docs.rs][9])
* Use **a simple installer pattern** (tarball + `install.sh`) so labs can adopt without cargo. ([GitHub][1])
* For large cases, bias to **sparse paths** (`sprs`) and graph-aware updates.

---

# 14) Easiest wins: drop-in adapters

Leverage publicly maintained datasets that already publish MATPOWER/CSV/Parquet slices so GAT can ingest them with existing adapters (`gat-io`, `gat-ts`, `polars`, `netcdf`). Focus on the commands below to hydrate test cases, telemetry, and resource weather onto the Arrow/Parquet stack, then reuse the `pf/opf/nminus1/ts` paths you already built.

## Keys to success

1. **RTS-GMLC** – publish `gat dataset rts-gmlc fetch [--tag vX.Y]` to download releases, then `gat import matpower …` + `gat ts import …` to produce `grid.arrow` + `timeseries.parquet`, ready for `pf`, `opf`, and contingency commands. ([NREL][24])
2. **Test Case Repository for High Renewable Study** – add `gat dataset hiren list` / `gat dataset hiren fetch <case>` and normalizing importers that map their MATPOWER/CSV bundles into Arrow/Parquet benchmarks (9-bus → 240-bus). ([NREL][25])
3. **dsgrid Parquet API** – expose `gat ts import dsgrid <url-or-path>` that reads county/BA demand scenarios, joins to `polars` frames, and feeds `ts resample/join/agg`. ([NREL][26])
4. **Sup3rCC resource data** – add `gat weather sup3rcc fetch …` plus `gat weather sample --grid grid.arrow --out weather.parquet` to spatially tie resource grids to bus coordinates (use `geo` KD-tree or grid lookup) for climate-driven OPF stress testing. ([NREL][27])
5. **PRAS outputs** – `gat adequacy import-pras <path>` normalizes LOLE/EUE forecasts (by region/season/hour), optionally emitting Gat scenarios for PRAS. ([NREL][28])

### Minimal glue

* OEDI fetcher – helper to pull dsgrid/Sup3r catalogs (HTTP range, checksums) straight into Polars/NetCDF.
* Spatial join – `geo`/KD-trees for bus ↔ Sup3r grid mapping or county FIPS ↔ dsgrid records.
* Case normalizers – thin wrappers to standardize RTS-GMLC/HIREN tables before writing Arrow/Parquet.

### Nice-to-have (not day-1)

* **SMART-DS** – ingest OpenDSS/GRIDLAB outputs when you later add distribution adapters. ([NREL][29])
* **AGILE** – stream sensor data to `gat-ts` (Kafka/InfluxDB/TCP) as a downstream integration. ([NREL][30])

### Suggested CLI verbs

```
gat dataset rts-gmlc fetch [--tag vX.Y] [--out data/rts-gmlc]
gat dataset hiren {list|fetch <case>}
gat ts import dsgrid <path-or-url> [--filter county=... --filter enduse=...]
gat weather sup3rcc {fetch|sample} --grid grid.arrow --out weather.parquet
gat adequacy import-pras <dir> --out pras.parquet
```

---

# 13) Recent deliverables

* Time-series suite now provides resample/join/agg helpers (`gat ts …`) plus vetted CLI regression tests covering telemetry flows, import + DC PF, and GUI stubs (`docs/TS.md`, `docs/GUI.md`).
* Packaging scripts (`scripts/package.sh`, `scripts/install.sh`, documented in `docs/PACKAGING.md`) yield release tarballs and repeatable local installs, matching the installer pattern from M10.
* CLI tests use workspace fixtures (`test_data/ts`, `test_data/matpower`) and `assert_cmd` so future releases keep working end-to-end.

[1]: https://github.com/monistowl/elf "GitHub - monistowl/elf: Extensible Lab Framework"
[2]: https://docs.rs/anyhow?utm_source=chatgpt.com "anyhow - Rust"
[3]: https://docs.rs/clap?utm_source=chatgpt.com "clap - Rust"
[4]: https://docs.rs/polars/latest/polars/?utm_source=chatgpt.com "polars - Rust"
[5]: https://docs.rs/ndarray/?utm_source=chatgpt.com "ndarray - Rust"
[6]: https://docs.rs/petgraph/?utm_source=chatgpt.com "petgraph - Rust"
[7]: https://crates.io/crates/power_flow_data?utm_source=chatgpt.com "power_flow_data - crates.io: Rust Package Registry"
[8]: https://crates.io/crates/caseformat?utm_source=chatgpt.com "caseformat - crates.io: Rust Package Registry"
[9]: https://docs.rs/quick-xml?utm_source=chatgpt.com "quick_xml - Rust"
[10]: https://www.pnnl.gov/main/publications/external/technical_reports/PNNL-34946.pdf?utm_source=chatgpt.com "A Power Application Developer's Guide to the Common ..."
[11]: https://docs.rs/petgraph/latest/petgraph/algo/index.html?utm_source=chatgpt.com "petgraph::algo - Rust"
[12]: https://docs.rs/sprs?utm_source=chatgpt.com "sprs - Rust"
[13]: https://lib.rs/crates/power_flow?utm_source=chatgpt.com "power_flow"
[14]: https://docs.rs/rustpower?utm_source=chatgpt.com "rustpower - Rust"
[15]: https://github.com/rust-or/good_lp?utm_source=chatgpt.com "rust-or/good_lp"
[16]: https://argmin-rs.org/?utm_source=chatgpt.com "argmin | argmin - Optimization in pure Rust"
[17]: https://docs.rs/polars/latest/polars/prelude/struct.LazyFrame.html?utm_source=chatgpt.com "LazyFrame in polars::prelude - Rust"
[18]: https://crates.io/crates/netcdf?utm_source=chatgpt.com "netcdf - crates.io: Rust Package Registry"
[19]: https://docs.rs/egui_dock/latest/egui_dock/?utm_source=chatgpt.com "egui_dock - Rust"
[20]: https://docs.rs/geo/?utm_source=chatgpt.com "Crate geo - Rust"
[21]: https://matpower.org/docs/ref/matpower5.0/caseformat.html?utm_source=chatgpt.com "Description of caseformat"
[22]: https://docs.rs/caseformat?utm_source=chatgpt.com "caseformat - Rust"
[23]: https://docs.rs/polars/latest/polars/prelude/struct.LazyFrame.html?utm_source=chatgpt.com "LazyFrame in polars::prelude - Rust"
[24]: https://www.nrel.gov/grid/reliability-test-system "RTS-GMLC - NREL"
[25]: https://www.nrel.gov/grid/test-case-repository "Test Case Repository for High Renewable Study - NREL"
[26]: https://www.nrel.gov/analysis/dsgrid.html "dsgrid - Demand-Side Grid Toolkit - NREL"
[27]: https://www.nrel.gov/analysis/sup3rcc.html "Sup3r: Super‐Resolution for Renewable Energy Resource Data"
[28]: https://www.nrel.gov/analysis/pras.html "PRAS: Probabilistic Resource Adequacy Suite"
[29]: https://www.nrel.gov/grid/smart-ds "SMART-DS - NREL"
[30]: https://www.nrel.gov/grid/agile "AGILE: Autonomous Grids – Identification, Learning, and Estimation"
[23]: https://arrow.apache.org/rust/parquet/index.html?utm_source=chatgpt.com "parquet - Rust"
