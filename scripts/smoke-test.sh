#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${WORK_DIR:-$(mktemp -d 2>/dev/null || mktemp -d -t gat-smoke)}"
GAT_BIN="${GAT_BIN:-}"

if [[ -z "$GAT_BIN" ]]; then
  if command -v gat >/dev/null 2>&1; then
    GAT_BIN="gat"
  elif command -v gat-cli >/dev/null 2>&1; then
    GAT_BIN="gat-cli"
  fi
fi

if [[ -z "$GAT_BIN" ]]; then
  echo "No gat binary found. Set GAT_BIN to a built gat binary (e.g., target/release/gat-cli)." >&2
  exit 1
fi

echo "Using gat binary: $GAT_BIN"
echo "Work directory: $WORK_DIR"

GRID_RAW="$ROOT_DIR/test_data/matpower/ieee14.case"
COSTS_CSV="$ROOT_DIR/test_data/opf/costs.csv"
GRID_ARROW="$WORK_DIR/grid.arrow"
PF_OUT="$WORK_DIR/flows.parquet"
OPF_OUT="$WORK_DIR/dispatch.parquet"

"$GAT_BIN" import matpower --m "$GRID_RAW" -o "$GRID_ARROW"
"$GAT_BIN" pf dc "$GRID_ARROW" --out "$PF_OUT"
"$GAT_BIN" opf dc "$GRID_ARROW" --cost "$COSTS_CSV" --out "$OPF_OUT"

echo "Smoke test complete. Outputs:"
echo "  Grid:     $GRID_ARROW"
echo "  PF flows: $PF_OUT"
echo "  OPF:      $OPF_OUT"
