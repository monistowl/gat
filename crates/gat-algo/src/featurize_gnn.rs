use crate::io::persist_dataframe;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use gat_core::{BusId, Edge, Network, Node};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration for GNN featurization: grouping behavior and output stage names.
///
/// **Purpose:** Controls how flows are grouped into distinct graphs (by scenario/time)
/// and where output Parquet files are written. This enables Power-GNN models to consume
/// graph-structured data with both static topology features and dynamic flow features.
///
/// **Graph grouping:** Each unique (scenario_id, time) combination becomes a separate
/// graph instance, allowing GNNs to learn from multiple operating conditions.
#[derive(Debug, Clone)]
pub struct FeaturizeGnnConfig {
    /// If true, treat each distinct scenario_id as a separate graph (if present in flows)
    pub group_by_scenario: bool,
    /// If true, treat each distinct time as a separate graph (if present in flows)
    pub group_by_time: bool,
    /// Stage name for node feature output (subdirectory name)
    pub nodes_stage: String,
    /// Stage name for edge feature output (subdirectory name)
    pub edges_stage: String,
    /// Stage name for graph metadata output (subdirectory name)
    pub graphs_stage: String,
}

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

/// Key identifying a unique graph instance (scenario/time combination).
///
/// **Purpose:** Each graph represents one operating condition snapshot. Multiple graphs
/// allow GNNs to learn patterns across different scenarios (e.g., N-1 contingencies)
/// and time periods (e.g., peak vs. off-peak load).
#[derive(Debug, Clone)]
struct GraphKey {
    graph_id: i64,
    scenario_id: Option<String>,
    time: Option<DateTime<Utc>>,
}

/// Static node features extracted from network topology.
///
/// **Features:**
/// - `node_id`: Contiguous index (0..N-1) for GNN node ordering
/// - `bus_id`: Original GAT bus identifier
/// - `voltage_kv`: Nominal voltage level
/// - Aggregated generator/load statistics per bus
///
/// These features are topology-invariant and reused across all graph instances.
struct NodeStaticFeatures {
    node_id: i64,
    bus_id: i64,
    name: String,
    voltage_kv: f64,
    num_gens: i64,
    p_gen_mw: f64,
    q_gen_mvar: f64,
    num_loads: i64,
    p_load_mw: f64,
    q_load_mvar: f64,
}

/// Static edge features extracted from network topology.
///
/// **Features:**
/// - `edge_id`: Contiguous index (0..M-1) for GNN edge ordering
/// - `branch_id`: Original GAT branch identifier
/// - `src`, `dst`: Node indices (0..N-1) for adjacency construction
/// - `resistance`, `reactance`: Branch impedance parameters
///
/// These features are topology-invariant and reused across all graph instances.
struct EdgeStaticFeatures {
    edge_id: i64,
    branch_id: i64,
    src: i64,
    dst: i64,
    resistance: f64,
    reactance: f64,
}

/// Export grid topology and flow data as GNN-ready graph features.
///
/// **Algorithm:**
/// 1. Extract static node/edge features from network topology (voltage, impedance, etc.)
/// 2. Load branch flows from PF/OPF results (Parquet)
/// 3. Group flows by (scenario_id, time) to create distinct graph instances
/// 4. For each graph, combine static topology with dynamic flow features
/// 5. Write three Parquet tables: nodes, edges, graphs
///
/// **Output format:**
/// - **Nodes table**: One row per (graph_id, node_id) with static + dynamic features
/// - **Edges table**: One row per (graph_id, edge_id) with static + dynamic features (flow_mw)
/// - **Graphs table**: One row per graph_id with metadata (scenario_id, time, counts)
///
/// This format is compatible with PyTorch Geometric, DGL, and other GNN frameworks.
/// See doi:10.1109/TPWRS.2020.3041234 for graph neural networks in power systems.
///
/// **Inputs:**
/// - `network`: Base grid topology (buses, branches, generators, loads)
/// - `flows_parquet`: Branch flows from DC/AC PF or OPF (must have branch_id, flow_mw)
/// - `output_root`: Base directory for output Parquet files
/// - `partitions`: Partition columns for Parquet (e.g., ["graph_id", "scenario_id"])
/// - `cfg`: Grouping configuration and output stage names
pub fn featurize_gnn_dc(
    network: &Network,
    flows_parquet: &Path,
    output_root: &Path,
    partitions: &[String],
    cfg: &FeaturizeGnnConfig,
) -> Result<()> {
    // Step 1: Precompute static node features from network topology
    // These are invariant across scenarios/time and represent the base grid structure
    let (node_features, bus_id_to_node_idx) = extract_node_features(network)?;

    // Step 2: Precompute static edge features from network topology
    // These represent branch impedance parameters and connectivity
    let (edge_features, _branch_id_to_edge_idx) =
        extract_edge_features(network, &bus_id_to_node_idx)?;

    // Step 3: Load branch flows from PF/OPF results
    // Expected schema: branch_id, flow_mw, optionally scenario_id, time
    let flows_df = LazyFrame::scan_parquet(flows_parquet.to_str().unwrap(), Default::default())?
        .collect()
        .context("loading flows parquet for GNN featurization")?;

    // Validate required columns
    if !flows_df.get_column_names().contains(&"branch_id") {
        return Err(anyhow!("flows parquet must contain 'branch_id' column"));
    }
    if !flows_df.get_column_names().contains(&"flow_mw") {
        return Err(anyhow!("flows parquet must contain 'flow_mw' column"));
    }

    // Step 4: Determine grouping columns (scenario_id, time)
    // Each unique combination becomes a separate graph instance
    let group_cols = determine_group_columns(&flows_df, cfg)?;

    // Step 5: Group flows and create graph keys
    let graph_keys = create_graph_keys(&flows_df, &group_cols)?;

    if graph_keys.is_empty() {
        return Err(anyhow!("no graph instances found in flows data"));
    }

    // Step 6: Build DataFrames for nodes, edges, and graphs
    let (nodes_df, edges_df, graphs_df) = build_feature_dataframes(
        &node_features,
        &edge_features,
        &graph_keys,
        &flows_df,
        &group_cols,
    )?;

    // Step 7: Persist to Parquet with optional partitioning
    persist_dataframe(
        &mut nodes_df.clone(),
        output_root,
        partitions,
        &cfg.nodes_stage,
    )?;
    persist_dataframe(
        &mut edges_df.clone(),
        output_root,
        partitions,
        &cfg.edges_stage,
    )?;
    persist_dataframe(
        &mut graphs_df.clone(),
        output_root,
        partitions,
        &cfg.graphs_stage,
    )?;

    println!(
        "GNN features: {} graphs, {} nodes, {} edges -> {}",
        graph_keys.len(),
        node_features.len(),
        edge_features.len(),
        output_root.display()
    );

    Ok(())
}

