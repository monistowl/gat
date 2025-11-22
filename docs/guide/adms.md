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

## Integration with Reliability Metrics

ADMS commands output islanded/unsupplied load estimates that feed into:
- Energy Not Served (ENS)
- Loss-of-Load Expectation (LOLE)
- Customer Average Interruption Frequency Index (CAIFI)

See `docs/guide/analytics.md` for reliability aggregation.

## References

- **crate**: `crates/gat-adms/README.md`
- **CLI**: `gat adms --help`
- **Schema**: `docs/schemas/adms_output.json`
