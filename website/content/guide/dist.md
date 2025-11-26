+++
title = "Distribution Analysis"
description = "DIST (Distribution System Analysis)"
weight = 22
+++

# DIST (Distribution System Analysis)

DIST tools analyze and optimize distribution networks: power flows, optimal dispatch, and hosting capacity for renewable energy.

## Key Concepts

**Distribution Network Modeling**
- Imported from MATPOWER (.m, .case files)
- AC power flow with transformer impedances, load models, capacitor banks
- Voltage regulators and switching devices
- Integration points to transmission (substation).

**AC Optimal Power Flow**
- Minimizes loss or cost on distribution system
- Respects voltage bands, thermal limits, reactive capability
- Can incorporate DER dispatch (via `gat derms`)

**Hosting Capacity**
- Maximum amount of solar/wind/storage that can be connected at each bus
- Constrained by:
  - Voltage rise (from increased injection)
  - Thermal overloads on feeder sections
  - Reverse power flow limits

## Usage Examples

### Import a distribution system from MATPOWER

```bash
gat import matpower --m ieee13_feeder.m -o dist_network.arrow
```

Output: `dist_network.arrow` (Arrow format with bus, branch, generator data).

### Run AC power flow

```bash
gat dist pf \
  --grid dist_network.arrow \
  --out dist_pf.parquet
```

Output:
- Bus voltage (pu and angle)
- Branch flow (MW, MVAr, I)
- Loss summary
- Voltage violations (if any)

### Optimal power flow with cost minimization

```bash
gat dist opf \
  --grid dist_network.arrow \
  --costs gen_costs.csv \
  --limits dispatch_limits.csv \
  --out dist_opf.parquet
```

Output:
- Optimal dispatch (MW per generator)
- Resulting power flow
- Total cost
- Constraint violations (if binding)

### Hosting capacity analysis

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --type solar \
  --voltage-band "[0.95,1.05]" \
  --thermal-margin 0.1 \
  --out hosting_solar.parquet
```

Output (per bus):
- Maximum solar (MW)
- Limiting constraint (voltage rise vs. thermal)
- Incremental capacity vs. penetration

### With DER integration

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --ders ders_envelope.parquet \
  --type wind \
  --out hosting_wind_with_ders.parquet
```

## Integration with Other Tools

- **Featurization**: Generate network features for ML:
  ```bash
  gat featurize gnn --grid dist_network.arrow --out features.parquet
  ```

- **Reliability**: Evaluate CAIFI, SAIDI under outage scenarios:
  ```bash
  gat analytics reliability --grid dist_network.arrow --outages contingencies.yaml --out reliability.parquet
  ```

- **ADMS coordination**: Combine VVO and hosting capacity:
  ```bash
  gat adms vvo --grid dist_network.arrow --regulator-deadbands vvo_config.yaml --out vvo_dispatch.parquet
  ```

## References

- **crate**: `crates/gat-dist/README.md`
- **CLI**: `gat dist --help`
- **Schema**: `docs/schemas/dist_output.json`
