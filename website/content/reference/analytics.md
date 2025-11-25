+++
title = "Reliability Metrics"
description = "Analytics & Reliability Metrics"
weight = 14
+++

# Analytics & Reliability Metrics

Analytics commands extract grid insights: power transfer distribution, reliability indices, and interconnection limits.

## Key Metrics

**PTDF** — Power Transfer Distribution Factors
- Sensitivity: 1 MW injection at bus A → how much MW flows on each branch?
- Use case: Determine congestion risks for renewable injection
- Linear analysis (post-contingency or steady-state)

**Reliability Metrics**
- **ENS** — Energy Not Served (MWh unserved per year)
- **LOLE** — Loss-of-Load Expectancy (hours/year at risk)
- **CAIFI** — Customer Average Interruption Frequency Index
- Based on outage scenarios (N-1, N-2) and restoration times

**ELCC** — Effective Load Carrying Capability
- How much load can be served with a new resource (wind/solar/battery)?
- Incorporates weather, demand, and existing resources

**Deliverability** — Transmission hosting capacity
- How much renewable energy can be delivered to load?
- Limited by transmission thermal ratings and voltage stability

**DS** — Demand Served
- Fraction of load met after N-1 screening
- Input to reliability index calculations

## Usage Examples

### Compute PTDF for a source-sink pair

```bash
gat analytics ptdf \
  --grid transmission_network.arrow \
  --source bus_1 \
  --sink bus_2 \
  --transfer 1.0 \
  --solver gauss \
  --out ptdf_1_2.parquet
```

Output:
- Branch ID, flow (MW), PTDF (fraction per MW)
- Summary: max PTDF, min PTDF, branches above threshold

### Reliability analysis

```bash
gat analytics reliability \
  --grid network.arrow \
  --outages contingency_scenarios.yaml \
  --restoration-times outage_mttr.csv \
  --demand demand_profile.csv \
  --out reliability_indices.parquet
```

Output:
- Outage ID
- Peak unserved load (MW)
- Energy not served (MWh)
- Estimated LOLE, CAIFI contributions

### ELCC for solar

```bash
gat analytics elcc \
  --grid network.arrow \
  --weather solar_irradiance.csv \
  --demand load_profile.csv \
  --existing-resources existing_gens.yaml \
  --candidate-capacity 50 \
  --candidate-type solar \
  --out elcc_solar.parquet
```

Output:
- Effective load carrying capability (MW)
- Margin above nameplate (if any)
- Sensitivity to weather data

### Deliverability screening

```bash
gat analytics deliverability \
  --grid network.arrow \
  --injection-point feeder_123 \
  --injection-ramp 100 \
  --max-penetration 30 \
  --out deliverability.parquet
```

## Integration with Planning Workflows

Use analytics in batch studies:

```bash
# Scenario-based reliability
gat scenarios materialize --spec scenarios.yaml --grid-file network.arrow --out-dir runs/scenarios
gat batch opf --manifest runs/scenarios/scenario_manifest.json --out runs/batch/opf_results

# Then compute reliability from batch results
gat analytics reliability \
  --grid network.arrow \
  --batch-root runs/batch/opf_results \
  --out reliability_summary.parquet
```

## References

- **CLI**: `gat analytics --help`
- **Schemas**: `docs/schemas/analytics_*.json`
- **Examples**: `test_data/analytics/`
