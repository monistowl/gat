#!/bin/bash
# data_pipeline.sh - End-to-end data pipeline for grid analytics
#
# This script demonstrates a complete data pipeline:
# 1. Import grid data from multiple formats
# 2. Validate and clean
# 3. Run power flow analysis
# 4. Extract features for ML
# 5. Export to multiple formats
#
# Usage: ./data_pipeline.sh <input_file> [output_dir]

set -euo pipefail

INPUT=${1:-"network.m"}
OUTPUT_DIR=${2:-"./pipeline_output"}
THREADS=${GAT_THREADS:-4}

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[PIPE]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

mkdir -p "$OUTPUT_DIR"/{raw,clean,analysis,features,export}

log "GAT Data Pipeline"
log "Input: $INPUT"
log "Output: $OUTPUT_DIR"
echo ""

# Track timing
PIPELINE_START=$(date +%s)

# =============================================================================
# Stage 1: Import
# =============================================================================
log "Stage 1: Importing data..."
stage_start=$(date +%s)

# Detect format and import
EXT="${INPUT##*.}"
case "$EXT" in
    m)
        FORMAT="matpower"
        ;;
    raw|RAW)
        FORMAT="psse"
        ;;
    json)
        # Could be pandapower or PowerModels
        if grep -q "pandapower" "$INPUT" 2>/dev/null; then
            FORMAT="pandapower"
        else
            FORMAT="powermodels"
        fi
        ;;
    xml|cim)
        FORMAT="cim"
        ;;
    arrow|parquet)
        FORMAT="arrow"
        cp "$INPUT" "$OUTPUT_DIR/raw/network.arrow"
        ;;
    *)
        FORMAT="auto"
        ;;
esac

if [[ "$FORMAT" != "arrow" ]]; then
    gat import "$INPUT" -o "$OUTPUT_DIR/raw/network.arrow"
fi

stage_end=$(date +%s)
success "Imported from $FORMAT format ($((stage_end - stage_start))s)"

# =============================================================================
# Stage 2: Validate and Clean
# =============================================================================
log "Stage 2: Validating and cleaning..."
stage_start=$(date +%s)

# Validate
gat validate "$OUTPUT_DIR/raw/network.arrow" > "$OUTPUT_DIR/clean/validation.log" 2>&1 || {
    warn "Validation found issues (see validation.log)"
}

# Check for islands
ISLANDS=$(gat graph islands "$OUTPUT_DIR/raw/network.arrow" --format json 2>/dev/null | jq 'length' || echo "1")
if [[ "$ISLANDS" -gt 1 ]]; then
    warn "Network has $ISLANDS islands - some analyses may fail"
fi

# Check power balance
gat inspect power-balance "$OUTPUT_DIR/raw/network.arrow" > "$OUTPUT_DIR/clean/power_balance.txt" 2>&1 || true

# Copy to clean directory (in production, would apply fixes here)
cp "$OUTPUT_DIR/raw/network.arrow" "$OUTPUT_DIR/clean/network.arrow"

stage_end=$(date +%s)
success "Validation complete ($((stage_end - stage_start))s)"

# =============================================================================
# Stage 3: Analysis
# =============================================================================
log "Stage 3: Running analyses..."
stage_start=$(date +%s)

GRID="$OUTPUT_DIR/clean/network.arrow"

# Run in parallel where possible
(
    # DC Power Flow
    log "  → DC Power Flow"
    gat pf dc "$GRID" -o "$OUTPUT_DIR/analysis/pf_dc.json"
) &
PF_DC_PID=$!

(
    # AC Power Flow
    log "  → AC Power Flow"
    gat pf ac "$GRID" -o "$OUTPUT_DIR/analysis/pf_ac.json" 2>/dev/null || \
        echo '{"error": "AC PF did not converge"}' > "$OUTPUT_DIR/analysis/pf_ac.json"
) &
PF_AC_PID=$!

