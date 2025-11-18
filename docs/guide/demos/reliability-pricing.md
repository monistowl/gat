# Reliability-Based Pricing Demo

This hands-on example is the first entry in the training demos series. It mirrors the quick conceptual pitch from `README.md` but adds runnable detail for a fresh checkout.

## What it teaches

* How "Tolerable Loss of Load" (TLoL) can act as virtual capacity and defer investment.
* How to compare deterministic (old) horizons to the reliability-based approach from the cited paper.
* How to wrap a self-contained Python script in the GAT workspace so learners see a full example without yet compiling the CLI.

## Files involved

* `test_data/demos/reliability_pricing.py` – the demo script. It contains constants from the paper (asset cost, MTTR, fail rate, EENS limit) and prints an HTML table comparing horizons and nodal charges.
* `test_data/demo.md` – the quick-start reference for this demo (useful to share in tutorials or README updates).
* `docs/guide/demos/README.md` – the training index describing how future demos should be structured.

## Running it on a fresh install

```bash
python3 test_data/demos/reliability_pricing.py > out/demo/reliability-pricing.html
```

1. `out/demo/` is a demo staging folder; create it with `mkdir -p out/demo` on the first run.
2. The script currently requires nothing beyond the standard library—no `cargo` or `gat` binaries—so it works on any machine with Python 3.
3. After running, open the HTML (`xdg-open out/demo/reliability-pricing.html` on Linux, or fetch it into a browser) to see the rendered table.

Because the output is HTML, you can also pipe it straight into `gat viz` or other components if you later build a training page that hosts the demo results.

## Teaching tips

* Use this demo to explain why reading a paper is easier when you can reproduce its key table with a few commands.
* Point learners to `test_data/demo.md` for the narrative, then walk through the script (`docs/guide/demos/reliability-pricing.md` can reference code sections).
* When adding the next demo, follow the same pattern: put the runner under `test_data/demos/`, document it here, and update `docs/guide/demos/README.md` with a link.

---

This README-style note keeps the training material consistent with `README.md`'s tone—it's CLI-first, linear, and focused on immediate payoff.
