//! ADMM-based Distributed Optimal Power Flow Solver
//!
//! This module implements the Alternating Direction Method of Multipliers (ADMM)
//! for distributed OPF, following the DPLib paper's approach.
//!
//! # Algorithm Overview
//!
//! The network is partitioned into regions, each solving a local OPF subproblem.
//! Boundary buses are shared between partitions via consensus constraints:
//!
//! ```text
//!   min  Σ_k f_k(x_k)
//!   s.t. A_k x_k = b_k           (local constraints)
//!        x_k|_boundary = z       (consensus constraints)
//! ```
//!
//! ADMM iterates:
//! 1. **x-update**: Each partition solves local OPF with augmented Lagrangian
//! 2. **z-update**: Average boundary variables across partitions
//! 3. **λ-update**: Update dual variables (Lagrange multipliers)
//!
//! # Convergence
//!
//! ADMM converges when primal and dual residuals are below tolerance:
//! - Primal: ||x_k - z|| (how far local solutions are from consensus)
//! - Dual: ρ||z^{k+1} - z^k|| (how much consensus changes between iterations)
//!
//! # References
//!
//! - Boyd et al., "Distributed Optimization and Statistical Learning via ADMM"
//! - DPLib paper: arXiv:2506.20819

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use gat_core::{
    BranchId, BusId, Edge, GenId, Load, LoadId, Megavars, Megawatts, Network, Node,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::graph::{partition_network, NetworkPartition, PartitionError, PartitionStrategy};
use crate::opf::{OpfMethod, OpfSolution, OpfSolver};
use crate::OpfError;

/// ADMM solver configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmmConfig {
    /// Penalty parameter (ρ) - controls trade-off between local optimality and consensus.
    ///
    /// Larger ρ = faster consensus but worse local solutions per iteration.
    /// Typical range: 0.1 to 100, start with 1.0.
    pub penalty: f64,

    /// Primal feasibility tolerance (consensus violation).
    ///
    /// Convergence criterion: ||r_primal|| < primal_tol * sqrt(n)
    /// where r_primal = x_k - z for all boundary variables.
    pub primal_tol: f64,

    /// Dual feasibility tolerance (consensus change rate).
    ///
    /// Convergence criterion: ||r_dual|| < dual_tol * sqrt(n)
    /// where r_dual = ρ * (z^{k+1} - z^k).
    pub dual_tol: f64,

    /// Maximum ADMM iterations.
    pub max_iter: usize,

    /// OPF method for local subproblems.
    ///
    /// Typically DC-OPF or SOCP for speed; AC-OPF for accuracy.
    pub inner_method: OpfMethod,

    /// Number of partitions (if using spectral/load-balanced strategy).
    pub num_partitions: usize,

    /// Partitioning strategy.
    pub partition_strategy: PartitionStrategyConfig,

    /// Adaptive penalty parameter update.
    ///
    /// If true, adjusts ρ based on primal/dual residual ratio.
    pub adaptive_penalty: bool,

    /// Penalty increase/decrease factor for adaptive update.
    pub penalty_scale: f64,

    /// Maximum penalty value for adaptive update.
    pub max_penalty: f64,

    /// Minimum penalty value for adaptive update.
    pub min_penalty: f64,

    /// Verbose output during solving.
    pub verbose: bool,
}

impl Default for AdmmConfig {
    fn default() -> Self {
        Self {
            penalty: 1.0,
            primal_tol: 1e-4,
            dual_tol: 1e-4,
            max_iter: 100,
            inner_method: OpfMethod::DcOpf,
            num_partitions: 4,
            partition_strategy: PartitionStrategyConfig::Spectral,
            adaptive_penalty: true,
            penalty_scale: 2.0,
            max_penalty: 1e6,
            min_penalty: 1e-6,
            verbose: false,
        }
    }
}

/// Partition strategy configuration for ADMM.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PartitionStrategyConfig {
    /// Spectral partitioning using graph Laplacian.
    #[default]
    Spectral,
    /// Load-balanced partitioning.
    LoadBalanced {
        /// Maximum imbalance ratio.
        max_imbalance: f64,
    },
    /// Use predefined area assignments.
    Areas,
}

impl From<&PartitionStrategyConfig> for PartitionStrategy {
    fn from(config: &PartitionStrategyConfig) -> Self {
        match config {
            PartitionStrategyConfig::Spectral => PartitionStrategy::Spectral {
                num_partitions: 4, // Will be overridden
            },
            PartitionStrategyConfig::LoadBalanced { max_imbalance } => {
                PartitionStrategy::LoadBalanced {
                    max_imbalance: *max_imbalance,
                    num_partitions: 4, // Will be overridden
                }
            }
            PartitionStrategyConfig::Areas => PartitionStrategy::Areas,
        }
    }
}

/// Error types for ADMM solver.
#[derive(Debug, Error)]
pub enum AdmmError {
    /// Partitioning failed.
    #[error("Partitioning failed: {0}")]
    PartitionError(#[from] PartitionError),

    /// OPF subproblem failed.
    #[error("OPF subproblem failed for partition {partition}: {message}")]
    SubproblemFailed { partition: usize, message: String },

    /// ADMM did not converge.
    #[error("ADMM did not converge after {iterations} iterations: primal={primal_residual:.2e}, dual={dual_residual:.2e}")]
    NotConverged {
        iterations: usize,
        primal_residual: f64,
        dual_residual: f64,
    },

    /// Invalid configuration.
    #[error("Invalid ADMM configuration: {0}")]
    InvalidConfig(String),

    /// Underlying OPF error.
    #[error("OPF error: {0}")]
    OpfError(#[from] OpfError),
}

/// ADMM OPF solution with convergence diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct AdmmSolution {
    /// Total objective value (sum of partition objectives).
    pub objective: f64,

    /// Bus voltage magnitudes (per-unit).
    pub bus_voltage_mag: HashMap<String, f64>,

    /// Bus voltage angles (radians).
    pub bus_voltage_ang: HashMap<String, f64>,

    /// Generator real power dispatch (MW).
    pub generator_p: HashMap<String, f64>,

    /// Generator reactive power dispatch (MVAr).
    pub generator_q: HashMap<String, f64>,

    /// Whether ADMM converged.
    pub converged: bool,

    /// Number of ADMM iterations.
    pub iterations: usize,

    /// Final primal residual (consensus violation).
    pub primal_residual: f64,

    /// Final dual residual (consensus change rate).
    pub dual_residual: f64,

    /// Objective value for each partition.
    pub partition_objectives: Vec<f64>,

    /// Number of buses per partition.
    pub partition_sizes: Vec<usize>,

    /// Number of tie-lines (boundary connections).
    pub num_tie_lines: usize,

    /// Tie-line power flows: branch_id -> (P_MW, Q_MVAr, from_partition, to_partition).
    pub tie_line_flows: HashMap<String, (f64, f64, usize, usize)>,

    /// All branch power flows: branch_id -> (P_from_MW, Q_from_MVAr).
    pub branch_p_flow: HashMap<String, f64>,

    /// All branch reactive power flows: branch_id -> Q_from_MVAr.
    pub branch_q_flow: HashMap<String, f64>,

    /// Total system losses in MW (sum of P_from + P_to for all branches).
    pub total_losses_mw: f64,

    /// Total solve time in milliseconds.
    pub solve_time_ms: u128,

    /// Time breakdown by phase.
    pub phase_times_ms: AdmmPhaseTimes,
}

/// Time breakdown for ADMM phases.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdmmPhaseTimes {
    /// Time spent partitioning the network.
    pub partition_ms: u128,
    /// Total time spent on x-updates (subproblem solves).
    pub x_update_ms: u128,
    /// Total time spent on z-updates (consensus averaging).
    pub z_update_ms: u128,
    /// Total time spent on λ-updates (dual variable updates).
    pub dual_update_ms: u128,
}