/// Extract static node features from network topology.
///
/// **Algorithm:**
/// 1. Build mapping from bus_id to contiguous node_id (0..N-1)
/// 2. For each bus, aggregate generator/load statistics
/// 3. Extract voltage level and other bus attributes
///
/// **Returns:** (node_features, bus_id_to_node_idx mapping)
fn extract_node_features(
    network: &Network,
) -> Result<(Vec<NodeStaticFeatures>, HashMap<BusId, i64>)> {
    let mut node_features = Vec::new();
    let mut bus_id_to_node_idx = HashMap::new();

    // First pass: collect all buses and assign contiguous node indices
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            let node_id = node_features.len() as i64;
            bus_id_to_node_idx.insert(bus.id, node_id);
            node_features.push(NodeStaticFeatures {
                node_id,
                bus_id: bus.id.value() as i64,
                name: bus.name.clone(),
                voltage_kv: bus.base_kv.value(),
                num_gens: 0,
                p_gen_mw: 0.0,
                q_gen_mvar: 0.0,
                num_loads: 0,
                p_load_mw: 0.0,
                q_load_mvar: 0.0,
            });
        }
    }

    // Second pass: aggregate generator and load statistics per bus
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Gen(gen) => {
                if let Some(node_feat) = node_features
                    .iter_mut()
                    .find(|n| n.bus_id == gen.bus.value() as i64)
                {
                    node_feat.num_gens += 1;
                    node_feat.p_gen_mw += gen.active_power.value();
                    node_feat.q_gen_mvar += gen.reactive_power.value();
                }
            }
            Node::Load(load) => {
                if let Some(node_feat) = node_features
                    .iter_mut()
                    .find(|n| n.bus_id == load.bus.value() as i64)
                {
                    node_feat.num_loads += 1;
                    node_feat.p_load_mw += load.active_power.value();
                    node_feat.q_load_mvar += load.reactive_power.value();
                }
            }
            _ => {}
        }
    }

    Ok((node_features, bus_id_to_node_idx))
}

/// Extract static edge features from network topology.
///
/// **Algorithm:**
/// 1. Iterate over all branch edges in the graph
/// 2. Map branch endpoints to node indices (using bus_id_to_node_idx)
/// 3. Extract impedance parameters (resistance, reactance)
/// 4. Assign contiguous edge_id (0..M-1)
///
/// **Returns:** (edge_features, branch_id_to_edge_idx mapping)
fn extract_edge_features(
    network: &Network,
    bus_id_to_node_idx: &HashMap<BusId, i64>,
) -> Result<(Vec<EdgeStaticFeatures>, HashMap<i64, i64>)> {
    let mut edge_features = Vec::new();
    let mut branch_id_to_edge_idx = HashMap::new();

    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            // Map branch endpoints to node indices
            let src_node = bus_id_to_node_idx
                .get(&branch.from_bus)
                .copied()
                .ok_or_else(|| {
                    anyhow!(
                        "branch {} references unknown from_bus {}",
                        branch.id.value(),
                        branch.from_bus.value()
                    )
                })?;
            let dst_node = bus_id_to_node_idx
                .get(&branch.to_bus)
                .copied()
                .ok_or_else(|| {
                    anyhow!(
                        "branch {} references unknown to_bus {}",
                        branch.id.value(),
                        branch.to_bus.value()
                    )
                })?;

            let edge_id = edge_features.len() as i64;
            let branch_id = branch.id.value() as i64;

            edge_features.push(EdgeStaticFeatures {
                edge_id,
                branch_id,
                src: src_node,
                dst: dst_node,
                resistance: branch.resistance,
                reactance: branch.reactance,
            });

            branch_id_to_edge_idx.insert(branch_id, edge_id);
        }
    }

    Ok((edge_features, branch_id_to_edge_idx))
}

/// Determine grouping columns based on flows DataFrame and configuration.
///
/// **Purpose:** Identifies which columns (scenario_id, time) should be used to group
/// flows into distinct graph instances. If no grouping columns exist, all flows become
/// a single graph.
///
/// **Returns:** Vector of column names to use for grouping
fn determine_group_columns(flows_df: &DataFrame, cfg: &FeaturizeGnnConfig) -> Result<Vec<String>> {
    let mut group_cols = Vec::new();
    let column_names: Vec<&str> = flows_df.get_column_names();

    if cfg.group_by_scenario && column_names.contains(&"scenario_id") {
        group_cols.push("scenario_id".to_string());
    }
    if cfg.group_by_time && column_names.contains(&"time") {
        group_cols.push("time".to_string());
    }

    Ok(group_cols)
}

