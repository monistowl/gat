+++
title = "Format conversion with Arrow intermediate"
description = "Use gat convert format to move between MATPOWER, PSS/E, CIM, PandaPower, and Arrow via the normalized Arrow schema."
weight = 30
+++

# Format conversion with Arrow intermediate

`gat convert format` lets you bridge MATPOWER, PSS/E, CIM, and pandapower case files by writing a temporary Arrow dataset and then running the appropriate exporter. The CLI auto-detects the source when you omit `--from`, and the Arrow intermediate keeps downstream tooling stable even when you move between formats.

## Usage

```bash
gat convert format \
  --input case14.m \
  --to matpower \
  --output case14_roundtrip.m
```

When `--from` is omitted the command auto-detects the input format; provide it explicitly when an extension is ambiguous (e.g., `.json`). `--force` overwrites existing directories or files.

## Target formats

- `--to arrow`: keep the temporary Arrow dataset for DuckDB, Polars, or downstream GAT solvers.
- `--to matpower`: export a `.m` file with bus/gen/branch/gencost data so you can re-use MATPOWER tools.
- `--to psse`: emit a PSS/E RAW file with bus, load, generator, and branch sections inspired by the legacy layout.
- `--to cim`: write a minimal CIM RDF/XML graph (BusbarSection, ACLineSegment, Load, SynchronousMachine) that the importer replays.
- `--to pandapower`: serialize bus, load, gen, line, and trafo tables back into pandapower-compatible JSON.

Each exporter reloads the Arrow dataset (`load_grid_from_arrow`) before writing the final target so conversions stay deterministic even when chained.

Export metadata from the Arrow manifest (source file, timestamp, and GAT version) is written into
the MATPOWER, PSS/E, CIM, and pandapower outputs for provenance.
