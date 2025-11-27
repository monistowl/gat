+++
title = "Network Inspection"
description = "Deep-dive network inspection and diagnostic tools"
weight = 25
+++

# Network Inspection

The `gat inspect` command provides diagnostic tools for examining network structure, validating data quality, and debugging power system models before running analysis.

## Overview

`gat inspect` helps you:
- **Validate imports** — Verify network data was imported correctly
- **Debug issues** — Find missing generators, disconnected buses, or data problems
- **Explore models** — Understand network topology before running analysis
- **Script automation** — Export network data as JSON for custom workflows

## Commands

### `gat inspect summary`

Show high-level network statistics:

```bash
gat inspect summary grid.arrow
```

**Output includes:**
- Bus count and voltage level distribution
- Branch count by type (lines, transformers)
- Generator count and total capacity (MW)
- Load count and total demand (MW)
- Network connectivity status

**Example output:**
```
Network Summary
───────────────
Buses:       14
Branches:    20 (17 lines, 3 transformers)
Generators:   5 (total capacity: 772 MW)
Loads:       11 (total demand: 259 MW)
Status:      Connected (1 island)
```

### `gat inspect generators`

List all generators with their bus assignments and limits:

```bash
# List all generators
gat inspect generators grid.arrow

# Filter by bus
gat inspect generators grid.arrow --bus 1

# Output as JSON
gat inspect generators grid.arrow --format json
```

**Options:**
- `--bus <BUS>` — Filter generators by bus ID
- `--format <FORMAT>` — Output format: `table` (default) or `json`

**Example output:**
```
Generators
──────────
ID   Bus  Pmin (MW)  Pmax (MW)  Qmin (MVAr)  Qmax (MVAr)  Status
───  ───  ─────────  ─────────  ──────────   ──────────   ──────
G1   1    0.0        332.4      -10.0        10.0         Online
G2   2    0.0        140.0      -40.0        50.0         Online
G3   3    0.0        100.0      0.0          40.0         Online
```

### `gat inspect branches`

List all branches with their endpoints and parameters:

```bash
gat inspect branches grid.arrow
```

**Output includes:**
- Branch ID and type (line/transformer)
- From/to bus IDs
- Impedance parameters (R, X, B)
- Rating and flow limits
- Tap ratio (for transformers)

### `gat inspect power-balance`

Show power balance analysis comparing generation capacity to load:

```bash
gat inspect power-balance grid.arrow
```

**Example output:**
```
Power Balance Analysis
──────────────────────
Total Generation Capacity:  772.4 MW
Total Load Demand:          259.0 MW
Reserve Margin:             198% (513.4 MW)

Status: Adequate capacity for base case
```

This is useful for quick sanity checks before running OPF or reliability analysis.

### `gat inspect json`

Export network data as JSON for scripting and automation:

```bash
# Compact JSON
gat inspect json grid.arrow

# Pretty-printed JSON
gat inspect json grid.arrow --pretty

# Pipe to jq for processing
gat inspect json grid.arrow | jq '.buses | length'
```

**Options:**
- `--pretty` — Pretty-print the JSON output with indentation

**Use cases:**
- Integration with Python/Julia scripts
- Custom analysis pipelines
- Data validation workflows
- Documentation generation

## Diagnostic Workflows

### Validate an Import

After importing a MATPOWER or PSS/E file, verify the data:

```bash
# Import the file
gat import matpower --m case118.m -o case118.arrow

# Check the summary
gat inspect summary case118.arrow

# Verify generator data
gat inspect generators case118.arrow

# Check power balance
gat inspect power-balance case118.arrow
```

### Debug Convergence Issues

When power flow doesn't converge, inspect the network:

```bash
# Check for adequate generation
gat inspect power-balance grid.arrow

# Look for generators with unusual limits
gat inspect generators grid.arrow --format json | jq '.[] | select(.pmax < .pmin)'

# Export full data for detailed analysis
gat inspect json grid.arrow --pretty > grid_debug.json
```

### Pre-flight Check Before Analysis

Before running OPF or contingency analysis:

```bash
# Quick validation
gat inspect summary grid.arrow
gat inspect power-balance grid.arrow

# If everything looks good, proceed
gat opf dc grid.arrow --out results.parquet
```

## Related Commands

- [Power Flow](/guide/pf/) — Run power flow analysis
- [OPF](/guide/opf/) — Optimal power flow
- [Convert](/guide/convert/) — Convert between formats
- [Import](/guide/datasets/) — Import network data
