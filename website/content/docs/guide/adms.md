+++
title = "ADMS Integration"
description = "ADMS (Automatic Distribution Management System)"
weight = 20
+++

# ADMS (Automatic Distribution Management System)

ADMS tools coordinate reliability and efficiency on distribution networks through automated switching and voltage control.

## Key Concepts

**FLISR** — Fault Location, Isolation, and Service Restoration
- Detects faults on a feeder
- Isolates the faulted section
- Restores service to affected loads via alternate feeder paths

**VVO** — Volt-Var Optimization
- Adjusts voltage setpoints on regulators and capacitors
- Minimizes losses while maintaining voltage limits
- Reactive power dispatch

**Outage Coordination** — Manages multiple simultaneous outages or planned maintenance
- Evaluates restoration priorities
- Coordinates tie-line usage
- Estimates service duration

## Usage Examples

### Run FLISR for a specific fault

```bash
gat adms flisr \
  --grid distribution_network.arrow \
  --fault-location feeder_1/branch_42 \
  --tie-lines tie_config.yaml \
  --out flisr_result.parquet
```

Output includes:
- Isolation sequence (which switches to open)
- Restored load (MW/MVAr)
- Restoration time estimate
- Backup sources used

### VVO optimization

```bash
gat adms vvo \
  --grid distribution_network.arrow \
  --voltage-limits voltage_bands.yaml \
  --regulator-deadbands deadbands.yaml \
  --max-iterations 10 \
  --out vvo_dispatch.parquet
```

Output:
- Optimal voltage setpoints (per regulator)
- Capacitor switching commands
- Estimated loss reduction
- Peak voltage violations (if any)

### Outage impact analysis

```bash
gat adms outage \
  --grid distribution_network.arrow \
  --outage-scenario outages.yaml \
  --out outage_impact.parquet
```

## Integration with Reliability Metrics (v0.3)

ADMS operations are now fully integrated with Monte Carlo reliability analysis:

### FLISR with Reliability Tracking

FLISR operations now compute before/after LOLE metrics:

```bash
# Execute FLISR and measure reliability impact
gat adms flisr \
  --grid pre_fault_network.arrow \
  --network-post post_restoration.arrow \
  --fault-location feeder_1/branch_42 \
  --out flisr_result.parquet
```

Output includes reliability impact:
- `lole_before` (hours/year before FLISR)
- `lole_after` (hours/year after FLISR)
- `lole_reduction_pct` (percentage improvement)
- `effectiveness` (0.0-1.0, where >0.5 means effective)

### VVO with Reliability Constraints

VVO now respects minimum deliverability scores and shifts objective weights based on current reliability:

```bash
# VVO maintaining 80+ reliability score
gat adms vvo \
  --grid distribution_network.arrow \
  --min-deliverability-score 80.0 \
  --loss-weight 0.6 \
  --voltage-weight 0.4 \
  --aggressive-mode false \
  --out vvo_dispatch.parquet
```

**Objective Weight Scheduling:**
- Score < threshold: 0.1 (heavily favor reliability)
- Score near threshold (±10): 0.5 (balanced)
- Score well above threshold (non-aggressive): 0.6 (favor losses)
- Score well above threshold (aggressive): 0.8 (maximize loss reduction)

### Multi-Area Maintenance Coordination

Schedule outages to minimize peak LOLE while coordinating across zones:

```bash
# Plan maintenance with multi-area coordination
gat adms schedule-maintenance \
  --system multi_area.yaml \
  --baseline-lole 5.0 \
  --max-peak-lole 8.0 \
  --out maintenance_schedule.parquet
```

Constraints:
- No two neighboring areas can have maintenance on same day
- Peak LOLE during any maintenance window must stay below threshold
- EUE reduction from coordinated scheduling (up to 15%)

## Reliability Concepts

See [Reliability Analysis](/docs/guide/reliability/) for detailed explanation of:
- **LOLE** (Loss of Load Expectation)
- **EUE** (Energy Unserved)
- **Deliverability Score** (0-100 composite metric)
- **Monte Carlo Simulation** (outage scenario generation)
- **CANOS** (multi-area coordination framework)

## Test Suite

Comprehensive integration tests validate:
- FLISR effectiveness measurement (17 tests)
- VVO objective weighting (8 tests)
- Maintenance window validation (6 tests)
- Multi-area coordination (14 tests)

Run with:
```bash
cargo test -p gat-adms --test integration_with_reliability -- --nocapture
```

## References

- **Reliability Metrics**: [Reliability Analysis](/docs/guide/reliability/)
- **Crate**: `crates/gat-adms/src/reliability_integration.rs`
- **CLI**: `gat adms --help`
- **Tests**: `crates/gat-adms/tests/integration_with_reliability.rs`
- **Schema**: `docs/schemas/adms_output.json`
