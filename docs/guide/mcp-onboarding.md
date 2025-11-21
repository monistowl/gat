# MCP onboarding for GAT

The MCP server should serve a ready-to-use tree the moment the machine is provisioned. The quick path is:

1. Install Rust + workspace dependencies (see `README.md`).
2. Run `scripts/mcp-onboard.sh`; it regenerates every doc target (`cargo xtask --features docs doc all`) and starts `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` so agents can immediately discover the latest CLI refs, schemas, and guide content.
3. Visit `http://127.0.0.1:4321/` (via MCP tooling) and open `docs/mcp/manifest.json`, which curates the most helpful commands, datasets, and guides. Use that manifest as your first checkpoint when learning or scripting with GAT.

## Manifest highlights (`docs/mcp/manifest.json`)

The manifest is intentionally small, with these sections:

* `onboarding` — the helper script that keeps the MCP server aligned with the repo.
* `commands` — sample CLI invocations for DC/AC PF, analytics, DER prep, and reproducible runs.
* `datasets` — curated dataset fetch commands that any MCP agent can execute before launching workflows.
* `guides` — quick links into the ADMS, DERMS, and distribution docs that explain how `gat-adms`, `gat-derms`, and `gat-dist` tie into the CLI.

Explore that manifest and copy the commands you need; they will still work from any shell that has `gat` installed and points to this workspace.

## Keeping everything fresh

After you make documentation or manifest changes:

1. Regenerate the docs (`cargo xtask --features docs doc all`).
2. Start (or restart) `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` so the new tree is served.
3. The manifest is part of the `docs` tree, so any restart of `scripts/mcp-onboard.sh` keeps the commands and guides current.

## Parallel workflow inspiration

These commands can run in parallel/automation contexts:

* `gat pf dc` + `gat pf ac` to explore solver outputs per grid.
* `gat analytics ptdf` to seed `gat-dist` or `gat-adms` heuristics.
* `gat dataset public fetch` tasks that download telemetry/datasets before analytic runs.

Use `parallel` or `xargs -P` with the manifest entries to fan-out workloads right from the MCP tree.
