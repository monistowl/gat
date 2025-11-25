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

/// Generator cost model for OPF optimization
#[derive(Debug, Clone, Default)]
pub enum CostModel {
    /// No cost function specified
    #[default]
    NoCost,
    /// Polynomial cost: cost = sum(coeffs[i] * P^i) where coeffs[0] is constant term
    /// For quadratic: coeffs = [c0, c1, c2] means cost = c0 + c1*P + c2*P^2
    Polynomial(Vec<f64>),
    /// Piecewise linear cost: Vec<(mw, $/hr)> breakpoints
    PiecewiseLinear(Vec<(f64, f64)>),
}

impl CostModel {
    /// Create quadratic cost: c0 + c1*P + c2*P^2
    pub fn quadratic(c0: f64, c1: f64, c2: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1, c2])
    }

    /// Create linear cost: c0 + c1*P (marginal cost c1 in $/MWh)
    pub fn linear(c0: f64, c1: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1])
    }

    /// Evaluate cost at given power output ($/hr)
    pub fn evaluate(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => coeffs
                .iter()
                .enumerate()
                .map(|(i, c)| c * p_mw.powi(i as i32))
                .sum(),
            CostModel::PiecewiseLinear(points) => {
                if points.is_empty() {
                    return 0.0;
                }
                if p_mw <= points[0].0 {
                    return points[0].1;
                }
                if p_mw >= points.last().unwrap().0 {
                    return points.last().unwrap().1;
                }
                for i in 0..points.len() - 1 {
                    if p_mw >= points[i].0 && p_mw <= points[i + 1].0 {
                        let t = (p_mw - points[i].0) / (points[i + 1].0 - points[i].0);
                        return points[i].1 + t * (points[i + 1].1 - points[i].1);
                    }
                }
                0.0
            }
        }
    }

    /// Get marginal cost at given power ($/MWh, derivative of cost function)
    pub fn marginal_cost(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => {
                // d/dP[sum(c_i * P^i)] = sum(i * c_i * P^(i-1))
                coeffs
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(i, c)| (i as f64) * c * p_mw.powi(i as i32 - 1))
                    .sum()
            }
            CostModel::PiecewiseLinear(points) => {
                if points.len() < 2 {
                    return 0.0;
                }
                for i in 0..points.len() - 1 {
                    if p_mw >= points[i].0 && p_mw <= points[i + 1].0 {
                        return (points[i + 1].1 - points[i].1) / (points[i + 1].0 - points[i].0);
                    }
                }
                // Return last segment's slope if beyond range
                let n = points.len();
                (points[n - 1].1 - points[n - 2].1) / (points[n - 1].0 - points[n - 2].0)
            }
        }
    }

    /// Check if this cost model has actual cost data
    pub fn has_cost(&self) -> bool {
        !matches!(self, CostModel::NoCost)
    }
}

#[derive(Debug, Clone)]
pub struct Gen {
    pub id: GenId,
    pub name: String,
    pub bus: BusId,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
    /// Minimum active power output (MW)
    pub pmin_mw: f64,
    /// Maximum active power output (MW)
    pub pmax_mw: f64,
    /// Minimum reactive power output (MVAr)
    pub qmin_mvar: f64,
    /// Maximum reactive power output (MVAr)
    pub qmax_mvar: f64,
    /// Cost function for OPF
    pub cost_model: CostModel,
}

impl Gen {
    /// Create a new generator with default limits (no constraints)
    pub fn new(id: GenId, name: String, bus: BusId) -> Self {
        Self {
            id,
            name,
            bus,
            active_power_mw: 0.0,
            reactive_power_mvar: 0.0,
            pmin_mw: 0.0,
            pmax_mw: f64::INFINITY,
            qmin_mvar: f64::NEG_INFINITY,
            qmax_mvar: f64::INFINITY,
            cost_model: CostModel::NoCost,
        }
    }

    /// Set active power limits
    pub fn with_p_limits(mut self, pmin: f64, pmax: f64) -> Self {
        self.pmin_mw = pmin;
        self.pmax_mw = pmax;
        self
    }

    /// Set reactive power limits
    pub fn with_q_limits(mut self, qmin: f64, qmax: f64) -> Self {
        self.qmin_mvar = qmin;
        self.qmax_mvar = qmax;
        self
    }

    /// Set cost model
    pub fn with_cost(mut self, cost: CostModel) -> Self {
        self.cost_model = cost;
        self
    }
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
