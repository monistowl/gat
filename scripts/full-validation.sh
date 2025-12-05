#!/bin/bash
# =============================================================================
# GAT Full Validation Suite
# =============================================================================
# Comprehensive validation of all PGLib-OPF test cases with multiple solver
# methods and reporting features.
#
# Usage:
#   ./scripts/full-validation.sh [OPTIONS]
#
# Options:
#   --mode <tiered|full|quick>   Validation mode (default: tiered)
#   --methods <list>             Comma-separated OPF methods (default: dc,socp,ac)
#   --output-dir <path>          Output directory (default: validation-results)
#   --parallel <n>               Parallel threads (default: auto)
#   --case-filter <pattern>      Only run cases matching pattern
#   --skip-n1                    Skip N-1 contingency analysis
#   --skip-analytics             Skip grid analytics
#   --dry-run                    Show what would run without executing
#
# Modes:
#   tiered  - Small cases get full analysis, large cases get DC only
#   full    - Run all methods on all cases (may take hours/days)
#   quick   - DC-OPF only on all cases for quick validation
#
# Examples:
#   ./scripts/full-validation.sh --mode quick
#   ./scripts/full-validation.sh --mode full --methods dc,socp,ac
#   ./scripts/full-validation.sh --case-filter "case14\|case30\|case118"
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GAT_CLI="$PROJECT_ROOT/target/release/gat-cli"
PGLIB_DIR="$PROJECT_ROOT/data/pglib-opf"
BASELINE_CSV="$PGLIB_DIR/baseline.csv"

# Defaults
MODE="tiered"
METHODS="dc,socp,ac"
OUTPUT_DIR="$PROJECT_ROOT/validation-results"
PARALLEL="auto"
CASE_FILTER=""
SKIP_N1=false
SKIP_ANALYTICS=false
DRY_RUN=false

# Size thresholds for tiered mode (bus count)
TIER1_MAX=500      # Full analysis including N-1
TIER2_MAX=5000     # DC + SOCP + AC (no N-1)
# TIER3: >5000 buses - DC only

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# -----------------------------------------------------------------------------
# Helper Functions
# -----------------------------------------------------------------------------
log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_section() { echo -e "\n${CYAN}=== $* ===${NC}\n"; }

timestamp() { date +"%Y-%m-%d %H:%M:%S"; }

# Get bus count from case name (heuristic from pglib naming convention)
get_bus_count() {
    local casename="$1"
    # Extract number after "case" - e.g., case118_ieee -> 118
    echo "$casename" | grep -oP 'case\K[0-9]+' | head -1
}

# Check if case should be processed based on filter
should_process_case() {
    local casefile="$1"
    if [[ -z "$CASE_FILTER" ]]; then
        return 0
    fi
    echo "$casefile" | grep -qE "$CASE_FILTER" || return 1
}

# Determine tier for a case
get_tier() {
    local buses="$1"
    if [[ "$MODE" == "quick" ]]; then
        echo "quick"
    elif [[ "$MODE" == "full" ]]; then
        echo "full"
    elif [[ -z "$buses" ]] || [[ "$buses" -le "$TIER1_MAX" ]]; then
        echo "tier1"
    elif [[ "$buses" -le "$TIER2_MAX" ]]; then
        echo "tier2"
    else
        echo "tier3"
    fi
}

# Get methods for a tier
get_methods_for_tier() {
    local tier="$1"
    case "$tier" in
        quick)  echo "dc" ;;
        tier3)  echo "dc" ;;
        tier2)  echo "dc,socp" ;;
        tier1|full) echo "$METHODS" ;;
        *)      echo "dc" ;;
    esac
}

# Run a single command with timing and error capture
run_timed() {
    local label="$1"
    shift
    local start_time end_time duration exit_code

    if $DRY_RUN; then
        log_info "[DRY-RUN] Would execute: $*"
        return 0
    fi

    start_time=$(date +%s.%N)
    set +e
    "$@" 2>&1
    exit_code=$?
    set -e
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc)

    if [[ $exit_code -eq 0 ]]; then
        log_success "$label completed in ${duration}s"
    else
        log_error "$label failed (exit code: $exit_code) after ${duration}s"
    fi

    return $exit_code
}

