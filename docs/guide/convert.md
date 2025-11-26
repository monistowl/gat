# Format Conversion with Arrow Intermediary

`gat convert format` wraps the importer/exporter pipeline so you can interoperate between MATPOWER, PSS/E, CIM, and pandapower datasets without touching serialized arrow tables manually. The command auto-detects the source format (unless you override it with `--from`), writes a temporary Arrow directory, then either hands that dataset to downstream tools or emits the requested target format through the new exporters.

## Examples

- Convert MATPOWER → Arrow directory:
  ```bash
  gat convert format --input case14.m --to arrow -o case14.arrow
  ```
- Emit PSS/E RAW from an Arrow dataset:
  ```bash
  gat convert format --input grid.arrow --to psse -o grid.raw
  ```
- Round-trip Pandapower → Arrow → Pandapower to normalize column ordering:
  ```bash
  gat convert format --input net.json --to arrow
  gat convert format --input net.arrow --to pandapower -o net_roundtrip.json
  ```
- Force an overwrite when re-generating Arrow data:
  ```bash
  gat convert format --input ieee14.raw --to arrow -o ieee14.arrow --force
  ```

## Export Targets

The `--to` flag now selects a format-specific exporter that consumes the temporary Arrow dataset:

- `--to matpower` writes a `.m` file with buses, generators, loads, branches, and gencost tables.
- `--to psse` emits a PSS/E RAW file with bus, generator, load, and branch sections modeled after the legacy "BUS DATA FOLLOWS" layout.
- `--to cim` writes a minimal RDF/XML graph referencing `BusbarSection`, `ACLineSegment`, `Load`, and `SynchronousMachine` elements for round-tripping through the CIM importer.
- `--to pandapower` creates the subset of the pandapower JSON schema that our importer understands, including the `bus`, `load`, `gen`, `line`, and `trafo` tables.

Each exporter is wired into `gat convert format`, so the same commandline that already produced Arrow directories can now drive format-to-format conversion via Arrow as the canonical intermediate.

## How It Works

1. Detect the source format (MATPOWER, PSS/E, CIM, pandapower, or Arrow). `--from` forces a parser when the extension is ambiguous.
2. Import to a temporary Arrow directory using `gat-io`’s unified importer (`Format::parse`) and `ArrowDirectoryWriter`.
3. If the target is Arrow, rename or copy the temp directory to `-o`. Otherwise:
   1. Load the Arrow dataset with `load_grid_from_arrow`.
   2. Run the format-specific exporter for `matpower`, `psse`, `cim`, or `pandapower`.
4. Print the success message with the chosen format label so automation scripts can check the result.
5. Metadata from the Arrow manifest (source filename, creation timestamp, GAT version) is
   propagated into the exported MATPOWER/PSS/E/CIM/pandapower dataset for traceability.