/// Create graph keys by grouping flows DataFrame.
///
/// **Algorithm:**
/// 1. If group_cols is empty, treat entire DataFrame as one graph (graph_id = 0)
/// 2. Otherwise, use Polars group_by to iterate over groups
/// 3. For each group, extract scenario_id/time and assign sequential graph_id
///
/// **Returns:** Vector of GraphKey, one per unique (scenario_id, time) combination
fn create_graph_keys(flows_df: &DataFrame, group_cols: &[String]) -> Result<Vec<GraphKey>> {
    let mut graph_keys = Vec::new();

    if group_cols.is_empty() {
        // Single graph: all flows belong to one graph instance
        graph_keys.push(GraphKey {
            graph_id: 0,
            scenario_id: None,
            time: None,
        });
    } else {
        // Multiple graphs: group by scenario_id and/or time
        let group_by = flows_df.group_by(group_cols)?;
        let groups = group_by.get_groups();

        for (graph_id, group) in groups.iter().enumerate() {
            let first_row_idx = match group {
                GroupsIndicator::Idx((first, _)) => first,
                GroupsIndicator::Slice([first, _]) => first,
            };

            // Extract scenario_id and time from first row of group
            let scenario_id = flows_df
                .column("scenario_id")
                .ok()
                .and_then(|col| col.get(first_row_idx as usize).ok())
                .and_then(|val| {
                    let s = val.to_string();
                    if s == "null" {
                        None
                    } else {
                        Some(s)
                    }
                });

            let time = flows_df.column("time").ok().and_then(|col| {
                col.get(first_row_idx as usize).ok().and_then(|val| {
                    let s = val.to_string();
                    if s == "null" {
                        None
                    } else {
                        // Try to parse as DateTime (RFC3339 or ISO8601)
                        s.parse::<DateTime<Utc>>().ok()
                    }
                })
            });

            graph_keys.push(GraphKey {
                graph_id: graph_id as i64,
                scenario_id,
                time,
            });
        }
    }

    Ok(graph_keys)
}

/// Build feature DataFrames for nodes, edges, and graphs.
///
/// **Algorithm:**
/// 1. For each graph key, extract flows for that graph's group
/// 2. Build node DataFrame: static features repeated for each graph
/// 3. Build edge DataFrame: static features + dynamic flow_mw per graph
/// 4. Build graph DataFrame: metadata (scenario_id, time, counts)
///
/// **Returns:** (nodes_df, edges_df, graphs_df)
fn build_feature_dataframes(
    node_features: &[NodeStaticFeatures],
    edge_features: &[EdgeStaticFeatures],
    graph_keys: &[GraphKey],
    flows_df: &DataFrame,
    group_cols: &[String],
) -> Result<(DataFrame, DataFrame, DataFrame)> {
    let num_graphs = graph_keys.len();
    let mut all_nodes = Vec::with_capacity(num_graphs * node_features.len());
    let mut all_edges = Vec::with_capacity(num_graphs * edge_features.len());
    let mut all_graphs = Vec::with_capacity(num_graphs);

    // Extract flows grouped by graph
    let flows_by_graph = if group_cols.is_empty() {
        // Single graph: use entire flows DataFrame
        vec![(0, flows_df.clone())]
    } else {
        // Multiple graphs: group by scenario_id/time
        let group_by = flows_df.group_by(group_cols)?;
        let groups = group_by.get_groups();
        let mut flows_map = Vec::with_capacity(groups.len());

        for (graph_idx, group) in groups.iter().enumerate() {
            let group_df = match group {
                GroupsIndicator::Idx((_first, idx_vec)) => {
                    // Extract rows using IdxVec directly
                    let idx_ca = IdxCa::new("row_idx", idx_vec.as_slice());
                    flows_df.take(&idx_ca)?
                }
                GroupsIndicator::Slice([first, len]) => {
                    // Extract rows using slice
                    flows_df.slice(first as i64, len as usize)
                }
            };
            flows_map.push((graph_idx, group_df));
        }
        flows_map
    };

    // Build flow maps: branch_id -> flow_mw for each graph
    let mut flow_maps = Vec::with_capacity(flows_by_graph.len());
    for (graph_idx, group_df) in &flows_by_graph {
        let mut flow_map = HashMap::with_capacity(group_df.height());
        let branch_col = group_df.column("branch_id")?.i64()?;
        let flow_col = group_df.column("flow_mw")?.f64()?;

        for idx in 0..group_df.height() {
            if let (Some(branch_id), Some(flow)) = (branch_col.get(idx), flow_col.get(idx)) {
                flow_map.insert(branch_id, flow);
            }
        }
        flow_maps.push((*graph_idx, flow_map));
    }

    // Build nodes DataFrame: static features repeated for each graph
    for graph_key in graph_keys {
        for node_feat in node_features {
            all_nodes.push((
                graph_key.graph_id,
                node_feat.node_id,
                node_feat.bus_id,
                node_feat.name.clone(),
                node_feat.voltage_kv,
                node_feat.num_gens,
                node_feat.p_gen_mw,
                node_feat.q_gen_mvar,
                node_feat.num_loads,
                node_feat.p_load_mw,
                node_feat.q_load_mvar,
            ));
        }
    }

    // Build edges DataFrame: static features + dynamic flow_mw per graph
    for graph_key in graph_keys {
        // Find flow map for this graph, or use empty map if not found
        let empty_map = HashMap::new();
        let flow_map = flow_maps
            .iter()
            .find(|(idx, _)| *idx == graph_key.graph_id as usize)
            .map(|(_, map)| map)
            .unwrap_or(&empty_map);

        for edge_feat in edge_features {
            let flow_mw = flow_map.get(&edge_feat.branch_id).copied().unwrap_or(0.0);
            all_edges.push((
                graph_key.graph_id,
                edge_feat.edge_id,
                edge_feat.src,
                edge_feat.dst,
                edge_feat.branch_id,
                edge_feat.resistance,
                edge_feat.reactance,
                flow_mw,
            ));
        }
    }

    // Build graphs DataFrame: metadata per graph
    for graph_key in graph_keys {
        all_graphs.push((
            graph_key.graph_id,
            graph_key.scenario_id.clone(),
            graph_key.time.map(|t| t.to_rfc3339()),
            node_features.len() as i64,
            edge_features.len() as i64,
        ));
    }

    // Construct DataFrames
    let nodes_df = DataFrame::new(vec![
        Series::new(
            "graph_id",
            all_nodes.iter().map(|n| n.0).collect::<Vec<_>>(),
        ),
        Series::new("node_id", all_nodes.iter().map(|n| n.1).collect::<Vec<_>>()),
        Series::new("bus_id", all_nodes.iter().map(|n| n.2).collect::<Vec<_>>()),
        Series::new(
            "name",
            all_nodes.iter().map(|n| n.3.clone()).collect::<Vec<_>>(),
        ),
        Series::new(
            "voltage_kv",
            all_nodes.iter().map(|n| n.4).collect::<Vec<_>>(),
        ),
        Series::new(
            "num_gens",
            all_nodes.iter().map(|n| n.5).collect::<Vec<_>>(),
        ),
        Series::new(
            "p_gen_mw",
            all_nodes.iter().map(|n| n.6).collect::<Vec<_>>(),
        ),
        Series::new(
            "q_gen_mvar",
            all_nodes.iter().map(|n| n.7).collect::<Vec<_>>(),
        ),
        Series::new(
            "num_loads",
            all_nodes.iter().map(|n| n.8).collect::<Vec<_>>(),
        ),
        Series::new(
            "p_load_mw",
            all_nodes.iter().map(|n| n.9).collect::<Vec<_>>(),
        ),
        Series::new(
            "q_load_mvar",
            all_nodes.iter().map(|n| n.10).collect::<Vec<_>>(),
        ),
    ])?;

    let edges_df = DataFrame::new(vec![
        Series::new(
            "graph_id",
            all_edges.iter().map(|e| e.0).collect::<Vec<_>>(),
        ),
        Series::new("edge_id", all_edges.iter().map(|e| e.1).collect::<Vec<_>>()),
        Series::new("src", all_edges.iter().map(|e| e.2).collect::<Vec<_>>()),
        Series::new("dst", all_edges.iter().map(|e| e.3).collect::<Vec<_>>()),
        Series::new(
            "branch_id",
            all_edges.iter().map(|e| e.4).collect::<Vec<_>>(),
        ),
        Series::new(
            "resistance",
            all_edges.iter().map(|e| e.5).collect::<Vec<_>>(),
        ),
        Series::new(
            "reactance",
            all_edges.iter().map(|e| e.6).collect::<Vec<_>>(),
        ),
        Series::new("flow_mw", all_edges.iter().map(|e| e.7).collect::<Vec<_>>()),
    ])?;

    let graphs_df = DataFrame::new(vec![
        Series::new(
            "graph_id",
            all_graphs.iter().map(|g| g.0).collect::<Vec<_>>(),
        ),
        Series::new(
            "scenario_id",
            all_graphs.iter().map(|g| g.1.clone()).collect::<Vec<_>>(),
        ),
        Series::new(
            "time",
            all_graphs.iter().map(|g| g.2.clone()).collect::<Vec<_>>(),
        ),
        Series::new(
            "num_nodes",
            all_graphs.iter().map(|g| g.3).collect::<Vec<_>>(),
        ),
        Series::new(
            "num_edges",
            all_graphs.iter().map(|g| g.4).collect::<Vec<_>>(),
        ),
    ])?;

    Ok((nodes_df, edges_df, graphs_df))
}

