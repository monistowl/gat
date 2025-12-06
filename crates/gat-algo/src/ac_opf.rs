use gat_core::{Edge, Gen, Network, Node};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// AC OPF solver errors
#[derive(Debug, Clone, Error)]
pub enum AcOpfError {
    /// Network is infeasible (demand exceeds supply)
    #[error("AC OPF infeasible: {0}")]
    Infeasible(String),

    /// Problem is unbounded
    #[error("AC OPF unbounded")]
    Unbounded,

    /// Solver timeout
    #[error("AC OPF timeout after {0:?}")]
    SolverTimeout(Duration),

    /// Numerical convergence issue
    #[error("AC OPF numerical issue: {0}")]
    NumericalIssue(String),

    /// Input data validation error
    #[error("AC OPF data validation: {0}")]
    DataValidation(String),

    /// Convergence failure with residual info
    #[error("AC OPF failed to converge after {iterations} iterations (residual: {residual:.2e})")]
    ConvergenceFailure { iterations: usize, residual: f64 },

    /// Method not yet implemented
    #[error("OPF method not implemented: {0}")]
    NotImplemented(String),
}

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

/// Optimal Power Flow solver using economic dispatch
///
/// This solver currently implements a simplified DC-OPF approximation using
/// merit-order economic dispatch. It:
/// - Ignores reactive power and voltage constraints (DC approximation)
/// - Dispatches generators in order of marginal cost (merit order)
/// - Respects generator Pmin/Pmax limits
/// - Estimates losses at 1% of load
///
/// Future versions may implement full AC-OPF with nonlinear constraints.
pub struct AcOpfSolver {
    // Note: These fields are reserved for future AC-OPF implementation.
    // Currently, the solver uses merit-order dispatch which doesn't need them.
    #[allow(dead_code)]
    max_iterations: usize,
    #[allow(dead_code)]
    tolerance: f64,
}