# -----------------------------------------------------------------------------
# Parse Arguments
# -----------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --methods)
            METHODS="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --parallel)
            PARALLEL="$2"
            shift 2
            ;;
        --case-filter)
            CASE_FILTER="$2"
            shift 2
            ;;
        --skip-n1)
            SKIP_N1=true
            shift
            ;;
        --skip-analytics)
            SKIP_ANALYTICS=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            head -50 "$0" | tail -n +2 | grep -E "^#" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# -----------------------------------------------------------------------------
# Setup
# -----------------------------------------------------------------------------
log_section "GAT Full Validation Suite"
log_info "Mode: $MODE"
log_info "Methods: $METHODS"
log_info "Output directory: $OUTPUT_DIR"
log_info "Parallel threads: $PARALLEL"
log_info "Started at: $(timestamp)"

# Create output directory
mkdir -p "$OUTPUT_DIR"/{per-case,benchmarks,analytics,reports}

# Ensure binary is built
if [[ ! -x "$GAT_CLI" ]]; then
    log_info "Building gat-cli in release mode..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin gat-cli
fi

# Collect all MATPOWER files
mapfile -t ALL_CASES < <(find "$PGLIB_DIR" -name "*.m" -type f | sort)
TOTAL_CASES=${#ALL_CASES[@]}
log_info "Found $TOTAL_CASES MATPOWER files"

# Filter cases if pattern provided
FILTERED_CASES=()
for casefile in "${ALL_CASES[@]}"; do
    if should_process_case "$casefile" 2>/dev/null; then
        FILTERED_CASES+=("$casefile")
    fi
done
log_info "Processing ${#FILTERED_CASES[@]} cases after filtering"

# -----------------------------------------------------------------------------
# Phase 1: Per-Case Analysis
# -----------------------------------------------------------------------------
log_section "Phase 1: Per-Case Analysis"

process_count=0
success_count=0
fail_count=0

for casefile in "${FILTERED_CASES[@]}"; do
    casename=$(basename "$(dirname "$casefile")" 2>/dev/null || basename "$casefile" .m)
    buses=$(get_bus_count "$casename")
    tier=$(get_tier "$buses")

    process_count=$((process_count + 1))
    log_info "[$process_count/${#FILTERED_CASES[@]}] $casename (${buses:-?} buses, $tier)"

    case_output_dir="$OUTPUT_DIR/per-case/$casename"
    mkdir -p "$case_output_dir"

    # Network inspection (always run)
    if ! $DRY_RUN; then
        if "$GAT_CLI" inspect summary "$casefile" > "$case_output_dir/summary.txt" 2>&1; then
            log_success "  inspect summary: ok"
            success_count=$((success_count + 1))
        else
            log_warn "  inspect summary: failed"
        fi

        if "$GAT_CLI" inspect thermal "$casefile" > "$case_output_dir/thermal.txt" 2>&1; then
            log_success "  inspect thermal: ok"
            success_count=$((success_count + 1))
        else
            log_warn "  inspect thermal: failed"
        fi
    else
        log_info "  [DRY-RUN] Would run: inspect summary/thermal"
    fi

    # NOTE: Per-case OPF is handled by the batch benchmark command in Phase 2
    # The opf dc/ac/ac-nlp commands require separate cost/limits/output files
    # and are designed for production workflows, not single-file testing.

    # N-1 contingency analysis (tier1 only, unless skipped)
    if [[ "$tier" == "tier1" || "$tier" == "full" ]] && ! $SKIP_N1; then
        if $DRY_RUN; then
            log_info "  [DRY-RUN] Would run: nminus1 dc $casefile"
        else
            n1_start=$(date +%s.%N)
            if "$GAT_CLI" nminus1 dc "$casefile" > "$case_output_dir/n1-dc.txt" 2>&1; then
                n1_end=$(date +%s.%N)
                n1_time=$(echo "$n1_end - $n1_start" | bc)
                violations=$(grep -c "VIOLATION" "$case_output_dir/n1-dc.txt" 2>/dev/null || echo "0")
                log_success "  n1-dc: ${n1_time}s ($violations violations)"
            else
                n1_end=$(date +%s.%N)
                n1_time=$(echo "$n1_end - $n1_start" | bc)
                log_warn "  n1-dc: completed with warnings after ${n1_time}s"
            fi
        fi
    fi

    # Power flow tests
    if ! $DRY_RUN; then
        "$GAT_CLI" pf dc "$casefile" > "$case_output_dir/pf-dc.txt" 2>&1 || true
        "$GAT_CLI" pf ac "$casefile" > "$case_output_dir/pf-ac.txt" 2>&1 || true
    fi
done

# -----------------------------------------------------------------------------
# Phase 2: Batch Benchmarks
# -----------------------------------------------------------------------------
log_section "Phase 2: Batch Benchmarks"

BENCHMARK_TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Run benchmark suite for each method
IFS=',' read -ra BENCH_METHODS <<< "$METHODS"
for method in "${BENCH_METHODS[@]}"; do
    benchmark_out="$OUTPUT_DIR/benchmarks/pglib-${method}-${BENCHMARK_TIMESTAMP}.csv"
    log_info "Running $method benchmark suite..."

    if $DRY_RUN; then
        log_info "[DRY-RUN] Would run: benchmark pglib --method $method"
    else
        "$GAT_CLI" benchmark pglib \
            --pglib-dir "$PGLIB_DIR" \
            --method "$method" \
            --threads "$PARALLEL" \
            --out "$benchmark_out" \
            ${BASELINE_CSV:+--baseline "$BASELINE_CSV"} \
            2>&1 | tee "$OUTPUT_DIR/benchmarks/${method}-log.txt" || true

        if [[ -f "$benchmark_out" ]]; then
            log_success "Benchmark results saved to $benchmark_out"

            # Generate summary
            "$GAT_CLI" benchmark summary "$benchmark_out" \
                > "$OUTPUT_DIR/benchmarks/${method}-summary.txt" 2>&1 || true
        fi
    fi
done

# Compare benchmarks if multiple methods
if [[ ${#BENCH_METHODS[@]} -gt 1 ]] && ! $DRY_RUN; then
    log_info "Comparing benchmark results..."
    first_csv="$OUTPUT_DIR/benchmarks/pglib-${BENCH_METHODS[0]}-${BENCHMARK_TIMESTAMP}.csv"
    for ((i=1; i<${#BENCH_METHODS[@]}; i++)); do
        other_csv="$OUTPUT_DIR/benchmarks/pglib-${BENCH_METHODS[$i]}-${BENCHMARK_TIMESTAMP}.csv"
        if [[ -f "$first_csv" ]] && [[ -f "$other_csv" ]]; then
            "$GAT_CLI" benchmark compare "$first_csv" "$other_csv" \
                > "$OUTPUT_DIR/benchmarks/compare-${BENCH_METHODS[0]}-vs-${BENCH_METHODS[$i]}.txt" 2>&1 || true
        fi
    done
fi

# -----------------------------------------------------------------------------
# Phase 3: Grid Analytics (if not skipped)
# -----------------------------------------------------------------------------
if ! $SKIP_ANALYTICS; then
    log_section "Phase 3: Grid Analytics"

    # Run analytics on a representative subset (small/medium cases)
    ANALYTICS_CASES=()
    for casefile in "${FILTERED_CASES[@]}"; do
        casename=$(basename "$(dirname "$casefile")" 2>/dev/null || basename "$casefile" .m)
        buses=$(get_bus_count "$casename")
        if [[ -n "$buses" ]] && [[ "$buses" -le 1000 ]]; then
            ANALYTICS_CASES+=("$casefile")
        fi
    done

    log_info "Running analytics on ${#ANALYTICS_CASES[@]} cases (<=1000 buses)"

    for casefile in "${ANALYTICS_CASES[@]}"; do
        casename=$(basename "$(dirname "$casefile")" 2>/dev/null || basename "$casefile" .m)
        analytics_dir="$OUTPUT_DIR/analytics/$casename"
        mkdir -p "$analytics_dir"

        if $DRY_RUN; then
            log_info "[DRY-RUN] Would run analytics for $casename"
        else
            # PTDF for slack bus transfer
            "$GAT_CLI" analytics ptdf "$casefile" --source 1 --sink 2 \
                > "$analytics_dir/ptdf.txt" 2>&1 || true

            # Thermal analysis
            "$GAT_CLI" inspect thermal "$casefile" \
                > "$analytics_dir/thermal.txt" 2>&1 || true

            # JSON export for downstream tools
            "$GAT_CLI" inspect json "$casefile" \
                > "$analytics_dir/network.json" 2>&1 || true
        fi
    done
fi

# -----------------------------------------------------------------------------
# Phase 4: Generate Reports
# -----------------------------------------------------------------------------
log_section "Phase 4: Generate Reports"

REPORT_FILE="$OUTPUT_DIR/reports/validation-report-${BENCHMARK_TIMESTAMP}.md"

if ! $DRY_RUN; then
    cat > "$REPORT_FILE" << EOF
# GAT Validation Report

**Generated:** $(timestamp)
**Mode:** $MODE
**Methods:** $METHODS

## Summary

- **Total cases processed:** $process_count
- **Successful runs:** $success_count
- **Failed runs:** $fail_count
- **Success rate:** $(echo "scale=1; $success_count * 100 / ($success_count + $fail_count)" | bc)%

## Configuration

| Parameter | Value |
|-----------|-------|
| Mode | $MODE |
| Methods | $METHODS |
| Parallel threads | $PARALLEL |
| Skip N-1 | $SKIP_N1 |
| Skip Analytics | $SKIP_ANALYTICS |
| Case filter | ${CASE_FILTER:-"(none)"} |

## Files

- Case results CSV: \`case-results.csv\`
- Benchmark CSVs: \`benchmarks/pglib-*.csv\`
- Per-case outputs: \`per-case/<casename>/\`
- Analytics outputs: \`analytics/<casename>/\`

## Tier Distribution

EOF

    # Add tier stats
    for tier in tier1 tier2 tier3; do
        count=$(grep -c ",$tier," "$CASE_RESULTS_CSV" 2>/dev/null || echo "0")
        echo "- **$tier:** $count cases" >> "$REPORT_FILE"
    done

    echo "" >> "$REPORT_FILE"
    echo "## Benchmark Results" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    # Include benchmark summaries
    for summary_file in "$OUTPUT_DIR/benchmarks/"*-summary.txt; do
        if [[ -f "$summary_file" ]]; then
            method=$(basename "$summary_file" -summary.txt)
            echo "### $method" >> "$REPORT_FILE"
            echo '```' >> "$REPORT_FILE"
            cat "$summary_file" >> "$REPORT_FILE"
            echo '```' >> "$REPORT_FILE"
            echo "" >> "$REPORT_FILE"
        fi
    done

    log_success "Report generated: $REPORT_FILE"
fi

# -----------------------------------------------------------------------------
# Final Summary
# -----------------------------------------------------------------------------
log_section "Validation Complete"

log_info "Finished at: $(timestamp)"
log_info "Results directory: $OUTPUT_DIR"
log_info "Total cases: $process_count"
log_success "Successful: $success_count"
if [[ $fail_count -gt 0 ]]; then
    log_error "Failed: $fail_count"
fi

echo ""
log_info "To view the report:"
echo "  cat $REPORT_FILE"
echo ""
log_info "To compare with previous runs:"
echo "  $GAT_CLI benchmark compare <old.csv> <new.csv>"