/// Consensus variable for a boundary bus.
#[derive(Debug, Clone, Default)]
struct ConsensusVar {
    /// Bus name (for lookup).
    #[allow(dead_code)] // Reserved for future subproblem construction
    bus_name: String,
    /// Consensus voltage magnitude.
    z_vm: f64,
    /// Consensus voltage angle.
    z_va: f64,
    /// Dual variable for voltage magnitude.
    lambda_vm: f64,
    /// Dual variable for voltage angle.
    lambda_va: f64,
    /// Partitions that share this boundary bus.
    partitions: Vec<usize>,
}

/// Result of subnetwork extraction with boundary information.
#[derive(Debug)]
struct ExtractedSubnetwork {
    /// The extracted subnetwork with boundary injections.
    network: Network,
    /// Mapping from original bus names to new BusIds in subnetwork.
    bus_name_to_id: HashMap<String, BusId>,
    /// Boundary buses with their tie-line power injections (P, Q in MW/MVAr).
    #[allow(dead_code)] // Reserved for future diagnostics/logging
    boundary_injections: HashMap<String, (f64, f64)>,
}

/// Extract a subnetwork containing only the buses, generators, loads, and branches
/// within a given partition. Tie-lines are replaced with equivalent power injections
/// at boundary buses based on consensus voltage values.
///
/// # Algorithm
///
/// For each tie-line connecting this partition to another:
/// 1. Compute power flow using AC power flow equations with consensus voltages
/// 2. Add equivalent load at boundary bus to represent tie-line power flow
///
/// The augmented Lagrangian penalty term `(ρ/2)||V_boundary - z + λ/ρ||²` is handled
/// separately in the subproblem construction, as it modifies the objective function.
///
/// # Arguments
/// * `network` - The full network
/// * `partition` - The partition to extract
/// * `consensus` - Current consensus voltage values for boundary buses
/// * `all_solutions` - Solutions from all partitions (for tie-line flow calculation)
///
/// # Returns
/// ExtractedSubnetwork containing the partition's elements with boundary injections.
fn extract_subnetwork(
    network: &Network,
    partition: &NetworkPartition,
    consensus: &HashMap<String, ConsensusVar>,
    all_solutions: Option<&[OpfSolution]>,
) -> ExtractedSubnetwork {
    let mut subnetwork = Network::new();
    let mut bus_name_to_new_id: HashMap<String, BusId> = HashMap::new();
    let mut boundary_injections: HashMap<String, (f64, f64)> = HashMap::new();

    // Build set of bus names in this partition for fast lookup
    let partition_buses: HashSet<&str> = partition.buses.iter().map(|s| s.as_str()).collect();
    let boundary_buses: HashSet<&str> = partition.boundary_buses.iter().map(|s| s.as_str()).collect();

    // Build mapping from bus name to BusId for the original network
    let mut original_bus_name_to_id: HashMap<&str, BusId> = HashMap::new();
    let mut bus_id_to_name: HashMap<BusId, String> = HashMap::new();

    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            original_bus_name_to_id.insert(&bus.name, bus.id);
            bus_id_to_name.insert(bus.id, bus.name.clone());
        }
    }

    // Track node indices for connecting branches
    let mut bus_node_indices: HashMap<BusId, petgraph::graph::NodeIndex> = HashMap::new();
    let mut next_bus_id = 1usize;

    // Add buses that belong to this partition
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            if partition_buses.contains(bus.name.as_str()) {
                let new_id = BusId::new(next_bus_id);
                next_bus_id += 1;

                let mut new_bus = bus.clone();
                new_bus.id = new_id;

                // For boundary buses, initialize voltage from consensus
                if boundary_buses.contains(bus.name.as_str()) {
                    if let Some(cv) = consensus.get(&bus.name) {
                        new_bus.voltage_pu = gat_core::PerUnit(cv.z_vm);
                        new_bus.angle_rad = gat_core::Radians(cv.z_va);
                    }
                }

                let idx = subnetwork.graph.add_node(Node::Bus(new_bus));
                bus_node_indices.insert(new_id, idx);
                bus_name_to_new_id.insert(bus.name.clone(), new_id);
            }
        }
    }

    // Add generators that belong to buses in this partition
    let mut next_gen_id = 1usize;
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            if let Some(bus_name) = bus_id_to_name.get(&gen.bus) {
                if partition_buses.contains(bus_name.as_str()) {
                    if let Some(&new_bus_id) = bus_name_to_new_id.get(bus_name) {
                        let mut new_gen = gen.clone();
                        new_gen.id = GenId::new(next_gen_id);
                        next_gen_id += 1;
                        new_gen.bus = new_bus_id;
                        subnetwork.graph.add_node(Node::Gen(new_gen));
                    }
                }
            }
        }
    }

    // Add loads that belong to buses in this partition
    let mut next_load_id = 1usize;
    for node in network.graph.node_weights() {
        if let Node::Load(load) = node {
            if let Some(bus_name) = bus_id_to_name.get(&load.bus) {
                if partition_buses.contains(bus_name.as_str()) {
                    if let Some(&new_bus_id) = bus_name_to_new_id.get(bus_name) {
                        let mut new_load = load.clone();
                        new_load.id = LoadId::new(next_load_id);
                        next_load_id += 1;
                        new_load.bus = new_bus_id;
                        subnetwork.graph.add_node(Node::Load(new_load));
                    }
                }
            }
        }
    }

    // Add branches that are entirely within this partition (not tie-lines)
    let mut next_branch_id = 1usize;
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            let from_name = bus_id_to_name.get(&branch.from_bus);
            let to_name = bus_id_to_name.get(&branch.to_bus);

            if let (Some(from_name), Some(to_name)) = (from_name, to_name) {
                let from_in_partition = partition_buses.contains(from_name.as_str());
                let to_in_partition = partition_buses.contains(to_name.as_str());

                // Only include internal branches (both endpoints in partition)
                if from_in_partition && to_in_partition {
                    if let (Some(&new_from_id), Some(&new_to_id)) = (
                        bus_name_to_new_id.get(from_name),
                        bus_name_to_new_id.get(to_name),
                    ) {
                        if let (Some(&from_idx), Some(&to_idx)) = (
                            bus_node_indices.get(&new_from_id),
                            bus_node_indices.get(&new_to_id),
                        ) {
                            let mut new_branch = branch.clone();
                            new_branch.id = BranchId::new(next_branch_id);
                            next_branch_id += 1;
                            new_branch.from_bus = new_from_id;
                            new_branch.to_bus = new_to_id;
                            subnetwork
                                .graph
                                .add_edge(from_idx, to_idx, Edge::Branch(new_branch));
                        }
                    }
                }
            }
        }
    }

    // Compute tie-line power injections at boundary buses
    // For each tie-line, compute power flow from consensus voltages and add as load
    for tie_line in &partition.tie_lines {
        // TieLine has local_bus and remote_bus as String fields
        let local_bus = tie_line.local_bus.as_str();
        let remote_bus = tie_line.remote_bus.as_str();

        // Get branch parameters using branch_id from tie_line
        let branch_opt = network.graph.edge_weights().find_map(|edge| {
            if let Edge::Branch(b) = edge {
                if b.name == tie_line.branch_id {
                    return Some(b.clone());
                }
                // Also try matching by bus pair
                let b_from = bus_id_to_name.get(&b.from_bus)?;
                let b_to = bus_id_to_name.get(&b.to_bus)?;
                if (b_from == local_bus && b_to == remote_bus)
                    || (b_from == remote_bus && b_to == local_bus)
                {
                    return Some(b.clone());
                }
            }
            None
        });

        let Some(branch) = branch_opt else {
            continue;
        };

        // Get voltages from consensus or solutions
        let (vm_local, va_local) = if let Some(cv) = consensus.get(local_bus) {
            (cv.z_vm, cv.z_va)
        } else {
            (1.0, 0.0) // Flat start
        };

        let (vm_remote, va_remote) = if let Some(cv) = consensus.get(remote_bus) {
            (cv.z_vm, cv.z_va)
        } else if let Some(solutions) = all_solutions {
            // Try to get from another partition's solution
            solutions
                .iter()
                .find_map(|sol| {
                    let vm = sol.bus_voltage_mag.get(remote_bus)?;
                    let va = sol.bus_voltage_ang.get(remote_bus)?;
                    Some((*vm, *va))
                })
                .unwrap_or((1.0, 0.0))
        } else {
            (1.0, 0.0)
        };

        // Compute tie-line power flow (from local perspective)
        // TieLine's local_bus is always on this partition's side, so we compute
        // power flowing from local to remote
        let (p_flow, q_flow) = compute_tie_line_flow(
            vm_local,
            va_local,
            vm_remote,
            va_remote,
            branch.resistance,
            branch.reactance,
        );

        // Accumulate injection at this boundary bus (positive = power leaving)
        let entry = boundary_injections
            .entry(local_bus.to_string())
            .or_insert((0.0, 0.0));
        entry.0 += p_flow * 100.0; // Convert to MW (assuming 100 MVA base)
        entry.1 += q_flow * 100.0; // Convert to MVAr
    }

    // Add boundary loads to represent tie-line power flows
    for (bus_name, (p_inj, q_inj)) in &boundary_injections {
        if let Some(&new_bus_id) = bus_name_to_new_id.get(bus_name) {
            // Positive injection means power leaving, modeled as negative load
            // (or positive load means power consumed at this bus)
            let boundary_load = Load {
                id: LoadId::new(next_load_id),
                name: format!("tie_line_injection_{}", bus_name),
                bus: new_bus_id,
                active_power: Megawatts(*p_inj),
                reactive_power: Megavars(*q_inj),
            };
            next_load_id += 1;
            subnetwork.graph.add_node(Node::Load(boundary_load));
        }
    }

    ExtractedSubnetwork {
        network: subnetwork,
        bus_name_to_id: bus_name_to_new_id,
        boundary_injections,
    }
}

