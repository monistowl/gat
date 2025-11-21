Here’s a roadmap for **`gat featurize gnn`** that fits the existing GAT codebase and gives you a clean, ML-friendly graph dataset for Power-GNN.

I’ll assume v0 is:

> “Given a `Network` (Arrow grid) and one or more PF/OPF flow snapshots (Parquet), `gat featurize gnn` exports graph data suitable for PyTorch Geometric/DGL: separate node/edge/graph tables with static + optional dynamic features, keyed by scenario/time.”

---

## 0. Semantics & target format

### 0.1 What we want out

**Core idea**: standard GNN “triple”:

1. **Graphs table** – one row per graph:

   * `graph_id` (int)
   * optional `scenario_id`, `time`
   * metadata like `num_nodes`, `num_edges`, maybe `description`.

2. **Nodes table** – one row per node per graph:

   * `graph_id`
   * `node_id` (0..N-1, contiguous)
   * `bus_id` (original GAT bus ID)
   * static features:

     * `voltage_kv`
     * counts & P/Q sums of gens/loads on that bus
   * dynamic features (optional, v1):

     * net injection, voltage magnitude, etc.

3. **Edges table** – one row per edge per graph:

   * `graph_id`
   * `edge_id` (0..M-1, contiguous)
   * `src`, `dst` (node indices)
   * `branch_id` (original)
   * static features:

     * `resistance`, `reactance` (from `Branch`)
   * dynamic:

     * `flow_mw` from PF/OPF flows
     * optionally `loading` if branch limits are known

These go to **Parquet** for easy interop.

### 0.2 What we take in

For v0:

* **Grid**: Arrow IPC, as usual:

  * `gat_io::importers::load_grid_from_arrow(grid_file) -> Network`.
* **Flows**: Parquet output from:

  * `gat pf dc` (stage `pf-dc`) or future `pf ac`/`opf`:

    * at minimum: `branch_id`, `from_bus`, `to_bus`, `flow_mw`.
    * possibly `scenario_id`, `time`, `run_id`, etc.
* **Grouping semantics**:

  * If flows have `scenario_id` and/or `time`:

    * each `(scenario_id, time)` group becomes a separate **graph**.
  * Otherwise:

    * treat all flows as a single graph, `graph_id = 0`.

---

## 1. Where code lives

To align with current crate structure:

* Keep featurization logic in **`gat-algo`** (like PF & PTDF).
* Expose via **`gat-cli`** as `gat featurize gnn`.

### 1.1 New module in `gat-algo`

Add:

* `crates/gat-algo/src/featurize_gnn.rs`

And re-export in `crates/gat-algo/src/lib.rs`:

```rust
pub mod io;
pub mod power_flow;
pub mod test_utils;
pub mod featurize_gnn;

pub use io::*;
pub use power_flow::*;
pub use featurize_gnn::*;
```

---

## 2. Data model & feature design

### 2.1 Graph IDs and grouping

