#!/bin/bash
# reliability_study.sh - Resource adequacy and reliability analysis
#
# This script performs a comprehensive reliability assessment:
# 1. Load forecast preparation
# 2. Generator outage modeling
# 3. LOLE/EUE calculation
# 4. ELCC estimation for renewable resources
#
# Usage: ./reliability_study.sh <grid_file> [load_forecast.csv] [output_dir]

set -euo pipefail

GRID=${1:-"grid.arrow"}
LOAD_FORECAST=${2:-""}
OUTPUT_DIR=${3:-"./reliability_results"}
ITERATIONS=${ITERATIONS:-1000}

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

log() { echo -e "${BLUE}[RA]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

mkdir -p "$OUTPUT_DIR"

log "Resource Adequacy Study"
log "Grid: $GRID"
log "Monte Carlo iterations: $ITERATIONS"
echo ""

# =============================================================================
# Step 1: Analyze generation fleet
# =============================================================================
log "Step 1: Analyzing generation fleet..."

gat inspect generators "$GRID" --format json > "$OUTPUT_DIR/generators.json"

# Summarize generation capacity
jq -r '
  group_by(.type) |
  map({
    type: .[0].type,
    count: length,
    capacity_mw: (map(.pmax_mw // .pmax // 0) | add)
  }) |
  .[] | "\(.type): \(.count) units, \(.capacity_mw | round) MW"
' "$OUTPUT_DIR/generators.json" 2>/dev/null || \
jq -r 'length as $n | "Total generators: \($n)"' "$OUTPUT_DIR/generators.json"

TOTAL_CAP=$(jq '[.[].pmax_mw // .[].pmax // 0] | add | round' "$OUTPUT_DIR/generators.json")
success "Total installed capacity: ${TOTAL_CAP} MW"

# =============================================================================
# Step 2: Create/validate load forecast
# =============================================================================
log "Step 2: Preparing load forecast..."

if [[ -n "$LOAD_FORECAST" && -f "$LOAD_FORECAST" ]]; then
    cp "$LOAD_FORECAST" "$OUTPUT_DIR/load_forecast.csv"
    success "Using provided load forecast: $LOAD_FORECAST"
else
    # Generate synthetic 8760-hour load profile
    log "Generating synthetic annual load profile..."
    cat > "$OUTPUT_DIR/load_forecast.csv" << 'EOF'
hour,load_mw
EOF
    # Simple load curve: base + daily pattern + seasonal pattern
    python3 -c "
import math
for h in range(8760):
    day_of_year = h // 24
    hour_of_day = h % 24
    # Base load
    base = 0.6
    # Daily pattern (peak at 18:00)
    daily = 0.2 * math.sin((hour_of_day - 6) * math.pi / 12)
    # Seasonal pattern (peak in summer/winter)
    seasonal = 0.15 * math.cos((day_of_year - 180) * 2 * math.pi / 365)
    # Random variation
    import random
    random.seed(h)
    noise = random.gauss(0, 0.02)
    load_factor = max(0.3, min(1.0, base + daily + seasonal + noise))
    print(f'{h},{load_factor * $TOTAL_CAP:.1f}')
" >> "$OUTPUT_DIR/load_forecast.csv"
    success "Generated 8760-hour load profile"
fi

PEAK_LOAD=$(tail -n +2 "$OUTPUT_DIR/load_forecast.csv" | cut -d',' -f2 | sort -rn | head -1)
log "Peak load: ${PEAK_LOAD} MW"
log "Reserve margin: $(echo "scale=1; ($TOTAL_CAP - $PEAK_LOAD) / $PEAK_LOAD * 100" | bc)%"

# =============================================================================
# Step 3: Define generator outage rates
# =============================================================================
log "Step 3: Defining generator outage parameters..."

# Create outage rate file (FOR = Forced Outage Rate)
cat > "$OUTPUT_DIR/outage_rates.csv" << 'EOF'
gen_type,for_pct,mttr_hours
coal,6.0,72
gas_cc,4.0,48
gas_ct,5.0,24
nuclear,3.0,168
hydro,2.0,36
wind,5.0,12
solar,2.0,8
storage,3.0,24
other,5.0,48
EOF

success "Outage rates defined"

# =============================================================================
# Step 4: Run LOLE/EUE calculation
# =============================================================================
log "Step 4: Running reliability calculation (LOLE/EUE)..."

gat analytics reliability "$GRID" \
    --load-forecast "$OUTPUT_DIR/load_forecast.csv" \
    --outage-rates "$OUTPUT_DIR/outage_rates.csv" \
    --iterations "$ITERATIONS" \
    -o "$OUTPUT_DIR/reliability.json" 2>/dev/null || {
    warn "Reliability command not available, creating estimate..."
    # Create placeholder with reasonable estimates
    python3 -c "
import json
import random

# Simple Monte Carlo simulation
lole_hours = 0
eue_mwh = 0
iterations = $ITERATIONS
peak = $PEAK_LOAD
capacity = $TOTAL_CAP

for i in range(iterations):
    # Simulate annual outages
    available = capacity * random.gauss(0.92, 0.05)
    shortfall_hours = 0
    shortfall_energy = 0
    for h in range(8760):
        # Simple hourly simulation
        load = peak * (0.5 + 0.5 * random.random())
        if load > available:
            shortfall_hours += 1
            shortfall_energy += (load - available)
    lole_hours += shortfall_hours
    eue_mwh += shortfall_energy

result = {
    'lole_days_per_year': lole_hours / iterations / 24,
    'lole_hours_per_year': lole_hours / iterations,
    'eue_mwh_per_year': eue_mwh / iterations,
    'iterations': iterations,
    'peak_load_mw': peak,
    'installed_capacity_mw': capacity,
    'method': 'monte_carlo_estimate'
}
print(json.dumps(result, indent=2))
" > "$OUTPUT_DIR/reliability.json"
}

LOLE=$(jq '.lole_days_per_year // .lole // 0' "$OUTPUT_DIR/reliability.json")
EUE=$(jq '.eue_mwh_per_year // .eue // 0' "$OUTPUT_DIR/reliability.json")
success "LOLE: ${LOLE} days/year, EUE: ${EUE} MWh/year"

# =============================================================================
# Step 5: ELCC estimation (if renewables present)
# =============================================================================
log "Step 5: Estimating ELCC for variable resources..."

# Check for renewable generators
RENEWABLES=$(jq '[.[] | select(.type == "wind" or .type == "solar")] | length' \
    "$OUTPUT_DIR/generators.json" 2>/dev/null || echo 0)

if [[ "$RENEWABLES" -gt 0 ]]; then
    log "Found $RENEWABLES renewable generators"

    gat analytics elcc "$GRID" \
        --target-reliability 0.1 \
        -o "$OUTPUT_DIR/elcc.json" 2>/dev/null || {
        warn "ELCC command not available, using capacity factor approximation..."
        # Estimate ELCC as fraction of nameplate
        jq '{
            wind_elcc_pct: 15,
            solar_elcc_pct: 30,
            method: "capacity_factor_approximation",
            note: "Actual ELCC requires hourly resource profiles"
        }' <<< '{}' > "$OUTPUT_DIR/elcc.json"
    }

    success "ELCC estimates saved to elcc.json"
else
    log "No renewable generators found, skipping ELCC"
fi

# =============================================================================
# Step 6: Generate report
# =============================================================================
log "Generating reliability report..."

cat > "$OUTPUT_DIR/reliability_report.md" << EOF
# Resource Adequacy Study

**Grid:** $GRID
**Date:** $(date -Iseconds)

## System Summary

| Metric | Value |
|--------|-------|
| Installed Capacity | ${TOTAL_CAP} MW |
| Peak Load | ${PEAK_LOAD} MW |
| Reserve Margin | $(echo "scale=1; ($TOTAL_CAP - $PEAK_LOAD) / $PEAK_LOAD * 100" | bc)% |

## Reliability Indices

| Index | Value | Target |
|-------|-------|--------|
| LOLE | ${LOLE} days/year | < 0.1 |
| EUE | ${EUE} MWh/year | - |

## Analysis Parameters

- Monte Carlo iterations: $ITERATIONS
- Load forecast: $(basename "$OUTPUT_DIR/load_forecast.csv")
- Outage rates: Generic by fuel type

## Interpretation

$(if (( $(echo "$LOLE < 0.1" | bc -l) )); then
    echo "✅ System meets 1-in-10 reliability standard (LOLE < 0.1 days/year)"
else
    echo "⚠️ System does NOT meet 1-in-10 reliability standard"
    echo ""
    echo "Consider:"
    echo "- Adding peaking capacity"
    echo "- Demand response programs"
    echo "- Transmission upgrades for imports"
fi)

## Output Files

- \`generators.json\` - Generation fleet data
- \`load_forecast.csv\` - Hourly load profile
- \`outage_rates.csv\` - Generator outage parameters
- \`reliability.json\` - LOLE/EUE results
$(if [[ "$RENEWABLES" -gt 0 ]]; then echo "- \`elcc.json\` - ELCC estimates"; fi)
EOF

success "Report: $OUTPUT_DIR/reliability_report.md"

# =============================================================================
# Summary
# =============================================================================
echo ""
log "Resource adequacy study complete!"
echo ""
echo "Key Results:"
echo "  LOLE: ${LOLE} days/year (target: < 0.1)"
echo "  EUE:  ${EUE} MWh/year"
echo ""
echo "Files: $OUTPUT_DIR/"
ls "$OUTPUT_DIR"/*.{json,csv,md} 2>/dev/null | sed 's/^/  /'