(
    # DC-OPF
    log "  → DC-OPF"
    gat opf dc "$GRID" -o "$OUTPUT_DIR/analysis/opf_dc.json" 2>/dev/null || \
        echo '{"error": "DC OPF failed"}' > "$OUTPUT_DIR/analysis/opf_dc.json"
) &
OPF_DC_PID=$!

# Wait for analyses
wait $PF_DC_PID || warn "DC PF had issues"
wait $PF_AC_PID || warn "AC PF had issues"
wait $OPF_DC_PID || warn "DC OPF had issues"

# N-1 contingency (after DC PF completes)
log "  → N-1 Contingency Analysis"
gat nminus1 dc "$GRID" --threads "$THREADS" -o "$OUTPUT_DIR/analysis/n1.json" 2>/dev/null || \
    echo '{"error": "N-1 analysis failed"}' > "$OUTPUT_DIR/analysis/n1.json"

# PTDF
log "  → PTDF Calculation"
gat analytics ptdf "$GRID" -o "$OUTPUT_DIR/analysis/ptdf.csv" 2>/dev/null || \
    echo "# PTDF not available" > "$OUTPUT_DIR/analysis/ptdf.csv"

stage_end=$(date +%s)
success "Analysis complete ($((stage_end - stage_start))s)"

# =============================================================================
# Stage 4: Feature Extraction
# =============================================================================
log "Stage 4: Extracting ML features..."
stage_start=$(date +%s)

# Extract bus features
jq -r '
  .buses[] |
  [.id, .name, .type, .vm, .va, .p_load, .q_load, .voltage_kv] |
  @csv
' "$OUTPUT_DIR/analysis/pf_dc.json" > "$OUTPUT_DIR/features/bus_features.csv" 2>/dev/null || {
    gat inspect buses "$GRID" --format csv > "$OUTPUT_DIR/features/bus_features.csv"
}

# Extract branch features with flows
jq -r '
  .branches[] |
  [.from, .to, .r, .x, .b, .p_flow, .loading_pct, .status] |
  @csv
' "$OUTPUT_DIR/analysis/pf_dc.json" > "$OUTPUT_DIR/features/branch_features.csv" 2>/dev/null || {
    gat inspect branches "$GRID" --format csv > "$OUTPUT_DIR/features/branch_features.csv"
}

# Extract generator features
gat inspect generators "$GRID" --format csv > "$OUTPUT_DIR/features/gen_features.csv"

# Create combined feature matrix (for ML)
cat > "$OUTPUT_DIR/features/feature_summary.json" << EOF
{
  "grid_file": "$INPUT",
  "timestamp": "$(date -Iseconds)",
  "bus_count": $(jq '.buses | length' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo 0),
  "branch_count": $(jq '.branches | length' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo 0),
  "gen_count": $(wc -l < "$OUTPUT_DIR/features/gen_features.csv"),
  "dc_converged": $(jq '.converged // true' "$OUTPUT_DIR/analysis/pf_dc.json"),
  "ac_converged": $(jq '.converged // false' "$OUTPUT_DIR/analysis/pf_ac.json"),
  "total_load_mw": $(jq '[.buses[].p_load // 0] | add' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo 0),
  "total_gen_mw": $(jq '[.generators[].p_gen // 0] | add' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo 0),
  "max_loading_pct": $(jq '[.branches[].loading_pct // 0] | max' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo 0),
  "n1_violations": $(jq '.contingencies_with_violations // 0' "$OUTPUT_DIR/analysis/n1.json" 2>/dev/null || echo 0)
}
EOF

# GNN-ready features (if available)
gat featurize gnn "$GRID" -o "$OUTPUT_DIR/features/gnn_features.pt" 2>/dev/null || \
    log "  (GNN featurization not available)"

stage_end=$(date +%s)
success "Feature extraction complete ($((stage_end - stage_start))s)"

# =============================================================================
# Stage 5: Export to Multiple Formats
# =============================================================================
log "Stage 5: Exporting to multiple formats..."
stage_start=$(date +%s)