Define a small internal struct:

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct GraphKey {
    pub graph_id: i64,
    pub scenario_id: Option<String>,
    pub time: Option<DateTime<Utc>>,
}
```

Rules:

* Inspect flows DataFrame columns:

  * If `scenario_id` exists, group by it.
  * If `time` exists (Polars `Datetime`), group by `(scenario_id?, time?)`.
* Each group → one GraphKey; assign monotonically increasing `graph_id` starting at 0.

### 2.2 Node indexing & static features (from `Network`)

Use `gat-core::Network` (graph of `Node::Bus/Gen/Load/...`):

1. Build mapping `BusId -> node_idx` (0..N-1):

   ```rust
   use gat_core::{Network, Node, BusId};

   let mut bus_node_indices = Vec::new(); // index -> bus_id
   let mut bus_id_to_idx = HashMap::<BusId, i64>::new(); // BusId -> index

   for node_idx in network.graph.node_indices() {
       if let Node::Bus(bus) = &network.graph[node_idx] {
           let idx = bus_node_indices.len() as i64;
           bus_node_indices.push(bus.id);
           bus_id_to_idx.insert(bus.id, idx);
       }
   }
   ```

2. Aggregate gen/load info per bus:

   ```rust
   struct BusStaticFeatures {
       pub bus_id: i64,
       pub node_id: i64,
       pub name: String,
       pub voltage_kv: f64,
       pub num_gens: i64,
       pub p_gen_mw: f64,
       pub q_gen_mvar: f64,
       pub num_loads: i64,
       pub p_load_mw: f64,
       pub q_load_mvar: f64,
   }

   let mut bus_features = HashMap::<BusId, BusStaticFeatures>::new();

   // initialize with bus info
   for node_idx in network.graph.node_indices() {
       if let Node::Bus(bus) = &network.graph[node_idx] {
           let node_id = *bus_id_to_idx.get(&bus.id).unwrap();
           bus_features.insert(bus.id, BusStaticFeatures {
               bus_id: bus.id.value() as i64,
               node_id,
               name: bus.name.clone(),
               voltage_kv: bus.voltage_kv,
               num_gens: 0,
               p_gen_mw: 0.0,
               q_gen_mvar: 0.0,
               num_loads: 0,
               p_load_mw: 0.0,
               q_load_mvar: 0.0,
           });
       }
   }

   // fold in gens & loads
   for node_idx in network.graph.node_indices() {
       match &network.graph[node_idx] {
           Node::Gen(gen) => {
               if let Some(b) = bus_features.get_mut(&gen.bus) {
                   b.num_gens += 1;
                   b.p_gen_mw += gen.active_power_mw;
                   b.q_gen_mvar += gen.reactive_power_mvar;
               }
           }
           Node::Load(load) => {
               if let Some(b) = bus_features.get_mut(&load.bus) {
                   b.num_loads += 1;
                   b.p_load_mw += load.active_power_mw;
                   b.q_load_mvar += load.reactive_power_mvar;
               }
           }
           _ => {}
       }
   }
   ```

3. This gives static node features that are independent of scenario/time.

### 2.3 Edge indexing & static features

Edge = `Edge::Branch` only (ignore transformers for v0 or treat as branches later).

```rust
use gat_core::{Edge, BranchId};

struct EdgeStaticFeatures {
    pub edge_id: i64,
    pub branch_id: i64,
    pub src: i64,
    pub dst: i64,
    pub resistance: f64,
    pub reactance: f64,
}

let mut edges = Vec::<EdgeStaticFeatures>::new();
let mut branch_id_to_edge_idx = HashMap::<BranchId, i64>::new();

for edge_idx in network.graph.edge_indices() {
    if let Edge::Branch(branch) = &network.graph[edge_idx] {
        let src_bus_idx = bus_id_to_idx[&branch.from_bus];
        let dst_bus_idx = bus_id_to_idx[&branch.to_bus];
        let edge_id = edges.len() as i64;

        edges.push(EdgeStaticFeatures {
            edge_id,
            branch_id: branch.id.value() as i64,
            src: src_bus_idx,
            dst: dst_bus_idx,
            resistance: branch.resistance,
            reactance: branch.reactance,
        });

        branch_id_to_edge_idx.insert(branch.id, edge_id);
    }
}
```

This gives a consistent node/edge indexing scheme used across all graphs.

---

## 3. Featurization library API

### 3.1 Config struct

In `featurize_gnn.rs`, define:

```rust
#[derive(Debug, Clone)]
pub struct FeaturizeGnnConfig {
    /// Treat each distinct scenario_id as a separate graph (if present)
    pub group_by_scenario: bool,
    /// Treat each distinct time as a separate graph (if present)
    pub group_by_time: bool,
    /// Stage for node output (directory name)
    pub nodes_stage: String,
    /// Stage for edge output
    pub edges_stage: String,
    /// Stage for graph metadata output
    pub graphs_stage: String,
}
```

Default:

```rust
impl Default for FeaturizeGnnConfig {
    fn default() -> Self {
        Self {
            group_by_scenario: true,
            group_by_time: true,
            nodes_stage: "featurize-gnn-nodes".to_string(),
            edges_stage: "featurize-gnn-edges".to_string(),
            graphs_stage: "featurize-gnn-graphs".to_string(),
        }
    }
}
```

### 3.2 Public entry point

```rust
use std::path::Path;
use anyhow::Result;
use gat_core::Network;

