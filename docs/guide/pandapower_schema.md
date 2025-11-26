# PandaPower Arrow Schema Notes

This note summarizes how the PandaPower importer maps JSON elements to the normalized Arrow schema.
PandaPower uses pandas-backed DataFrames for buses, gens, lines, transformers, and cost models, so the
importer must convert those tables into the same table-per-concept layout used by GAT’s Arrow schema.

## System/table mappings

- `system.arrow`: `net.sn_mva` → `base_mva` and `net.f_hz` → `base_frequency_hz`. When `net.name` is
  provided, it fills the `name` field, helping trace synchronization runs across Pandas and Rust.
- `buses.arrow`: `net.bus` provides `vn_kv`, `type`, `zone`, `max_vm_pu`, `min_vm_pu`, and the
  results table `net.res_bus` supplies voltages/angles when available. Slack detection is done by
  joining through `net.ext_grid`.
- `generators.arrow`: combinations of `net.gen` and `net.sgen` (static generators) map to a single
  `Gen` record per machine. The importer explicitly stores PV/Slack control data, voltage setpoints,
  and status. Cost models pulled from `net.poly_cost`/`net.pwl_cost` fill `cost_coeffs`/`cost_values`.
- `loads.arrow`: `net.load` provides loads, while `net.asymmetric_load` (if present) can extend this
  table.
- `branches.arrow`: `net.line`, `net.trafo`, and `net.trafo3w` are all flattened into the common
  branch schema, computing per-unit impedances from physical parameters so both line and transformer
  logic share the same table.

## Highlights for learners

1. The schema keeps numeric columns typed (`Float64`, `Int64`, `List<Float64>`) matching the
   Arrow definitions in `gat_io::arrow_schema`.
2. Each importer (MATPOWER, PandaPower, CIM, etc.) emits the same tables/interfaces, which means
   analytic tools (DuckDB, Polars, Arrow C++) can consume any dataset once it is exported to a folder
   of `.arrow` files plus `manifest.json`.
3. PandaPower’s JSON structure is converted into `BranchInput`, `GenInput`, and other helper structs
   before NetworkBuilder writes the canonical `gat_core::Network`. That conversion is explained in
   `docs/guide/arrow_schema.md` and this document helps map pandas-specific tables back to Arrow.

### Reference

1. PandaPower developers, *pandapower: Convenient power system analysis using pandas DataFrames*
   (2018). DOI: [10.5281/zenodo.1736517](https://doi.org/10.5281/zenodo.1736517).
