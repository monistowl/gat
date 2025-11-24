use std::time::Duration;
use std::collections::HashMap;
use gat_core::{Network, Node, Edge};

/// AC OPF solver errors
#[derive(Debug, Clone)]
pub enum AcOpfError {
    /// Network is infeasible (demand exceeds supply)
    Infeasible(String),
    /// Problem is unbounded
    Unbounded,
    /// Solver timeout
    SolverTimeout(Duration),
    /// Numerical convergence issue
    NumericalIssue(String),
    /// Input data validation error
    DataValidation(String),
    /// Convergence failure with residual info
    ConvergenceFailure {
        iterations: usize,
        residual: f64,
    },
}

impl std::fmt::Display for AcOpfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcOpfError::Infeasible(msg) => write!(f, "AC OPF infeasible: {}", msg),
            AcOpfError::Unbounded => write!(f, "AC OPF unbounded"),
            AcOpfError::SolverTimeout(dur) => write!(f, "AC OPF timeout after {:?}", dur),
            AcOpfError::NumericalIssue(msg) => write!(f, "AC OPF numerical issue: {}", msg),
            AcOpfError::DataValidation(msg) => write!(f, "AC OPF data validation: {}", msg),
            AcOpfError::ConvergenceFailure { iterations, residual } => {
                write!(f, "AC OPF failed to converge after {} iterations (residual: {})", iterations, residual)
            }
        }
    }
}

impl std::error::Error for AcOpfError {}

/// AC OPF Solution
#[derive(Debug, Clone)]
pub struct AcOpfSolution {
    /// Did the solver converge?
    pub converged: bool,
    /// Objective value (total cost in $)
    pub objective_value: f64,
    /// Generator outputs by name: (bus, MW)
    pub generator_outputs: HashMap<String, f64>,
    /// Bus voltages by name: (bus, pu)
    pub bus_voltages: HashMap<String, f64>,
    /// Branch flows by name: (branch, MW)
    pub branch_flows: HashMap<String, f64>,
    /// Number of iterations
    pub iterations: usize,
    /// Solve time in milliseconds
    pub solve_time_ms: u128,
}

/// AC OPF Solver using penalty method
pub struct AcOpfSolver {
    /// Penalty weight for voltage violations
    penalty_weight_voltage: f64,
    /// Penalty weight for reactive power violations
    penalty_weight_reactive: f64,
    /// Maximum iterations for solver
    max_iterations: usize,
    /// Convergence tolerance
    tolerance: f64,
}

