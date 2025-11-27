//! TEP problem data structures
//!
//! Defines the input data for Transmission Expansion Planning problems.

use gat_core::{BusId, Network};
use std::collections::HashMap;

/// A candidate transmission line that could be built.
///
/// Candidate lines have associated investment costs and technical parameters.
/// The TEP solver decides which candidates to build (binary decision).
#[derive(Debug, Clone)]
pub struct CandidateLine {
    /// Unique identifier for this candidate
    pub id: CandidateId,
    /// Human-readable name
    pub name: String,
    /// From bus ID
    pub from_bus: BusId,
    /// To bus ID
    pub to_bus: BusId,
    /// Line reactance in per-unit (on system base)
    pub reactance_pu: f64,
    /// Maximum power flow capacity (MW)
    pub capacity_mw: f64,
    /// Investment cost to build this line ($)
    pub investment_cost: f64,
    /// Optional: Maximum number of parallel circuits that can be built
    /// If Some(n), up to n parallel circuits can be built (integer variable 0..n)
    /// If None, treated as binary (build or not)
    pub max_circuits: Option<usize>,
}

/// Unique identifier for a candidate line
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateId(pub usize);

impl CandidateId {
    pub fn new(id: usize) -> Self {
        CandidateId(id)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

impl CandidateLine {
    /// Create a new candidate line with required parameters
    pub fn new(
        id: usize,
        name: impl Into<String>,
        from_bus: BusId,
        to_bus: BusId,
        reactance_pu: f64,
        capacity_mw: f64,
        investment_cost: f64,
    ) -> Self {
        Self {
            id: CandidateId::new(id),
            name: name.into(),
            from_bus,
            to_bus,
            reactance_pu,
            capacity_mw,
            investment_cost,
            max_circuits: None,
        }
    }

    /// Set the maximum number of parallel circuits
    pub fn with_max_circuits(mut self, max: usize) -> Self {
        self.max_circuits = Some(max);
        self
    }

    /// Compute susceptance (1/x) in per-unit
    pub fn susceptance(&self) -> f64 {
        if self.reactance_pu.abs() < 1e-12 {
            0.0
        } else {
            1.0 / self.reactance_pu
        }
    }
}

/// TEP problem definition combining network with candidates
#[derive(Debug)]
pub struct TepProblem {
    /// Base network (existing infrastructure)
    pub network: Network,
    /// Candidate lines that could be built
    pub candidates: Vec<CandidateLine>,
    /// System base MVA (for per-unit conversions)
    pub base_mva: f64,
    /// Load scaling factors by bus (optional, for scenario analysis)
    pub load_scaling: HashMap<BusId, f64>,
    /// Generator scaling factors by name (optional)
    pub gen_scaling: HashMap<String, f64>,
    /// Big-M value for disjunctive constraints
    /// Should be large enough to not bind when lines are active
    pub big_m: f64,
    /// Annual operating hours (for cost annualization)
    pub operating_hours: f64,
    /// Discount rate for investment cost amortization
    pub discount_rate: f64,
    /// Planning horizon (years)
    pub planning_years: usize,
}

impl TepProblem {
    /// Create a new TEP problem from a network
    pub fn new(network: Network) -> Self {
        Self {
            network,
            candidates: Vec::new(),
            base_mva: 100.0,
            load_scaling: HashMap::new(),
            gen_scaling: HashMap::new(),
            big_m: 1e4, // Default Big-M (10,000 MW)
            operating_hours: 8760.0, // Full year
            discount_rate: 0.10, // 10% discount rate
            planning_years: 10,
        }
    }

    /// Add a candidate line
    pub fn add_candidate(&mut self, candidate: CandidateLine) {
        self.candidates.push(candidate);
    }

    /// Add multiple candidates
    pub fn add_candidates(&mut self, candidates: Vec<CandidateLine>) {
        self.candidates.extend(candidates);
    }

    /// Set the Big-M value for disjunctive constraints
    pub fn with_big_m(mut self, big_m: f64) -> Self {
        self.big_m = big_m;
        self
    }

    /// Set planning parameters
    pub fn with_planning_params(
        mut self,
        operating_hours: f64,
        discount_rate: f64,
        planning_years: usize,
    ) -> Self {
        self.operating_hours = operating_hours;
        self.discount_rate = discount_rate;
        self.planning_years = planning_years;
        self
    }

    /// Compute the Capital Recovery Factor for annualizing investment costs
    ///
    /// CRF = r(1+r)^n / ((1+r)^n - 1)
    ///
    /// where r = discount rate, n = planning years
    pub fn capital_recovery_factor(&self) -> f64 {
        let r = self.discount_rate;
        let n = self.planning_years as f64;
        if r < 1e-10 {
            // No discounting
            1.0 / n
        } else {
            r * (1.0 + r).powf(n) / ((1.0 + r).powf(n) - 1.0)
        }
    }

    /// Compute annualized investment cost for a candidate
    pub fn annualized_investment_cost(&self, candidate: &CandidateLine) -> f64 {
        candidate.investment_cost * self.capital_recovery_factor()
    }

    /// Number of buses in the network
    pub fn num_buses(&self) -> usize {
        self.network.stats().num_buses
    }

    /// Number of candidate lines
    pub fn num_candidates(&self) -> usize {
        self.candidates.len()
    }

    /// Get the total maximum investment cost (if all candidates built)
    pub fn max_investment_cost(&self) -> f64 {
        self.candidates
            .iter()
            .map(|c| {
                let multiplier = c.max_circuits.unwrap_or(1) as f64;
                c.investment_cost * multiplier
            })
            .sum()
    }
}

/// Builder for constructing TEP problems
pub struct TepProblemBuilder {
    problem: TepProblem,
    next_candidate_id: usize,
}

impl TepProblemBuilder {
    /// Start building a TEP problem from a network
    pub fn new(network: Network) -> Self {
        Self {
            problem: TepProblem::new(network),
            next_candidate_id: 1,
        }
    }

    /// Set base MVA
    pub fn base_mva(mut self, base_mva: f64) -> Self {
        self.problem.base_mva = base_mva;
        self
    }

    /// Set Big-M value
    pub fn big_m(mut self, big_m: f64) -> Self {
        self.problem.big_m = big_m;
        self
    }

    /// Set planning parameters
    pub fn planning_params(
        mut self,
        operating_hours: f64,
        discount_rate: f64,
        planning_years: usize,
    ) -> Self {
        self.problem.operating_hours = operating_hours;
        self.problem.discount_rate = discount_rate;
        self.problem.planning_years = planning_years;
        self
    }

    /// Add a candidate line with auto-generated ID
    pub fn candidate(
        mut self,
        name: impl Into<String>,
        from_bus: BusId,
        to_bus: BusId,
        reactance_pu: f64,
        capacity_mw: f64,
        investment_cost: f64,
    ) -> Self {
        let candidate = CandidateLine::new(
            self.next_candidate_id,
            name,
            from_bus,
            to_bus,
            reactance_pu,
            capacity_mw,
            investment_cost,
        );
        self.next_candidate_id += 1;
        self.problem.candidates.push(candidate);
        self
    }

    /// Add a candidate line with parallel circuit option
    pub fn candidate_with_circuits(
        mut self,
        name: impl Into<String>,
        from_bus: BusId,
        to_bus: BusId,
        reactance_pu: f64,
        capacity_mw: f64,
        investment_cost: f64,
        max_circuits: usize,
    ) -> Self {
        let candidate = CandidateLine::new(
            self.next_candidate_id,
            name,
            from_bus,
            to_bus,
            reactance_pu,
            capacity_mw,
            investment_cost,
        )
        .with_max_circuits(max_circuits);
        self.next_candidate_id += 1;
        self.problem.candidates.push(candidate);
        self
    }

    /// Add load scaling factor for a bus
    pub fn scale_load(mut self, bus: BusId, factor: f64) -> Self {
        self.problem.load_scaling.insert(bus, factor);
        self
    }

    /// Add generator scaling factor
    pub fn scale_gen(mut self, name: impl Into<String>, factor: f64) -> Self {
        self.problem.gen_scaling.insert(name.into(), factor);
        self
    }

    /// Build the TEP problem
    pub fn build(self) -> TepProblem {
        self.problem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Bus, Gen, GenId, Load, LoadId, Node};

    fn create_simple_network() -> Network {
        let mut network = Network::new();

        // Add 3 buses
        for i in 1..=3 {
            network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(i),
                name: format!("Bus {}", i),
                voltage_kv: 138.0,
                voltage_pu: 1.0,
                angle_rad: 0.0,
                ..Bus::default()
            }));
        }

