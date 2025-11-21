# gat-adms

The `gat-adms` crate contains distribution-automation helpers for FLISR / VVO planning and outage sampling. It wires `gat-core`, `gat-algo`, `gat-dist`, and Polars into the CLI commands that orchestration scripts need when evaluating reliability scenarios.

See `docs/guide/adms.md` for the GAT workflow that consumes these helpers and explains how to supply grids, reliability tables, and solver settings.
