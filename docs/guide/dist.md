# Distribution workflows (`gat dist`)

The `gat dist` namespace introduces distribution-specific table schemas and thin CLI wrappers that mirror the existing PF/OPF patterns. The current implementation focuses on schema validation and lightweight summaries suitable for quick sandboxing.

## Schemas
- `dist_nodes.parquet`: `node_id`, `phase`, `type`, `v_min`, `v_max`, `load_p`, `load_q`, `feeder_id`.
- `dist_branches.parquet`: `branch_id`, `from_node`, `to_node`, `r`, `x`, `b`, `tap`, `status`, `thermal_limit`.

## Commands
- `gat dist import --m case.zip --nodes-out dist_nodes.parquet --branches-out dist_branches.parquet [--feeder feeder-1]`
  - Imports a MATPOWER case and materializes feeder-friendly node/branch tables.
- `gat dist pf --nodes dist_nodes.parquet --branches dist_branches.parquet --out pf_summary.parquet`
  - Validates the tables and emits per-feeder load/voltage summaries.
- `gat dist opf --nodes dist_nodes.parquet --branches dist_branches.parquet --out opf_summary.parquet`
  - Mirrors `pf` while adding placeholder loss/objective fields.
- `gat dist hostcap --nodes dist_nodes.parquet --branches dist_branches.parquet --summary-out hostcap_summary.parquet --detail-out hostcap_detail.parquet [--max-mw 5.0 --step-mw 0.5 --targets n1,n2]`
  - Sweeps incremental DER injections for selected nodes and writes summary and per-step feasibility tables.

## Notes
- Outputs follow the same Parquet + `run.json` mindset as other commands, so they can be composed in batch runs.
- Host-capacity sweeps default to probing all nodes if no `--targets` are provided.