# Arrow (already have it)
cp "$GRID" "$OUTPUT_DIR/export/network.arrow"

# MATPOWER
gat convert "$GRID" -f matpower -o "$OUTPUT_DIR/export/network.m" 2>/dev/null || \
    log "  (MATPOWER export not available)"

# PowerModels JSON
gat convert "$GRID" -f powermodels -o "$OUTPUT_DIR/export/network_pm.json" 2>/dev/null || \
    log "  (PowerModels export not available)"

# CSV exports for each component
gat inspect buses "$GRID" --format csv > "$OUTPUT_DIR/export/buses.csv"
gat inspect branches "$GRID" --format csv > "$OUTPUT_DIR/export/branches.csv"
gat inspect generators "$GRID" --format csv > "$OUTPUT_DIR/export/generators.csv"

stage_end=$(date +%s)
success "Export complete ($((stage_end - stage_start))s)"

# =============================================================================
# Pipeline Summary
# =============================================================================
PIPELINE_END=$(date +%s)
TOTAL_TIME=$((PIPELINE_END - PIPELINE_START))

log "Generating pipeline report..."

cat > "$OUTPUT_DIR/pipeline_report.md" << EOF
# Data Pipeline Report

**Input:** $INPUT
**Format:** $FORMAT
**Completed:** $(date -Iseconds)
**Total Time:** ${TOTAL_TIME}s

## Pipeline Stages

| Stage | Status | Output |
|-------|--------|--------|
| Import | ✓ | raw/network.arrow |
| Validate | ✓ | clean/validation.log |
| DC Power Flow | $(jq -r 'if .converged // true then "✓" else "⚠" end' "$OUTPUT_DIR/analysis/pf_dc.json" 2>/dev/null || echo "?") | analysis/pf_dc.json |
| AC Power Flow | $(jq -r 'if .converged // false then "✓" else "⚠" end' "$OUTPUT_DIR/analysis/pf_ac.json" 2>/dev/null || echo "?") | analysis/pf_ac.json |
| DC-OPF | $(if jq -e '.error' "$OUTPUT_DIR/analysis/opf_dc.json" >/dev/null 2>&1; then echo "⚠"; else echo "✓"; fi) | analysis/opf_dc.json |
| N-1 Analysis | $(if jq -e '.error' "$OUTPUT_DIR/analysis/n1.json" >/dev/null 2>&1; then echo "⚠"; else echo "✓"; fi) | analysis/n1.json |
| Feature Extract | ✓ | features/*.csv |
| Export | ✓ | export/* |

## Network Summary

$(cat "$OUTPUT_DIR/features/feature_summary.json" | jq -r '
"- Buses: \(.bus_count)
- Branches: \(.branch_count)
- Generators: \(.gen_count)
- Total Load: \(.total_load_mw | round) MW
- Max Branch Loading: \(.max_loading_pct | round)%
- N-1 Violations: \(.n1_violations)"
' 2>/dev/null || echo "See feature_summary.json")

## Output Structure

\`\`\`
$OUTPUT_DIR/
├── raw/           # Original imported data
├── clean/         # Validated data
├── analysis/      # Power flow, OPF, contingency results
├── features/      # ML-ready feature matrices
└── export/        # Multiple format exports
\`\`\`

## Usage Examples

\`\`\`bash
# Use Arrow format for fast analysis
gat pf dc $OUTPUT_DIR/export/network.arrow

# Load features in Python
import pandas as pd
buses = pd.read_csv('$OUTPUT_DIR/features/bus_features.csv')

# Use PowerModels format in Julia
using PowerModels
network = parse_file("$OUTPUT_DIR/export/network_pm.json")
\`\`\`
EOF

success "Pipeline complete in ${TOTAL_TIME}s"
echo ""
echo "Output: $OUTPUT_DIR/"
find "$OUTPUT_DIR" -type f | sort | sed 's|^|  |'
