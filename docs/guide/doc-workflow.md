# Documentation & issue workflow

This project treats documentation updates and issue tracking as linked operations. Follow these steps when you touch docs:

1. **Claim the work via `bd`**
   * Run `bd ready --json` to see unblocked items.
   * Use `bd update <id> --status in_progress --json` once you start.
   * If you discover a new task while editing docs, `bd create "..." -p 1 --deps discovered-from:<parent-id> --json` keeps the graph tidy.
2. **Edit the Markdown under `docs/`**
   * Pull content into `docs/guide/` or the generated folders (`docs/cli`, `docs/schemas`, etc.).
   * Keep ephemeral design notes (plans, designs, tests) under `history/` so the repository root stays clean.
3. **Regenerate the auto-docs**
   * Run `cargo xtask --features docs doc all` (or the shorthand `cargo xtask --features docs doc:all`) to refresh CLI references, schemas, Arrow dumps, and the guide that `gat-mcp-docs` serves.
   * The `docs` feature pulls in the heavier CLI/doc dependencies; leave it disabled for fast checks and only enable it when you need to regenerate outputs.
   * Optionally run `cargo xtask --features docs doc site` before releasing the built `site/book/` bundle.
   * Confirm that `docs/index.md`, `docs/README.md`, and the new guide files sync with your changes.
4. **Preview the docs**
   * Run `scripts/mcp-onboard.sh` (or `cargo xtask --features docs doc all` followed by `gat-mcp-docs --docs docs --addr 127.0.0.1:4321`) so the regenerated tree is served via MCP.
   * Open `docs/mcp/manifest.json` after the server starts to review the curated commands, datasets, and guide links that are available to every agent.
5. **Wrap up with `bd`**
   * Update the issue: `bd update <id> --status review --json` or `--status completed` when merged.
   * Close the issue with `bd close <id> --reason "Completed" --json` once everything lands.

Following this workflow keeps the `docs/` tree in lockstep with the code, keeps `cargo xtask doc all` evergreen, and ensures `bd` stays authoritative for open work.
