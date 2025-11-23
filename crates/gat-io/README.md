# gat-io — Data I/O & Schemas

Data format support, schema definitions, and I/O utilities for grid models, results, and time-series data.

## Quick Overview

**Formats:**
- Arrow/IPC — In-memory columnar format (fast, zero-copy)
- Parquet — Compressed columnar storage (persistent, cloud-friendly)
- CSV — Human-readable tabular data
- JSON — Metadata and configuration

**Grid Formats:**
- MATPOWER (`.raw`, `.m`)
- PSS/E (`.raw`, `.dyr`)
- CIM/XML (Common Information Model)

## Core APIs

```rust
// Load grid from various formats
let grid = load_grid_from_arrow(path)?;
let grid = load_grid_from_matpower(path)?;

// Save results in common formats
save_parquet(df, output_path)?;
save_csv(df, output_path)?;
save_arrow(df, output_path)?;
```

## Schemas

Pre-defined Arrow schemas for consistency:

- `GridSchema` — Bus, branch, generator, load definitions
- `PFResultSchema` — Voltage magnitudes, angles, flows
- `OPFResultSchema` — Dispatch, costs, constraint violations
- `TimeSeriesSchema` — Timestamp, value, metadata
- `ManifestSchema` — Run metadata and traceability

See `docs/schemas/` for JSON schema definitions.

## Features

- Automatic format detection from file extension
- Efficient Parquet compression and snappy encoding
- Time-series resampling and aggregation
- Consistent null/missing-data handling
- Metadata preservation through run.json files

## Testing

```bash
cargo test -p gat-io
```

## Related Crates

- **gat-core** — Consumes schemas for validation
- **gat-cli** — Uses gat-io for command I/O
- **gat-batch** — Reads/writes result manifests

## See Also

- [GAT Main README](../../README.md)
- `docs/guide/cli-architecture.md` — I/O pipeline
- `docs/schemas/` — Detailed schema definitions
