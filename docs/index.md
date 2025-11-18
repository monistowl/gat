# GRID ANALYSIS TOOLKIT Documentation

This tree mixes auto-generated references (`cargo xtask doc:*`) with curated guides that track the current CLI surface.

## Contents

- **CLI reference** – `docs/cli/gat.md` (with `docs/man/gat.1` for Unix installs).
- **Schemas** – `docs/schemas/manifest.schema.json`, `docs/schemas/flows.schema.json`.
- **Arrow layouts** – `docs/arrow/README.md` plus exported `schema.json` files.
- **Roadmap** – `docs/ROADMAP.md` is the canonical plan with milestone status notes.
- **Guides** – start with `docs/guide/overview.md`, then dive into topical docs:
  - `docs/guide/opf.md` for DC/AC OPF inputs, outputs, and fixtures.
  - `docs/guide/state_estimation.md` for the WLS estimator and measurement schema.
  - `docs/guide/scaling.md` for practical fan-out and throughput guidance.
- **Archive** – `docs/archive/README.md` summarizes older narrative docs preserved for context.

Refer to `README.md` and `docs/README.md` for workflow tips and generation commands.
