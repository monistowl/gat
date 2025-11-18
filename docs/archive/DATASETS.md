# Dataset adapters

These helpers grab publicly available testbeds and demand/weather agents built into the roadmap (#14).

## RTS-GMLC

```
gat dataset rts-gmlc fetch --out data/rts-gmlc
```

Copies the staged MATPOWER file (`grid.matpower`) and telemetry CSV (`timeseries.csv`) into the output directory. Use `gat import matpower` + `gat ts import dsgrid` afterwards to hydrate the Arrow + timeseries pipeline.

## HIREN test cases

```
gat dataset hiren list
gat dataset hiren fetch case_big --out data/hiren
```

List available cases (curated 9–240 bus) and copy a MATPOWER file into the destination directory.

## dsgrid import

```
gat dataset dsgrid --out data/dsgrid/demand.parquet
```

Copies the bundled dsgrid Parquet into the requested path (a stub for a full HTTP fetch).

## Sup3r weather

```
gat dataset sup3rcc-fetch --out data/sup3r/weather.parquet
gat dataset sup3rcc-sample --grid grid.arrow --out data/sup3r/sample.parquet
```

Fetch offline weather data and sample it for a grid (currently just copies the fixture).

## PRAS adequacy

```
gat dataset pras --path test_data/datasets/pras --out data/pras/pras.csv
```

Copies normalized LOLE/EUE outputs for Gat scenarios.
