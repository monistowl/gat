+++
title = "Arrow Schema Deep Dive"
description = "Explore the folder-based Arrow schema that GAT used as its canonical intermediate."
weight = 20
+++

# Arrow Schema Deep Dive

GAT stores every imported grid in a **folder-based Arrow dataset** (`system.arrow`, `buses.arrow`, `generators.arrow`, `loads.arrow`, `branches.arrow`). This pattern was introduced to support:

- **Zero-copy analysis** – Apache Arrow columnar tables are memory-map friendly and interchangeable with DuckDB/Polars/Pandas without re-serializing.
- **Schema fidelity** – Each importer (MATPOWER, PSS/E, CIM, PandaPower, etc.) populates the same columns so solvers/operators can reason about voltages, limits, costs, and topology without format sniffing.
- **Provenance + diagnostics** – The accompanying `manifest.json` stores checksums plus `SourceInfo`, making it easy to surface “imported from case14.m on 2025-11-26” in dashboards.

## Table summary

| Table | Key columns | Purpose |
| --- | --- | --- |
| `system.arrow` | `base_mva`, `base_frequency_hz`, `name`, `description` | Sets the per-unit context for Newton–Raphson or DC power flow solvers. |
| `buses.arrow` | Bus IDs, voltage_kV, voltage/angle PU, bus type, vmin/vmax, area/zone | Defines nodes in the graph; every solver references these fields. |
| `generators.arrow` | Active/reactive setpoints, limits, cost model, voltage setpoints | Economic dispatch and initial guesses depend on these columns. |
| `loads.arrow` | Power demand, status | Simple representation of demand at known buses. |
| `branches.arrow` | From/to buses, impedance, transformer taps, thermal limits, angle locks | Contains both lines and transformers in one schema for topology imports. |

## Why this matters

- **Cross-format parity** — Because MATPOWER, PandaPower, CIM, and PSS/E importers write the exact same schema, you can run `gat pf` or `gat opf` on any dataset once it’s been converted to Arrow.
- **Fast consumption** — Arrow tables are ready for Polars/DuckDB/Spark; no format-specific shims are required.
- **Published algorithms** — The schema maps directly to established power-flow representations (e.g., Newton-Raphson [\[doi:10.1109/TPAS.1984.318546\]](#references)).

## Learn more

- Need to interconvert formats? See [gat convert format](@/guide/convert.md), which uses this Arrow schema as the intermediary.
- For detailed schema specifications, see `docs/guide/arrow_schema.md` in your local GAT repository.

## References

1. Boukobza, M. et al., *Newton–Raphson load flow formulation*, IEEE (1984). DOI: [10.1109/TPAS.1984.318546](https://doi.org/10.1109/TPAS.1984.318546)
2. Apache Arrow contributors, *Arrow: cross-language columnar format* (2025). DOI: [10.32614/CRAN.package.arrow](https://doi.org/10.32614/CRAN.package.arrow)