pub fn featurize_gnn_dc(
    network: &Network,
    flows_parquet: &Path,
    output_root: &Path,
    partitions: &[String],
    cfg: &FeaturizeGnnConfig,
) -> Result<()>
```

Semantics:

* `network`: base grid topology and static attributes.
* `flows_parquet`: DC PF/OPF flows table (branch-level).
* `output_root`: base directory for outputs.
* `partitions`: columns used to partition Parquet files (typically `["graph_id"]` and maybe `["scenario_id"]`).
* `cfg`: grouping and stage names.

### 3.3 Implementation outline

In `featurize_gnn_dc`:

1. **Precompute static node/edge maps** (as in §2.2–2.3).

2. **Load flows** with Polars:

   ```rust
   use polars::prelude::*;

   let lf = LazyFrame::scan_parquet(flows_parquet.to_str().unwrap(), Default::default())?;
   let flows_df = lf.collect()?;

   // Expect at least: branch_id, flow_mw
   require_column(&flows_df, "branch_id")?;
   require_column(&flows_df, "flow_mw")?;
   ```

3. **Determine group columns**:

   ```rust
   let mut group_cols: Vec<&str> = Vec::new();
   if cfg.group_by_scenario && flows_df.get_column_names().iter().any(|c| c == "scenario_id") {
       group_cols.push("scenario_id");
   }
   if cfg.group_by_time && flows_df.get_column_names().iter().any(|c| c == "time") {
       group_cols.push("time");
   }
   ```

   * If `group_cols.is_empty()`: treat entire DF as one group.
   * Else: use Polars `groupby` to iterate groups.

4. **Iterate over groups and build GraphKey list**:

   * `graph_id` increments from 0 as you traverse groups.
   * For each group, extract optional `scenario_id` and `time` values (from the first row of group).

5. **Per-group: build dynamic edge features**

   For each `DataFrame` group `gdf`:

   * Build map `branch_id -> flow_mw`:

     ```rust
     let branch_ids = gdf.column("branch_id")?.i64()?;
     let flows = gdf.column("flow_mw")?.f64()?;
     let mut flow_map = HashMap::<i64, f64>::new();
     for (bid_opt, flow_opt) in branch_ids.into_iter().zip(flows.into_iter()) {
         if let (Some(bid), Some(flow)) = (bid_opt, flow_opt) {
             flow_map.insert(bid, flow);
         }
     }
     ```

   * For each static edge `e`:

     * Look up `f = flow_map.get(&e.branch_id).copied().unwrap_or(0.0)`.
     * Record `flow_mw` as dynamic feature for that edge in this graph.

6. **Build nodes & edges DataFrames per group**

   For each `GraphKey` / group:

   * **Nodes**:

     Build `Vec`s:

     ```rust
     let mut graph_id_col = Vec::new();
     let mut node_id_col = Vec::new();
     let mut bus_id_col = Vec::new();
     let mut name_col = Vec::new();
     let mut voltage_kv_col = Vec::new();
     let mut num_gens_col = Vec::new();
     let mut p_gen_mw_col = Vec::new();
     let mut num_loads_col = Vec::new();
     let mut p_load_mw_col = Vec::new();

     for (_bus_id, b) in bus_features.iter() {
         graph_id_col.push(graph_key.graph_id);
         node_id_col.push(b.node_id);
         bus_id_col.push(b.bus_id);
         name_col.push(b.name.clone());
         voltage_kv_col.push(b.voltage_kv);
         num_gens_col.push(b.num_gens);
         p_gen_mw_col.push(b.p_gen_mw);
         num_loads_col.push(b.num_loads);
         p_load_mw_col.push(b.p_load_mw);
     }

     let mut nodes_df = DataFrame::new(vec![
         Series::new("graph_id", graph_id_col),
         Series::new("node_id", node_id_col),
         Series::new("bus_id", bus_id_col),
         Series::new("name", name_col),
         Series::new("voltage_kv", voltage_kv_col),
         Series::new("num_gens", num_gens_col),
         Series::new("p_gen_mw", p_gen_mw_col),
         Series::new("num_loads", num_loads_col),
         Series::new("p_load_mw", p_load_mw_col),
     ])?;
     ```

     (You can add more columns like `q_gen_mvar`, `q_load_mvar`.)

   * **Edges**:

     ```rust
     let mut graph_id_col = Vec::new();
     let mut edge_id_col = Vec::new();
     let mut src_col = Vec::new();
     let mut dst_col = Vec::new();
     let mut branch_id_col = Vec::new();
     let mut resistance_col = Vec::new();
     let mut reactance_col = Vec::new();
     let mut flow_mw_col = Vec::new();

     for e in &edges {
         graph_id_col.push(graph_key.graph_id);
         edge_id_col.push(e.edge_id);
         src_col.push(e.src);
         dst_col.push(e.dst);
         branch_id_col.push(e.branch_id);
         resistance_col.push(e.resistance);
         reactance_col.push(e.reactance);
         let flow = flow_map.get(&e.branch_id).copied().unwrap_or(0.0);
         flow_mw_col.push(flow);
     }

     let mut edges_df = DataFrame::new(vec![
         Series::new("graph_id", graph_id_col),
         Series::new("edge_id", edge_id_col),
         Series::new("src", src_col),
         Series::new("dst", dst_col),
         Series::new("branch_id", branch_id_col),
         Series::new("resistance", resistance_col),
         Series::new("reactance", reactance_col),
         Series::new("flow_mw", flow_mw_col),
     ])?;
     ```

   * **Graph metadata**:

     Keep a `Vec<GraphMeta>` while looping, then build `graphs_df` after:

     ```rust
     struct GraphMeta {
         graph_id: i64,
         scenario_id: Option<String>,
         time: Option<DateTime<Utc>>,
         num_nodes: i64,
         num_edges: i64,
     }
     ```

     Later:

     ```rust
     let mut graphs_df = DataFrame::new(vec![
         Series::new("graph_id", graph_ids),
         Series::new("scenario_id", scenario_ids),
         Series::new("time", times),
         Series::new("num_nodes", num_nodes),
         Series::new("num_edges", num_edges),
     ])?;
     ```

7. **Persist** with `persist_dataframe`:

   In `gat-algo::io`, we currently use an `OutputStage` enum, but for this module you can either:

   * Add new variants, or
   * Pass literal stage strings.

   To keep consistency, extend `OutputStage`:

   ```rust
   pub enum OutputStage {
       PfDc,
       PfAc,
       OpfDc,
       OpfAc,
       Nminus1Dc,
       SeWls,
       AnalyticsPtdf,
       FeaturizeGnnNodes,
       FeaturizeGnnEdges,
       FeaturizeGnnGraphs,
   }

   impl OutputStage {
       pub fn as_str(&self) -> &'static str {
           match self {
               // ...
               OutputStage::FeaturizeGnnNodes => "featurize-gnn-nodes",
               OutputStage::FeaturizeGnnEdges => "featurize-gnn-edges",
               OutputStage::FeaturizeGnnGraphs => "featurize-gnn-graphs",
           }
       }
   }
   ```

   Then:

   ```rust
   use crate::io::persist_dataframe;
   use crate::OutputStage;

   persist_dataframe(
       &mut nodes_df,
       output_root,
       partitions,
       OutputStage::FeaturizeGnnNodes.as_str(),
   )?;

   persist_dataframe(
       &mut edges_df,
       output_root,
       partitions,
       OutputStage::FeaturizeGnnEdges.as_str(),
   )?;

   persist_dataframe(
       &mut graphs_df,
       output_root,
       partitions,
       OutputStage::FeaturizeGnnGraphs.as_str(),
   )?;
   ```

   Typical partitions: `["graph_id".to_string()]` or `["scenario_id".to_string(), "time".to_string()]`.

---

## 4. CLI integration: `gat featurize gnn`

### 4.1 CLI enum additions

In `crates/gat-cli/src/cli.rs`, add a new command group:

```rust
#[derive(Subcommand, Debug)]
pub enum FeaturizeCommands {
    /// Export graph features for GNN models (nodes/edges/graphs as Parquet)
    Gnn {
        /// Path to the grid data file (Arrow format)
        grid_file: String,

        /// Parquet file with branch flows (pf-dc, pf-ac, or opf output)
        #[arg(long)]
        flows: String,

        /// Output root directory for featurized data
        #[arg(short, long)]
        out: String,

        /// Partition columns for Parquet (comma separated, e.g. "graph_id,scenario_id")
        #[arg(long)]
        out_partitions: Option<String>,

        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,

        /// How to group flows into graphs: "auto" | "scenario" | "scenario_time" | "single"
        #[arg(long, default_value = "auto")]
        group_by: String,
    },
}
```

And hook into top-level `Commands`:

```rust
pub enum Commands {
    // ...
    Featurize {
        #[command(subcommand)]
        command: FeaturizeCommands,
    },
}
```

Update `crates/gat-cli/src/main.rs`:

```rust
use crate::commands::featurize as command_featurize;