// =============================================================================
// JSON Output Formats (NeurIPS PowerGraph + PyTorch Geometric)
// =============================================================================

/// Output format enumeration for GNN featurization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GnnOutputFormat {
    /// GAT native Arrow/Parquet format
    #[default]
    Arrow,
    /// NeurIPS PowerGraph benchmark JSON format
    NeuripsJson,
    /// PyTorch Geometric compatible JSON format
    PytorchGeometric,
}

/// NeurIPS PowerGraph JSON format structure.
///
/// Matches the format from the PowerGraph NeurIPS 2024 benchmark:
/// - Node features: `[N, F_node]` array (P_net, S_net, V, etc.)
/// - Edge features: `[E, F_edge]` array (P_flow, Q_flow, reactance, etc.)
/// - Edge index: COO format `[[src...], [dst...]]`
/// - Label: single value (binary, regression, or multiclass index)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuripsGraphJson {
    /// Number of nodes in the graph
    pub num_nodes: usize,
    /// Number of edges in the graph
    pub num_edges: usize,
    /// Node features as `[N, F_node]` array
    pub node_features: Vec<Vec<f64>>,
    /// Edge features as `[E, F_edge]` array
    pub edge_features: Vec<Vec<f64>>,
    /// Edge index in COO format: `[[src indices], [dst indices]]`
    pub edge_index: [Vec<usize>; 2],
    /// Graph-level label (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<serde_json::Value>,
    /// Metadata (scenario_id, time, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// PyTorch Geometric JSON format structure.
///
/// Compatible with PyG's `torch_geometric.data.Data` when loaded:
/// ```python
/// import torch
/// data = Data(
///     x=torch.tensor(json['x']),
///     edge_index=torch.tensor(json['edge_index']),
///     edge_attr=torch.tensor(json['edge_attr']),
///     y=torch.tensor([json['y']]) if 'y' in json else None,
/// )
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PytorchGeometricJson {
    /// Node features `[N, F]` (PyG field: `x`)
    pub x: Vec<Vec<f64>>,
    /// Edge index in COO format `[2, E]` (PyG field: `edge_index`)
    pub edge_index: [Vec<usize>; 2],
    /// Edge attributes `[E, D]` (PyG field: `edge_attr`)
    pub edge_attr: Vec<Vec<f64>>,
    /// Graph-level target (optional, PyG field: `y`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<serde_json::Value>,
    /// Number of nodes
    pub num_nodes: usize,
}

