use std::collections::HashMap;

use fdg_sim::{
    force::fruchterman_reingold, ForceGraph, ForceGraphHelper, Simulation, SimulationParameters,
};
use gat_core::{Edge, Network, Node};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LayoutNode {
    pub id: usize,
    pub label: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutEdge {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

impl Default for LayoutResult {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

/// Runs a force-directed layout on the provided `Network`.
pub fn layout_network(network: &Network, iterations: usize) -> LayoutResult {
    if network.graph.node_count() == 0 {
        return LayoutResult::default();
    }

    let mut graph: ForceGraph<usize, ()> = ForceGraph::default();
    let mut index_map: HashMap<usize, _> = HashMap::new();
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            let idx = graph.add_force_node(bus.name.clone(), bus.id.value());
            index_map.insert(bus.id.value(), idx);
        }
    }

    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            if let (Some(&from), Some(&to)) = (
                index_map.get(&branch.from_bus.value()),
                index_map.get(&branch.to_bus.value()),
            ) {
                graph.add_edge(from, to, ());
            }
        }
    }

    let mut params = SimulationParameters::default();
    params.set_force(fruchterman_reingold(45.0, 0.95));
    let mut simulation = Simulation::from_graph(graph, params);
    for _ in 0..iterations {
        simulation.update(0.02);
    }

    let graph = simulation.get_graph();

    let nodes = graph
        .node_indices()
        .map(|idx| {
            let node = &graph[idx];
            LayoutNode {
                id: node.data,
                label: node.name.clone(),
                x: node.location.x,
                y: node.location.y,
            }
        })
        .collect();

    let edges = graph
        .edge_references()
        .map(|edge| LayoutEdge {
            from: graph[edge.source()].data,
            to: graph[edge.target()].data,
        })
        .collect();

    LayoutResult { nodes, edges }
}
