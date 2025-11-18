# Training Demos

This directory collects lightweight, user-facing demonstrations that complement the automated docs in `docs/cli` and the "learn by doing" tone of `README.md`.

Each demo lives in two parts:

1. A small script under `test_data/demos/` that runs with minimal dependencies (`python3`, `bash`, or simple `gat` commands).
2. A Markdown walkthrough in this directory describing the scenario, the commands to run, and what the output means.

The `test_data/demo.md` overview points to the reliability-pricing script so newcomers can immediately reproduce the figures shown in the README. Add more demos by creating new scripts under `test_data/demos/` and pairing them with markdown notes here. Link back to this index from `docs/README.md` once you add more entries so the MCP docs server highlights the training series.

## Current entries

* `test_data/demos/reliability_pricing.py` → `docs/guide/demos/reliability-pricing.md`
* `test_data/demos/storage_cournot.sh` → `docs/guide/demos/storage-cournot.md`
