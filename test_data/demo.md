# Reliability-Pricing Demo

This demo lives under `test_data/demos/reliability_pricing.py` and is meant to give new users a short, self-contained example with no GAT binaries required. The script re-implements the deterministic vs. reliability-based pricing worked example from [10.1109/TPWRS.2012.2187686], printing an HTML table that contrasts the classic ``n`` horizon against a TLoL-aware calculation.

## Why it matters

* Shows how the core CLI workflows can be complemented with small Python helpers when you just want to explain a concept.
* Runs on any machine with `python3` installed and does not depend on the Rust toolchain, so it is easy to ship in training decks or tutorials.
* Produces HTML output, which you can open in a browser or pipe into the same CLI-based dashboarding conventions you use elsewhere in the repo.

## Run it (fresh checkout)

```bash
python3 test_data/demos/reliability_pricing.py > out/demo/reliability-pricing.html
```

* `out/demo/` is a shared staging area for tutorial artifacts; you can inspect the HTML with `less`, `bat`, or a browser (e.g., `xdg-open out/demo/reliability-pricing.html`).
* The script prints TLoL, investment horizons, and the implied nodal charges for the deterministic vs. reliability-based approach so readers immediately see the numeric gap.

## Extend it for future demos

Add more Python helpers under `test_data/demos/` and a short Markdown summary in this file when you want to teach another theme (time-series resampling, dataset adapters, etc.).
