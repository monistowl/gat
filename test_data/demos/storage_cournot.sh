#!/usr/bin/env bash

# A training-grade Cournot/storage demo inspired by arXiv:2509.26568 (econ.GN).
# It sweeps storage ownership from 1..MAX_FIRMS and records price, EENS, and welfare.

set -euo pipefail

# === Configuration ===
ROOT=$(cd "$(dirname "$0")/.." && pwd)
N_SCENARIOS=500          # wind/solar uncertainty scenarios
MAX_FIRMS=10             # solve Cournot for 1..MAX_FIRMS identical storage owners
BASE_STORAGE_MWH=8000    # total system storage energy capacity (split equally)
BASE_STORAGE_MW=4000     # total power capacity for charging/discharging
WIND_SOLAR_PENETRATION=1.2   # 120% of peak load → reliability stress
OUTPUT="cournot_results.csv"
WORKDIR="$ROOT/out/demos/cournot"

mkdir -p "$WORKDIR"
cd "$WORKDIR"

# 1. Prepare base case (RTS-GMLC) with high renewables and no thermal flexibility
cp "$ROOT/data/rts-gmlc"/{network,gen,bus,branch}.parquet .
cp "$ROOT/data/rts-gmlc/load.parquet" .
cp "$ROOT/data/rts-gmlc/res.parquet" .

# Scale renewables up and kill cheap thermal to force reliability issues
gat modify scale-gen --type WIND,SOLAR --factor $WIND_SOLAR_PENETRATION
gat modify set-cost --type THERMAL --cost 1000   # make thermal extremely expensive → rarely used

# Generate uncertain RES scenarios (high variance)
gat ts sample-gaussian \
  --base-res res.parquet \
  --res-std 0.60 \
  --scenarios $N_SCENARIOS \
  --seed 123 \
  --out scenarios/

echo "N_firms,Total_Storage_MWh,Price_MWh,EENS_MWh,Consumer_Surplus,Storage_Profit_per_Firm,Total_Welfare" > $OUTPUT

# 2. Cournot loop: for each possible number of symmetric storage firms
for ((n=1; n<=MAX_FIRMS; n++)); do
  echo "Solving Cournot for $n symmetric storage firms..."

  # Each firm gets equal share of total storage capacity
  energy_per_firm=$(( BASE_STORAGE_MWH / n ))
  power_per_firm=$(( BASE_STORAGE_MW / n ))

  # Clear previous storage definition
  rm -f storage.parquet

  # Create n identical storage units (gat accepts multi-unit storage tables)
  for ((i=1; i<=n; i++)); do
    gat storage create \
      --name "Storage_Firm$i" \
      --bus 118 \
      --energy $energy_per_firm \
      --charge-power $power_per_firm \
      --discharge-power $power_per_firm \
      --efficiency 0.95 \
      --initial-soc 0.5 \
      --degradation-cost 0.01 >> storage_tmp.parquet
  done
  # Merge into one table
  gat storage merge storage_tmp_*.parquet --out storage.parquet
  rm storage_tmp_*.parquet

  total_eens=0
  total_cost=0

  # Run OPF on every scenario (parallel, blazing fast)
  for scen in scenarios/scenario_*.parquet; do
    gat opf dc \
      --network network.parquet \
      --gen gen.parquet \
      --load load.parquet \
      --res $scen \
      --storage storage.parquet \
      --storage-degradation-cost 0.01 \
      --solver clarabel \
      --quiet \
      --out sol_$(basename $scen) &
  done
  wait

  # Aggregate results across scenarios
  for sol in sol_scenario_*.parquet; do
    price=$(gat query average-lmp --solution $sol)
    cost=$(gat query objective --solution $sol)
    # Compute EENS = max(0, load - served) summed
    eens=$(gat query shortfall --solution $sol | awk '{sum+=$1} END{print sum}')
    total_eens=$(echo "$total_eens + $eens" | bc -l)
    total_cost=$(echo "$total_cost + $cost" | bc -l)
    rm $sol
  done

  avg_price=$(echo "$total_cost / $N_SCENARIOS" | bc -l)
  avg_eens=$(echo "$total_eens / $N_SCENARIOS" | bc -l)

  # Rough consumer surplus ≈ (value of load - total cost)
  value_of_load=200  # $/MWh VoLL
  load_mwh=$(gat query total-load --load load.parquet)
  consumer_surplus=$(echo "$value_of_load * $load_mwh - $total_cost" | bc -l)

  # Storage profit = arbitrage + reliability value
  storage_revenue=$(gat summary storage-revenue sol_* 2>/dev/null || echo 0)
  profit_per_firm=$(echo "$storage_revenue / $n / $N_SCENARIOS" | bc -l)

  welfare=$(echo "$consumer_surplus + $storage_revenue" | bc -l)

  printf "%d,%.0f,%.2f,%.1f,%.0f,%.2f,%.0f\n" \
    $n $BASE_STORAGE_MWH $avg_price $avg_eens $consumer_surplus $profit_per_firm $welfare >> $OUTPUT

  echo "   → Avg Price: $avg_price $/MWh   EENS: $avg_eens MWh   Profit/firm: $profit_per_firm $"
done

echo
echo "=== Cournot Storage Oligopoly Results ==="
cat $OUTPUT
echo
echo "Plot with: gnuplot -e \"set datafile separator ','; plot 'cournot_results.csv' u 1:3 w lp title 'Price', '' u 1:4 w lp title 'EENS'\""
