use petgraph::{prelude::*, Undirected};

pub mod graph_utils;
pub use graph_utils::*;
pub mod solver;
pub use petgraph::graph::NodeIndex;
pub use solver::*;

// Newtype wrappers for IDs for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BranchId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransformerId(usize);

impl BusId {
    pub fn new(value: usize) -> Self {
        BusId(value)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl BranchId {
    pub fn new(value: usize) -> Self {
        BranchId(value)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl GenId {
    pub fn new(value: usize) -> Self {
        GenId(value)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl LoadId {
    pub fn new(value: usize) -> Self {
        LoadId(value)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl TransformerId {
    pub fn new(value: usize) -> Self {
        TransformerId(value)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

// Basic component structs
#[derive(Debug, Clone)]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    pub voltage_kv: f64,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    pub from_bus: BusId,
    pub to_bus: BusId,
    pub resistance: f64,
    pub reactance: f64,
}

#[derive(Debug, Clone)]
pub struct Gen {
    pub id: GenId,
    pub name: String,
    pub bus: BusId,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
}

#[derive(Debug, Clone)]
pub struct Load {
    pub id: LoadId,
    pub name: String,
    pub bus: BusId,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
}

#[derive(Debug, Clone)]
pub struct Transformer {
    pub id: TransformerId,
    pub name: String,
    pub from_bus: BusId,
    pub to_bus: BusId,
    pub ratio: f64,
}

// Enum to represent different types of nodes in the graph
#[derive(Debug, Clone)]
pub enum Node {
    Bus(Bus),
    Gen(Gen),
    Load(Load),
}

// Enum to represent different types of edges in the graph
#[derive(Debug, Clone)]
pub enum Edge {
    Branch(Branch),
    Transformer(Transformer),
}

/// The core power network graph
#[derive(Debug, Default)]
pub struct Network {
    pub graph: Graph<Node, Edge, Undirected>,
}

// The physical grid is represented as a graph where buses, generators, and loads are nodes,
// while branches and transformers are edges. This mirrors the standard approach to power
// system modeling and keeps topology explicit for algorithms such as DC power flow and
// contingency screening (see doi:10.1109/TPWRS.2012.2187686).

impl Network {
    pub fn new() -> Self {
        Self {
            graph: Graph::new_undirected(),
        }
    }
}

impl Node {
    /// Returns a human-readable label for the node (bus/gen/load name).
    pub fn label(&self) -> &str {
        match self {
            Node::Bus(bus) => &bus.name,
            Node::Gen(gen) => &gen.name,
            Node::Load(load) => &load.name,
        }
    }
}

impl Edge {
    /// Returns a human-readable label for the edge (branch/transformer name).
    pub fn label(&self) -> &str {
        match self {
            Edge::Branch(branch) => &branch.name,
            Edge::Transformer(tx) => &tx.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_creation() {
        let mut network = Network::new();

        let bus1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(0),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));
        let bus2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(1),
            name: "Bus 2".to_string(),
            voltage_kv: 138.0,
        }));

        let _branch = network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch {
                id: BranchId(0),
                name: "Branch 1-2".to_string(),
                from_bus: BusId(0),
                to_bus: BusId(1),
                resistance: 0.01,
                reactance: 0.1,
            }),
        );

        assert_eq!(network.graph.node_count(), 2);
        assert_eq!(network.graph.edge_count(), 1);

        if let Node::Bus(b) = network.graph[bus1].clone() {
            assert_eq!(b.name, "Bus 1");
        } else {
            panic!("Expected Bus node");
        }
    }
}