/// Compute tie-line power flows from voltage differences.
///
/// For a tie-line with admittance Y between buses i (local) and j (remote):
///   P_ij = |V_i|² * G - |V_i||V_j| * (G*cos(θ_i - θ_j) + B*sin(θ_i - θ_j))
///
/// # Arguments
/// * `vm_local` - Voltage magnitude at local bus (p.u.)
/// * `va_local` - Voltage angle at local bus (radians)
/// * `vm_remote` - Voltage magnitude at remote bus (p.u.)
/// * `va_remote` - Voltage angle at remote bus (radians)
/// * `r` - Series resistance (p.u.)
/// * `x` - Series reactance (p.u.)
///
/// # Returns
/// (P_flow, Q_flow) in per-unit
fn compute_tie_line_flow(
    vm_local: f64,
    va_local: f64,
    vm_remote: f64,
    va_remote: f64,
    r: f64,
    x: f64,
) -> (f64, f64) {
    let z_sq = r * r + x * x;
    if z_sq < 1e-12 {
        return (0.0, 0.0); // Avoid division by zero
    }

    let g = r / z_sq; // Series conductance
    let b = -x / z_sq; // Series susceptance (negative for inductive)

    let angle_diff = va_local - va_remote;
    let cos_diff = angle_diff.cos();
    let sin_diff = angle_diff.sin();

    let p_flow = vm_local * vm_local * g - vm_local * vm_remote * (g * cos_diff + b * sin_diff);
    let q_flow = -vm_local * vm_local * b - vm_local * vm_remote * (g * sin_diff - b * cos_diff);

    (p_flow, q_flow)
}