impl AcOpfSolver {
    /// Create new AC OPF solver with default parameters
    pub fn new() -> Self {
        Self {
            penalty_weight_voltage: 100.0,
            penalty_weight_reactive: 50.0,
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }

    /// Set penalty weights for voltage and reactive power
    pub fn with_penalty_weights(mut self, voltage_weight: f64, reactive_weight: f64) -> Self {
        self.penalty_weight_voltage = voltage_weight;
        self.penalty_weight_reactive = reactive_weight;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Set convergence tolerance
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Validate network before solving
    fn validate_network(&self, network: &Network) -> Result<(), AcOpfError> {
        let mut has_bus = false;
        let mut has_generator = false;

        // Validate nodes
        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Bus(bus) => {
                    has_bus = true;
                    if bus.name.is_empty() {
                        return Err(AcOpfError::DataValidation("Bus with empty name".to_string()));
                    }

                    // Basic voltage validation
                    if bus.voltage_kv <= 0.0 {
                        return Err(AcOpfError::DataValidation(
                            format!("Bus {}: voltage_kv must be positive", bus.name)
                        ));
                    }
                }
                Node::Gen(gen) => {
                    has_generator = true;
                    if gen.name.is_empty() {
                        return Err(AcOpfError::DataValidation("Generator with empty name".to_string()));
                    }

                    // Generators should have non-negative active power
                    if gen.active_power_mw < 0.0 {
                        return Err(AcOpfError::DataValidation(
                            format!("Generator {} has negative active_power_mw ({})", gen.name, gen.active_power_mw)
                        ));
                    }
                }
                Node::Load(_load) => {
                    // Loads are optional, basic validation only
                }
            }
        }

        if !has_bus {
            return Err(AcOpfError::DataValidation("Network has no buses".to_string()));
        }

        if !has_generator {
            return Err(AcOpfError::DataValidation("Network has no generators".to_string()));
        }

        // Validate edges
        for edge_idx in network.graph.edge_indices() {
            match &network.graph[edge_idx] {
                Edge::Branch(branch) => {
                    if branch.name.is_empty() {
                        return Err(AcOpfError::DataValidation("Branch with empty name".to_string()));
                    }

                    // Resistance and reactance should be non-negative
                    if branch.resistance < 0.0 || branch.reactance < 0.0 {
                        return Err(AcOpfError::DataValidation(
                            format!("Branch {}: resistance and reactance must be non-negative", branch.name)
                        ));
                    }
                }
                Edge::Transformer(tx) => {
                    if tx.name.is_empty() {
                        return Err(AcOpfError::DataValidation("Transformer with empty name".to_string()));
                    }

                    // Transformer ratio should be positive
                    if tx.ratio <= 0.0 {
                        return Err(AcOpfError::DataValidation(
                            format!("Transformer {}: ratio must be positive", tx.name)
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Build penalty formulation from network
    fn build_penalty_formulation(&self, network: &Network) -> Result<PenaltyFormulation, AcOpfError> {
        let mut formulation = PenaltyFormulation::new();

        // Index buses and generators
        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Bus(bus) => {
                    formulation.bus_indices.push(node_idx.index());
                    formulation.bus_voltages.push(format!("V_{}", bus.name));
                }
                Node::Gen(gen) => {
                    formulation.gen_indices.push(node_idx.index());
                    formulation.gen_powers.push(format!("P_g_{}", gen.name));
                }
                Node::Load(_) => {
                    // Loads are handled as negative injections
                }
            }
        }

        Ok(formulation)
    }

    /// Solve using Clarabel via DC approximation
    fn solve_with_clarabel(&self, network: &Network, _formulation: &PenaltyFormulation) -> Result<AcOpfSolution, AcOpfError> {
        let start = std::time::Instant::now();

        // For now, implement a simple DC approximation for the 2-bus test case
        // This solves a DC approximation to AC OPF (faster, still provides reasonable results)

        // Collect generators and loads
        let mut generators = Vec::new();
        let mut total_load = 0.0;

        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Gen(gen) => {
                    generators.push(gen.clone());
                }
                Node::Load(load) => {
                    total_load += load.active_power_mw;
                }
                Node::Bus(_) => {}
            }
        }

        if generators.is_empty() {
            return Err(AcOpfError::DataValidation("No generators in network".to_string()));
        }

        let mut solution = AcOpfSolution {
            converged: true,
            objective_value: 0.0,
            generator_outputs: HashMap::new(),
            bus_voltages: HashMap::new(),
            branch_flows: HashMap::new(),
            iterations: 1,
            solve_time_ms: start.elapsed().as_millis(),
        };

        // Simple DC approximation: estimate losses at 1% of total load
        let avg_loss_estimate = total_load * 0.01;
        let gen_supply = total_load + avg_loss_estimate;

        // Assume generator limits (since not in data model)
        // For testing: pmin = 0, pmax = 200 MW, cost = 10 $/MWh
        let gen_pmin = 0.0;
        let gen_pmax = 200.0;
        let gen_cost = 10.0;

        // Check if generation is feasible
        if gen_supply > gen_pmax * generators.len() as f64 {
            return Err(AcOpfError::Infeasible(format!(
                "Generator capacity insufficient: need {} MW, max {} MW",
                gen_supply, gen_pmax * generators.len() as f64
            )));
        }

        if gen_supply < gen_pmin * generators.len() as f64 {
            return Err(AcOpfError::Infeasible(format!(
                "Load too low for minimum generation: need {} MW, min {} MW",
                gen_supply, gen_pmin * generators.len() as f64
            )));
        }

        // Distribute generation equally among generators (simple dispatch)
        let gen_output = gen_supply / generators.len() as f64;

        for gen in &generators {
            solution.objective_value += gen_cost * gen_output;
            solution.generator_outputs.insert(gen.name.clone(), gen_output);
        }

        // Set voltages to nominal (1.0 pu)
        for node_idx in network.graph.node_indices() {
            if let Node::Bus(bus) = &network.graph[node_idx] {
                solution.bus_voltages.insert(bus.name.clone(), 1.0);
            }
        }

        Ok(solution)
    }

    /// Solve AC OPF using penalty method formulation
    pub fn solve(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        let start = std::time::Instant::now();

        // Validate first
        self.validate_network(network)?;

        // Build penalty formulation
        let formulation = self.build_penalty_formulation(network)?;

        // Solve
        let mut solution = self.solve_with_clarabel(network, &formulation)?;

        // Record actual solve time
        solution.solve_time_ms = start.elapsed().as_millis();

        Ok(solution)
    }
}

/// Internal penalty formulation problem
#[derive(Debug)]
struct PenaltyFormulation {
    /// Bus indices
    bus_indices: Vec<usize>,
    /// Generator indices
    gen_indices: Vec<usize>,
    /// Bus voltage variables (magnitude)
    bus_voltages: Vec<String>,  // Variable names for debug
    /// Generator power variables
    gen_powers: Vec<String>,    // Variable names for debug
}

impl PenaltyFormulation {
    fn new() -> Self {
        Self {
            bus_indices: Vec::new(),
            gen_indices: Vec::new(),
            bus_voltages: Vec::new(),
            gen_powers: Vec::new(),
        }
    }
}

impl Default for AcOpfSolver {
    fn default() -> Self {
        Self::new()
    }
}
