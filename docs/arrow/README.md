# Arrow Schemas

Arrow schema JSON comes from `cargo xtask doc schemas`. Close the Arrow schema describe branch/opf output tables so agents can preview column names and types without loading the actual Parquet files.

## Grid exports

Grid datasets encode the internal network as a single Arrow table where each row represents a bus, generator, load, branch, or transformer. Every record contains the component `type`, `id`, and `name`, along with optional connectivity columns (`voltage_kv`, `from_bus`, `to_bus`, `resistance`, `reactance`). Generator and load rows now populate the `active_power_mw`/`reactive_power_mvar` columns so that downstream consumers (PF/OPF/WLS solvers, analytics pipelines) can replay the net injections exactly as they were imported. When a dataset was created without generation/load data, these columns remain `null` and the solver falls back to the synthetic slack pair recorded in the manifest.

Refer to `docs/schemas/grid.schema.json` for the full column definitions.
