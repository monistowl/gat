# GAT CLI Cheatsheet

Quick reference for common GAT commands. Version 0.5.4+.

## Import & Convert

```bash
# Auto-detect format
gat import network.m -o grid.arrow         # MATPOWER
gat import network.raw -o grid.arrow       # PSS/E
gat import network.json -o grid.arrow      # pandapower

# Convert between formats
gat convert grid.arrow -f matpower -o out.m
gat convert grid.arrow -f psse -o out.raw
gat convert grid.arrow -f powermodels -o out.json
```

## Inspect

```bash
gat inspect summary grid.arrow             # Quick overview
gat inspect buses grid.arrow --format csv  # Export bus data
gat inspect branches grid.arrow            # Branch data
gat inspect generators grid.arrow          # Generator data
gat inspect power-balance grid.arrow       # P/Q balance check
```

## Power Flow

```bash
# DC Power Flow (fast, linear)
gat pf dc grid.arrow
gat pf dc grid.arrow -o results.json
gat pf dc grid.arrow -o - | jq '.'         # Pipe to jq

# AC Power Flow (Newton-Raphson)
gat pf ac grid.arrow
gat pf ac grid.arrow --show-iterations     # Convergence details
gat pf ac grid.arrow --slack-bus 1         # Override slack
gat pf ac grid.arrow --linear-solver faer  # Fast backend
```

## Optimal Power Flow

```bash
# DC-OPF
gat opf dc grid.arrow
gat opf dc grid.arrow --costs costs.csv
gat opf dc grid.arrow --method socp        # SOCP relaxation
gat opf dc grid.arrow --method socp --enhanced

# AC-OPF
gat opf ac grid.arrow                      # Fast-decoupled
gat opf ac-nlp grid.arrow                  # Full NLP (L-BFGS)
gat opf ac-nlp grid.arrow --nlp-solver ipopt
gat opf ac-nlp grid.arrow --warm-start socp

# Direct on MATPOWER file
gat opf run case118.m
gat opf run case118.m --method socp
```

## Contingency Analysis

```bash
gat nminus1 dc grid.arrow                  # N-1 screening
gat nminus1 dc grid.arrow --rating-type rate-b
gat nminus1 dc grid.arrow --threads 8
gat nminus1 dc grid.arrow -o - | jq '.contingencies[] | select(.has_violations)'
```

## Analytics

```bash
# PTDF sensitivity
gat analytics ptdf grid.arrow
gat analytics ptdf grid.arrow --source 1 --sink 50

# Deliverability score
gat analytics ds grid.arrow

# Reliability metrics
gat analytics reliability grid.arrow --load-forecast load.csv
```

## Batch & Scenarios

```bash
# Scenario workflow
gat scenarios validate scenarios.yaml
gat scenarios expand scenarios.yaml -d ./cases
gat batch pf scenarios.yaml -d ./results
gat batch opf scenarios.yaml -d ./results --method dc
```

## Distribution

```bash
gat dist import feeder.dss -o feeder.arrow
gat dist pf feeder.arrow
gat dist opf feeder.arrow
gat dist hostcap feeder.arrow --voltage-limits 0.95,1.05
```

## DERMS

```bash
gat derms envelope feeder.arrow -o envelope.json
gat derms schedule feeder.arrow --prices prices.csv
gat derms stress-test feeder.arrow --scenario max-export
```

## State Estimation

```bash
gat se wls grid.arrow --measurements meas.csv -o se.json
```

## Graph / Topology

```bash
gat graph stats grid.arrow                 # Basic stats
gat graph islands grid.arrow               # Detect islands
gat graph validate grid.arrow              # Check connectivity
```

## Data Management

```bash
# Public datasets
gat dataset pglib -d ./pglib
gat dataset rts-gmlc -d ./rts
gat dataset eia -d ./eia

# Solver management
gat solver list
gat solver install ipopt
gat solver status
```

## Output Formats

| Flag | Format | Use Case |
|------|--------|----------|
| `--format table` | Human-readable | Terminal viewing |
| `--format json` | Full JSON | Scripting |
| `--format jsonl` | JSON Lines | Streaming |
| `--format csv` | CSV | Spreadsheets |
| `-o -` | Stdout | Piping |

## Useful Patterns

```bash
# Find overloaded branches
gat pf dc grid.arrow -o - | jq '.branches[] | select(.loading_pct > 80)'

# Export generators to CSV
gat inspect generators grid.arrow --format csv > gen.csv

# Parallel batch processing
ls *.arrow | parallel -j4 'gat pf dc {} -o {.}_pf.json'

# Top 5 worst N-1 contingencies
gat nminus1 dc grid.arrow -o - | \
  jq -r '.contingencies | sort_by(-.max_loading_pct) | .[0:5]'

# Voltage violations
gat pf ac grid.arrow -o - | jq '.buses[] | select(.vm < 0.95 or .vm > 1.05)'
```

## Environment Variables

```bash
export GAT_THREADS=8          # Parallel threads
export GAT_LOG_LEVEL=debug    # Logging verbosity
export GAT_OUTPUT_DIR=./out   # Default output directory
export GAT_SOLVER=highs       # Solver preference
```

## Getting Help

```bash
gat --help                    # Main help
gat pf --help                 # Subcommand help
gat pf dc --help              # Command help
gat doctor                    # Environment check
gat completions bash          # Shell completions
```
