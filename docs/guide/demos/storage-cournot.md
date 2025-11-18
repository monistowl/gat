# Cournot Storage Oligopoly Demo

This shell script lives in `test_data/demos/storage_cournot.sh`. It reproduces the stylized oligopoly example from [arXiv:2509.26568](https://arxiv.org/abs/2509.26568) by looping over 1..N symmetric storage owners and measuring the impact on price, EENS, and welfare under high renewable penetration.

## What it teaches

* How to wrap GAT modifiers (`gat modify`) and scenarios (`gat ts sample-gaussian`) to stress-test reliability.
* How to broadcast parallel DC OPFs across many RES scenarios and aggregate outcomes yourself.
* How to use `gat storage create`/`gat storage merge` to build multi-unit portfolios by hand.
* That storage ownership structure matters for market price, consumer surplus, and reliability (EENS). The script prints a CSV you can plot from the command line.

## Prerequisites (fresh checkout)

1. Install the Rust toolchain + `gat` via `cargo install --path .` (or use `cargo xtask doc all` until the binaries exist; this demo assumes `gat` on `$PATH`).
2. Download or stage the RTS-GMLC fixtures under `data/rts-gmlc/` (MATPOWER + Parquet). The script copies `network.parquet`, `gen.parquet`, `load.parquet`, and `res.parquet`.
3. Make sure GNU `bc`, `gnuplot`, and `awk` are available; the demo uses them to reduce outputs and suggest a quick plot.

## Running the demo

```bash
bash test_data/demos/storage_cournot.sh
```

* The script runs inside `out/demos/cournot/` so intermediate files stay under `out/` (create it with `mkdir -p out/demos` first if needed).
* The final CSV `cournot_results.csv` lists `N_firms`, price, EENS, consumer surplus, storage profits, and welfare; the `echo` at the end shows a quick `gnuplot` command you can paste to visualize price vs. EENS.
* Since the script is heavy, you may want to reduce `N_SCENARIOS` or the renewable scaling factor when experimenting.

## Teaching notes

* Walk learners through `gat modify scale-gen` to bump renewables (the scenario uses 120% of peak load to trigger shortages).
* Show how the `gat ts sample-gaussian` command produces the `scenarios/` folder used by the DC OPF loop.
* Emphasize that the storage creation loop uses identical units for simplicity, but you could insert heterogeneity or locations via CLI flags.
* Finally, link the CSV output back to the reliability concepts: more storage firms usually lower prices/EENS but affect profits.

---

Add future Cournot-style demos by copying this structure: a script under `test_data/demos/` plus a Markdown walkthrough here that explains dependencies, run commands, and learning points.
