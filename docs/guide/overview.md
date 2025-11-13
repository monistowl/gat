# GAT Documentation Guide

Auto-generated docs live under `docs/cli`, `docs/schemas`, and `docs/arrow`. The `xtask` targets keep the Markdown, manifest schema, and CLI output definitions versioned with the code:

- CLI reference (Markdown + man page): `cargo xtask doc cli`.
- Schema exports: `cargo xtask doc schemas`.
- Minimal site bundle: `cargo xtask doc site`.

Run `cargo xtask doc all` after making CLI, manifest, or schema changes to refresh everything. The MCP docs server (`git mcp-docs`) reads this layout and exposes it via HTTP/Model Context Protocol resources.
