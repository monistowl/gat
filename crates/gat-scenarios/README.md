# gat-scenarios — Scenario Definition & Materialization

YAML-based scenario specification system for defining what-if cases, generating batches of scenarios, and preparing them for parallel execution.

## Quick Overview

**Define Scenarios Using YAML:**
- Template grids with scaling factors
- Time-varying parameters
- Contingency definitions
- Scenario combinations

**Materialize to Executables:**
- Expand templates into concrete scenarios
- Generate manifests for batch execution
- Validate for feasibility before solving

## Example Scenario Definition

```yaml
# scenarios/rts_nminus1.yaml
base_grid: test_data/matpower/rts96.arrow

scenarios:
  base:
    description: "Base case"
    grid_file: base.arrow
    load_scaling: 1.0
    gen_scaling: 1.0

  peak:
    description: "Peak demand"
    grid_file: base.arrow
    load_scaling: 1.2
    gen_scaling: 1.0

  n_minus_1:
    description: "Single contingencies"
    base: base
    outage_list: test_data/contingencies.yaml
```

## Core APIs

```rust
// Load and validate scenario spec
let spec = ScenarioSpec::from_yaml(path)?;
spec.validate(&grid)?;

// Materialize scenarios
let materialized = spec.materialize(
    grid_path,
    output_dir,
    MatOptions::default(),
)?;

// Generate manifest for batch execution
let manifest = materialized.to_batch_manifest()?;
```

## Features

- **Template Variables** — Define parameterized scenarios
- **Contingency Enumeration** — Automatically generate N-1, N-2 contingencies
- **Scaling Factors** — Adjust loads, generation, or branch parameters
- **Validation** — Check scenarios are feasible before execution
- **Manifest Generation** — Output ready for `gat batch` commands

## Output Structure

```
runs/scenarios/rts_nminus1/
├── scenario_manifest.json      # Ready for gat batch
├── base/
│   ├── grid.arrow              # Modified network model
│   └── metadata.json           # Scenario details
├── peak/
│   ├── grid.arrow
│   └── metadata.json
└── n_minus_1/
    ├── scenario_0001/
    │   ├── grid.arrow
    │   └── metadata.json
    ├── scenario_0002/
    ...
```

## Workflow Integration

```bash
# 1. Define scenarios
gat scenarios validate --spec scenarios/rts_nminus1.yaml

# 2. Materialize into executable form
gat scenarios materialize \
  --spec scenarios/rts_nminus1.yaml \
  --grid-file grid.arrow \
  --out-dir runs/scenarios

# 3. Execute as batch
gat batch pf \
  --manifest runs/scenarios/scenario_manifest.json \
  --max-jobs 8 \
  --out runs/batch
```

## Configuration

```yaml
# Global options
parallel_jobs: 8
timeout_per_scenario: 60
output_format: parquet

# Scenario template syntax
templates:
  contingency:
    pattern: "test_data/contingencies/*.yaml"
    apply_to: "base"
```

## Testing

```bash
cargo test -p gat-scenarios
```

## Related Crates

- **gat-core** — Grid models being templated
- **gat-io** — Manifest and scenario I/O
- **gat-batch** — Executes materialized scenarios

## See Also

- [GAT Main README](../../README.md)
- `docs/guide/cli-architecture.md` — Scenario pipeline
- [gat-cli README](../gat-cli/README.md) — `gat scenarios` commands