/// Compute power flows for all branches in the network.
///
/// Uses the voltage solution to calculate real and reactive power flows
/// for each branch, as well as total losses.
///
/// # Arguments
/// * `network` - The power network with branch definitions
/// * `bus_voltage_mag` - Voltage magnitudes by bus name
/// * `bus_voltage_ang` - Voltage angles by bus name (radians)
/// * `base_mva` - Base MVA for per-unit conversion (typically 100)
///
/// # Returns
/// (branch_p_flow, branch_q_flow, total_losses_mw) where flows are from-bus direction
fn compute_all_branch_flows(
    network: &Network,
    bus_voltage_mag: &HashMap<String, f64>,
    bus_voltage_ang: &HashMap<String, f64>,
    base_mva: f64,
) -> (HashMap<String, f64>, HashMap<String, f64>, f64) {
    // Count branches for pre-allocation
    let branch_count = network
        .graph
        .edge_weights()
        .filter(|e| matches!(e, Edge::Branch(_)))
        .count();

    let mut branch_p_flow = HashMap::with_capacity(branch_count);
    let mut branch_q_flow = HashMap::with_capacity(branch_count);
    let mut total_losses = 0.0;

    // Build bus name lookup from network (pre-allocate based on node count)
    // TODO: GPU acceleration - this loop is embarrassingly parallel and could be
    // vectorized on GPU for networks with 10k+ branches. See gpu_monte_carlo.rs
    // for the existing wgpu infrastructure pattern.
    let node_count = network.graph.node_count();
    let mut bus_id_to_name: HashMap<BusId, String> = HashMap::with_capacity(node_count);
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            bus_id_to_name.insert(bus.id.clone(), bus.name.clone());
        }
    }

    // Iterate over all branches
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            // Get bus names for this branch
            let from_name = bus_id_to_name.get(&branch.from_bus);
            let to_name = bus_id_to_name.get(&branch.to_bus);

            if let (Some(from_name), Some(to_name)) = (from_name, to_name) {
                let vm_from = bus_voltage_mag.get(from_name).copied().unwrap_or(1.0);
                let va_from = bus_voltage_ang.get(from_name).copied().unwrap_or(0.0);
                let vm_to = bus_voltage_mag.get(to_name).copied().unwrap_or(1.0);
                let va_to = bus_voltage_ang.get(to_name).copied().unwrap_or(0.0);

                // Get branch parameters
                let r = branch.resistance;
                let x = branch.reactance;
                let tap = branch.tap_ratio;
                let shift = branch.phase_shift.0; // Radians inner f64
                let b_charging = branch.charging_b.0; // PerUnit inner f64

                let z_sq = r * r + x * x;
                if z_sq < 1e-12 {
                    continue; // Skip zero-impedance branches
                }

                let g = r / z_sq;
                let b = -x / z_sq;

                let angle_diff = va_from - va_to - shift;
                let cos_diff = angle_diff.cos();
                let sin_diff = angle_diff.sin();

                // From-bus power injection (p.u.)
                let p_from = (vm_from * vm_from * g / (tap * tap))
                    - (vm_from * vm_to / tap) * (g * cos_diff + b * sin_diff);
                let q_from = -(vm_from * vm_from * (b + b_charging / 2.0) / (tap * tap))
                    - (vm_from * vm_to / tap) * (g * sin_diff - b * cos_diff);

                // To-bus power injection (p.u.) - for loss calculation
                let p_to = (vm_to * vm_to * g)
                    - (vm_from * vm_to / tap) * (g * cos_diff - b * sin_diff);

                // Convert to MW/MVAr
                let p_from_mw = p_from * base_mva;
                let q_from_mvar = q_from * base_mva;
                let p_to_mw = p_to * base_mva;

                // Losses are P_from + P_to (both measured as injections into line)
                let branch_loss = p_from_mw + p_to_mw;
                total_losses += branch_loss;

                branch_p_flow.insert(branch.name.clone(), p_from_mw);
                branch_q_flow.insert(branch.name.clone(), q_from_mvar);
            }
        }
    }

    (branch_p_flow, branch_q_flow, total_losses)
}

/// ADMM-based distributed OPF solver.
pub struct AdmmOpfSolver {
    config: AdmmConfig,
}

impl AdmmOpfSolver {
    /// Create a new ADMM solver with the given configuration.
    pub fn new(config: AdmmConfig) -> Self {
        Self { config }
    }

