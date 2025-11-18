Here’s a focused, buildable roadmap to add **auto-documentation** to GAT and make it **discoverable via MCP**—modeled on the ELF plan but tuned to your CLI-first, shared-lib architecture.

# Goals

* **One source of truth**: Rustdoc comments, CLI metadata, and schemas generate all docs.
* **Zero-drift**: docs are rebuilt on CI for every commit/tag.
* **Agent-ready**: an **MCP server** exposes docs as browsable **resources** and queryable **tools**, so Claude/ChatGPT/OpenAI Agents/Windows hosts can use them directly. ([Model Context Protocol][1])

---

# Phase 0 — Foundations (week 1)

**Doc sources to standardize**

* **Rustdoc** on all public items (crates: `gat-core`, `gat-io`, `gat-algo`, `gat-ts`, `gat-viz`, `gat-cli`, `gat-gui`).
* **CLI help** via `clap` derives; wire flags/env defaults carefully so they render into man/MD. Use:

  * `clap_mangen` → **man pages**. ([Crates][2])
  * `clap-markdown` → **Markdown command refs**. ([Docs.rs][3])
* **Schemas**: derive JSON Schema for config structs (`schemars`), dump **Arrow** schemas for tabular outputs, and include Parquet metadata dumps. (Arrow JSON/IPC and schema export are standard). ([GitHub][4])

**Repo plumbing**

* Add `xtask/` with `xtask doc:all` to orchestrate all generators.
* Decide “docs home”: `docs/` (source) → `site/` (built).

**Acceptance**

* `cargo xtask doc:all` produces:

  * `docs/cli/*.md`, `docs/man/*`, `docs/schemas/*.json`, `docs/arrow/*.json`, `docs/guide/*.md`.

---

# Phase 1 — Generators (week 2)

**1) CLI reference (auto)**

* `gat-cli` builds a `Command` tree at runtime. Add `--dump-markdown` and `--dump-man` (dev-only) that call the generators, but rely on `xtask` in CI.
* Output: `docs/cli/gat.md` (with subcommand anchors) + `docs/man/gat.1`.

**2) API reference (auto)**

* Use **rustdoc** HTML for humans and **rustdoc-json** (nightly) for tooling (e.g., to auto-list important types/functions). Mark it as best-effort/unstable. ([Rust Documentation][5])
* Output: `site/rustdoc/` and `docs/api/rustdoc.json`.

**3) Data schemas (auto)**

* Add `#[derive(JsonSchema)]` to config/manifest types; emit to `docs/schemas/*.schema.json`.
* At runtime, dump **Arrow** schemas for key tables (`grid.arrow`, `flows.parquet`, etc.) to `docs/arrow/*.schema.json`.

**4) Book/site (auto)**

* Assemble everything with **mdBook**:

  * `SUMMARY.md` includes *Guide*, *CLI Reference*, *Schemas*, *Examples*, *Changelogs*.
  * Build to `site/book/` for local viewing and GH Pages. ([rust-lang.github.io][6])

**Acceptance**

* `cargo xtask doc:site` → browsable mdBook with live links to CLI pages, schemas, and rustdoc.

---

# Phase 2 — MCP surface for docs (week 3)

**Ship `gat-mcp-docs` server** (Rust, tokio):

* **Resources** (read-only):

  * `docs:` virtual tree exposing `docs/cli/*.md`, `docs/schemas/*.json`, `docs/arrow/*.json`, `site/book/` index, and `runs/<id>/manifest.json` links.
* **Tools** (read-only):

  * `search_docs(query) -> [{title, uri, snippet}]` (simple full-text over MD/JSON).
  * `get_doc(uri) -> {mime, bytes}`
  * `list_sections() -> tree` (for agent UIs to render menus).
  * `explain(command|schema_id) -> markdown` (template-based synth from sources).
* **Prompts** (quality-of-life): “How do I run N-1 DC?”, “What does `opf dc` output look like?”—return pre-curated instructions + links.

Test with the **MCP Inspector**; document **stdio** and **streamable-HTTP** transports so different hosts can attach easily. ([Model Context Protocol][7])

**Acceptance**

* From MCP Inspector, you can browse the docs tree, fetch pages, search, and open schemas.
* From OpenAI Agents/Claude Desktop, the docs show up as resources/tools. ([OpenAI GitHub][8])

---

# Phase 3 — Wire into the GUI & runs (week 4)

**Contextual help**

* In **gat-gui (egui)**, add a **Docs** side panel backed by the MCP client:

  * When a user focuses `pf ac`, the panel calls `explain("pf ac")` and shows examples + the exact schema links.
  * Link “copy-as-CLI” snippets.