// in match:
Some(Commands::Featurize { command }) => {
    run_and_log("featurize", || command_featurize::handle(command))
}
```

### 4.2 `commands/featurize` module

Create:

* `crates/gat-cli/src/commands/featurize/mod.rs`
* `crates/gat-cli/src/commands/featurize/gnn.rs`

`mod.rs`:

```rust
use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;

pub mod gnn;

pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    match command {
        FeaturizeCommands::Gnn { .. } => gnn::handle(command),
    }
}
```

### 4.3 `gnn.rs` handler

Patterned after `pf.rs` and `analytics/ptdf.rs`:

```rust
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;
use gat_core::solver::SolverKind; // only if you want solver, otherwise drop
use gat_io::importers;
use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    let FeaturizeCommands::Gnn {
        grid_file,
        flows,
        out,
        out_partitions,
        threads,
        group_by,
    } = command else {
        unreachable!()
    };

    let start = Instant::now();

    configure_threads(threads);

    let partitions = parse_partitions(out_partitions.as_ref());
    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();

    let (group_by_scenario, group_by_time) = match group_by.as_str() {
        "auto" => (true, true),
        "scenario" => (true, false),
        "scenario_time" => (true, true),
        "single" => (false, false),
        other => anyhow::bail!("unknown group_by mode '{}'", other),
    };

    let res = (|| -> Result<()> {
        let network = importers::load_grid_from_arrow(grid_file.as_str())?;
        let cfg = gat_algo::FeaturizeGnnConfig {
            group_by_scenario,
            group_by_time,
            ..Default::default()
        };
        gat_algo::featurize_gnn_dc(
            &network,
            Path::new(flows),
            Path::new(out),
            &partitions,
            &cfg,
        )
    })();

    record_run_timed(
        out,
        "featurize gnn",
        &[
            ("grid_file", grid_file),
            ("flows", flows),
            ("group_by", group_by),
            ("out_partitions", &partition_spec),
        ],
        start,
        &res,
    );
    res
}
```

---

## 5. Telemetry & `gat runs`

* Each `gat featurize gnn` invocation becomes a single `run` entry (via `record_run_timed`).
* Useful parameters to log:

  * `num_graphs`, `num_nodes`, `num_edges` (if returned by `featurize_gnn_dc` as a small `Summary` struct).

You can tweak `featurize_gnn_dc` to return:

```rust
pub struct FeaturizeSummary {
    pub num_graphs: usize,
    pub num_nodes: usize,
    pub num_edges: usize,
}
```

and log those.

---

## 6. Tests & fixtures

### 6.1 Unit tests in `gat-algo::featurize_gnn`

* Build a tiny synthetic network (like the test in `gat-core`):

  * 3 buses, 2 branches, 1 gen, 1 load.

* Construct an in-memory flows DataFrame:

  * `branch_id` = 0,1; `flow_mw` = nonzero values.
  * optionally `scenario_id` and `time`.

* Call an internal function like `featurize_gnn_dc_into_dfs(network, flows_df, cfg)` that returns `(nodes_df, edges_df, graphs_df)` without hitting disk.

* Assertions:

  * `nodes_df.height() == num_buses`.
  * `edges_df.height() == num_branches`.
  * `graphs_df.height() == num_graphs` (1 or more depending on grouping).
  * `edge.src`/`dst` indices are in 0..num_nodes.
  * `edge.flow_mw` matches `flows_df` values by `branch_id`.

### 6.2 Integration tests in `gat-cli`

Under `crates/gat-cli/tests/featurize_gnn.rs`:

1. Use an existing MATPOWER example (case9) converted to Arrow in `test_data`.

2. Run DC PF:

   ```rust
   let mut cmd_pf = assert_cmd::Command::cargo_bin("gat").unwrap();
   cmd_pf.args([
       "pf", "dc",
       "--grid-file", grid_path,
       "--out", pf_out,
   ]);
   cmd_pf.assert().success();
   ```

3. Run featurize:

   ```rust
   let mut cmd_feat = assert_cmd::Command::cargo_bin("gat").unwrap();
   cmd_feat.args([
       "featurize", "gnn",
       "--grid-file", grid_path,
       "--flows", pf_out,
       "--out", out_dir,
   ]);
   cmd_feat.assert().success();
   ```

4. Check that:

   * `out_dir/featurize-gnn-nodes/*.parquet` exists.
   * `out_dir/featurize-gnn-edges/*.parquet` exists.
   * `out_dir/featurize-gnn-graphs/*.parquet` exists.

5. Optionally load Parquet with Polars and assert:

   * For each `graph_id`, node and edge counts match what you expect.
   * `branch_id` coverage in edges equals those in PF output.

---

## 7. Docs & examples

### 7.1 README section

Add something like:

````markdown
### GNN-ready graph features (`gat featurize gnn`)

`gat featurize gnn` exports graph representations suitable for GNN models
(e.g. PyTorch Geometric, DGL). It uses the same `Network` topology used
for power flow and PTDF, and joins in branch flow results as dynamic
edge features.

Example:

```bash
# 1. Run DC power flow to produce branch flows
gat pf dc \
  --grid-file data/rts.arrow \
  --out runs/rts_pf_dc.parquet

# 2. Featurize as graphs/nodes/edges for GNNs
gat featurize gnn \
  --grid-file data/rts.arrow \
  --flows runs/rts_pf_dc.parquet \
  --out runs/rts_gnn

# Nodes: runs/rts_gnn/featurize-gnn-nodes/...
# Edges: runs/rts_gnn/featurize-gnn-edges/...
# Graph meta: runs/rts_gnn/featurize-gnn-graphs/...
````

Each graph corresponds to a single scenario / time slice if those
columns are present in the flow results. If not, all flows are treated
as a single graph.

```

### 7.2 Short design doc in `docs/`

A one-pager:

- Describe the mapping `Network → (nodes, edges)` and which features are included.
- Describe how scenario/time grouping works.
- Show minimal PyTorch Geometric snippet converting Parquet to tensors.

---

## 8. Bead-sized tasks for agents

1. **Add `featurize_gnn.rs` module** with:
   - `FeaturizeGnnConfig`.
   - Static node/edge feature extraction (no flows yet).

2. **Wire `featurize_gnn_dc` to load flows, group by scenario/time, and compute dynamic edge features.**

3. **Extend `OutputStage` (or use literal stage strings) and call `persist_dataframe` for nodes/edges/graphs.**

4. **Create `FeaturizeCommands::Gnn` in `cli.rs` and `commands::featurize::gnn::handle` wired to `gat_algo::featurize_gnn_dc`.**

5. **Unit tests in `gat-algo`** for:
   - node/edge indexing,
   - flows → edge `flow_mw`.

6. **Integration test in `gat-cli`**:
   - run `pf dc` then `featurize gnn` on a tiny case.

7. **Update README/docs** with usage examples and note that this is the recommended path for building Power-GNN datasets.

---

If you want, the next step could be to lock down a **specific column schema** (names/dtypes) for nodes/edges/graphs so you can treat it as a contract for Power-GNN code and tests.
```

