#!/bin/bash
# solver_benchmark.sh - Compare OPF solver performance across methods
#
# This script benchmarks different OPF solution methods on PGLib test cases:
# - DC-OPF (linear)
# - SOCP relaxation
# - Enhanced SOCP (with QC envelopes)
# - AC-OPF NLP (L-BFGS)
# - AC-OPF NLP (IPOPT, if available)
#
# Usage: ./solver_benchmark.sh [pglib_dir] [output_dir]

set -euo pipefail

PGLIB_DIR=${1:-"$HOME/.gat/cache/pglib"}
OUTPUT_DIR=${2:-"./benchmark_results"}
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Test cases (small to large)
CASES=(
    "pglib_opf_case14_ieee"
    "pglib_opf_case30_ieee"
    "pglib_opf_case57_ieee"
    "pglib_opf_case118_ieee"
    "pglib_opf_case300_ieee"
)

# Methods to benchmark
METHODS=("dc" "socp" "ac")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[BENCH]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Initialize results CSV
RESULTS_CSV="$OUTPUT_DIR/benchmark_${TIMESTAMP}.csv"
echo "case,buses,method,status,objective,solve_time_ms,iterations" > "$RESULTS_CSV"

# Check for IPOPT
HAS_IPOPT=false
if gat solver status ipopt >/dev/null 2>&1; then
    HAS_IPOPT=true
    METHODS+=("ac-nlp-ipopt")
    log "IPOPT detected - including NLP benchmarks"
else
    warn "IPOPT not found - skipping NLP benchmarks"
fi

# Download PGLib if needed
if [[ ! -d "$PGLIB_DIR" ]]; then
    log "Downloading PGLib test cases..."
    gat dataset pglib -d "$PGLIB_DIR" 2>/dev/null || {
        warn "Could not download PGLib, using local cases if available"
        PGLIB_DIR="."
    }
fi

# =============================================================================
# Run benchmarks
# =============================================================================

log "Starting solver benchmark suite"
log "Output: $RESULTS_CSV"
echo ""

for case_name in "${CASES[@]}"; do
    # Find case file
    case_file=$(find "$PGLIB_DIR" -name "${case_name}.m" 2>/dev/null | head -1)

    if [[ -z "$case_file" ]]; then
        warn "Case not found: $case_name"
        continue
    fi

    # Get bus count
    buses=$(grep -c "mpc.bus\s*=" "$case_file" 2>/dev/null || echo "?")

    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    log "Testing: $case_name ($buses buses)"

    for method in "${METHODS[@]}"; do
        printf "  %-15s " "$method"

        # Build command based on method
        case "$method" in
            dc)
                cmd="gat opf dc"
                ;;
            socp)
                cmd="gat opf dc --method socp"
                ;;
            socp-enhanced)
                cmd="gat opf dc --method socp --enhanced"
                ;;
            ac)
                cmd="gat opf ac"
                ;;
            ac-nlp)
                cmd="gat opf ac-nlp"
                ;;
            ac-nlp-ipopt)
                cmd="gat opf ac-nlp --nlp-solver ipopt"
                ;;
            *)
                warn "Unknown method: $method"
                continue
                ;;
        esac

        # Run benchmark with timeout
        start_time=$(date +%s%3N)

        result=$(timeout 120s $cmd "$case_file" --format json 2>/dev/null) && status="success" || status="failed"

        end_time=$(date +%s%3N)
        elapsed=$((end_time - start_time))

        if [[ "$status" == "success" && -n "$result" ]]; then
            objective=$(echo "$result" | jq -r '.objective // .total_cost // "N/A"' 2>/dev/null || echo "N/A")
            iterations=$(echo "$result" | jq -r '.iterations // "N/A"' 2>/dev/null || echo "N/A")

            echo -e "${GREEN}✓${NC} ${elapsed}ms, obj=${objective}"
            echo "$case_name,$buses,$method,success,$objective,$elapsed,$iterations" >> "$RESULTS_CSV"
        else
            echo -e "${RED}✗${NC} ${status} (${elapsed}ms)"
            echo "$case_name,$buses,$method,$status,N/A,$elapsed,N/A" >> "$RESULTS_CSV"
        fi
    done
    echo ""
done

# =============================================================================
# Generate summary
# =============================================================================

log "Generating summary..."

# Summary statistics
cat > "$OUTPUT_DIR/summary_${TIMESTAMP}.md" << EOF
# Solver Benchmark Summary

**Date:** $(date -Iseconds)
**PGLib Directory:** $PGLIB_DIR

## Test Cases

| Case | Buses |
|------|-------|
$(for c in "${CASES[@]}"; do echo "| $c | - |"; done)

## Methods Tested

$(for m in "${METHODS[@]}"; do echo "- $m"; done)

## Results

\`\`\`
$(column -t -s',' "$RESULTS_CSV" 2>/dev/null || cat "$RESULTS_CSV")
\`\`\`

## Performance Summary

### By Method (Average Solve Time)

\`\`\`
$(awk -F',' 'NR>1 && $4=="success" {
    times[$3] += $6;
    counts[$3]++;
}
END {
    for (m in times) {
        printf "%-20s %.1f ms (n=%d)\n", m, times[m]/counts[m], counts[m]
    }
}' "$RESULTS_CSV" | sort -k2 -n)
\`\`\`

## Files

- \`benchmark_${TIMESTAMP}.csv\` - Raw results
- \`summary_${TIMESTAMP}.md\` - This summary

EOF

success "Benchmark complete!"
echo ""
echo "Results: $RESULTS_CSV"
echo "Summary: $OUTPUT_DIR/summary_${TIMESTAMP}.md"
echo ""

# Quick stats
echo "Performance Summary:"
echo "-------------------"
awk -F',' 'NR>1 && $4=="success" {
    times[$3] += $6;
    counts[$3]++;
}
END {
    for (m in times) {
        printf "  %-15s avg: %6.1f ms (n=%d)\n", m, times[m]/counts[m], counts[m]
    }
}' "$RESULTS_CSV" | sort -k3 -n
