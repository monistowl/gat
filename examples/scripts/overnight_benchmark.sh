#!/bin/bash
# Overnight batch benchmark script for GAT
# Runs comprehensive benchmarks on PGLib, PFDelta, and OPFData datasets

set -e

# Configuration
GAT_ROOT="${GAT_ROOT:-/home/tom/Code/gat}"
RESULTS_DIR="$GAT_ROOT/results/overnight_$(date +%Y%m%d_%H%M%S)"
LOG_FILE="$RESULTS_DIR/benchmark.log"

# Create results directory
mkdir -p "$RESULTS_DIR"
echo "=== GAT Overnight Benchmark ===" | tee "$LOG_FILE"
echo "Started: $(date)" | tee -a "$LOG_FILE"
echo "Results: $RESULTS_DIR" | tee -a "$LOG_FILE"

# Build release binary
echo "" | tee -a "$LOG_FILE"
echo "=== Building release binary ===" | tee -a "$LOG_FILE"
cd "$GAT_ROOT"
cargo build --release 2>&1 | tee -a "$LOG_FILE"

CLI="$GAT_ROOT/target/release/gat-cli"

# PGLib-OPF Benchmark
echo "" | tee -a "$LOG_FILE"
echo "=== PGLib-OPF Benchmark ===" | tee -a "$LOG_FILE"
echo "Running all 68 PGLib cases..." | tee -a "$LOG_FILE"
$CLI benchmark pglib \
    --pglib-dir "$GAT_ROOT/data/pglib-opf" \
    --out "$RESULTS_DIR/pglib_full.csv" \
    --baseline "$GAT_ROOT/data/pglib-opf/baseline.csv" \
    2>&1 | tee -a "$LOG_FILE"

# PFDelta Benchmark - all contingencies
echo "" | tee -a "$LOG_FILE"
echo "=== PFDelta Benchmark ===" | tee -a "$LOG_FILE"

for CASE in case30 case57 case118; do
    for CONT in n n1 n2; do
        echo "Running PFDelta $CASE $CONT..." | tee -a "$LOG_FILE"
        $CLI benchmark pfdelta \
            --pfdelta-dir "$GAT_ROOT/data/pfdelta/$CASE" \
            --contingency "$CONT" \
            --out "$RESULTS_DIR/pfdelta_${CASE}_${CONT}.csv" \
            2>&1 | tee -a "$LOG_FILE"
    done
done

# OPFData Benchmark - extract all groups first
echo "" | tee -a "$LOG_FILE"
echo "=== OPFData Benchmark ===" | tee -a "$LOG_FILE"

OPFDATA_DIR="$GAT_ROOT/data/opfdata/pglib_opf_case118_ieee"

# Extract all tar.gz files if not already extracted
echo "Extracting OPFData archives..." | tee -a "$LOG_FILE"
cd "$OPFDATA_DIR"
for f in group_*.tar.gz; do
    if [ -f "$f" ]; then
        GROUP="${f%.tar.gz}"
        if [ ! -d "$GROUP" ]; then
            echo "  Extracting $f..." | tee -a "$LOG_FILE"
            tar -xzf "$f"
        fi
    fi
done
cd "$GAT_ROOT"

# Run OPFData benchmark with all available samples
echo "Running OPFData benchmark (all samples)..." | tee -a "$LOG_FILE"
$CLI benchmark opfdata \
    --opfdata-dir "$OPFDATA_DIR" \
    --max-cases 0 \
    --out "$RESULTS_DIR/opfdata_case118_full.csv" \
    2>&1 | tee -a "$LOG_FILE"

# Generate analysis
echo "" | tee -a "$LOG_FILE"
echo "=== Generating Analysis ===" | tee -a "$LOG_FILE"
python3 "$GAT_ROOT/examples/scripts/analyze_benchmarks.py" "$RESULTS_DIR" 2>&1 | tee -a "$LOG_FILE"

# Summary
echo "" | tee -a "$LOG_FILE"
echo "=== Benchmark Complete ===" | tee -a "$LOG_FILE"
echo "Finished: $(date)" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"
echo "Results saved to: $RESULTS_DIR" | tee -a "$LOG_FILE"
echo "Log file: $LOG_FILE" | tee -a "$LOG_FILE"

# List result files
echo "" | tee -a "$LOG_FILE"
echo "Result files:" | tee -a "$LOG_FILE"
ls -la "$RESULTS_DIR"/*.csv 2>/dev/null | tee -a "$LOG_FILE"

# Count total samples
echo "" | tee -a "$LOG_FILE"
echo "Sample counts:" | tee -a "$LOG_FILE"
for f in "$RESULTS_DIR"/*.csv; do
    if [ -f "$f" ]; then
        COUNT=$(($(wc -l < "$f") - 1))
        echo "  $(basename $f): $COUNT samples" | tee -a "$LOG_FILE"
    fi
done
