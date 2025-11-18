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
