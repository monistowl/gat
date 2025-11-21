Here’s a full-blown implementation roadmap for **`gat scenarios`** grounded in the current GAT layout and the CANOS spec.

I’ll assume the goal of v0 is:

> “Given a base grid + time-series profiles + a scenario spec file, `gat scenarios` can validate, expand, and materialize scenario definitions into per-scenario grid artifacts and a clean manifest that CANOS / `gat batch` can consume.”

---

## 0. High-level design

**Conceptual pieces:**

1. **Scenario spec format**
   A versioned YAML/JSON spec that mirrors the `ScenarioRunRequest.scenarios` entries:

   * `scenario_id`, `description`, `tags`
   * `outages` (branch/gen/bus)
   * `dispatch_overrides` (optional, future)
   * `load_scale`, `renewable_scale`
   * `time_slices` (strings, ISO8601 / RFC3339)
   * optional `weight`, `metadata`

2. **Core library (`gat-scenarios` crate)**

   * Data types (`ScenarioSet`, `Scenario`, `Outage`, etc.)
   * Load/save spec (YAML/JSON).
   * Expansion logic (defaults, cartesian products, maybe simple generators).
   * Apply-to-network logic: given a `Network` (from `gat-io::importers::load_grid_from_arrow`), emit a mutated `Network` for a scenario.

3. **CLI integration (`gat-cli` -> `gat scenarios`)**

   * `gat scenarios validate`
   * `gat scenarios list`
   * `gat scenarios expand`
   * `gat scenarios materialize`
   * All backed by `gat-scenarios`, with telemetry (manifests) via `gat_cli::manifest`.

4. **Artifacts & file layout**

   * Scenario manifest: Parquet/JSON summary for downstream `gat batch` / CANOS-style engines.
   * Per-scenario grid snapshots: Arrow/IPC files under a consistent directory structure.

---

## 1. Data model & spec design (in `gat-scenarios`)

### 1.1 Create new crate: `crates/gat-scenarios`

**Files:**

* `crates/gat-scenarios/Cargo.toml`
* `crates/gat-scenarios/src/lib.rs`
* `crates/gat-scenarios/src/spec.rs`
* `crates/gat-scenarios/src/apply.rs`
* `crates/gat-scenarios/src/expand.rs` (optional, for generators)

**Cargo.toml sketch (dependencies):**

* `serde`, `serde_json`, `serde_yaml`
* `anyhow`
* `chrono` (for `DateTime<Utc>` parsing)
* `uuid` (for internal IDs if needed)
* `gat-core` (for `Network`, `Bus`, `Gen`, `Branch`, IDs)
* `gat-io` (for load/write grid Arrow)
* `schemars` (optional, gated behind `docs` for schema docs)

### 1.2 Spec types in `spec.rs`