/// GNN graph sample with all features extracted.
///
/// Internal representation that can be converted to any output format.
#[derive(Debug, Clone)]
pub struct GnnGraphSample {
    pub graph_id: i64,
    pub scenario_id: Option<String>,
    pub time: Option<String>,
    pub num_nodes: usize,
    pub num_edges: usize,
    /// Node features: each row is [voltage_kv, p_gen_mw, q_gen_mvar, p_load_mw, q_load_mvar, num_gens, num_loads]
    pub node_features: Vec<Vec<f64>>,
    /// Edge features: each row is [resistance, reactance, flow_mw]
    pub edge_features: Vec<Vec<f64>>,
    /// Edge index in COO format: (src_indices, dst_indices)
    pub edge_index: (Vec<usize>, Vec<usize>),
}

impl GnnGraphSample {
    /// Convert to NeurIPS PowerGraph JSON format.
    pub fn to_neurips_json(&self) -> NeuripsGraphJson {
        NeuripsGraphJson {
            num_nodes: self.num_nodes,
            num_edges: self.num_edges,
            node_features: self.node_features.clone(),
            edge_features: self.edge_features.clone(),
            edge_index: [self.edge_index.0.clone(), self.edge_index.1.clone()],
            label: None, // Labels would come from external source
            metadata: Some(serde_json::json!({
                "graph_id": self.graph_id,
                "scenario_id": self.scenario_id,
                "time": self.time,
            })),
        }
    }

    /// Convert to PyTorch Geometric JSON format.
    pub fn to_pytorch_geometric_json(&self) -> PytorchGeometricJson {
        PytorchGeometricJson {
            x: self.node_features.clone(),
            edge_index: [self.edge_index.0.clone(), self.edge_index.1.clone()],
            edge_attr: self.edge_features.clone(),
            y: None, // Labels would come from external source
            num_nodes: self.num_nodes,
        }
    }

    /// Create a GnnGraphSample from PyTorch Geometric JSON format.
    ///
    /// This enables round-trip testing: export to PyG → import back → verify equivalence.
    /// Note: graph_id, scenario_id, and time are set to defaults since PyG format
    /// doesn't carry this metadata.
    pub fn from_pytorch_geometric_json(pyg: &PytorchGeometricJson) -> Self {
        Self {
            graph_id: 0,
            scenario_id: None,
            time: None,
            num_nodes: pyg.num_nodes,
            num_edges: pyg.edge_index[0].len(),
            node_features: pyg.x.clone(),
            edge_features: pyg.edge_attr.clone(),
            edge_index: (pyg.edge_index[0].clone(), pyg.edge_index[1].clone()),
        }
    }

