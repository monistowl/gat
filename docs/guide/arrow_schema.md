# Arrow Schema Overview

The GAT Arrow dataset is a normalized, folder-based representation of a power system case.
It is designed for high-performance analysis pipelines that follow the zero-copy, columnar
performance principles of Apache Arrow [\[\[doi:10.32614/CRAN.package.arrow\]\]](#references), so
each table is optimized for vectorized access and multi-language consumption.

## Table Layout

1. **system.arrow** — single-row metadata record. Stores the system base MVA, the base frequency,
   and optional descriptive fields (name/description). This metadata enables downstream workflows
   (time series, contingencies) to align per-unit conversions before running the Newton-Raphson or
   other iterative power flow solvers [\[\[doi:10.1109/TPAS.1984.318546\]\]](#references).
2. **buses.arrow** — every node in the graph. Fields cover the bus ID, voltage/kV level, per-unit
   voltage & angle, type (PQ/PV/Slack/Isolated), voltage limits, and area/zone identifiers. These
   columns are extracted directly from each source format and map one-to-one to the IEEE-style
   test case representation.
3. **generators.arrow** — one record per synchronous machine or synthetic plant, including status,
   operating point, limits, voltage setpoint, machine base, and cost data. Cost models are stored as
   a discrete `cost_model` flag plus nested `cost_coeffs`/`cost_values` column vectors so that
   dispatch solvers can recover polynomial or piecewise linear curves without requiring format-specific
   hacks.
4. **loads.arrow** — loads preserve only active/reactive demand, status, and bus references.
5. **branches.arrow** — lines and transformers share one schema that records element type, impedance,
   transformer parameters (`tap_ratio`, `phase_shift`), thermal ratings `rate_a/b/c`, and angle limits.
   This ensures both AC and DC solvers can read the same orbitals while retaining the graph topology.

## Schema Guarantees

- **Round-trip fidelity.** Every importer (`matpower`, `psse`, `cim`, `pandapower`, etc.) writes to
  the same schema, so export/import cycles preserve all parameters (generators’ voltage targets,
  branch thermal limits, etc.).
- **Reference provenance.** Each folder includes a `manifest.json` that stores SHA256 hashes and
  optional `SourceInfo` metadata for governance and reproducibility.
- **Extensible analytics.** Add new tables (e.g., `ts_powerflow.arrow`) by following the same
  columnar layout and manifest strategy; consumers already expect folder + manifest layout, so
  adding new tables is backward compatible.

## Using the Schema

1. Import a source (e.g., MATPOWER) to `grid.arrow`.
2. Open the folder with `ArrowDirectoryReader::open`.
3. Query tables using DuckDB, Polars, or any Arrow-compatible tool to review per-table data before
   running algorithms such as Newton-Raphson power flow or contingency screening.

This schema is intentionally orthogonal to solver design: the same Arrow dataset can feed DC, AC, or
stochastic power flow expansions because all voltage, angle, cost, and limit columns are explicitly
typed and versioned via `manifest.schema_version`.

## References

1. Richardson N. et al., *arrow: Integration to ‘Apache’ ‘Arrow’*. R package (2025). DOI: [10.32614/CRAN.package.arrow](https://doi.org/10.32614/CRAN.package.arrow).
2. Wamser R.J. & Slutsker I.W., *Power flow solution by the Newton-Raphson method in transient stability studies*. IEEE Trans. Power Appar. Syst. (1984). DOI: [10.1109/TPAS.1984.318546](https://doi.org/10.1109/TPAS.1984.318546).