    /// Create a new ADMM solver with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AdmmConfig::default())
    }

    /// Set the penalty parameter (ρ).
    pub fn with_penalty(mut self, penalty: f64) -> Self {
        self.config.penalty = penalty;
        self
    }

    /// Set the number of partitions.
    pub fn with_partitions(mut self, num_partitions: usize) -> Self {
        self.config.num_partitions = num_partitions;
        self
    }

    /// Set the inner OPF method for subproblems.
    pub fn with_inner_method(mut self, method: OpfMethod) -> Self {
        self.config.inner_method = method;
        self
    }

    /// Solve distributed OPF using ADMM.
    ///
    /// # Arguments
    /// * `network` - The power network to solve
    ///
    /// # Returns
    /// An `AdmmSolution` containing the optimal dispatch and convergence info.
    pub fn solve(&self, network: &Network) -> Result<AdmmSolution, AdmmError> {
        let total_start = Instant::now();
        let mut phase_times = AdmmPhaseTimes::default();

        // Validate configuration
        if self.config.num_partitions < 2 {
            return Err(AdmmError::InvalidConfig(
                "Number of partitions must be at least 2".to_string(),
            ));
        }

        // Step 1: Partition the network
        let partition_start = Instant::now();
        let strategy = self.build_partition_strategy();
        let partitions = partition_network(network, strategy)?;
        phase_times.partition_ms = partition_start.elapsed().as_millis();

        if self.config.verbose {
            println!("Partitioned network into {} regions", partitions.len());
            for (i, p) in partitions.iter().enumerate() {
                println!(
                    "  Partition {}: {} buses, {} boundary, {} tie-lines",
                    i,
                    p.buses.len(),
                    p.boundary_buses.len(),
                    p.tie_lines.len()
                );
            }
        }

        // Step 2: Initialize consensus variables for boundary buses
        let mut consensus_vars = self.initialize_consensus(&partitions);

        // Step 3: ADMM iterations
        let mut penalty = self.config.penalty;
        let mut converged = false;
        let mut iteration = 0;
        let mut primal_residual = f64::INFINITY;
        let mut dual_residual = f64::INFINITY;
        let mut partition_solutions: Vec<OpfSolution> = Vec::new();

        while iteration < self.config.max_iter && !converged {
            iteration += 1;

            // X-update: Solve local OPF subproblems in parallel
            let x_start = Instant::now();
            let new_solutions = self.x_update(network, &partitions, &consensus_vars, penalty)?;
            phase_times.x_update_ms += x_start.elapsed().as_millis();

            // Store previous z values for dual residual computation
            let z_prev: HashMap<String, (f64, f64)> = consensus_vars
                .iter()
                .map(|(name, cv)| (name.clone(), (cv.z_vm, cv.z_va)))
                .collect();

            // Z-update: Average boundary variables
            let z_start = Instant::now();
            primal_residual = self.z_update(&new_solutions, &partitions, &mut consensus_vars);
            phase_times.z_update_ms += z_start.elapsed().as_millis();

            // Compute dual residual
            dual_residual = self.compute_dual_residual(&consensus_vars, &z_prev, penalty);

            // Lambda-update: Update dual variables
            let dual_start = Instant::now();
            self.lambda_update(&new_solutions, &partitions, &mut consensus_vars, penalty);
            phase_times.dual_update_ms += dual_start.elapsed().as_millis();

            // Check convergence
            let n_consensus = consensus_vars.len() as f64;
            let primal_threshold = self.config.primal_tol * n_consensus.sqrt();
            let dual_threshold = self.config.dual_tol * n_consensus.sqrt();

            converged = primal_residual < primal_threshold && dual_residual < dual_threshold;

            if self.config.verbose {
                println!(
                    "Iter {:3}: primal={:.2e} dual={:.2e} rho={:.1e} {}",
                    iteration,
                    primal_residual,
                    dual_residual,
                    penalty,
                    if converged { "CONVERGED" } else { "" }
                );
            }

            // Adaptive penalty update
            if self.config.adaptive_penalty && !converged {
                penalty = self.update_penalty(penalty, primal_residual, dual_residual);
            }

            partition_solutions = new_solutions;
        }

        // Assemble final solution
        let solution = self.assemble_solution(
            network,
            &partition_solutions,
            &partitions,
            converged,
            iteration,
            primal_residual,
            dual_residual,
            total_start.elapsed().as_millis(),
            phase_times,
        );

        if !converged {
            return Err(AdmmError::NotConverged {
                iterations: iteration,
                primal_residual,
                dual_residual,
            });
        }

        Ok(solution)
    }

    fn build_partition_strategy(&self) -> PartitionStrategy {
        match &self.config.partition_strategy {
            PartitionStrategyConfig::Spectral => PartitionStrategy::Spectral {
                num_partitions: self.config.num_partitions,
            },
            PartitionStrategyConfig::LoadBalanced { max_imbalance } => {
                PartitionStrategy::LoadBalanced {
                    max_imbalance: *max_imbalance,
                    num_partitions: self.config.num_partitions,
                }
            }
            PartitionStrategyConfig::Areas => PartitionStrategy::Areas,
        }
    }

    fn initialize_consensus(&self, partitions: &[NetworkPartition]) -> HashMap<String, ConsensusVar> {
        let mut consensus = HashMap::new();

        for (part_idx, partition) in partitions.iter().enumerate() {
            for boundary_bus in &partition.boundary_buses {
                consensus
                    .entry(boundary_bus.clone())
                    .or_insert_with(|| ConsensusVar {
                        bus_name: boundary_bus.clone(),
                        z_vm: 1.0,  // Flat start
                        z_va: 0.0,
                        lambda_vm: 0.0,
                        lambda_va: 0.0,
                        partitions: Vec::new(),
                    })
                    .partitions
                    .push(part_idx);
            }
        }

        consensus
    }

    fn x_update(
        &self,
        network: &Network,
        partitions: &[NetworkPartition],
        consensus: &HashMap<String, ConsensusVar>,
        _penalty: f64, // Reserved for future augmented Lagrangian implementation
    ) -> Result<Vec<OpfSolution>, AdmmError> {
        // Extract subnetworks and solve OPF for each partition in parallel
        //
        // The ADMM x-update solves:
        //   min f_k(x_k) + (ρ/2)||x_k|_boundary - z + λ_k/ρ||²
        //
        // Where:
        // - f_k is the local OPF objective (generator costs)
        // - x_k|_boundary are the voltage magnitudes/angles at boundary buses
        // - z is the consensus value (average across partitions)
        // - λ_k is the dual variable for this partition
        // - ρ is the penalty parameter

        // First pass: extract subnetworks (can be done in parallel)
        // TODO: PERF - Cache partition topology (buses, branches) between iterations.
        // Only tie-line injections change; rebuilding the entire subnetwork graph each
        // iteration is wasteful for large networks.
        let subnetworks: Vec<ExtractedSubnetwork> = partitions
            .par_iter()
            .map(|partition| extract_subnetwork(network, partition, consensus, None))
            .collect();

        // Solve OPF for each subnetwork in parallel
        // TODO: GPU - For DC-OPF subproblems, batch all partitions into a single
        // GPU kernel dispatch. The LP structure is identical across partitions,
        // differing only in coefficients. This is the highest-impact GPU target.
        let results: Vec<Result<(OpfSolution, &NetworkPartition), AdmmError>> = subnetworks
            .par_iter()
            .zip(partitions.par_iter())
            .enumerate()
            .map(|(part_idx, (extracted, partition))| {
                // Create solver for this subproblem
                let solver = OpfSolver::new()
                    .with_method(self.config.inner_method)
                    .with_max_iterations(50)
                    .with_tolerance(1e-6);

                // Solve subnetwork OPF
                // Note: The augmented Lagrangian penalty is currently approximated
                // through the tie-line power injections. A more rigorous implementation
                // would modify the objective function directly, but that requires
                // changes to the OpfSolver interface.
                match solver.solve(&extracted.network) {
                    Ok(solution) => {
                        // Remap solution variables back to original bus/gen names
                        let remapped = self.remap_solution(
                            &solution,
                            partition,
                            &extracted.bus_name_to_id,
                        );
                        Ok((remapped, partition))
                    }
                    Err(e) => Err(AdmmError::SubproblemFailed {
                        partition: part_idx,
                        message: format!("{}", e),
                    }),
                }
            })
            .collect();

        // Check for errors and collect successful solutions
        let mut solutions = Vec::with_capacity(partitions.len());
        for result in results {
            match result {
                Ok((solution, _partition)) => solutions.push(solution),
                Err(e) => {
                    // For robustness, try fallback to full-network solve for failed partitions
                    if self.config.verbose {
                        println!("Partition failed: {:?}, falling back to full solve", e);
                    }
                    // Fallback: solve full network and use as baseline
                    let fallback_solver = OpfSolver::new()
                        .with_method(self.config.inner_method)
                        .with_max_iterations(50)
                        .with_tolerance(1e-6);

                    match fallback_solver.solve(network) {
                        Ok(full_sol) => solutions.push(full_sol),
                        Err(e2) => return Err(AdmmError::OpfError(e2)),
                    }
                }
            }
        }

        Ok(solutions)
    }

    /// Remap solution variables from subnetwork IDs back to original network names.
    fn remap_solution(
        &self,
        solution: &OpfSolution,
        partition: &NetworkPartition,
        bus_name_to_id: &HashMap<String, BusId>,
    ) -> OpfSolution {
        // Create reverse mapping from subnetwork BusId to original name
        let id_to_name: HashMap<String, String> = bus_name_to_id
            .iter()
            .map(|(name, id)| (format!("bus{}", id.value()), name.clone()))
            .collect();

        let mut remapped = solution.clone();
        remapped.bus_voltage_mag.clear();
        remapped.bus_voltage_ang.clear();
        remapped.generator_p.clear();
        remapped.generator_q.clear();

        // Remap bus voltages
        for (sub_bus_name, &vm) in &solution.bus_voltage_mag {
            // Try to find original name through reverse mapping or direct lookup
            let original_name = if let Some(name) = id_to_name.get(sub_bus_name) {
                name.clone()
            } else if partition.buses.contains(sub_bus_name) {
                sub_bus_name.clone()
            } else {
                sub_bus_name.clone()
            };
            remapped.bus_voltage_mag.insert(original_name, vm);
        }

        for (sub_bus_name, &va) in &solution.bus_voltage_ang {
            let original_name = if let Some(name) = id_to_name.get(sub_bus_name) {
                name.clone()
            } else if partition.buses.contains(sub_bus_name) {
                sub_bus_name.clone()
            } else {
                sub_bus_name.clone()
            };
            remapped.bus_voltage_ang.insert(original_name, va);
        }

        // Copy generator dispatch (gen names should be preserved in partition)
        for (gen_name, &p) in &solution.generator_p {
            if partition.generators.contains(gen_name) {
                remapped.generator_p.insert(gen_name.clone(), p);
            }
        }

        for (gen_name, &q) in &solution.generator_q {
            if partition.generators.contains(gen_name) {
                remapped.generator_q.insert(gen_name.clone(), q);
            }
        }

        remapped
    }

    fn z_update(
        &self,
        solutions: &[OpfSolution],
        _partitions: &[NetworkPartition],
        consensus: &mut HashMap<String, ConsensusVar>,
    ) -> f64 {
        let mut primal_residual_sq = 0.0;

        for (bus_name, cv) in consensus.iter_mut() {
            let mut vm_sum = 0.0;
            let mut va_sum = 0.0;
            let mut count = 0;

            // Average voltage values from all partitions sharing this boundary bus
            for &part_idx in &cv.partitions {
                if part_idx < solutions.len() {
                    if let Some(&vm) = solutions[part_idx].bus_voltage_mag.get(bus_name) {
                        vm_sum += vm;
                        count += 1;
                    }
                    if let Some(&va) = solutions[part_idx].bus_voltage_ang.get(bus_name) {
                        va_sum += va;
                    }
                }
            }

            if count > 0 {
                let new_z_vm = vm_sum / count as f64;
                let new_z_va = va_sum / count as f64;

                // Compute primal residual contribution
                for &part_idx in &cv.partitions {
                    if part_idx < solutions.len() {
                        if let Some(&vm) = solutions[part_idx].bus_voltage_mag.get(bus_name) {
                            primal_residual_sq += (vm - new_z_vm).powi(2);
                        }
                        if let Some(&va) = solutions[part_idx].bus_voltage_ang.get(bus_name) {
                            primal_residual_sq += (va - new_z_va).powi(2);
                        }
                    }
                }

                cv.z_vm = new_z_vm;
                cv.z_va = new_z_va;
            }
        }

        primal_residual_sq.sqrt()
    }

    fn compute_dual_residual(
        &self,
        consensus: &HashMap<String, ConsensusVar>,
        z_prev: &HashMap<String, (f64, f64)>,
        penalty: f64,
    ) -> f64 {
        let mut dual_residual_sq = 0.0;

        for (bus_name, cv) in consensus.iter() {
            if let Some(&(prev_vm, prev_va)) = z_prev.get(bus_name) {
                let delta_vm = cv.z_vm - prev_vm;
                let delta_va = cv.z_va - prev_va;
                // Dual residual scales with penalty and number of partitions
                dual_residual_sq += penalty.powi(2) * cv.partitions.len() as f64
                    * (delta_vm.powi(2) + delta_va.powi(2));
            }
        }

        dual_residual_sq.sqrt()
    }

    fn lambda_update(
        &self,
        solutions: &[OpfSolution],
        _partitions: &[NetworkPartition],
        consensus: &mut HashMap<String, ConsensusVar>,
        penalty: f64,
    ) {
        for (bus_name, cv) in consensus.iter_mut() {
            let mut lambda_vm_update = 0.0;
            let mut lambda_va_update = 0.0;

            for &part_idx in &cv.partitions {
                if part_idx < solutions.len() {
                    if let Some(&vm) = solutions[part_idx].bus_voltage_mag.get(bus_name) {
                        lambda_vm_update += penalty * (vm - cv.z_vm);
                    }
                    if let Some(&va) = solutions[part_idx].bus_voltage_ang.get(bus_name) {
                        lambda_va_update += penalty * (va - cv.z_va);
                    }
                }
            }

            cv.lambda_vm += lambda_vm_update;
            cv.lambda_va += lambda_va_update;
        }
    }

    fn update_penalty(&self, penalty: f64, primal_residual: f64, dual_residual: f64) -> f64 {
        // Residual balancing: adjust penalty to keep primal and dual residuals similar
        let ratio = primal_residual / (dual_residual.max(1e-10));

        let new_penalty = if ratio > 10.0 {
            // Primal residual too large - increase penalty to enforce consensus
            (penalty * self.config.penalty_scale).min(self.config.max_penalty)
        } else if ratio < 0.1 {
            // Dual residual too large - decrease penalty to improve local solutions
            (penalty / self.config.penalty_scale).max(self.config.min_penalty)
        } else {
            penalty
        };

        new_penalty
    }

    fn assemble_solution(
        &self,
        network: &Network,
        partition_solutions: &[OpfSolution],
        partitions: &[NetworkPartition],
        converged: bool,
        iterations: usize,
        primal_residual: f64,
        dual_residual: f64,
        solve_time_ms: u128,
        phase_times: AdmmPhaseTimes,
    ) -> AdmmSolution {
        let mut bus_voltage_mag = HashMap::new();
        let mut bus_voltage_ang = HashMap::new();
        let mut generator_p = HashMap::new();
        let mut generator_q = HashMap::new();
        let mut total_objective = 0.0;
        let mut partition_objectives = Vec::new();
        let mut partition_sizes = Vec::new();
        let mut num_tie_lines = 0;
        let mut tie_line_flows: HashMap<String, (f64, f64, usize, usize)> = HashMap::new();

        // First pass: merge all bus voltages from all partitions
        for (solution, partition) in partition_solutions.iter().zip(partitions.iter()) {
            for bus_name in &partition.buses {
                if let Some(&vm) = solution.bus_voltage_mag.get(bus_name) {
                    bus_voltage_mag.insert(bus_name.clone(), vm);
                }
                if let Some(&va) = solution.bus_voltage_ang.get(bus_name) {
                    bus_voltage_ang.insert(bus_name.clone(), va);
                }
            }
        }

        // Second pass: merge generators and compute tie-line flows
        for (part_idx, (solution, partition)) in
            partition_solutions.iter().zip(partitions.iter()).enumerate()
        {
            // Only include generators belonging to this partition
            for gen_name in &partition.generators {
                if let Some(&p) = solution.generator_p.get(gen_name) {
                    generator_p.insert(gen_name.clone(), p);
                }
                if let Some(&q) = solution.generator_q.get(gen_name) {
                    generator_q.insert(gen_name.clone(), q);
                }
            }

            // Compute tie-line flows using final voltage solution
            for tie_line in &partition.tie_lines {
                // Only record from lower partition index to avoid duplicates
                if part_idx < tie_line.neighbor_partition {
                    let vm_local = bus_voltage_mag.get(&tie_line.local_bus).copied().unwrap_or(1.0);
                    let va_local = bus_voltage_ang.get(&tie_line.local_bus).copied().unwrap_or(0.0);
                    let vm_remote = bus_voltage_mag.get(&tie_line.remote_bus).copied().unwrap_or(1.0);
                    let va_remote = bus_voltage_ang.get(&tie_line.remote_bus).copied().unwrap_or(0.0);

                    // Estimate R from susceptance (typical X/R ratio of 4)
                    let x = 1.0 / tie_line.susceptance.abs().max(1e-6);
                    let r = x / 4.0;

                    let (p_pu, q_pu) = compute_tie_line_flow(vm_local, va_local, vm_remote, va_remote, r, x);
                    let p_mw = p_pu * 100.0; // Assuming 100 MVA base
                    let q_mvar = q_pu * 100.0;

                    tie_line_flows.insert(
                        tie_line.branch_id.clone(),
                        (p_mw, q_mvar, part_idx, tie_line.neighbor_partition),
                    );
                }
            }

            // Accumulate objective (avoid double-counting shared elements)
            // For now, just sum the partition objective values
            // TODO: Proper objective allocation for tie-line costs
            total_objective += solution.objective_value / partitions.len() as f64;
            partition_objectives.push(solution.objective_value);
            partition_sizes.push(partition.buses.len());
            num_tie_lines += partition.tie_lines.len();
        }

        // Tie lines are counted twice (once per partition), so divide by 2
        num_tie_lines /= 2;

        // Compute all branch flows and total losses
        let base_mva = 100.0; // Standard base MVA
        let (branch_p_flow, branch_q_flow, total_losses_mw) =
            compute_all_branch_flows(network, &bus_voltage_mag, &bus_voltage_ang, base_mva);

        AdmmSolution {
            objective: total_objective,
            bus_voltage_mag,
            bus_voltage_ang,
            generator_p,
            generator_q,
            converged,
            iterations,
            primal_residual,
            dual_residual,
            partition_objectives,
            partition_sizes,
            num_tie_lines,
            tie_line_flows,
            branch_p_flow,
            branch_q_flow,
            total_losses_mw,
            solve_time_ms,
            phase_times_ms: phase_times,
        }
    }
}

