# GAT Examples

Practical examples, workflows, and scripts demonstrating GAT capabilities.

## Quick Start

```bash
# Run the data pipeline on any grid file
./scripts/data_pipeline.sh path/to/network.m

# Run a DERMS analysis workflow
./scripts/derms_workflow.sh path/to/feeder.dss

# Benchmark OPF solvers
./scripts/solver_benchmark.sh
```

## Directory Structure

```
examples/
├── workflows/          # Documentation and tutorials
│   ├── CLI_COOKBOOK.md    # Comprehensive CLI recipes
│   └── CHEATSHEET.md      # Quick reference card
├── scripts/            # Runnable workflow scripts
│   ├── data_pipeline.sh      # End-to-end data processing
│   ├── derms_workflow.sh     # DER management analysis
│   ├── solver_benchmark.sh   # OPF solver comparison
│   ├── state_estimation.sh   # WLS state estimation
│   ├── reliability_study.sh  # Resource adequacy (LOLE/EUE)
│   ├── overnight_benchmark.sh # Nightly validation suite
│   └── analyze_benchmarks.py  # Result analysis tools
├── scenarios/          # Scenario definition files
│   └── rts_nminus1.yaml      # N-1 contingency scenarios
└── experiments/        # Research documentation
    ├── AC_OPF_CENT.md       # AC-OPF benchmarking plan
    ├── PF_CONTINGENCIES.md  # Power flow validation
    └── PAPERS.md            # Academic references
```

## Workflow Scripts

### `data_pipeline.sh` - End-to-End Data Processing

Complete pipeline from raw grid data to ML-ready features:

```bash
./scripts/data_pipeline.sh network.m ./output

# Pipeline stages:
# 1. Import (auto-detect format)
# 2. Validate and clean
# 3. Run analyses (PF, OPF, N-1)
# 4. Extract features (bus, branch, gen)
# 5. Export to multiple formats
```

### `derms_workflow.sh` - DER Management

Distributed energy resource analysis workflow:

```bash
./scripts/derms_workflow.sh feeder.dss ./derms_results

# Analysis includes:
# 1. Import distribution model
# 2. Calculate DER flexibility envelopes
# 3. Generate optimal schedules
# 4. Run stress tests
# 5. Hosting capacity analysis
```

### `solver_benchmark.sh` - OPF Solver Comparison

Benchmark different OPF solution methods:

```bash
./scripts/solver_benchmark.sh ./pglib ./benchmark_results

# Methods tested:
# - DC-OPF (linear)
# - SOCP relaxation
# - AC-OPF (fast-decoupled)
# - AC-OPF NLP (L-BFGS, IPOPT)
```

### `state_estimation.sh` - WLS State Estimation

Demonstrates state estimation with synthetic measurements:

```bash
./scripts/state_estimation.sh grid.arrow ./se_results

# Workflow:
# 1. Generate "true" state via AC power flow
# 2. Create measurements with noise
# 3. Run WLS state estimation
# 4. Compare estimated vs true state
```

### `reliability_study.sh` - Resource Adequacy

LOLE/EUE calculation and ELCC estimation:

```bash
./scripts/reliability_study.sh grid.arrow load_forecast.csv ./ra_results

# Analysis includes:
# 1. Generation fleet analysis
# 2. Load forecast preparation
# 3. Monte Carlo reliability simulation
# 4. ELCC estimation for renewables
```

## Documentation

### CLI Cookbook (`workflows/CLI_COOKBOOK.md`)

Comprehensive recipes including:
- Import and format conversion
- Power flow analysis patterns
- OPF method comparison
- Contingency screening
- Data pipeline recipes
- Batch processing
- Analytics workflows

### Cheatsheet (`workflows/CHEATSHEET.md`)

Quick reference for:
- Common commands
- Output format options
- Useful patterns
- Environment variables

## Requirements

Most scripts require:
- GAT CLI (`gat`) in PATH
- `jq` for JSON processing
- `bash` 4.0+

Optional:
- `python3` for analysis scripts
- `parallel` for batch processing
- IPOPT for NLP benchmarks

## Environment Variables

```bash
export GAT_THREADS=8          # Parallel processing
export GAT_LOG_LEVEL=info     # Logging verbosity
export GAT_OUTPUT_DIR=./out   # Default output directory
```

## Contributing

To add new examples:

1. Scripts go in `scripts/` with `.sh` extension
2. Documentation goes in `workflows/` as `.md`
3. Scenarios go in `scenarios/` as `.yaml`
4. Include header comments explaining usage
5. Handle errors gracefully with fallbacks
6. Generate summary reports where appropriate

## Related Documentation

- [CLI Reference](../docs/man/gat.1)
- [OPF Guide](../website/content/guide/opf.md)
- [Quick Start](../website/content/guide/quickstart.md)
- [API Migration Guide](../docs/guide/cli-migration.md)