        // Add generator at bus 1
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            pmax_mw: 100.0,
            pmin_mw: 0.0,
            ..Gen::default()
        }));

        // Add load at bus 3
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(3),
            active_power_mw: 50.0,
            reactive_power_mvar: 0.0,
        }));

        network
    }

    #[test]
    fn test_candidate_line_creation() {
        let candidate = CandidateLine::new(
            1,
            "New Line 1-2",
            BusId::new(1),
            BusId::new(2),
            0.1, // 0.1 p.u. reactance
            100.0, // 100 MW capacity
            1_000_000.0, // $1M investment
        );

        assert_eq!(candidate.id.value(), 1);
        assert_eq!(candidate.susceptance(), 10.0); // 1/0.1
        assert!(candidate.max_circuits.is_none());
    }

    #[test]
    fn test_candidate_with_circuits() {
        let candidate = CandidateLine::new(
            1,
            "Multi-circuit line",
            BusId::new(1),
            BusId::new(2),
            0.2,
            150.0,
            2_000_000.0,
        )
        .with_max_circuits(3);

        assert_eq!(candidate.max_circuits, Some(3));
    }

    #[test]
    fn test_tep_problem_builder() {
        let network = create_simple_network();

        let problem = TepProblemBuilder::new(network)
            .base_mva(100.0)
            .big_m(10000.0)
            .planning_params(8760.0, 0.1, 10)
            .candidate("Line 1-2", BusId::new(1), BusId::new(2), 0.1, 100.0, 1e6)
            .candidate("Line 2-3", BusId::new(2), BusId::new(3), 0.15, 80.0, 0.8e6)
            .build();

        assert_eq!(problem.num_candidates(), 2);
        assert_eq!(problem.num_buses(), 3);
        assert!((problem.capital_recovery_factor() - 0.1627).abs() < 0.01);
    }

    #[test]
    fn test_capital_recovery_factor() {
        let network = Network::new();
        let problem = TepProblem::new(network)
            .with_planning_params(8760.0, 0.10, 10);

        // CRF for 10% over 10 years ≈ 0.1627
        let crf = problem.capital_recovery_factor();
        assert!((crf - 0.1627).abs() < 0.01);
    }

    #[test]
    fn test_annualized_cost() {
        let network = Network::new();
        let problem = TepProblem::new(network)
            .with_planning_params(8760.0, 0.10, 10);

        let candidate = CandidateLine::new(
            1,
            "Test Line",
            BusId::new(1),
            BusId::new(2),
            0.1,
            100.0,
            1_000_000.0, // $1M
        );

        // Annualized cost should be ~$162,745 per year
        let annual = problem.annualized_investment_cost(&candidate);
        assert!((annual - 162745.0).abs() < 1000.0);
    }
}
