# GAT Documentation Guide

Auto-generated docs live under `docs/cli`, `docs/schemas`, and `docs/arrow`. The `xtask` targets keep the Markdown, manifest schema, and CLI output definitions versioned with the code:

- CLI reference (Markdown + man page): `cargo xtask doc cli`.
- Schema exports: `cargo xtask doc schemas`.
- Minimal site bundle: `cargo xtask doc site`.

Run `cargo xtask doc all` after making CLI, manifest, or schema changes to refresh everything. The MCP docs server (`git mcp-docs`) reads this layout and exposes it via HTTP/Model Context Protocol resources.

## Outputs and partitions

Every heavy command writes into a stage-named directory (for example `pf-dc`, `opf-dc`, `nminus1-dc`, or `se-wls`) so dashboards and artifact stores can tell where work was produced. Use `--out-partitions <comma-separated-columns>` to split the Parquet output inside that stage directory by column values (e.g., `--out-partitions run_id,date/contingency` writes `stage/run_id=.../date=.../part-0000.parquet`). The stage-aware helper also respects the `run.json`/manifest layout so `gat runs resume` and downstream tools can follow the same tree.