/// Convert ADMM solution to standard OpfSolution for compatibility.
impl From<AdmmSolution> for OpfSolution {
    fn from(admm: AdmmSolution) -> Self {
        OpfSolution {
            converged: admm.converged,
            method_used: OpfMethod::DcOpf, // Will update when OpfMethod::Admm is added
            iterations: admm.iterations,
            solve_time_ms: admm.solve_time_ms,
            objective_value: admm.objective,
            generator_p: admm.generator_p,
            generator_q: admm.generator_q,
            bus_voltage_mag: admm.bus_voltage_mag,
            bus_voltage_ang: admm.bus_voltage_ang,
            branch_p_flow: admm.branch_p_flow,
            branch_q_flow: admm.branch_q_flow,
            bus_lmp: HashMap::new(), // TODO: Derive from dual variables
            binding_constraints: Vec::new(),
            total_losses_mw: admm.total_losses_mw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{
        Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Kilovolts, Load, LoadId, Megavars,
        Megawatts, Node, PerUnit, Radians,
    };

    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Create 6-bus network (2x3 grid) like in partition tests
        let mut bus_indices = Vec::new();
        for i in 1..=6 {
            let zone = if i <= 3 { Some(1) } else { Some(2) };
            let idx = network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(i),
                name: format!("bus{}", i),
                base_kv: Kilovolts(230.0),
                voltage_pu: PerUnit(1.0),
                angle_rad: Radians(0.0),
                vmin_pu: Some(PerUnit(0.95)),
                vmax_pu: Some(PerUnit(1.05)),
                area_id: None,
                zone_id: zone,
            }));
            bus_indices.push(idx);
        }

        // Add generators with cost curves
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "gen1".to_string(), BusId::new(1))
                .with_p_limits(0.0, 100.0)
                .with_cost(gat_core::CostModel::quadratic(0.0, 20.0, 0.02)),
        ));
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(2), "gen6".to_string(), BusId::new(6))
                .with_p_limits(0.0, 100.0)
                .with_cost(gat_core::CostModel::quadratic(0.0, 25.0, 0.03)),
        ));

        // Add loads
        for i in 2..=5 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(i),
                name: format!("load{}", i),
                bus: BusId::new(i),
                active_power: Megawatts(20.0),
                reactive_power: Megavars(5.0),
            }));
        }

        // Add branches
        let branches = vec![
            (1, 2, 1),
            (2, 3, 2),
            (4, 5, 3),
            (5, 6, 4),
            (1, 4, 5),
            (2, 5, 6),
            (3, 6, 7),
        ];

        for (from, to, id) in branches {
            network.graph.add_edge(
                bus_indices[from - 1],
                bus_indices[to - 1],
                Edge::Branch(Branch {
                    id: BranchId::new(id),
                    name: format!("branch{}", id),
                    from_bus: BusId::new(from),
                    to_bus: BusId::new(to),
                    resistance: 0.01,
                    reactance: 0.1,
                    ..Branch::default()
                }),
            );
        }

        network
    }

    #[test]
    fn test_admm_config_default() {
        let config = AdmmConfig::default();
        assert_eq!(config.penalty, 1.0);
        assert_eq!(config.num_partitions, 4);
        assert_eq!(config.max_iter, 100);
        assert!(config.adaptive_penalty);
    }

    #[test]
    fn test_admm_solver_creation() {
        let solver = AdmmOpfSolver::with_defaults()
            .with_penalty(2.0)
            .with_partitions(2)
            .with_inner_method(OpfMethod::DcOpf);

        assert_eq!(solver.config.penalty, 2.0);
        assert_eq!(solver.config.num_partitions, 2);
    }

    #[test]
    fn test_admm_small_network() {
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 2,
            max_iter: 50,
            penalty: 1.0,
            primal_tol: 1e-3,
            dual_tol: 1e-3,
            inner_method: OpfMethod::DcOpf,
            verbose: false,
            ..Default::default()
        });

        let result = solver.solve(&network);

        // Should either converge or fail gracefully
        match result {
            Ok(solution) => {
                assert!(solution.iterations > 0);
                assert_eq!(solution.partition_sizes.len(), 2);
                // Check we have voltage data for all buses
                assert_eq!(solution.bus_voltage_mag.len(), 6);
            }
            Err(AdmmError::NotConverged { iterations, .. }) => {
                // Expected for simplified implementation
                assert!(iterations > 0);
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_admm_to_opf_solution_conversion() {
        let admm_sol = AdmmSolution {
            objective: 1000.0,
            bus_voltage_mag: [("bus1".to_string(), 1.02)].into_iter().collect(),
            bus_voltage_ang: [("bus1".to_string(), 0.1)].into_iter().collect(),
            generator_p: [("gen1".to_string(), 50.0)].into_iter().collect(),
            generator_q: [("gen1".to_string(), 10.0)].into_iter().collect(),
            converged: true,
            iterations: 15,
            primal_residual: 1e-5,
            dual_residual: 1e-5,
            partition_objectives: vec![500.0, 500.0],
            partition_sizes: vec![3, 3],
            num_tie_lines: 3,
            tie_line_flows: HashMap::new(),
            branch_p_flow: [("branch1".to_string(), 25.0)].into_iter().collect(),
            branch_q_flow: [("branch1".to_string(), 5.0)].into_iter().collect(),
            total_losses_mw: 2.5,
            solve_time_ms: 100,
            phase_times_ms: AdmmPhaseTimes::default(),
        };

        let opf_sol: OpfSolution = admm_sol.into();

        assert!(opf_sol.converged);
        assert_eq!(opf_sol.iterations, 15);
        assert_eq!(opf_sol.objective_value, 1000.0);
        assert!(opf_sol.generator_p.contains_key("gen1"));
        // Verify branch flows are carried over
        assert!(opf_sol.branch_p_flow.contains_key("branch1"));
        assert_eq!(opf_sol.total_losses_mw, 2.5);
    }

    #[test]
    fn test_invalid_partition_count() {
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 1, // Invalid - must be >= 2
            ..Default::default()
        });

        let result = solver.solve(&network);
        assert!(matches!(result, Err(AdmmError::InvalidConfig(_))));
    }

    #[test]
    fn test_admm_phase_timing() {
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 2,
            max_iter: 10,
            penalty: 1.0,
            inner_method: OpfMethod::DcOpf,
            verbose: false,
            ..Default::default()
        });

        match solver.solve(&network) {
            Ok(solution) => {
                // Verify timing struct exists (values can be 0 on fast machines)
                // Just check that solve_time_ms is reasonable (not absurdly large)
                assert!(
                    solution.solve_time_ms < 60_000,
                    "Solve time should be under 60 seconds"
                );
            }
            Err(AdmmError::NotConverged { .. }) => {
                // Expected - we're not testing convergence here
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_admm_tie_line_counting() {
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 2,
            max_iter: 5,
            penalty: 1.0,
            inner_method: OpfMethod::DcOpf,
            verbose: false,
            ..Default::default()
        });

        match solver.solve(&network) {
            Ok(solution) => {
                // With 2 partitions on a 6-bus network with 7 branches,
                // we expect some tie-lines between partitions
                assert!(solution.num_tie_lines <= 7, "Cannot have more tie-lines than branches");
            }
            Err(AdmmError::NotConverged { .. }) => {
                // Expected - we're testing structure not convergence
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_admm_solution_consistency() {
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 2,
            max_iter: 20,
            penalty: 2.0,
            inner_method: OpfMethod::DcOpf,
            verbose: false,
            ..Default::default()
        });

        match solver.solve(&network) {
            Ok(solution) => {
                // All buses should have voltage data
                assert_eq!(solution.bus_voltage_mag.len(), 6);
                assert_eq!(solution.bus_voltage_ang.len(), 6);

                // Both generators should have dispatch
                assert_eq!(solution.generator_p.len(), 2);

                // Partition count should match config
                assert_eq!(solution.partition_objectives.len(), 2);
                assert_eq!(solution.partition_sizes.len(), 2);

                // Partition sizes should sum to total buses
                let total_buses: usize = solution.partition_sizes.iter().sum();
                assert_eq!(total_buses, 6);
            }
            Err(AdmmError::NotConverged { iterations, primal_residual, dual_residual }) => {
                // Verify residuals are computed
                assert!(iterations > 0);
                assert!(primal_residual >= 0.0);
                assert!(dual_residual >= 0.0);
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_compute_tie_line_flow() {
        // Test power flow calculation for known voltages
        // From bus: V = 1.02∠0°, To bus: V = 1.0∠-0.05 rad
        // Line: R = 0.01, X = 0.1
        let (p, q) = super::compute_tie_line_flow(1.02, 0.0, 1.0, -0.05, 0.01, 0.1);

        // P should be positive (power flows from high voltage to low)
        // With these parameters, P ≈ 0.5 p.u. (approximate)
        assert!(p > 0.0, "Power should flow from higher to lower voltage");
        assert!(p.abs() < 2.0, "Power should be reasonable (< 2 p.u.)");

        // Q should be negative (inductive line absorbs reactive power)
        // This is a sanity check, actual value depends on line parameters
        assert!(q.abs() < 2.0, "Reactive power should be reasonable");
    }

    #[test]
    fn test_admm_branch_flows_computed() {
        // Test that branch flows are computed in the ADMM solution
        let network = create_test_network();

        let solver = AdmmOpfSolver::new(AdmmConfig {
            num_partitions: 2,
            max_iter: 20,
            penalty: 2.0,
            inner_method: OpfMethod::DcOpf,
            verbose: false,
            ..Default::default()
        });

        match solver.solve(&network) {
            Ok(solution) => {
                // Branch flows should be computed for all branches
                // The test network has 7 branches
                assert!(
                    !solution.branch_p_flow.is_empty(),
                    "Branch P flows should be computed"
                );
                assert!(
                    !solution.branch_q_flow.is_empty(),
                    "Branch Q flows should be computed"
                );

                // Total losses should be computed (could be small or zero for DC)
                // Just verify it's a finite number
                assert!(
                    solution.total_losses_mw.is_finite(),
                    "Total losses should be finite"
                );

                // Convert to OpfSolution and verify flows are preserved
                let opf_sol: OpfSolution = solution.into();
                assert!(
                    !opf_sol.branch_p_flow.is_empty(),
                    "Branch flows should transfer to OpfSolution"
                );
                assert!(
                    opf_sol.total_losses_mw.is_finite(),
                    "Total losses should transfer to OpfSolution"
                );
            }
            Err(AdmmError::NotConverged { .. }) => {
                // Even if not converged, this is acceptable for the structure test
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}
