+++
title = "DERMS Workflows"
description = "DERMS (Distributed Energy Resource Management System)"
weight = 21
+++

# DERMS (Distributed Energy Resource Management System)

DERMS tools help aggregators and utilities manage portfolios of solar, storage, EV chargers, and demand response.

## Key Concepts

**DER Envelope Aggregation**
- Aggregates individual DER capabilities (power, energy, ramp rate)
- Produces a dispatch envelope: at each time step, what power ranges are achievable?
- Accounts for:
  - Battery state-of-charge constraints
  - Ramp limits
  - Reserve margins
  - Device availability

**Pricing-Based Scheduling**
- Given dynamic electricity prices (or locational prices)
- Optimizes individual and portfolio charging/discharging
- Maximizes arbitrage profit while respecting envelope limits

**Stress Testing**
- Simulates response to scenarios (peak demand, low wind, sudden outages)
- Verifies that aggregated DERs meet reliability targets (e.g., minimum reserve, ramp capability)

## Usage Examples

### Aggregate a DER portfolio

```bash
gat derms aggregate \
  --devices device_catalog.csv \
  --ders_metadata der_list.yaml \
  --start 2024-01-01 \
  --end 2024-01-31 \
  --out ders_envelope.parquet
```

Output (time-indexed):
- Available power (MW)
- Available energy (MWh)
- Maximum charge rate (MW)
- Maximum discharge rate (MW)
- Reserve requirement (MW)

### Schedule DERs for price response

```bash
gat derms schedule \
  --envelope ders_envelope.parquet \
  --prices spot_prices.csv \
  --strategy max_arbitrage \
  --soc_bounds "[0.1,0.9]" \
  --out dispatch_schedule.parquet
```

Output:
- Dispatch setpoint (MW) per device per interval
- Predicted profit
- Battery state-of-charge trajectory
- Constraint violations (if any)

### Run stress tests

```bash
gat derms stress \
  --envelope ders_envelope.parquet \
  --scenarios stress_test_matrix.yaml \
  --out stress_results.parquet
```

## Integration with Distribution Analysis

DER aggregates can be incorporated into DIST hosting-capacity and ADMS voltage-support workflows:

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --ders ders_envelope.parquet \
  --out hosting_curves.parquet
```

See `docs/guide/dist.md` for full workflow.

## References

- **crate**: `crates/gat-derms/README.md`
- **CLI**: `gat derms --help`
- **Schema**: `docs/schemas/derms_output.json`
