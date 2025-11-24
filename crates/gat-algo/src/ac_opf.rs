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

    /// Solve AC OPF - placeholder for Task 7
    pub fn solve(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        // Validate first
        self.validate_network(network)?;

        // For now, return a placeholder solution
        // Task 7 will implement the actual penalty method formulation
        Ok(AcOpfSolution {
            converged: false,
            objective_value: 0.0,
            generator_outputs: HashMap::new(),
            bus_voltages: HashMap::new(),
            branch_flows: HashMap::new(),
            iterations: 0,
            solve_time_ms: 0,
        })
    }
}

impl Default for AcOpfSolver {
    fn default() -> Self {
        Self::new()
    }
}
