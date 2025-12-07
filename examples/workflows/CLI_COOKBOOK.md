# GAT CLI Cookbook

Practical recipes for common power systems workflows using GAT 0.5.7+.

## Table of Contents

1. [Quick Start Patterns](#quick-start-patterns)
2. [Power Flow Workflows](#power-flow-workflows)
3. [OPF Analysis](#opf-analysis)
4. [Contingency Screening](#contingency-screening)
5. [Data Pipeline Recipes](#data-pipeline-recipes)
6. [Batch Processing](#batch-processing)
7. [Analytics & Sensitivities](#analytics--sensitivities)

---

## Quick Start Patterns

### Import → Validate → Analyze (One-Liner)

```bash
# Import MATPOWER case, validate, run DC power flow
gat import case118.m -o grid.arrow && \
gat validate grid.arrow && \
gat pf dc grid.arrow --format table
```

### Format Auto-Detection

GAT auto-detects input format from file extension:

```bash
gat import network.m        # MATPOWER
gat import network.raw      # PSS/E RAW
gat import network.json     # pandapower JSON
gat import network.xml      # CIM-XML
```

### Quick Network Summary

```bash
# Human-readable summary
gat inspect summary grid.arrow

# JSON for scripting
gat inspect summary grid.arrow --format json | jq '.buses, .branches'
```

---

## Power Flow Workflows

### DC Power Flow (Fast, Linear)

```bash
# Basic DC power flow
gat pf dc grid.arrow

# Output to JSON for further processing
gat pf dc grid.arrow -o results.json

# Pipe directly to jq for filtering
gat pf dc grid.arrow -o - | jq '.branches[] | select(.loading_pct > 80)'
```

### AC Power Flow (Newton-Raphson)

```bash
# Standard AC power flow
gat pf ac grid.arrow

# Show iteration convergence
gat pf ac grid.arrow --show-iterations

# Override slack bus selection
gat pf ac grid.arrow --slack-bus 1

# Use Faer linear algebra backend (faster for large systems)
gat pf ac grid.arrow --linear-solver faer
```

### Power Flow with Custom Base MVA

For non-standard per-unit systems:

```bash
# 50 MVA base (common in distribution)
gat pf dc grid.arrow --base-mva 50

# 1000 MVA base (bulk transmission)
gat pf dc grid.arrow --base-mva 1000
```

### Compare AC vs DC Results

```bash
# Run both and compare
gat pf dc grid.arrow -o dc_results.json
gat pf ac grid.arrow -o ac_results.json

# Quick comparison with jq
echo "DC vs AC branch flows:"
paste <(jq -r '.branches[].flow_mw' dc_results.json) \
      <(jq -r '.branches[].flow_mw' ac_results.json) | \
  awk '{diff=$2-$1; printf "%.2f MW difference\n", diff}'
```

---

## OPF Analysis

### DC-OPF (Economic Dispatch)

```bash
# Basic DC-OPF
gat opf dc grid.arrow

# With generator costs from CSV
gat opf dc grid.arrow --costs gen_costs.csv

# With branch limits from CSV
gat opf dc grid.arrow --limits branch_limits.csv

# Both cost and limits
gat opf dc grid.arrow --costs costs.csv --limits limits.csv -o dispatch.json
```

### AC-OPF Methods Comparison

```bash
# Fast-decoupled approximation (quick)
gat opf ac grid.arrow

# SOCP relaxation (tighter bounds)
gat opf dc grid.arrow --method socp

# Enhanced SOCP with QC envelopes
gat opf dc grid.arrow --method socp --enhanced

# Full nonlinear AC-OPF with L-BFGS
gat opf ac-nlp grid.arrow

# Full AC-OPF with IPOPT (requires native solver)
gat opf ac-nlp grid.arrow --nlp-solver ipopt
```

### Warm-Starting AC-OPF

```bash
# Use SOCP solution as warm start for NLP
gat opf ac-nlp grid.arrow --warm-start socp --show-iterations
```

### Swiss Army Knife: opf run

Direct MATPOWER file processing without pre-import:

```bash
# Run OPF directly on .m file
gat opf run case118.m

# Specify method
gat opf run case300.m --method socp

# With verbose output
gat opf run pglib_opf_case500_goc.m --show-iterations -o results.json
```

---

## Contingency Screening

### N-1 Analysis

```bash
# Basic N-1 screening (all branches)
gat nminus1 dc grid.arrow

# Use Rate-B emergency ratings
gat nminus1 dc grid.arrow --rating-type rate-b

# Parallel execution for large grids
gat nminus1 dc grid.arrow --threads 8

# Output violations only
gat nminus1 dc grid.arrow -o - | \
  jq '.contingencies[] | select(.has_violations)'
```

### Find Critical Contingencies

```bash
# Top 10 worst contingencies by loading
gat nminus1 dc grid.arrow -o - | \
  jq -r '.contingencies | sort_by(-.max_loading_pct) | .[0:10] |
         .[] | "\(.outage_from)-\(.outage_to): \(.max_loading_pct | round)%"'
```

### Contingency Analysis Pipeline

```bash
#!/bin/bash
# contingency_report.sh - Generate contingency analysis report

GRID=$1
OUTPUT_DIR=${2:-./contingency_results}

mkdir -p "$OUTPUT_DIR"

echo "Running N-1 analysis on $GRID..."
gat nminus1 dc "$GRID" -o "$OUTPUT_DIR/n1_full.json"

echo "Extracting violations..."
jq '.contingencies[] | select(.has_violations)' \
   "$OUTPUT_DIR/n1_full.json" > "$OUTPUT_DIR/violations.json"

echo "Summary:"
jq -r '"Total contingencies: \(.total_contingencies)
Violations: \(.contingencies_with_violations)
Failed (islands): \(.contingencies_failed)
Worst case: \(.worst_contingency.outage_from)-\(.worst_contingency.outage_to) at \(.worst_contingency.max_loading_pct | round)%"' \
   "$OUTPUT_DIR/n1_full.json"
```

---

## Data Pipeline Recipes

### Format Conversion Pipeline

```bash
# MATPOWER → Arrow → PSS/E
gat import case118.m -o grid.arrow
gat convert grid.arrow -f psse -o grid.raw

# pandapower → Arrow → PowerModels JSON
gat import network.json -o grid.arrow
gat convert grid.arrow -f powermodels -o grid_pm.json
```

### Extract Specific Data

```bash
# Get all generator data as CSV
gat inspect generators grid.arrow --format csv > generators.csv

# Get branch data with flows (after solving)
gat pf dc grid.arrow -o - | jq -r '
  .branches[] | [.from, .to, .flow_mw, .loading_pct] | @csv
' > branch_flows.csv

# Get buses with voltage violations
gat pf ac grid.arrow -o - | jq '
  .buses[] | select(.vm < 0.95 or .vm > 1.05)
'
```

### Join with External Data

```bash
# Export bus locations, join with GIS data
gat inspect buses grid.arrow --format csv > buses.csv
gat geo join buses.csv zones.geojson --on coordinates -o bus_zones.csv
```

---

## Batch Processing

### Scenario Manifest

Create `scenarios.yaml`:

```yaml
name: load_variations
base_case: grid.arrow
scenarios:
  - name: peak_load
    modifications:
      - type: scale_load
        factor: 1.2
  - name: light_load
    modifications:
      - type: scale_load
        factor: 0.7
  - name: gen_outage_1
    modifications:
      - type: disable_generator
        bus: 10
```

### Run Batch Analysis

```bash
# Expand scenarios to individual files
gat scenarios expand scenarios.yaml -d ./scenario_cases

# Run batch power flow
gat batch pf scenarios.yaml -d ./results --threads 4

# Run batch OPF
gat batch opf scenarios.yaml -d ./results --method dc
```

### Parallel Processing Pattern

```bash
# Process multiple grids in parallel
find ./grids -name "*.arrow" | \
  parallel -j4 'gat pf dc {} -o {.}_pf.json'

# Run OPF on all PGLib cases
ls pglib_opf_*.m | \
  parallel -j8 'gat opf run {} -o results/{/.}.json'
```

---

## Analytics & Sensitivities

### PTDF Analysis

```bash
# Basic PTDF calculation
gat analytics ptdf grid.arrow

# Specific source/sink transfer
gat analytics ptdf grid.arrow --source 1 --sink 50

# Output PTDF matrix
gat analytics ptdf grid.arrow -o ptdf_matrix.csv
```

### Deliverability Score

```bash
# Calculate DS for all generators
gat analytics ds grid.arrow

# With specific rating type
gat analytics ds grid.arrow --rating-type rate-b

# JSON output for integration
gat analytics ds grid.arrow --format json -o ds_scores.json
```

### Reliability Metrics

```bash
# LOLE/EUE calculation
gat analytics reliability grid.arrow \
  --load-forecast load_2025.csv \
  --outage-rates outages.csv

# With Monte Carlo iterations
gat analytics reliability grid.arrow --iterations 10000
```

### ELCC Estimation

```bash
# Equivalent Load Carrying Capability
gat analytics elcc grid.arrow \
  --resource-profile solar_cf.csv \
  --target-reliability 0.1  # 0.1 day/year LOLE
```

---

## Output Format Reference

### Table (Default, Human-Readable)

```bash
gat pf dc grid.arrow --format table
```

### JSON (Scripting, Full Detail)

```bash
gat pf dc grid.arrow --format json -o results.json
```

### JSON Lines (Streaming)

```bash
gat inspect branches grid.arrow --format jsonl | head -10
```

### CSV (Spreadsheets, Data Analysis)

```bash
gat inspect generators grid.arrow --format csv > gen.csv
```

### Stdout Piping

```bash
# Pipe to jq
gat pf dc grid.arrow -o - | jq '.summary'

# Pipe to Python
gat inspect buses grid.arrow --format jsonl | \
  python -c "import sys,json; [print(json.loads(l)['name']) for l in sys.stdin]"
```

---

## Environment Variables

```bash
# Set default output directory
export GAT_OUTPUT_DIR=./results

# Set default thread count
export GAT_THREADS=8

# Set log level
export GAT_LOG_LEVEL=debug

# Set solver preference
export GAT_SOLVER=highs
```

---

## Troubleshooting

### Convergence Issues

```bash
# Show iteration details
gat pf ac grid.arrow --show-iterations

# Try different linear solver
gat pf ac grid.arrow --linear-solver gauss

# Check for islands
gat graph islands grid.arrow
```

### Performance

```bash
# Enable parallel processing
gat pf dc grid.arrow --threads $(nproc)

# Use release profile
gat --profile release pf dc large_grid.arrow
```

### Validation Errors

```bash
# Detailed validation
gat validate grid.arrow --verbose

# Check specific issues
gat graph validate grid.arrow
gat inspect power-balance grid.arrow
```
