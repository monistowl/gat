use crate::io::persist_dataframe;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use gat_core::{BusId, Edge, Network, Node};
use polars::prelude::*;
use std::collections::HashMap;
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

/// Graph metadata for each graph instance.
///
/// **Purpose:** Provides graph-level attributes for GNN batch construction and
/// dataset organization. Used by PyTorch Geometric, DGL, etc. to group nodes/edges.
#[allow(dead_code)]
struct GraphMeta {
    graph_id: i64,
    scenario_id: Option<String>,
    time: Option<DateTime<Utc>>,
    num_nodes: i64,
    num_edges: i64,
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
    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();
    let mut all_graphs = Vec::new();

    // Extract flows grouped by graph
    let flows_by_graph = if group_cols.is_empty() {
        // Single graph: use entire flows DataFrame
        vec![(0, flows_df.clone())]
    } else {
        // Multiple graphs: group by scenario_id/time
        let group_by = flows_df.group_by(group_cols)?;
        let groups = group_by.get_groups();
        let mut flows_map = Vec::new();

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
    let mut flow_maps = Vec::new();
    for (graph_idx, group_df) in &flows_by_graph {
        let mut flow_map = HashMap::new();
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
