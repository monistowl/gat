# Format Conversion with Arrow Intermediary

`gat convert format` wraps the importer/exporter pipeline so you can interoperate between MATPOWER, PSS/E, CIM, pandapower, and PowerModels.jl datasets without touching serialized Arrow tables manually. The command auto-detects the source format (unless you override it with `--from`), writes a temporary Arrow directory, then either hands that dataset to downstream tools or emits the requested target format through the new exporters.

## Supported Formats

GAT supports bidirectional conversion between six power system data formats:

| Format | Extension | Description | Typical Use Case |
|--------|-----------|-------------|------------------|
| **Arrow** | `.arrow/` | Native GAT directory format | High-performance analytics, DuckDB queries |
| **MATPOWER** | `.m` | MATLAB power system format | Academic research, MATLAB toolbox integration |
| **PSS/E** | `.raw` | Siemens PSS/E RAW format | Utility planning studies, commercial tools |
| **CIM** | `.rdf`, `.xml` | IEC 61970 CIM RDF/XML | Smart grid, SCADA system interchange |
| **pandapower** | `.json` | Python pandapower format | Python ecosystem integration |
| **PowerModels.jl** | `.json` | Julia PowerModels format | Julia optimization, PGLib benchmarks |

### Format Detection

GAT auto-detects input formats using file extensions and content sniffing:

- **Unambiguous extensions**: `.m` → MATPOWER, `.raw` → PSS/E
- **JSON disambiguation**: Checks for `baseMVA` (PowerModels), `pandapowerNet` (pandapower)
- **XML disambiguation**: Checks for `cim:` namespace (CIM RDF)

Use `--from` to force a specific parser when auto-detection is ambiguous.

## Examples

### Basic Conversion

```bash
# MATPOWER to Arrow (for analytics)
gat convert format --input case14.m --to arrow -o case14.arrow

# Arrow to MATPOWER (roundtrip)
gat convert format --input case14.arrow --to matpower -o case14_roundtrip.m

# PSS/E to pandapower
gat convert format --input ieee118.raw --to pandapower -o ieee118.json
```

### PowerModels.jl Conversion

```bash
# Import PGLib benchmark and export to PowerModels.jl
gat convert format --input pglib_opf_case14_ieee.m --to powermodels -o case14_pm.json

# PowerModels.jl to MATPOWER for MATLAB analysis
gat convert format --input network.json --from powermodels --to matpower -o network.m

# Chain: MATPOWER → Arrow → PowerModels
gat convert format --input case30.m --to arrow -o case30.arrow
gat convert format --input case30.arrow --to powermodels -o case30_pm.json
```

### CIM and Industrial Formats

```bash
# CIM RDF/XML to Arrow
gat convert format --input grid_model.rdf --to arrow -o grid.arrow

# Arrow to CIM (for SCADA interchange)
gat convert format --input grid.arrow --to cim -o grid_export.rdf

# PSS/E to CIM
gat convert format --input system.raw --to cim -o system.rdf
```

### Force Overwrite

```bash
# Regenerate existing output
gat convert format --input case14.m --to arrow -o case14.arrow --force
```

## Export Targets

The `--to` flag selects a format-specific exporter that consumes the intermediate Arrow dataset:

| Target | Output | Notes |
|--------|--------|-------|
| `--to arrow` | Directory with Parquet + manifest | Native format for GAT solvers |
| `--to matpower` | `.m` file | Buses, generators, branches, gencost tables |
| `--to psse` | `.raw` file | Bus, generator, load, branch sections |
| `--to cim` | `.rdf` file | BusbarSection, ACLineSegment, Load, SynchronousMachine |
| `--to pandapower` | `.json` file | bus, load, gen, line, trafo tables |
| `--to powermodels` | `.json` file | Dictionary-based with bus/gen/branch/load |

## How It Works

1. **Detect Source Format** – Identify file type from extension and content. `--from` forces a parser when the extension is ambiguous (e.g., `.json` for pandapower vs PowerModels).

2. **Import to Arrow** – Parse the source using `gat-io`'s unified importer (`Format::parse`) and write to a temporary Arrow directory via `ArrowDirectoryWriter`.

3. **Export to Target** – If the target is Arrow, rename/copy the temp directory. Otherwise:
   - Load the Arrow dataset with `load_grid_from_arrow`
   - Run the format-specific exporter (matpower, psse, cim, pandapower, powermodels)

4. **Metadata Propagation** – Export metadata from the Arrow manifest (source filename, creation timestamp, GAT version) is written into the exported file for traceability.

## Data Preservation

### Preserved During Conversion

- **Bus data**: ID, name, voltage (kV, p.u.), limits (vmin/vmax), area/zone
- **Generator data**: Bus, P/Q output, limits, cost model (polynomial/piecewise)
- **Load data**: Bus, active/reactive power
- **Branch data**: From/to bus, R/X, charging B, tap ratio, ratings, status
- **Transformer data**: Tap ratio, phase shift, thermal limits

### Cost Model Handling

Generator cost models are preserved through conversion:

- **Polynomial costs**: `c0 + c1*P + c2*P²` stored as coefficient arrays
- **Piecewise linear**: (MW, cost) breakpoints for bid curves

**Note**: PowerModels.jl uses reversed coefficient order `[c2, c1, c0]`. GAT handles this automatically during import/export.

## Format-Specific Notes

### MATPOWER
- Supports versions 1-2.1 with all field variations
- `gencost` table preserved with model type (1=piecewise, 2=polynomial)
- Output uses consistent column ordering for diff-friendly roundtrips

### PSS/E RAW
- Supports versions 29-35
- Fixed-width format with "BUS DATA FOLLOWS" section markers
- Partial transformer/phase-shifter support

### CIM RDF/XML
- IEC 61970 CIM standard with `cim:` namespace
- Most verbose format; larger files than others
- Best for utility SCADA system interchange

### pandapower
- Native JSON from Python pandapower library
- Includes `_module` and `_class` metadata
- Round-trips cleanly with Python code

### PowerModels.jl
- Dictionary-based JSON with `baseMVA` at root
- Component indices as string keys: `"bus": {"1": {...}, "2": {...}}`
- Coefficient ordering: highest degree first for polynomials
- Compatible with PGLib benchmark cases

## Error Handling

Conversion reports diagnostics without aborting on warnings:

```bash
$ gat convert format --input broken.m --to arrow
⚠ Warning: Bus 5: voltage out of typical range (2.5 p.u.)
⚠ Warning: Branch 12: very high reactance (10.0 p.u.)
✓ Converted to Arrow format: broken.arrow
```

Use `--strict` (if available) to fail on warnings, or check the exit code for errors.

## Integration with GAT Solvers

After conversion to Arrow, use GAT's analysis tools:

```bash
# Convert and run power flow
gat convert format --input case14.m --to arrow -o case14.arrow
gat pf dc --dataset case14.arrow

# Convert and run optimal power flow
gat convert format --input pglib_case118.m --to arrow -o case118.arrow
gat opf dc --dataset case118.arrow
```

## See Also

- [Arrow Schema Reference](arrow-schema.md) – Table-by-table schema documentation
- [pandapower Schema Mapping](pandapower_schema.md) – Column mappings for pandapower conversion
- [OPF Guide](opf.md) – Running optimal power flow after conversion