Define minimal but aligned structs:

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSet {
    pub version: u32,          // e.g. 0
    pub grid_file: Option<String>,    // default base Arrow; CLI may override
    pub profiles_file: Option<String>,// optional, reserved for future
    pub defaults: ScenarioDefaults,
    pub scenarios: Vec<ScenarioSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefaults {
    pub load_scale: f64,
    pub renewable_scale: f64,
    pub time_slices: Vec<String>, // parse to DateTime<Utc> later
    pub weight: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub scenario_id: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub outages: Option<Vec<OutageSpec>>,
    pub dispatch_overrides: Option<Vec<DispatchOverrideSpec>>,
    pub load_scale: Option<f64>,
    pub renewable_scale: Option<f64>,
    pub time_slices: Option<Vec<String>>,
    pub weight: Option<f64>,
    pub metadata: Option<HashMap<String, String>>,
}
```

**Outage structures:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")] // "branch", "gen", "bus"
pub enum OutageSpec {
    Branch { id: String },
    Gen { id: String },
    Bus { id: String },
}
```

**Dispatch overrides (v0 stub, forward-compatible):**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchOverrideSpec {
    pub resource_id: String,
    pub p_max_mw: Option<f64>,
    pub p_min_mw: Option<f64>,
    pub must_run: Option<bool>,
    pub cost_multiplier: Option<f64>,
}
```

**Internal resolved type:**

Add a concrete “resolved scenario” that has defaults filled:

```rust
#[derive(Debug, Clone)]
pub struct ResolvedScenario {
    pub scenario_id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub outages: Vec<OutageSpec>,
    pub dispatch_overrides: Vec<DispatchOverrideSpec>,
    pub load_scale: f64,
    pub renewable_scale: f64,
    pub time_slices: Vec<DateTime<Utc>>,
    pub weight: f64,
    pub metadata: HashMap<String, String>,
}
```

### 1.3 Helper functions API

In `spec.rs`, implement:

* `fn load_spec_from_path(path: &Path) -> Result<ScenarioSet>`

  * Detect YAML vs JSON by extension or by `serde_*` fallbacks.
* `fn resolve_scenarios(set: &ScenarioSet) -> Result<Vec<ResolvedScenario>>`

  * Apply defaults.
  * Validate fields (non-empty, valid ISO timestamps).
* `fn validate(set: &ScenarioSet) -> Result<()>`

  * Check `scenario_id` uniqueness.
  * Check for obviously invalid values (negative scales, empty time_slices, etc.).

---

## 2. Scenario expansion & generators (optional but powerful)

**Goal:** Provide a way to generate many scenarios programmatically (e.g., “all N-1 branches”), but keep v0 minimal.

### 2.1 Minimal v0: No generators, just defaults

For **v0**, `resolve_scenarios` only:

* Fills defaults.
* Parses timestamps.
* Leaves `ScenarioSpec` 1:1 with `ResolvedScenario`.

### 2.2 v1 (optional): `ScenarioGenerator` in `expand.rs`

Design forward-compatible:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioGeneratorSpec {
    Nminus1Branches {
        /// Optional prefix for scenario_id (e.g. "n-1_line_")
        id_prefix: Option<String>,
        /// Optional tag to add
        tag: Option<String>,
    },
    // Future: Nminus2, load_sweep, renewable_sweep, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedScenarioSet {
    #[serde(flatten)]
    pub base: ScenarioSet,
    pub generators: Option<Vec<ScenarioGeneratorSpec>>,
}
```

Implement:

* `fn expand_with_generators(set: &ExtendedScenarioSet, network: &Network) -> Result<Vec<ResolvedScenario>>`

  * Start from explicit `scenarios`.
  * For `Nminus1Branches`, iterate all branches in `network`, creating a scenario per branch with an appropriate `OutageSpec::Branch` and `scenario_id` pattern like `n-1_line_{branch_name}`.

This can be wired into `gat scenarios expand` (see below) and reused by `gat batch` later.

---

## 3. Apply-to-network logic (`apply.rs`)

**Goal:** Given a base `Network` and a `ResolvedScenario`, produce a mutated `Network` suitable for writing to Arrow and/or passing to PF/OPF.

### 3.1 API design

In `apply.rs`:

```rust
use gat_core::{Network, BranchId, GenId, BusId};

pub struct ScenarioApplyOptions {
    pub drop_outaged_elements: bool, // vs. marking them with flags in metadata
}

pub fn apply_scenario_to_network(
    base: &Network,
    scenario: &ResolvedScenario,
    opts: &ScenarioApplyOptions,
) -> anyhow::Result<Network> {
    // clone, mutate
}
```

### 3.2 Outage application rules (v0)

* **Branch outages:**

  * Find branches by some key (for v0: by `name` field of `Branch`; later: allow a `branch_id` mapping from an external table).
  * Simplest: mark as “outaged” by **removing the edge** from `network.graph` when `drop_outaged_elements = true`.
  * If you prefer non-destructive representation, add a boolean on `Branch` (requires `gat-core` change). For v0, removing edges is easier.

* **Gen outages:**

  * Set `Gen.active_power_mw = 0.0`.
  * Optionally set `p_max_mw = 0.0` (if added to `Gen` struct; otherwise, we might need to extend `Gen` with min/max fields later).

* **Bus outages:**

  * More complex; v0 can:

    * Disallow bus outages (return `Err`), or
    * Remove bus and incident edges (careful with indexing).
  * For v0: recommend returning an error `"Bus outages not yet supported"`.

### 3.3 Scaling loads and renewables

Given `Network` currently has `Load` and `Gen` structs:

* Add simple heuristics:

  * `load_scale`:

    * Multiply `Load.active_power_mw` and `Load.reactive_power_mvar` by `scenario.load_scale`.
  * `renewable_scale`:

    * For `Gen` with name or metadata indicating renewable (not yet in `gat-core`), you may:

      * v0: treat all `Gen` as scalable by `renewable_scale`.
      * v1: add a `fuel_type` or `is_renewable` flag to `Gen` and only scale those.

**Implementation detail:**

In `apply_scenario_to_network`, clone the network (or write in-place with caution):

```rust
let mut network = base.clone();
// find and modify loads/generators/branches
Ok(network)
```

### 3.4 Dispatch overrides (stubbed)

For v0, you can:

* Accept `dispatch_overrides` but no-op them (log a warning), or
* If `Gen` has enough fields, implement:

  * adjust cost coefficients (if/when added to `Gen`),
  * or set `must_run` flag.

Better to design `DispatchOverrideSpec` now but implement later.

---

## 4. Writing scenario artifacts

**Goal:** Materialize:

* per-scenario network snapshots in Arrow format, and
* a scenario manifest that lists scenarios, grid paths, and time slices.

### 4.1 Use `gat-io::importers::load_grid_from_arrow` and `write_network_to_arrow`

* `importers::load_grid_from_arrow` returns `Network`.
* `importers::arrow::write_network_to_arrow` is currently `pub(super)`; you’ll need to expose a public wrapper in `gat-io`:

In `crates/gat-io/src/importers/arrow.rs`:

* Add a `pub fn export_network_to_arrow(network: &Network, output_file: &str) -> Result<()>` that just calls `write_network_to_arrow`.

In `crates/gat-io/src/importers/mod.rs`:

* Re-export:

```rust
#[cfg(feature = "ipc")]
pub use arrow::export_network_to_arrow;
```

(And a placeholder/mocked version in `arrow_disabled.rs` that returns an error like “Arrow IPC export not built, enable ipc feature”.)

### 4.2 Scenario manifest struct

In `gat-scenarios/src/lib.rs` or `manifest.rs`:

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioArtifact {
    pub scenario_id: String,
    pub grid_file: String,
    pub time_slices: Vec<DateTime<Utc>>,
    pub load_scale: f64,
    pub renewable_scale: f64,
    pub weight: f64,
    pub tags: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}
```

Add an API:

```rust
pub fn materialize_scenarios(
    base_grid_file: &Path,
    out_dir: &Path,
    scenarios: &[ResolvedScenario],
    opts: &ScenarioApplyOptions,
) -> anyhow::Result<Vec<ScenarioArtifact>> {
    // load base grid once, reuse
}
```

Implementation details:

* Load `Network` once from `base_grid_file`.

* For each scenario:

  * Clone `Network`.
  * Apply modifications (`apply_scenario_to_network`).
  * Write to `out_dir/scenario_id/grid.arrow` or similar:

    * e.g. `out_dir.join(&scenario_id).join("grid.arrow")`.
  * Collect `ScenarioArtifact` entry.

* At the end, write `scenario_manifest.json` and optionally `scenario_manifest.parquet`:

  * For Parquet: use Polars (already in workspace via `gat-algo` / CLI features).

---

## 5. CLI integration (`gat-cli`: `gat scenarios`)

### 5.1 Extend the CLI enum in `crates/gat-cli/src/cli.rs`

Add to `pub enum Commands`:

```rust
    /// Scenario definitions and materialization
    Scenarios {
        #[command(subcommand)]
        command: ScenariosCommands,
    },
```

Define `ScenariosCommands` (near other command enums):

```rust
#[derive(Subcommand, Debug)]
pub enum ScenariosCommands {
    /// Validate a scenario spec file
    Validate {
        /// Path to the scenario spec (YAML or JSON)
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
    },
    /// List scenarios defined in a spec file
    List {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Output format (table or json)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Expand generators into fully resolved scenarios
    Expand {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Optional grid file (Arrow IPC); overrides spec.grid_file
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: Option<String>,
        /// Output file for expanded spec (YAML or JSON)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
    },
    /// Materialize per-scenario grid files and a manifest
    Materialize {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Base grid file (Arrow IPC); overrides spec.grid_file
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: Option<String>,
        /// Output directory where per-scenario grids + manifest go
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out_dir: String,
        /// Whether to drop outaged elements from the grid (default: true)
        #[arg(long, default_value = "true")]
        drop_outaged: bool,
    },
}
```

### 5.2 Wire it through `main.rs` and `commands/mod.rs`

In `crates/gat-cli/src/main.rs`:

* `use crate::commands::runs as command_runs;` already exists; add:

```rust
use crate::commands::scenarios as command_scenarios;
```

* Extend main `match`:

```rust
        Some(Commands::Scenarios { command }) => {
            run_and_log("scenarios", || command_scenarios::handle(command))
        }
```

In `crates/gat-cli/src/commands/mod.rs`:

* Add:

```rust
pub mod scenarios;
```

### 5.3 Implement `commands/scenarios` module

Create directory:

* `crates/gat-cli/src/commands/scenarios/mod.rs`

Content:

```rust
use anyhow::Result;
use gat_cli::cli::ScenariosCommands;

pub mod list;
pub mod validate;
pub mod expand;
pub mod materialize;

pub fn handle(command: &ScenariosCommands) -> Result<()> {
    match command {
        ScenariosCommands::Validate { spec } => validate::handle(spec),
        ScenariosCommands::List { spec, format } => list::handle(spec, format),
        ScenariosCommands::Expand { spec, grid_file, out } => {
            expand::handle(spec, grid_file.as_deref(), out)
        }
        ScenariosCommands::Materialize {
            spec,
            grid_file,
            out_dir,
            drop_outaged,
        } => materialize::handle(spec, grid_file.as_deref(), out_dir, *drop_outaged),
    }
}
```

Then implement individual files:

#### `validate.rs`

* Use `gat_scenarios::spec::load_spec_from_path` and `validate`.

```rust
use anyhow::Result;
use std::path::Path;
use gat_scenarios::spec::{load_spec_from_path, validate};

pub fn handle(spec_path: &str) -> Result<()> {
    let set = load_spec_from_path(Path::new(spec_path))?;
    validate(&set)?;
    println!("Scenario spec '{}' is valid.", spec_path);
    Ok(())
}
```

(Optionally, add `record_run` for telemetry with type `"scenarios validate"`.)

#### `list.rs`

* Load and resolve scenarios.
* Print as a table (like other commands) or JSON depending on `format`.

Use a simple tabwriter or `println!`-based formatting (see other CLI commands for precedent).

#### `expand.rs`

* Load spec.
* If `grid_file` or `set.grid_file` present:

  * Load `Network`.
  * Use `expand_with_generators` (if implemented); else `resolve_scenarios`.
* Then write a fully expanded YAML file with one scenario per `ResolvedScenario` (no generators).

For v0, this can be a stub that simply normalizes defaults and writes a JSON/YAML with all `ResolvedScenario` fields.

#### `materialize.rs`

* Resolve which grid file to use:

  * CLI `grid_file` arg > spec.grid_file > error.
* Load spec and resolve scenarios.
* Build `ScenarioApplyOptions` from `drop_outaged`.
* Call `gat_scenarios::materialize_scenarios`.
* Write manifest file(s) into `out_dir`.
* Use `record_run` or `record_run_timed` (as in `opf.rs`, `pf.rs`) to create a run manifest:

  * Command name: `"scenarios materialize"`.
  * Params: `spec`, `grid_file`, `out_dir`, `num_scenarios`, flags.

---

## 6. Telemetry & run manifests

Use the same pattern as PF/OPF and Dist commands:

* In each handler that actually mutates or emits files (`expand`, `materialize`):

  * After successful completion, call `record_run` or `record_run_timed`.

Example in `materialize.rs`:

```rust
use crate::commands::telemetry::record_run_timed;
use std::time::Instant;

// ...
let start = Instant::now();
// run materialize_scenarios...

record_run_timed(
    out_dir, // base output dir
    "scenarios materialize",
    &[
        ("spec", spec_path),
        ("grid_file", grid_file_path),
        ("drop_outaged", &drop_outaged.to_string()),
        ("num_scenarios", &scenarios.len().to_string()),
    ],
    start,
    &Ok(()), // or pass result
);
```

This ensures `gat runs list` and `gat runs describe` can see scenario materialization runs like any other command.

---

## 7. Tests & fixtures

### 7.1 Unit tests in `gat-scenarios`

* `spec.rs` tests:

  * Parsing a minimal YAML spec.
  * Defaults application.
  * Duplicate `scenario_id` -> error.
  * Invalid timestamp -> error.

* `apply.rs` tests:

  * Construct a tiny `Network` with 2 buses, 1 branch, 1 load, 1 gen.
  * Apply a branch outage scenario and assert edge removed.
  * Apply load scaling and assert load P/Q scaled correctly.

### 7.2 Integration tests in `gat-cli`

Under `crates/gat-cli/tests/`:

* Add `scenarios_materialize.rs`:

  * Use `test_data` grid (or create a tiny one using existing test helpers).

  * Create a temporary scenario spec file in `tempdir`.

  * Run the CLI via `assert_cmd`:

    ```rust
    let mut cmd = assert_cmd::Command::cargo_bin("gat").unwrap();
    cmd.args([
        "scenarios", "materialize",
        "--spec", spec_path.to_str().unwrap(),
        "--grid-file", grid_path.to_str().unwrap(),
        "--out-dir", out_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();
    ```

  * Check that:

    * `out_dir/<scenario_id>/grid.arrow` exists.
    * `scenario_manifest.json` (or `.parquet`) exists.
    * Optionally, re-load `grid.arrow` and check structure.

---

## 8. Docs & examples

### 8.1 README updates

In `README.md`:

* Add a new section under “Core CLI workflows”:

````markdown
### Scenario definitions (`gat scenarios`)

Use `gat scenarios` to define, validate, and materialize many “what if” cases
that share a common base grid.

Example:

```bash
# 1. Validate a scenario spec
gat scenarios validate --spec examples/scenarios/rts_nminus1.yaml

# 2. Materialize per-scenario grid snapshots
gat scenarios materialize --spec examples/scenarios/rts_nminus1.yaml \
  --grid-file data/rts_topology.arrow \
  --out-dir runs/scenarios/rts_nminus1
````

Scenario specs are YAML/JSON files that mirror the  CANOS API semantics:

```yaml
version: 0
grid_file: "data/rts_topology.arrow"
defaults:
  load_scale: 1.0
  renewable_scale: 1.0
  time_slices:
    - "2024-10-01T13:00:00Z"
  weight: 1.0
  metadata: {}
scenarios:
  - scenario_id: base
    description: "Base case"
    outages: []
  - scenario_id: n-1_line_345_77
    outages:
      - type: branch
        id: "line_345_77"
    time_slices:
      - "2024-10-01T13:00:00Z"
```

```

### 8.2 `docs/` and `gat-mcp-docs` integration (optional)

- Add a short doc page under `docs/` describing:
  - Spec schema.
  - Example scenario set.
  - Directory layout of outputs.
- Extend `gat-mcp-docs` generation so `ScenariosCommands` are included; that’s mostly automatic once you have proper `clap` definitions and the `docs` feature built, but you can ensure `gat_cli::docs` picks it up.

---

## 9. Suggested `bd` issue seeds (optional)

If you want to seed beads issues for agents, something like:

1. **Create `gat-scenarios` crate and spec types**  
   - Implement `ScenarioSet`, `ScenarioSpec`, `ScenarioDefaults`, `ResolvedScenario`, and load/validate helpers.

2. **Implement `apply_scenario_to_network` and materialization API**  
   - Outages + load/renewable scaling; export mutated grids to Arrow.

3. **Expose Arrow export function in `gat-io`**  
   - Public wrapper for `write_network_to_arrow`.

4. **Add `ScenariosCommands` to `gat-cli` and basic handlers**  
   - `validate`, `list`, `materialize` wired to new crate.

5. **Integration tests for `gat scenarios materialize`**  
   - Create test spec + tiny grid; assert outputs & run manifest entries.

6. **Docs & examples for scenarios**  
   - README section + example YAML in `examples/scenarios/`.

7. **(v1) Implement scenario generators (N-1 branches)**  
   - Optional second pass; requires loading network for `expand`.

---

If you’d like, next step can be: take just Phase 1–2 (crate + basic materialization) and I can write out skeleton Rust modules and function signatures that a coding agent could fill in almost mechanically.
```

