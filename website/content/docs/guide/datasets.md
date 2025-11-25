+++
title = "Dataset Management"
description = "Dataset adapters"
weight = 31
+++

# Dataset adapters

These helpers hydrate the bundled fixtures plus publicly available power-system testbeds so the CLI has ready-to-run inputs for PF, OPF, SE, and TS commands.

## RTS-GMLC

```bash
gat dataset rts-gmlc fetch --out data/rts-gmlc
```

Copies the staged MATPOWER network (`grid.matpower`) and telemetry CSV (`timeseries.csv`) into the destination directory. Follow up with `gat import matpower` and `gat ts import dsgrid` to build the Arrow/Parquet pipeline that every solver command consumes.

## HIREN test cases

```bash
gat dataset hiren list
gat dataset hiren fetch case_big --out data/hiren
```

Lists the curated 9–240 bus cases and copies the selected MATPOWER file into the requested output. Use the fetched case as the input for `gat import matpower` when you need a realistic testbed.

## dsgrid fixtures

```bash
gat dataset dsgrid --out data/dsgrid/demand.parquet
```

Writes the built-in dsgrid Parquet snapshot to the supplied path. It is a stand-in for a full HTTP fetch and lets you exercise telemetry joins without anything external.

## Sup3r weather

```bash
gat dataset sup3rcc-fetch --out data/sup3r/weather.parquet
```

```bash
gat dataset sup3rcc-sample --grid grid.arrow --out data/sup3r/sample.parquet
```

`sup3rcc-fetch` copies the offline weather PoP dataset, while `sup3rcc-sample` ties resources to buses in an Arrow grid. The sampling command is helpful when coupling weather to OPF or contingency scenarios.

## PRAS adequacy

```bash
gat dataset pras --path test_data/datasets/pras --out data/pras/pras.csv
```

Normalizes the Probabilistic Resource Adequacy Suite (PRAS) outputs (LOLE/EUE) for Gat scenarios. Use it to load region/season/hour forecasts before running adequacy or SE experiments.

## Benchmark datasets

These datasets are used by `gat benchmark` commands to validate solver accuracy against known reference solutions.

### PFΔ (Power Flow Delta)

Power flow perturbation dataset with reference bus voltages and angles for validation.

```bash
# Run PF benchmark against PFΔ test cases
gat benchmark pfdelta --pfdelta-root data/pfdelta --max-cases 100 -o pfdelta_results.csv
```

Output CSV contains: case_name, contingency_type, converged, max_vm_error, max_va_error_deg, solve_time_ms.

### PGLib-OPF

Standard IEEE/ARPA-E MATPOWER test cases from the PGLib-OPF repository. Includes baseline objective values for OPF validation.

```bash
# Run OPF benchmark against PGLib cases
gat benchmark pglib --pglib-dir data/pglib --baseline baseline.csv -o pglib_results.csv
```

Output CSV contains: case_name, converged, objective_value, baseline_objective, objective_gap_rel, solve_time_ms.

### OPFData (GridOpt)

Large-scale AC-OPF dataset with 300k+ solved instances per grid. Supports load perturbations (FullTop) and topology perturbations (N-1 line/gen/transformer outages). Uses GNN-format JSON.

Reference: [arxiv.org/abs/2406.07234](https://arxiv.org/abs/2406.07234)

```bash
# Run benchmark on OPFData samples
gat benchmark opfdata --opfdata-dir data/opfdata/case118/group_0 --max-cases 1000 -o opfdata_results.csv
```

Output CSV contains: sample_id, file_name, converged, objective_value, baseline_objective, objective_gap_rel, num_buses, num_branches, solve_time_ms.