impl AcOpfSolver {
    /// Create new OPF solver with default parameters
    pub fn new() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }

    /// Set maximum iterations (reserved for future AC-OPF implementation)
    ///
    /// Note: Currently unused - the merit-order dispatch converges in one pass.
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Set convergence tolerance (reserved for future AC-OPF implementation)
    ///
    /// Note: Currently unused - the merit-order dispatch is deterministic.
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
                        return Err(AcOpfError::DataValidation(
                            "Bus with empty name".to_string(),
                        ));
                    }

                    // Basic voltage validation
                    if bus.base_kv.value() <= 0.0 {
                        return Err(AcOpfError::DataValidation(format!(
                            "Bus {}: base_kv must be positive",
                            bus.name
                        )));
                    }
                }
                Node::Gen(gen) => {
                    has_generator = true;
                    if gen.name.is_empty() {
                        return Err(AcOpfError::DataValidation(
                            "Generator with empty name".to_string(),
                        ));
                    }

                    // Generators should have non-negative active power (unless synchronous condenser)
                    if gen.active_power.value() < 0.0 && !gen.is_synchronous_condenser {
                        return Err(AcOpfError::DataValidation(format!(
                            "Generator {} has negative active_power ({}). Use .as_synchronous_condenser() for reactive-only devices.",
                            gen.name, gen.active_power.value()
                        )));
                    }
                }
                Node::Load(_load) => {
                    // Loads are optional, basic validation only
                }
                Node::Shunt(_) => {
                    // Shunts don't require special validation
                }
            }
        }

        if !has_bus {
            return Err(AcOpfError::DataValidation(
                "Network has no buses".to_string(),
            ));
        }

        if !has_generator {
            return Err(AcOpfError::DataValidation(
                "Network has no generators".to_string(),
            ));
        }

        // Validate edges
        for edge_idx in network.graph.edge_indices() {
            match &network.graph[edge_idx] {
                Edge::Branch(branch) => {
                    if branch.name.is_empty() {
                        return Err(AcOpfError::DataValidation(
                            "Branch with empty name".to_string(),
                        ));
                    }

                    // Resistance and reactance should be non-negative (unless phase-shifter)
                    if (branch.resistance < 0.0 || branch.reactance < 0.0)
                        && !branch.is_phase_shifter
                    {
                        return Err(AcOpfError::DataValidation(format!(
                            "Branch {}: resistance and reactance must be non-negative (use .as_phase_shifter() for PSTs)",
                            branch.name
                        )));
                    }
                }
                Edge::Transformer(tx) => {
                    if tx.name.is_empty() {
                        return Err(AcOpfError::DataValidation(
                            "Transformer with empty name".to_string(),
                        ));
                    }

                    // Transformer ratio should be positive
                    if tx.ratio <= 0.0 {
                        return Err(AcOpfError::DataValidation(format!(
                            "Transformer {}: ratio must be positive",
                            tx.name
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Solve using merit-order economic dispatch (DC approximation)
    ///
    /// This implements a simplified economic dispatch that:
    /// 1. Collects all generators with their limits and cost functions
    /// 2. Sorts generators by marginal cost at Pmin (merit order)
    /// 3. Dispatches generators in merit order to meet load
    /// 4. Computes total cost using actual cost functions
    fn solve_economic_dispatch(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        let start = std::time::Instant::now();

        // Collect generators and loads
        let mut generators: Vec<Gen> = Vec::new();
        let mut total_load = 0.0;

        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Gen(gen) => {
                    generators.push(gen.clone());
                }
                Node::Load(load) => {
                    total_load += load.active_power.value();
                }
                Node::Bus(_) => {}
                Node::Shunt(_) => {}
            }
        }

        if generators.is_empty() {
            return Err(AcOpfError::DataValidation(
                "No generators in network".to_string(),
            ));
        }

        // Estimate losses at 1% of load for DC approximation
        let loss_estimate = total_load * 0.01;
        let required_generation = total_load + loss_estimate;

        // Check total capacity
        let total_pmax: f64 = generators.iter().map(|g| g.pmax.value()).sum();
        let total_pmin: f64 = generators.iter().map(|g| g.pmin.value()).sum();

        if required_generation > total_pmax {
            return Err(AcOpfError::Infeasible(format!(
                "Generator capacity insufficient: need {:.2} MW, max {:.2} MW",
                required_generation, total_pmax
            )));
        }

        if required_generation < total_pmin {
            return Err(AcOpfError::Infeasible(format!(
                "Load too low for minimum generation: need {:.2} MW, min {:.2} MW",
                required_generation, total_pmin
            )));
        }

        // Economic dispatch using merit order
        let dispatch = self.economic_dispatch(&generators, required_generation)?;

        // Compute objective value using actual cost functions
        let objective_value: f64 = generators
            .iter()
            .zip(dispatch.iter())
            .map(|(gen, &p)| gen.cost_model.evaluate(p))
            .sum();

        // Build solution
        let mut solution = AcOpfSolution {
            converged: true,
            objective_value,
            generator_outputs: HashMap::new(),
            bus_voltages: HashMap::new(),
            branch_flows: HashMap::new(),
            iterations: 1,
            solve_time_ms: start.elapsed().as_millis(),
        };

        // Record generator outputs
        for (gen, &output) in generators.iter().zip(dispatch.iter()) {
            solution.generator_outputs.insert(gen.name.clone(), output);
        }

        // Set voltages to nominal (1.0 pu) - simplified DC approximation
        for node_idx in network.graph.node_indices() {
            if let Node::Bus(bus) = &network.graph[node_idx] {
                solution.bus_voltages.insert(bus.name.clone(), 1.0);
            }
        }

        Ok(solution)
    }

    /// Economic dispatch using merit order
    ///
    /// Sorts generators by marginal cost at Pmin, then dispatches in order
    /// to minimize total cost while meeting load and respecting limits.
    fn economic_dispatch(
        &self,
        generators: &[Gen],
        required_generation: f64,
    ) -> Result<Vec<f64>, AcOpfError> {
        let n = generators.len();
        let mut dispatch = vec![0.0; n];

        // Start with minimum generation for all units
        for (i, gen) in generators.iter().enumerate() {
            dispatch[i] = gen.pmin.value();
        }

        // Calculate how much more we need beyond minimum
        let total_pmin: f64 = generators.iter().map(|g| g.pmin.value()).sum();
        let mut remaining = required_generation - total_pmin;

        if remaining < 0.0 {
            // Should have been caught earlier, but handle gracefully
            return Ok(dispatch);
        }

        // Create merit order: sort by marginal cost at Pmin
        let mut merit_order: Vec<usize> = (0..n).collect();
        merit_order.sort_by(|&a, &b| {
            let mc_a = generators[a]
                .cost_model
                .marginal_cost(generators[a].pmin.value());
            let mc_b = generators[b]
                .cost_model
                .marginal_cost(generators[b].pmin.value());
            mc_a.partial_cmp(&mc_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Dispatch in merit order
        for &idx in &merit_order {
            if remaining <= 1e-6 {
                break;
            }

            let gen = &generators[idx];
            let current = dispatch[idx];
            let headroom = (gen.pmax.value() - current).max(0.0);
            let increment = remaining.min(headroom);

            dispatch[idx] = current + increment;
            remaining -= increment;
        }

        if remaining > 1e-3 {
            return Err(AcOpfError::Infeasible(format!(
                "Cannot meet load: {:.3} MW unserved after dispatch",
                remaining
            )));
        }

        Ok(dispatch)
    }

    /// Solve AC OPF using merit-order economic dispatch
    pub fn solve(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        // Validate first
        self.validate_network(network)?;

        // Solve using economic dispatch
        self.solve_economic_dispatch(network)
    }
}

/// Internal penalty formulation problem (reserved for future AC-OPF implementation)
#[derive(Debug)]
#[allow(dead_code)]
struct PenaltyFormulation {
    /// Bus indices
    bus_indices: Vec<usize>,
    /// Generator indices
    gen_indices: Vec<usize>,
    /// Bus voltage variables (magnitude)
    bus_voltages: Vec<String>, // Variable names for debug
    /// Generator power variables
    gen_powers: Vec<String>, // Variable names for debug
}

impl PenaltyFormulation {
    #[allow(dead_code)]
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

/// Type alias for migration - use OpfError in new code
pub type OpfError = AcOpfError;
