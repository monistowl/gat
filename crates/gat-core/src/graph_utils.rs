use crate::Network;
use anyhow::{anyhow, Result};
use petgraph::algo::connected_components;
use petgraph::visit::EdgeRef;
use std::collections::{HashSet, VecDeque};

/// Summary statistics produced by `graph stats` (density/degree/connected components).
#[derive(Debug)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub connected_components: usize,
    pub min_degree: usize,
    pub avg_degree: f64,
    pub max_degree: usize,
    pub density: f64,
}

/// Island summary used in `graph islands` (standard components approach, see doi:10.1016/S0378-3758(96)00112-0).
#[derive(Debug)]
pub struct IslandSummary {
    pub island_id: usize,
    pub node_count: usize,
}

/// Node assignment info for `--emit` output so we can tag every bus with its component (useful for islanding analysis).
#[derive(Debug)]
pub struct NodeAssignment {
    pub node_index: usize,
    pub label: String,
    pub island_id: usize,
}

/// Aggregated island analysis result.
#[derive(Debug)]
pub struct IslandAnalysis {
    pub islands: Vec<IslandSummary>,
    pub assignments: Vec<NodeAssignment>,
}

/// Calculates graph-level statistics such as density, degree distribution, and component counts (classic network science measures).
pub fn graph_stats(network: &Network) -> Result<GraphStats> {
    let node_count = network.graph.node_count();
    let edge_count = network.graph.edge_count();
    let mut degrees = Vec::with_capacity(node_count);
    for node in network.graph.node_indices() {
        degrees.push(network.graph.neighbors(node).count());
    }
    let min_degree = *degrees.iter().min().unwrap_or(&0);
    let max_degree = *degrees.iter().max().unwrap_or(&0);
    let avg_degree = if node_count == 0 {
        0.0
    } else {
        degrees.iter().copied().sum::<usize>() as f64 / node_count as f64
    };
    let density = if node_count < 2 {
        0.0
    } else {
        2.0 * edge_count as f64 / (node_count as f64 * (node_count as f64 - 1.0))
    };
    let connected_components = connected_components(&network.graph);
    Ok(GraphStats {
        node_count,
        edge_count,
        connected_components,
        min_degree,
        avg_degree,
        max_degree,
        density,
    })
}

/// Labels connected components (breadth-first search) and pulls island metadata for CLI reporting.
pub fn find_islands(network: &Network) -> Result<IslandAnalysis> {
    let mut visited = HashSet::new();
    let mut islands = Vec::new();
    let mut assignments = Vec::new();
    let mut island_id = 0;
    for start in network.graph.node_indices() {
        if visited.contains(&start) {
            continue;
        }
        let mut queue = VecDeque::new();
        queue.push_back(start);
        let mut members = Vec::new();
        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            members.push(node);
            for neighbor in network.graph.neighbors(node) {
                if !visited.contains(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
        if members.is_empty() {
            continue;
        }
        islands.push(IslandSummary {
            island_id,
            node_count: members.len(),
        });
        for node in members {
            assignments.push(NodeAssignment {
                node_index: node.index(),
                label: network.graph[node].label().to_string(),
                island_id,
            });
        }
        island_id += 1;
    }
    assignments.sort_by_key(|assignment| assignment.node_index);
    Ok(IslandAnalysis {
        islands,
        assignments,
    })
}

/// Export the topology to a DOT string (Graphviz) so external tools can visualize the layout.
pub fn export_graph(network: &Network, format: &str) -> Result<String> {
    match format.to_ascii_lowercase().as_str() {
        "graphviz" | "dot" => Ok(render_dot(network)),
        other => Err(anyhow!("unsupported graph export format '{other}'")),
    }
}

fn render_dot(network: &Network) -> String {
    let mut buffer = String::new();
    buffer.push_str("graph gat_network {\n");
    for node in network.graph.node_indices() {
        let label = sanitize_label(network.graph[node].label());
        buffer.push_str(&format!("  n{} [label=\"{}\"];\n", node.index(), label));
    }
    for edge in network.graph.edge_references() {
        let source = edge.source().index();
        let target = edge.target().index();
        buffer.push_str(&format!("  n{source} -- n{target};\n"));
    }
    buffer.push('}');
    buffer
}

fn sanitize_label(label: &str) -> String {
    label.replace('"', "\\\"")
}
