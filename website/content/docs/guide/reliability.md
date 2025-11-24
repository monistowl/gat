+++
title = "Reliability Analysis"
description = "Evaluate power system reliability with Monte Carlo simulation and LOLE/EUE metrics"
weight = 40
+++

# Reliability Analysis and Metrics

GAT provides comprehensive reliability analysis tools for evaluating power system performance under uncertainty, including Monte Carlo simulation, LOLE/EUE metrics, multi-area coordination, and integration with ADMS operations.

## Core Concepts

### Loss of Load Expectation (LOLE)
Hours per year during which the system load cannot be fully served due to generation or transmission inadequacy. Calculated via Monte Carlo sampling of random outage scenarios with load variations.

### Energy Unserved (EUE)
MWh per year of customer demand that cannot be met. Accounts for both the duration and severity of shortfalls.

### Deliverability Score
Composite 0-100 metric combining LOLE, voltage violations, and thermal overloads with configurable weights. Provides a single reliability indicator for system health assessment.

**Score Ranges:**
- **90-100:** Excellent (< 0.5 hrs LOLE/year)
- **75-90:** Good (0.5-2 hrs LOLE/year)
- **60-75:** Fair (2-5 hrs LOLE/year)
- **40-60:** Poor (5-10 hrs LOLE/year)
- **< 40:** Critical (> 10 hrs LOLE/year)

### Multi-Area Reliability (CANOS)
Coordinated Automatic Network Operating System framework for multi-area grids with:
- **Zone-to-Zone LOLE**: Reliability contribution from inter-area transmission
- **Corridor Utilization**: Flow ratios during contingencies
- **Area Coordination**: Synchronized outage windows to avoid cascades

## Core Algorithms

### Monte Carlo Simulation

GAT uses random sampling of outage scenarios, including both **generator and transmission branch outages**:

```
For N scenarios (default 500):
  1. Sample generator outage probabilities (Weibull distribution)
  2. Sample branch (transmission line) outages (per N-1/N-2 contingencies)
  3. Sample load variation (±10% around baseline)
  4. Compute available generation considering network topology:
     - Generators may be isolated if connecting branches are offline
     - Use BFS to determine which generators can reach which loads
  5. Track shortfalls: max(0, Load - Deliverable_Gen)
  6. Compute LOLE = (hours_with_shortfall / N) × 8766
  7. Compute EUE = sum(shortfall_mw × duration) / N
```

**Configuration:**
- `scenarios`: Number of Monte Carlo samples (default 500)
- `outage_rate`: Annual failure rate for generators
- `mttr`: Mean time to repair (hours)
- `demand_variation`: Load scaling factor (default 0.8-1.2)

### Branch Outage Impact (v0.3)

When transmission branches are offline, generators may be unable to reach certain loads even if they have available capacity. GAT computes *deliverable generation* by performing a breadth-first search through the network graph, only traversing online branches:

```python
# Example: 118-bus system with one line outage
# Generator A: 100 MW, connected to Bus 1
# Critical line: Bus 1 ↔ Bus 5 (offline in this scenario)
# Load B: 80 MW, connected to Bus 50 (reachable through Bus 5)
#
# Result: Generator A cannot contribute to Load B
# Available capacity for Load B = 0 MW (not 100 MW)
# Shortfall = 80 MW (even though 100 MW generation exists)
```

This topology-aware approach correctly captures transmission-limited reliability.

### Deliverability Score Computation

```
score = 100 × [1 - w_lole * (LOLE/LOLE_max)
                    - w_voltage * (violations/max_violations)
                    - w_thermal * (overloads/max_overloads)]
```

**Parameters:**
- `LOLE_max`: Threshold LOLE (hours/year, default 5.0)
- `voltage_weight`: Relative importance (default 0.4)
- `thermal_weight`: Relative importance (default 0.6)

## Usage Examples

### Basic Reliability Calculation

```bash
# Compute LOLE/EUE for a network
gat adms reliability \
  --grid network.arrow \
  --scenarios 500 \
  --out reliability_results.parquet
```

Output includes:
- `lole` (hours/year)
- `eue` (MWh/year)
- `scenarios_analyzed` (count)
- `scenarios_with_shortfall` (count)
- `average_shortfall` (MW)

### Multi-Area Reliability Analysis

```bash
# Evaluate zone-to-zone LOLE and corridor utilization
gat adms multiarea-reliability \
  --system-config multi_area.yaml \
  --scenarios 500 \
  --out multiarea_results.parquet
```

Output per area:
- `area_id`
- `area_lole` (hours/year for that zone)
- `zone_to_zone_lole` (contribution from other areas)
- Corridor utilization (0-100%)

### Deliverability Score Assessment

```bash
# Compute composite reliability score
gat adms deliverability-score \
  --grid network.arrow \
  --scenarios 300 \
  --lole-max 2.0 \
  --voltage-weight 0.4 \
  --thermal-weight 0.6 \
  --out score_results.parquet
```