**Run manifests ↔ docs**

* Every heavy subcommand writes `run_manifest.json` (already planned). Add canonical **doc URIs** (MCP `resource://docs/cli/pf-ac.md`) inside manifests so agents can cite the exact versioned docs used for a run.

**Acceptance**

* Users can click from a result table to the matching schema and command doc without leaving the app.

---

# Phase 4 — CI/CD & versioning (week 5)

**CI steps (GitHub Actions)**

1. Build docs (`xtask doc:site`) on every PR; publish on tags to `gh-pages` or a static bucket.
2. Build and publish `gat-mcp-docs` container + binary.
3. Emit a **docs index** (`docs/index.json`) listing versions → URIs; MCP server reads this to expose **version-pinning**.

**Acceptance**

* Docs are **immutable per tag**; MCP server defaults to “latest” but can serve `vX.Y.Z`.

---

# Phase 5 — “Executable docs” & search UX (weeks 6–8)

**Executable examples**

* Add runnable **examples**: each page can embed a JSON **ChunkSpec** the CLI accepts (`gat … --chunk-spec`). GUI exposes “Run this example.”
* Optionally add **Arrow Flight** URLs for sample datasets so external tools can preview results quickly. ([GitHub][4])

**Better search**

* Build a tiny indexer (e.g., Tantivy) to power `search_docs` with ranking and section hits.

---

# Security & tenancy defaults

* MCP server is **read-only**, whitelisted roots only (e.g., `docs/` and a `/public/` artifacts bucket).
* No remote code execution; large blobs streamed with size caps.
* Surface provenance: build SHA, crate versions, and doc gen time on each page (agents can display it).
* Aligns with MCP host guidance on consent/registry and enterprise usage. ([Windows Blog][9])

---

# Deliverables checklist

* `xtask/` with targets:

  * `doc:cli` (clap-markdown, clap_mangen)
  * `doc:api` (rustdoc HTML + rustdoc-json)
  * `doc:schemas` (schemars + Arrow schema dumps)
  * `doc:book` (mdBook assembly)
  * `doc:site` (aggregate and stage to `site/`)
* `crates/gat-mcp-docs/` (server exposing resources/tools/prompts)
* `gat-gui` docs panel (MCP client)
* CI job: build + publish docs and MCP server; versioned `docs/index.json`.

---

# Minimal “day-1” user stories

1. **“What does `gat opf dc` expect and emit?”**

   * Agent calls `search_docs("opf dc")` → returns `cli/opf-dc.md` + `schemas/opf-dc-output.schema.json`.
   * User clicks “copy example command” or runs the embedded ChunkSpec.

2. **“What are the columns in `flows.parquet`?”**

   * Agent opens `arrow/flows.schema.json` (derived from Arrow/Parquet).
   * Shows types/units and links back to the producing command.

3. **“Pin docs to tag v0.4.2.”**

   * Agent requests `version=v0.4.2` in MCP capabilities; resources map to the frozen site.

---

If you want, I can stub `xtask` (clap MD/man, schemars export, mdBook glue), and a minimal `gat-mcp-docs` (stdio transport, resources + `search_docs`) so you can see the end-to-end flow in a single PR.

[1]: https://modelcontextprotocol.io/specification/2025-03-26?utm_source=chatgpt.com "Specification"
[2]: https://crates.io/crates/clap_mangen?utm_source=chatgpt.com "clap_mangen - crates.io: Rust Package Registry"
[3]: https://docs.rs/clap-markdown?utm_source=chatgpt.com "clap_markdown - Rust"
[4]: https://github.com/modelcontextprotocol/inspector?utm_source=chatgpt.com "modelcontextprotocol/inspector: Visual testing tool for MCP ..."
[5]: https://doc.rust-lang.org/rustdoc/unstable-features.html?utm_source=chatgpt.com "Unstable features - The rustdoc book"
[6]: https://rust-lang.github.io/mdBook/?utm_source=chatgpt.com "Introduction - mdBook Documentation"
[7]: https://modelcontextprotocol.io/docs/tools/inspector?utm_source=chatgpt.com "MCP Inspector"
[8]: https://openai.github.io/openai-agents-python/mcp/?utm_source=chatgpt.com "Model context protocol (MCP) - OpenAI Agents SDK"
[9]: https://blogs.windows.com/windowsexperience/2025/05/19/securing-the-model-context-protocol-building-a-safer-agentic-future-on-windows/?utm_source=chatgpt.com "Securing the Model Context Protocol: Building a safer agentic ..."