    /// Create a GnnGraphSample from NeurIPS PowerGraph JSON format.
    ///
    /// This enables round-trip testing: export to NeurIPS → import back → verify equivalence.
    /// Extracts graph_id, scenario_id, and time from metadata if present.
    pub fn from_neurips_json(neurips: &NeuripsGraphJson) -> Self {
        // Extract metadata fields if present
        let (graph_id, scenario_id, time) = if let Some(ref meta) = neurips.metadata {
            let gid = meta.get("graph_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let sid = meta
                .get("scenario_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let t = meta
                .get("time")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (gid, sid, t)
        } else {
            (0, None, None)
        };

        Self {
            graph_id,
            scenario_id,
            time,
            num_nodes: neurips.num_nodes,
            num_edges: neurips.num_edges,
            node_features: neurips.node_features.clone(),
            edge_features: neurips.edge_features.clone(),
            edge_index: (neurips.edge_index[0].clone(), neurips.edge_index[1].clone()),
        }
    }

    /// Validate that the graph sample is well-formed.
    ///
    /// Checks:
    /// - Edge index bounds: all src/dst indices must be < num_nodes
    /// - Dimension consistency: node_features.len() == num_nodes, edge_features.len() == num_edges
    /// - Feature width consistency: all node feature vectors have same length
    /// - Feature width consistency: all edge feature vectors have same length
    pub fn validate(&self) -> Result<()> {
        // Check node count matches
        if self.node_features.len() != self.num_nodes {
            return Err(anyhow!(
                "node_features.len() ({}) != num_nodes ({})",
                self.node_features.len(),
                self.num_nodes
            ));
        }

        // Check edge count matches
        if self.edge_features.len() != self.num_edges {
            return Err(anyhow!(
                "edge_features.len() ({}) != num_edges ({})",
                self.edge_features.len(),
                self.num_edges
            ));
        }

        // Check edge index lengths match
        if self.edge_index.0.len() != self.num_edges || self.edge_index.1.len() != self.num_edges {
            return Err(anyhow!(
                "edge_index lengths ({}, {}) != num_edges ({})",
                self.edge_index.0.len(),
                self.edge_index.1.len(),
                self.num_edges
            ));
        }

        // Check edge index bounds
        for (i, &src) in self.edge_index.0.iter().enumerate() {
            if src >= self.num_nodes {
                return Err(anyhow!(
                    "edge {} src index {} >= num_nodes {}",
                    i,
                    src,
                    self.num_nodes
                ));
            }
        }
        for (i, &dst) in self.edge_index.1.iter().enumerate() {
            if dst >= self.num_nodes {
                return Err(anyhow!(
                    "edge {} dst index {} >= num_nodes {}",
                    i,
                    dst,
                    self.num_nodes
                ));
            }
        }

        // Check node feature width consistency
        if !self.node_features.is_empty() {
            let width = self.node_features[0].len();
            for (i, nf) in self.node_features.iter().enumerate() {
                if nf.len() != width {
                    return Err(anyhow!(
                        "node {} has {} features, expected {}",
                        i,
                        nf.len(),
                        width
                    ));
                }
            }
        }

        // Check edge feature width consistency
        if !self.edge_features.is_empty() {
            let width = self.edge_features[0].len();
            for (i, ef) in self.edge_features.iter().enumerate() {
                if ef.len() != width {
                    return Err(anyhow!(
                        "edge {} has {} features, expected {}",
                        i,
                        ef.len(),
                        width
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Export grid topology and flow data as GNN features with configurable output format.
///
/// This is the unified entry point that supports all output formats:
/// - Arrow/Parquet: Tabular format with partitioning (original behavior)
/// - NeurIPS JSON: One JSON file per graph, PowerGraph benchmark compatible
/// - PyTorch Geometric: One JSON file per graph, PyG Data compatible
pub fn featurize_gnn_with_format(
    network: &Network,
    flows_parquet: &Path,
    output_root: &Path,
    partitions: &[String],
    cfg: &FeaturizeGnnConfig,
    format: GnnOutputFormat,
) -> Result<()> {
    match format {
        GnnOutputFormat::Arrow => {
            // Use existing Parquet-based implementation
            featurize_gnn_dc(network, flows_parquet, output_root, partitions, cfg)
        }
        GnnOutputFormat::NeuripsJson | GnnOutputFormat::PytorchGeometric => {
            // Extract graph samples and write as JSON
            let samples = extract_gnn_samples(network, flows_parquet, cfg)?;

            // Create output directory
            fs::create_dir_all(output_root)
                .with_context(|| format!("creating output directory: {}", output_root.display()))?;

            // Write each graph as a separate JSON file
            for sample in &samples {
                let filename = if let Some(ref scenario_id) = sample.scenario_id {
                    format!("graph_{}_s{}.json", sample.graph_id, scenario_id)
                } else {
                    format!("graph_{}.json", sample.graph_id)
                };

                let path = output_root.join(&filename);
                let json_value = match format {
                    GnnOutputFormat::NeuripsJson => {
                        serde_json::to_value(sample.to_neurips_json())?
                    }
                    GnnOutputFormat::PytorchGeometric => {
                        serde_json::to_value(sample.to_pytorch_geometric_json())?
                    }
                    GnnOutputFormat::Arrow => unreachable!(),
                };

                let file = fs::File::create(&path)
                    .with_context(|| format!("creating JSON file: {}", path.display()))?;
                serde_json::to_writer_pretty(file, &json_value)
                    .with_context(|| format!("writing JSON to: {}", path.display()))?;
            }

            println!(
                "GNN features ({}): {} graphs -> {}",
                match format {
                    GnnOutputFormat::NeuripsJson => "NeurIPS JSON",
                    GnnOutputFormat::PytorchGeometric => "PyTorch Geometric",
                    GnnOutputFormat::Arrow => "Arrow",
                },
                samples.len(),
                output_root.display()
            );

            Ok(())
        }
    }
}

/// Extract GNN samples from network and flows without writing to disk.
///
/// Used internally by JSON output formats and for testing.
fn extract_gnn_samples(
    network: &Network,
    flows_parquet: &Path,
    cfg: &FeaturizeGnnConfig,
) -> Result<Vec<GnnGraphSample>> {
    // Step 1: Extract static features
    let (node_features, bus_id_to_node_idx) = extract_node_features(network)?;
    let (edge_features, _branch_id_to_edge_idx) =
        extract_edge_features(network, &bus_id_to_node_idx)?;

    // Step 2: Load flows
    let flows_df = LazyFrame::scan_parquet(flows_parquet.to_str().unwrap(), Default::default())?
        .collect()
        .context("loading flows parquet for GNN featurization")?;

    // Validate required columns
    if !flows_df.get_column_names().contains(&"branch_id") {
        return Err(anyhow!("flows parquet must contain 'branch_id' column"));
    }
    if !flows_df.get_column_names().contains(&"flow_mw") {
        return Err(anyhow!("flows parquet must contain 'flow_mw' column"));
    }

    // Step 3: Determine grouping and create graph keys
    let group_cols = determine_group_columns(&flows_df, cfg)?;
    let graph_keys = create_graph_keys(&flows_df, &group_cols)?;

    if graph_keys.is_empty() {
        return Err(anyhow!("no graph instances found in flows data"));
    }

    // Step 4: Build flow maps per graph
    let flows_by_graph = if group_cols.is_empty() {
        vec![(0, flows_df.clone())]
    } else {
        let group_by = flows_df.group_by(&group_cols)?;
        let groups = group_by.get_groups();
        let mut flows_map = Vec::with_capacity(groups.len());

        for (graph_idx, group) in groups.iter().enumerate() {
            let group_df = match group {
                GroupsIndicator::Idx((_first, idx_vec)) => {
                    let idx_ca = IdxCa::new("row_idx", idx_vec.as_slice());
                    flows_df.take(&idx_ca)?
                }
                GroupsIndicator::Slice([first, len]) => flows_df.slice(first as i64, len as usize),
            };
            flows_map.push((graph_idx, group_df));
        }
        flows_map
    };

    // Build flow maps: branch_id -> flow_mw
    let mut flow_maps: Vec<(usize, HashMap<i64, f64>)> = Vec::with_capacity(flows_by_graph.len());
    for (graph_idx, group_df) in &flows_by_graph {
        let mut flow_map = HashMap::with_capacity(group_df.height());
        let branch_col = group_df.column("branch_id")?.i64()?;
        let flow_col = group_df.column("flow_mw")?.f64()?;

        for idx in 0..group_df.height() {
            if let (Some(branch_id), Some(flow)) = (branch_col.get(idx), flow_col.get(idx)) {
                flow_map.insert(branch_id, flow);
            }
        }
        flow_maps.push((*graph_idx, flow_map));
    }

    // Step 5: Build GnnGraphSample for each graph
    let mut samples = Vec::with_capacity(graph_keys.len());

    for graph_key in &graph_keys {
        // Find flow map for this graph
        let empty_map = HashMap::new();
        let flow_map = flow_maps
            .iter()
            .find(|(idx, _)| *idx == graph_key.graph_id as usize)
            .map(|(_, map)| map)
            .unwrap_or(&empty_map);

        // Build node features: [voltage_kv, p_gen_mw, q_gen_mvar, p_load_mw, q_load_mvar, num_gens, num_loads]
        let node_feat_vecs: Vec<Vec<f64>> = node_features
            .iter()
            .map(|n| {
                vec![
                    n.voltage_kv,
                    n.p_gen_mw,
                    n.q_gen_mvar,
                    n.p_load_mw,
                    n.q_load_mvar,
                    n.num_gens as f64,
                    n.num_loads as f64,
                ]
            })
            .collect();

        // Build edge features: [resistance, reactance, flow_mw]
        let edge_feat_vecs: Vec<Vec<f64>> = edge_features
            .iter()
            .map(|e| {
                let flow_mw = flow_map.get(&e.branch_id).copied().unwrap_or(0.0);
                vec![e.resistance, e.reactance, flow_mw]
            })
            .collect();

        // Build edge index (COO format)
        let src_indices: Vec<usize> = edge_features.iter().map(|e| e.src as usize).collect();
        let dst_indices: Vec<usize> = edge_features.iter().map(|e| e.dst as usize).collect();

        samples.push(GnnGraphSample {
            graph_id: graph_key.graph_id,
            scenario_id: graph_key.scenario_id.clone(),
            time: graph_key.time.map(|t| t.to_rfc3339()),
            num_nodes: node_features.len(),
            num_edges: edge_features.len(),
            node_features: node_feat_vecs,
            edge_features: edge_feat_vecs,
            edge_index: (src_indices, dst_indices),
        });
    }

    Ok(samples)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gnn_graph_sample_to_neurips_json() {
        let sample = GnnGraphSample {
            graph_id: 0,
            scenario_id: Some("base".to_string()),
            time: Some("2024-01-01T00:00:00Z".to_string()),
            num_nodes: 3,
            num_edges: 2,
            node_features: vec![
                vec![138.0, 100.0, 50.0, 80.0, 30.0, 1.0, 2.0],
                vec![138.0, 50.0, 25.0, 40.0, 15.0, 0.0, 1.0],
                vec![69.0, 0.0, 0.0, 60.0, 25.0, 0.0, 1.0],
            ],
            edge_features: vec![
                vec![0.01, 0.1, 50.0],
                vec![0.02, 0.2, 30.0],
            ],
            edge_index: (vec![0, 1], vec![1, 2]),
        };

        let neurips = sample.to_neurips_json();

        assert_eq!(neurips.num_nodes, 3);
        assert_eq!(neurips.num_edges, 2);
        assert_eq!(neurips.node_features.len(), 3);
        assert_eq!(neurips.edge_features.len(), 2);
        assert_eq!(neurips.edge_index[0].len(), 2); // src indices
        assert_eq!(neurips.edge_index[1].len(), 2); // dst indices
        assert!(neurips.metadata.is_some());
    }

    #[test]
    fn test_gnn_graph_sample_to_pytorch_geometric_json() {
        let sample = GnnGraphSample {
            graph_id: 1,
            scenario_id: None,
            time: None,
            num_nodes: 2,
            num_edges: 1,
            node_features: vec![
                vec![138.0, 100.0, 50.0, 80.0, 30.0, 1.0, 2.0],
                vec![138.0, 50.0, 25.0, 40.0, 15.0, 0.0, 1.0],
            ],
            edge_features: vec![vec![0.01, 0.1, 50.0]],
            edge_index: (vec![0], vec![1]),
        };

        let pyg = sample.to_pytorch_geometric_json();

        assert_eq!(pyg.num_nodes, 2);
        assert_eq!(pyg.x.len(), 2);
        assert_eq!(pyg.edge_attr.len(), 1);
        assert_eq!(pyg.edge_index[0].len(), 1);
        assert_eq!(pyg.edge_index[1].len(), 1);
        assert_eq!(pyg.edge_index[0][0], 0); // src
        assert_eq!(pyg.edge_index[1][0], 1); // dst
    }

    #[test]
    fn test_neurips_json_serialization_roundtrip() {
        let neurips = NeuripsGraphJson {
            num_nodes: 3,
            num_edges: 2,
            node_features: vec![
                vec![1.0, 2.0, 3.0],
                vec![4.0, 5.0, 6.0],
                vec![7.0, 8.0, 9.0],
            ],
            edge_features: vec![vec![0.1, 0.2], vec![0.3, 0.4]],
            edge_index: [vec![0, 1], vec![1, 2]],
            label: Some(serde_json::json!(1)),
            metadata: Some(serde_json::json!({"source": "test"})),
        };

        let json_str = serde_json::to_string(&neurips).unwrap();
        let parsed: NeuripsGraphJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.num_nodes, neurips.num_nodes);
        assert_eq!(parsed.num_edges, neurips.num_edges);
        assert_eq!(parsed.node_features, neurips.node_features);
        assert_eq!(parsed.edge_features, neurips.edge_features);
        assert_eq!(parsed.edge_index, neurips.edge_index);
    }

    #[test]
    fn test_pytorch_geometric_json_serialization_roundtrip() {
        let pyg = PytorchGeometricJson {
            x: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            edge_index: [vec![0], vec![1]],
            edge_attr: vec![vec![0.5, 0.6, 0.7]],
            y: Some(serde_json::json!(0.5)),
            num_nodes: 2,
        };

        let json_str = serde_json::to_string(&pyg).unwrap();
        let parsed: PytorchGeometricJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.x, pyg.x);
        assert_eq!(parsed.edge_index, pyg.edge_index);
        assert_eq!(parsed.edge_attr, pyg.edge_attr);
        assert_eq!(parsed.num_nodes, pyg.num_nodes);
    }

    #[test]
    fn test_gnn_output_format_default() {
        let format = GnnOutputFormat::default();
        assert_eq!(format, GnnOutputFormat::Arrow);
    }

    // =============================================================================
    // Round-Trip Tests: GnnGraphSample ↔ JSON formats
    // =============================================================================

    /// Helper to create a realistic GnnGraphSample for testing.
    fn create_test_sample() -> GnnGraphSample {
        GnnGraphSample {
            graph_id: 42,
            scenario_id: Some("contingency_n1".to_string()),
            time: Some("2024-06-15T14:30:00Z".to_string()),
            num_nodes: 4,
            num_edges: 3,
            // Node features: [voltage_kv, p_gen_mw, q_gen_mvar, p_load_mw, q_load_mvar, num_gens, num_loads]
            node_features: vec![
                vec![138.0, 100.0, 50.0, 80.0, 30.0, 1.0, 2.0], // bus 0: gen + loads
                vec![138.0, 0.0, 0.0, 40.0, 15.0, 0.0, 1.0],    // bus 1: load only
                vec![69.0, 50.0, 25.0, 0.0, 0.0, 1.0, 0.0],     // bus 2: gen only
                vec![69.0, 0.0, 0.0, 60.0, 25.0, 0.0, 1.0],     // bus 3: load only
            ],
            // Edge features: [resistance, reactance, flow_mw]
            edge_features: vec![
                vec![0.01, 0.10, 45.5],  // branch 0→1
                vec![0.02, 0.15, -30.2], // branch 1→2
                vec![0.015, 0.12, 25.8], // branch 2→3
            ],
            edge_index: (vec![0, 1, 2], vec![1, 2, 3]),
        }
    }

    #[test]
    fn test_gnn_sample_pyg_roundtrip() {
        // Original sample
        let original = create_test_sample();

        // Export to PyG JSON
        let pyg_json = original.to_pytorch_geometric_json();

        // Serialize to string (simulating file write)
        let json_str = serde_json::to_string_pretty(&pyg_json).unwrap();

        // Deserialize back (simulating file read)
        let parsed_pyg: PytorchGeometricJson = serde_json::from_str(&json_str).unwrap();

        // Import back to GnnGraphSample
        let imported = GnnGraphSample::from_pytorch_geometric_json(&parsed_pyg);

        // Validate the imported sample
        imported.validate().expect("imported sample should be valid");

        // Check feature equivalence (PyG doesn't preserve metadata)
        assert_eq!(imported.num_nodes, original.num_nodes);
        assert_eq!(imported.num_edges, original.num_edges);
        assert_eq!(imported.node_features, original.node_features);
        assert_eq!(imported.edge_features, original.edge_features);
        assert_eq!(imported.edge_index, original.edge_index);
    }

    #[test]
    fn test_gnn_sample_neurips_roundtrip() {
        // Original sample
        let original = create_test_sample();

        // Export to NeurIPS JSON
        let neurips_json = original.to_neurips_json();

        // Serialize to string (simulating file write)
        let json_str = serde_json::to_string_pretty(&neurips_json).unwrap();

        // Deserialize back (simulating file read)
        let parsed_neurips: NeuripsGraphJson = serde_json::from_str(&json_str).unwrap();

        // Import back to GnnGraphSample
        let imported = GnnGraphSample::from_neurips_json(&parsed_neurips);

        // Validate the imported sample
        imported.validate().expect("imported sample should be valid");

        // Check full equivalence (NeurIPS preserves metadata)
        assert_eq!(imported.graph_id, original.graph_id);
        assert_eq!(imported.scenario_id, original.scenario_id);
        assert_eq!(imported.time, original.time);
        assert_eq!(imported.num_nodes, original.num_nodes);
        assert_eq!(imported.num_edges, original.num_edges);
        assert_eq!(imported.node_features, original.node_features);
        assert_eq!(imported.edge_features, original.edge_features);
        assert_eq!(imported.edge_index, original.edge_index);
    }

    #[test]
    fn test_gnn_sample_validation_valid() {
        let sample = create_test_sample();
        assert!(sample.validate().is_ok());
    }

    #[test]
    fn test_gnn_sample_validation_bad_node_count() {
        let mut sample = create_test_sample();
        sample.num_nodes = 10; // Wrong count
        assert!(sample.validate().is_err());
    }

    #[test]
    fn test_gnn_sample_validation_bad_edge_count() {
        let mut sample = create_test_sample();
        sample.num_edges = 10; // Wrong count
        assert!(sample.validate().is_err());
    }

    #[test]
    fn test_gnn_sample_validation_edge_index_out_of_bounds() {
        let mut sample = create_test_sample();
        sample.edge_index.0[0] = 100; // Out of bounds src index
        assert!(sample.validate().is_err());
    }

    #[test]
    fn test_gnn_sample_validation_inconsistent_node_features() {
        let mut sample = create_test_sample();
        sample.node_features[1] = vec![1.0, 2.0]; // Different width than others
        assert!(sample.validate().is_err());
    }

    #[test]
    fn test_gnn_sample_validation_inconsistent_edge_features() {
        let mut sample = create_test_sample();
        sample.edge_features[0] = vec![1.0]; // Different width than others
        assert!(sample.validate().is_err());
    }

    #[test]
    fn test_neurips_json_with_label_and_metadata() {
        let sample = create_test_sample();
        let mut neurips = sample.to_neurips_json();

        // Add a label (e.g., binary classification)
        neurips.label = Some(serde_json::json!(1));

        // Serialize and deserialize
        let json_str = serde_json::to_string(&neurips).unwrap();
        let parsed: NeuripsGraphJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.label, Some(serde_json::json!(1)));
        assert!(parsed.metadata.is_some());
    }

    #[test]
    fn test_pyg_json_with_target() {
        let sample = create_test_sample();
        let mut pyg = sample.to_pytorch_geometric_json();

        // Add a regression target
        pyg.y = Some(serde_json::json!(0.95));

        // Serialize and deserialize
        let json_str = serde_json::to_string(&pyg).unwrap();
        let parsed: PytorchGeometricJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.y, Some(serde_json::json!(0.95)));
    }
}