Output:
- `score` (0-100)
- `status` (Excellent/Good/Fair/Poor/Critical)
- `lole` (hours/year)
- Component breakdown

### Sensitivity Analysis

```bash
# Analyze LOLE vs. capacity margin
for capacity in 100 110 120 130 150; do
  gat adms reliability \
    --grid network.arrow \
    --override-gen-capacity $capacity \
    --scenarios 500 \
    --out results_${capacity}mw.parquet
done
```

## Integration with ADMS Operations

### FLISR Impact on Reliability

FLISR operations reduce LOLE by restoring load during outages:

```rust
pub struct FlisrRestoration {
    pub operation_id: usize,
    pub faulted_component: String,
    pub total_duration: f64,    // detection + isolation + restoration
    pub load_restored: f64,     // MW
    pub lole_before: f64,       // before restoration
    pub lole_after: f64,        // after restoration
    pub effectiveness: f64,     // (before - after) / before
}
```

**Usage:**
```bash
# Track FLISR effectiveness
gat adms flisr-impact \
  --network-pre fault_network.arrow \
  --network-post restored_network.arrow \
  --component "line_15" \
  --load-restored 50.0 \
  --out flisr_impact.parquet
```

### VVO with Reliability Constraints

Volt-Var Optimization respects minimum deliverability scores:

```bash
# VVO that maintains 80+ reliability score
gat adms vvo-constrained \
  --grid network.arrow \
  --min-deliverability-score 80 \
  --loss-weight 0.6 \
  --voltage-weight 0.4 \
  --out vvo_dispatch.parquet
```

The optimizer reduces losses while keeping the score above the threshold. If score drops below 80, it shifts weight toward reliability (0.1 loss weight) and away from loss minimization.

### Maintenance Scheduling with Multi-Area Coordination

Schedule outages to minimize peak LOLE impact:

```bash
# Plan maintenance windows with coordination
gat adms schedule-maintenance \
  --system multi_area.yaml \
  --baseline-lole 5.0 \
  --max-peak-lole 8.0 \
  --out maintenance_schedule.parquet
```

Ensures:
- No two neighboring areas on same day
- Peak LOLE during worst maintenance window ≤ threshold
- ≤ 15% EUE reduction from coordinated scheduling

## Test Data

The crate includes comprehensive test cases validating reliability calculations:

- `test_nerc_lole_benchmark_range`: Validates against NERC standards (LOLE should be 0-8766 hours/year)
- `test_capacity_margin_effect`: Higher capacity → lower LOLE
- `test_deliverability_score_range`: Score always 0-100
- `test_multiarea_zone_to_zone_lole`: Zone-to-zone contributions >= 0
- `test_corridor_utilization_tracking`: Flow ratios stay 0-100%
- `test_branch_outage_impact`: Branch outages correctly reduce available generation

Run with:
```bash
cargo test -p gat-algo --test reliability_benchmarks -- --nocapture
cargo test -p gat-adms --test integration_with_reliability -- --nocapture
```

## Implementation Details

### Outage Scenario Generation

```rust
pub struct OutageScenario {
    pub offline_generators: HashSet<NodeIndex>,
    pub offline_branches: HashSet<usize>,  // Branch indices from outage scenarios
    pub demand_scale: f64,
    pub probability: f64,
}
```

Generated using:
- **Generator failures**: Weibull(shape=1.2, scale=0.02) annual rate
- **Branch failures**: Included for N-1/N-2 contingencies via realistic topology modeling
- **Demand variations**: Uniform [0.8, 1.2] × baseline

### Topology-Aware Generation Calculation

For each scenario, available generation is computed as the sum of capacity from generators that can reach at least one load through available (online) branches. This uses a breadth-first search to traverse the network graph, respecting branch outage status.

### Multi-Area Coordination

The `MultiAreaSystem` maintains:
- **Areas**: Independent sub-networks with separate LOLE
- **Corridors**: Transmission ties with flow limits (MVA)
- **Coordination constraints**: No two neighbors can be down simultaneously

Zone-to-zone LOLE = LOLE contribution when one area fails and must rely on others.

## Performance Considerations

- **Memory**: O(N scenarios × buses × branches)
- **Time**: O(N × AC_OPF_iterations) per evaluation
- **Parallelism**: Rayon work-stealing over scenarios

For 859,800 PFDelta instances (IEEE 14/30/57/118-bus cases):
- 500 scenarios × 118 buses ≈ 59k power flows
- ~500ms per case on 16-core system = ~8 hours full suite
- Use `--max-cases N` to sample subset

## References

- **CIM Standard**: IEC 61970-301 (CIM 3.0)
- **NERC Standards**: PJM MISO interconnection frequency standards
- **CANOS**: "Coordinated Automatic Network Operating System", TPWRS 2007
- **Crate**: `crates/gat-algo/src/reliability_monte_carlo.rs`
- **Integration**: `crates/gat-adms/src/reliability_integration.rs`
