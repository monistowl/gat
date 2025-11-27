+++
title = "Format Conversion"
description = "Convert between MATPOWER, PSS/E, CIM, pandapower, PowerModels.jl, and Arrow formats using gat convert."
weight = 30
+++

# Format Conversion with Arrow Intermediate

`gat convert format` lets you bridge MATPOWER, PSS/E, CIM, pandapower, and PowerModels.jl case files by writing a temporary Arrow dataset and then running the appropriate exporter. The CLI auto-detects the source when you omit `--from`, and the Arrow intermediate keeps downstream tooling stable even when you move between formats.

## Supported Formats

GAT supports bidirectional conversion between six power system data formats:

| Format | Extension | Description | Best For |
|--------|-----------|-------------|----------|
| **Arrow** | `.arrow/` | Native GAT directory format | Analytics, DuckDB, GAT solvers |
| **MATPOWER** | `.m` | MATLAB power system format | Academic research |
| **PSS/E** | `.raw` | Siemens PSS/E RAW format | Utility planning |
| **CIM** | `.rdf` | IEC 61970 CIM RDF/XML | SCADA interchange |
| **pandapower** | `.json` | Python pandapower format | Python integration |
| **PowerModels.jl** | `.json` | Julia PowerModels format | Julia optimization |

## Usage

```bash
gat convert format \
  --input case14.m \
  --to matpower \
  --output case14_roundtrip.m
```

When `--from` is omitted the command auto-detects the input format; provide it explicitly when an extension is ambiguous (e.g., `.json` could be pandapower or PowerModels.jl). `--force` overwrites existing directories or files.

## Target Formats

- `--to arrow`: keep the temporary Arrow dataset for DuckDB, Polars, or downstream GAT solvers.
- `--to matpower`: export a `.m` file with bus/gen/branch/gencost data for MATPOWER tools.
- `--to psse`: emit a PSS/E RAW file with bus, load, generator, and branch sections.
- `--to cim`: write a minimal CIM RDF/XML graph for round-tripping through the CIM importer.
- `--to pandapower`: serialize bus, load, gen, line, and trafo tables into pandapower JSON.
- `--to powermodels`: export PowerModels.jl JSON format for Julia optimization tools.

## Examples

### Basic Conversion

```bash
# MATPOWER to Arrow
gat convert format --input case14.m --to arrow -o case14.arrow

# Arrow to MATPOWER
gat convert format --input case14.arrow --to matpower -o case14.m

# PSS/E to pandapower
gat convert format --input ieee118.raw --to pandapower -o ieee118.json
```

### PowerModels.jl Workflows

```bash
# Export to PowerModels.jl for Julia optimization
gat convert format --input case30.m --to powermodels -o case30_pm.json

# Import from PowerModels.jl
gat convert format --input network.json --from powermodels --to arrow -o network.arrow

# Convert PGLib benchmarks to MATPOWER
gat convert format --input pglib_opf_case14.json --from powermodels --to matpower -o case14.m
```

### Industrial Format Interchange

```bash
# CIM to Arrow for analysis
gat convert format --input grid_model.rdf --to arrow -o grid.arrow

# Arrow to CIM for SCADA systems
gat convert format --input grid.arrow --to cim -o grid_export.rdf
```

## Data Preservation

Each exporter reloads the Arrow dataset (`load_grid_from_arrow`) before writing the final target, ensuring conversions stay deterministic even when chained.

### Preserved Fields

- **Bus**: ID, name, voltage (kV, p.u.), vmin/vmax limits, area/zone
- **Generator**: Bus, P/Q output, min/max limits, cost model
- **Load**: Bus, active/reactive power demand
- **Branch**: From/to bus, R/X, charging B, tap ratio, ratings, status
- **Cost models**: Polynomial (`c0 + c1*P + c2*P^2`) and piecewise linear

### Cost Model Handling

Generator cost models are preserved across all formats:

- **Polynomial**: Coefficient arrays (GAT stores `[c0, c1, c2]`)
- **Piecewise linear**: (MW, cost) breakpoint pairs

**Note**: PowerModels.jl uses reversed coefficient order `[c2, c1, c0]`. GAT handles this automatically.

## Format-Specific Notes

### PowerModels.jl
- Dictionary-based JSON with `baseMVA` at root level
- Component indices as string keys: `"bus": {"1": {...}, "2": {...}}`
- Compatible with [PGLib-OPF](https://github.com/power-grid-lib/pglib-opf) benchmarks
- Auto-detected by checking for `baseMVA` + `bus` without pandapower markers

### pandapower vs PowerModels.jl

Both use `.json` extension. GAT disambiguates by content:
- **pandapower**: Contains `pandapowerNet` or `_module` keys
- **PowerModels.jl**: Contains `baseMVA` at root without pandapower markers

Use `--from pandapower` or `--from powermodels` to force the parser.

## Metadata

Export metadata from the Arrow manifest (source file, timestamp, and GAT version) is written into
the MATPOWER, PSS/E, CIM, pandapower, and PowerModels outputs for provenance tracking.

## See Also

- [Arrow Schema](arrow-schema.md) - Table-by-table Arrow dataset documentation
- [pandapower Schema](pandapower_schema.md) - Column mappings for pandapower
- [OPF Guide](opf.md) - Running optimal power flow after conversion
