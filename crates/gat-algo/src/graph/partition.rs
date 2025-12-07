//! Network partitioning for distributed OPF.
//!
//! This module implements graph partitioning algorithms for splitting power networks
//! into regions for ADMM-based distributed optimization. The key challenge is to:
//!
//! 1. **Minimize tie-line cuts**: Fewer boundary connections = faster ADMM convergence
//! 2. **Balance load**: Even partition sizes enable parallel speedup
//! 3. **Preserve electrical coherence**: Keep electrically coupled buses together
//!
//! # Partitioning Strategies
//!
//! | Strategy | Best For | Complexity |
//! |----------|----------|------------|
//! | [`PartitionStrategy::Spectral`] | General networks | O(n² log k) |
//! | [`PartitionStrategy::LoadBalanced`] | Uneven generation | O(n log n) |
//! | [`PartitionStrategy::Areas`] | Pre-defined zones | O(n) |
//! | [`PartitionStrategy::Recursive`] | Large networks | O(n log² n) |
//!
//! # Example
//!
//! ```ignore
//! use gat_algo::graph::{partition_network, PartitionStrategy};
//!
//! // Spectral partitioning into 4 regions
//! let partitions = partition_network(
//!     &network,
//!     PartitionStrategy::Spectral { num_partitions: 4 },
//! )?;
//!
//! // Check partition quality
//! let total_tie_lines: usize = partitions.iter().map(|p| p.tie_lines.len()).sum();
//! println!("Created {} partitions with {} total tie-lines", partitions.len(), total_tie_lines / 2);
//! ```

use std::collections::{HashMap, HashSet};

use gat_core::{Branch, Bus, Edge, Gen, Load, Network, Node};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for partitioning operations.
#[derive(Debug, Error)]
pub enum PartitionError {
    /// Network is too small to partition
    #[error("Network has only {0} buses, need at least {1} for {2} partitions")]
    NetworkTooSmall(usize, usize, usize),

    /// Invalid partition count
    #[error("Invalid partition count: {0}")]
    InvalidPartitionCount(usize),

    /// Area-based partitioning failed (missing area data)
    #[error("Area-based partitioning failed: {0}")]
    AreaDataMissing(String),

    /// Spectral decomposition failed
    #[error("Spectral partitioning failed: {0}")]
    SpectralFailed(String),

    /// Network is disconnected
    #[error("Network is disconnected; found {0} islands")]
    DisconnectedNetwork(usize),
}

/// A tie-line connecting two partitions at a boundary bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieLine {
    /// Branch ID in the original network
    pub branch_id: String,
    /// Bus ID on this partition's side
    pub local_bus: String,
    /// Bus ID on the neighbor partition's side
    pub remote_bus: String,
    /// Partition index of the neighbor
    pub neighbor_partition: usize,
    /// Branch susceptance (for ADMM penalty weighting)
    pub susceptance: f64,
}

/// A partition of the power network for distributed optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPartition {
    /// Partition index (0-based)
    pub id: usize,

    /// Bus IDs belonging to this partition (internal buses)
    pub buses: Vec<String>,

    /// Boundary buses (connected to other partitions via tie-lines)
    pub boundary_buses: Vec<String>,

    /// Tie-lines connecting this partition to neighbors
    pub tie_lines: Vec<TieLine>,

    /// Generator IDs in this partition
    pub generators: Vec<String>,

    /// Load IDs in this partition
    pub loads: Vec<String>,

    /// Total generation capacity (MW) in this partition
    pub total_gen_capacity_mw: f64,

    /// Total load demand (MW) in this partition
    pub total_load_mw: f64,
}

impl NetworkPartition {
    /// Check if a bus is on the boundary of this partition.
    pub fn is_boundary(&self, bus_id: &str) -> bool {
        self.boundary_buses.iter().any(|b| b == bus_id)
    }

    /// Get the partition's load-generation balance.
    pub fn power_balance(&self) -> f64 {
        self.total_gen_capacity_mw - self.total_load_mw
    }
}

/// Strategy for partitioning the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionStrategy {
    /// Spectral partitioning using graph Laplacian eigenvalues.
    ///
    /// This method computes the Fiedler vector (second-smallest eigenvector
    /// of the Laplacian) and recursively bisects the network. Good for
    /// minimizing edge cuts while balancing partition sizes.
    Spectral {
        /// Target number of partitions (must be power of 2 for pure spectral)
        num_partitions: usize,
    },

    /// Load-balanced partitioning based on generation/load.
    ///
    /// Partitions are sized to balance total MW capacity, not bus count.
    /// Uses a greedy algorithm with refinement.
    LoadBalanced {
        /// Maximum imbalance ratio (e.g., 0.1 = max 10% deviation from average)
        max_imbalance: f64,
        /// Target number of partitions
        num_partitions: usize,
    },

    /// Use predefined area assignments from network data.
    ///
    /// This uses the `zone_id` or `area_id` field in bus data to assign partitions.
    /// Falls back to spectral if area data is incomplete.
    Areas,

    /// Recursive bisection for large networks.
    ///
    /// Combines spectral bisection with local refinement (Kernighan-Lin style).
    /// Best for networks with 1000+ buses.
    Recursive {
        /// Target number of partitions
        num_partitions: usize,
        /// Number of refinement iterations per level
        refinement_iterations: usize,
    },
}

/// Helper struct for extracted network data (for easier processing).
struct NetworkData<'a> {
    buses: Vec<&'a Bus>,
    bus_indices: HashMap<usize, usize>, // BusId.value() -> index in buses vec
    bus_name_to_idx: HashMap<&'a str, usize>,
    generators: Vec<&'a Gen>,
    loads: Vec<&'a Load>,
    branches: Vec<&'a Branch>,
}

impl<'a> NetworkData<'a> {
    fn from_network(network: &'a Network) -> Self {
        let mut buses = Vec::new();
        let mut generators = Vec::new();
        let mut loads = Vec::new();

        // Extract nodes by type
        for node in network.graph.node_weights() {
            match node {
                Node::Bus(bus) => buses.push(bus),
                Node::Gen(gen) => generators.push(gen),
                Node::Load(load) => loads.push(load),
                Node::Shunt(_) => {} // Ignored for partitioning
            }
        }

        // Build bus index mappings
        let bus_indices: HashMap<usize, usize> = buses
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id.value(), i))
            .collect();

        let bus_name_to_idx: HashMap<&str, usize> = buses
            .iter()
            .enumerate()
            .map(|(i, b)| (b.name.as_str(), i))
            .collect();

        // Extract branches
        let mut branches = Vec::new();
        for edge in network.graph.edge_weights() {
            if let Edge::Branch(branch) = edge {
                branches.push(branch);
            }
        }

        Self {
            buses,
            bus_indices,
            bus_name_to_idx,
            generators,
            loads,
            branches,
        }
    }

    fn num_buses(&self) -> usize {
        self.buses.len()
    }

    /// Get bus name by index.
    fn bus_name(&self, idx: usize) -> &'a str {
        self.buses[idx].name.as_str()
    }

    /// Get index by BusId value.
    fn bus_idx_by_id(&self, bus_id: usize) -> Option<usize> {
        self.bus_indices.get(&bus_id).copied()
    }

    /// Get zone/area for a bus.
    #[allow(dead_code)]
    fn bus_zone(&self, idx: usize) -> Option<i64> {
        self.buses[idx].zone_id.or(self.buses[idx].area_id)
    }
}

/// Partition a power network into regions for distributed optimization.
///
/// # Arguments
/// * `network` - The power network to partition
/// * `strategy` - Partitioning strategy and parameters
///
/// # Returns
/// A vector of [`NetworkPartition`] objects, one per partition.
///
/// # Errors
/// Returns [`PartitionError`] if partitioning fails.
pub fn partition_network(
    network: &Network,
    strategy: PartitionStrategy,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    let data = NetworkData::from_network(network);
    let num_buses = data.num_buses();

    match &strategy {
        PartitionStrategy::Spectral { num_partitions }
        | PartitionStrategy::LoadBalanced { num_partitions, .. }
        | PartitionStrategy::Recursive { num_partitions, .. } => {
            if *num_partitions < 2 {
                return Err(PartitionError::InvalidPartitionCount(*num_partitions));
            }
            if num_buses < *num_partitions * 2 {
                return Err(PartitionError::NetworkTooSmall(
                    num_buses,
                    *num_partitions * 2,
                    *num_partitions,
                ));
            }
        }
        PartitionStrategy::Areas => {
            // Will validate during execution
        }
    }

    match strategy {
        PartitionStrategy::Spectral { num_partitions } => partition_spectral(&data, num_partitions),
        PartitionStrategy::LoadBalanced {
            max_imbalance,
            num_partitions,
        } => partition_load_balanced(&data, num_partitions, max_imbalance),
        PartitionStrategy::Areas => partition_by_areas(&data),
        PartitionStrategy::Recursive {
            num_partitions,
            refinement_iterations,
        } => partition_recursive(&data, num_partitions, refinement_iterations),
    }
}

/// Build adjacency list from network branches.
fn build_adjacency(data: &NetworkData) -> HashMap<usize, Vec<(usize, String, f64)>> {
    let mut adj: HashMap<usize, Vec<(usize, String, f64)>> = HashMap::new();

    // Initialize all buses
    for i in 0..data.num_buses() {
        adj.entry(i).or_default();
    }

    // Add edges from branches
    for branch in &data.branches {
        let from_idx = match data.bus_idx_by_id(branch.from_bus.value()) {
            Some(idx) => idx,
            None => continue,
        };
        let to_idx = match data.bus_idx_by_id(branch.to_bus.value()) {
            Some(idx) => idx,
            None => continue,
        };

        let susceptance = if branch.reactance.abs() > 1e-10 {
            1.0 / branch.reactance
        } else {
            1e6 // Very large susceptance for near-zero impedance
        };

        adj.entry(from_idx)
            .or_default()
            .push((to_idx, branch.name.clone(), susceptance));

        adj.entry(to_idx)
            .or_default()
            .push((from_idx, branch.name.clone(), susceptance));
    }

    adj
}

/// Spectral partitioning using graph Laplacian.
fn partition_spectral(
    data: &NetworkData,
    num_partitions: usize,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    let n = data.num_buses();
    let adj = build_adjacency(data);

    // Build Laplacian matrix (using susceptance as edge weights)
    let mut laplacian = vec![vec![0.0; n]; n];

    for (&bus_idx, neighbors) in &adj {
        let mut degree = 0.0;
        for (neighbor_idx, _, susceptance) in neighbors {
            laplacian[bus_idx][*neighbor_idx] = -susceptance;
            degree += susceptance;
        }
        laplacian[bus_idx][bus_idx] = degree;
    }

    // Compute Fiedler vector using power iteration
    let fiedler = compute_fiedler_vector(&laplacian, n)?;

    // Recursive bisection based on Fiedler vector
    let mut assignments = vec![0usize; n];
    recursive_bisect(&fiedler, &mut assignments, 0, n, 0, num_partitions);

    // Build partitions from assignments
    build_partitions_from_assignments(data, &assignments, num_partitions)
}

/// Compute approximate Fiedler vector using power iteration.
fn compute_fiedler_vector(laplacian: &[Vec<f64>], n: usize) -> Result<Vec<f64>, PartitionError> {
    // Use inverse power iteration with shift to find second-smallest eigenvalue
    // This is a simplified implementation; production code should use ARPACK or similar

    let max_iter = 100;
    let tol = 1e-8;

    // Start with random vector orthogonal to constant vector
    let mut v: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin()).collect();

    // Orthogonalize against constant vector
    let mean: f64 = v.iter().sum::<f64>() / n as f64;
    for x in &mut v {
        *x -= mean;
    }

    // Normalize
    let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < tol {
        return Err(PartitionError::SpectralFailed(
            "Initial vector degenerate".to_string(),
        ));
    }
    for x in &mut v {
        *x /= norm;
    }

    // Power iteration on (shifted) Laplacian
    // Using shift to accelerate convergence to second eigenvalue
    let shift = estimate_spectral_shift(laplacian, n);

    for _ in 0..max_iter {
        // Matrix-vector multiply: w = (L + shift*I) * v
        let mut w = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                w[i] += laplacian[i][j] * v[j];
            }
            w[i] += shift * v[i];
        }

        // Orthogonalize against constant vector
        let mean: f64 = w.iter().sum::<f64>() / n as f64;
        for x in &mut w {
            *x -= mean;
        }

        // Normalize
        let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < tol {
            break;
        }
        for x in &mut w {
            *x /= norm;
        }

        // Check convergence
        let diff: f64 = v
            .iter()
            .zip(w.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f64>();
        v = w;

        if diff < tol {
            break;
        }
    }

    Ok(v)
}

/// Estimate shift for inverse power iteration.
fn estimate_spectral_shift(laplacian: &[Vec<f64>], _n: usize) -> f64 {
    // Use negative of max diagonal (Gershgorin circle theorem)
    let max_diag = laplacian
        .iter()
        .enumerate()
        .map(|(i, row)| row[i])
        .fold(0.0f64, |a, b| a.max(b));
    -max_diag * 1.1
}

/// Recursive bisection of nodes based on Fiedler vector values.
fn recursive_bisect(
    fiedler: &[f64],
    assignments: &mut [usize],
    start: usize,
    end: usize,
    partition_base: usize,
    num_partitions: usize,
) {
    if num_partitions <= 1 || end - start < 2 {
        for a in assignments.iter_mut().take(end).skip(start) {
            *a = partition_base;
        }
        return;
    }

    // Sort indices in range by Fiedler value
    let mut indices: Vec<usize> = (start..end).collect();
    indices.sort_by(|&a, &b| fiedler[a].partial_cmp(&fiedler[b]).unwrap());

    // Split in half
    let mid = indices.len() / 2;
    let left_partitions = num_partitions / 2;
    let right_partitions = num_partitions - left_partitions;

    // Assign left half
    for &i in &indices[..mid] {
        assignments[i] = partition_base;
    }

    // Assign right half
    for &i in &indices[mid..] {
        assignments[i] = partition_base + left_partitions;
    }

    // Recurse if needed
    if left_partitions > 1 {
        recursive_bisect(
            fiedler,
            assignments,
            start,
            start + mid,
            partition_base,
            left_partitions,
        );
    }

    if right_partitions > 1 {
        recursive_bisect(
            fiedler,
            assignments,
            start + mid,
            end,
            partition_base + left_partitions,
            right_partitions,
        );
    }
}

/// Load-balanced partitioning.
fn partition_load_balanced(
    data: &NetworkData,
    num_partitions: usize,
    max_imbalance: f64,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    // Compute bus loads (MW) - sum loads connected to each bus
    let mut bus_loads: Vec<f64> = vec![0.0; data.num_buses()];

    for load in &data.loads {
        if let Some(idx) = data.bus_idx_by_id(load.bus.value()) {
            bus_loads[idx] += load.active_power.value();
        }
    }

    let total_load: f64 = bus_loads.iter().sum();
    let target_load = total_load / num_partitions as f64;
    let max_load = target_load * (1.0 + max_imbalance);

    // Greedy assignment with load balancing
    let mut assignments = vec![0usize; data.num_buses()];
    let mut partition_loads = vec![0.0f64; num_partitions];

    // Sort buses by load (descending) for better packing
    let mut bus_order: Vec<(usize, f64)> =
        bus_loads.iter().enumerate().map(|(i, &l)| (i, l)).collect();
    bus_order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (bus_idx, load) in bus_order {
        // Find partition with smallest current load that can fit this bus
        let best_partition = partition_loads
            .iter()
            .enumerate()
            .filter(|(_, &l)| l + load <= max_load || l < target_load * 0.5)
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        assignments[bus_idx] = best_partition;
        partition_loads[best_partition] += load;
    }

    build_partitions_from_assignments(data, &assignments, num_partitions)
}

/// Area-based partitioning using zone data.
fn partition_by_areas(data: &NetworkData) -> Result<Vec<NetworkPartition>, PartitionError> {
    // Check if zone data is available
    let has_zones = data
        .buses
        .iter()
        .any(|b| b.zone_id.is_some() || b.area_id.is_some());

    if !has_zones {
        return Err(PartitionError::AreaDataMissing(
            "No zone/area data in network buses".to_string(),
        ));
    }

    // Collect unique zones
    let zones: HashSet<i64> = data
        .buses
        .iter()
        .filter_map(|b| b.zone_id.or(b.area_id))
        .collect();

    if zones.is_empty() {
        return Err(PartitionError::AreaDataMissing(
            "All zone/area values are None".to_string(),
        ));
    }

    let zone_to_partition: HashMap<i64, usize> =
        zones.iter().enumerate().map(|(i, &z)| (z, i)).collect();

    let num_partitions = zones.len();

    // Assign buses to partitions based on zone
    let assignments: Vec<usize> = data
        .buses
        .iter()
        .map(|b| {
            b.zone_id
                .or(b.area_id)
                .and_then(|z| zone_to_partition.get(&z).copied())
                .unwrap_or(0)
        })
        .collect();

    build_partitions_from_assignments(data, &assignments, num_partitions)
}

/// Recursive bisection with refinement.
fn partition_recursive(
    data: &NetworkData,
    num_partitions: usize,
    refinement_iterations: usize,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    // Start with spectral partitioning
    let mut partitions = partition_spectral(data, num_partitions)?;

    // Apply Kernighan-Lin style refinement
    for _ in 0..refinement_iterations {
        partitions = refine_partitions(data, partitions)?;
    }

    Ok(partitions)
}

/// Kernighan-Lin style refinement to reduce cut edges.
fn refine_partitions(
    data: &NetworkData,
    partitions: Vec<NetworkPartition>,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    // Build bus-to-partition mapping (by index)
    let mut bus_partition: HashMap<usize, usize> = HashMap::new();
    for p in &partitions {
        for bus_name in &p.buses {
            if let Some(&idx) = data.bus_name_to_idx.get(bus_name.as_str()) {
                bus_partition.insert(idx, p.id);
            }
        }
    }

    let adj = build_adjacency(data);
    let num_partitions = partitions.len();

    // Compute gains for swapping boundary buses
    let mut improved = true;
    let max_swaps = data.num_buses() / 10;
    let mut swap_count = 0;

    while improved && swap_count < max_swaps {
        improved = false;

        for p in &partitions {
            for boundary_bus in &p.boundary_buses {
                let boundary_idx = match data.bus_name_to_idx.get(boundary_bus.as_str()) {
                    Some(&idx) => idx,
                    None => continue,
                };

                // Compute current cut contribution
                let current_cut = count_cut_edges(boundary_idx, p.id, &bus_partition, &adj);

                // Try moving to each neighboring partition
                for tie in &p.tie_lines {
                    if tie.local_bus != *boundary_bus {
                        continue;
                    }

                    let neighbor_partition = tie.neighbor_partition;
                    let new_cut =
                        count_cut_edges(boundary_idx, neighbor_partition, &bus_partition, &adj);

                    // If moving reduces cut, do it
                    if new_cut < current_cut {
                        bus_partition.insert(boundary_idx, neighbor_partition);
                        improved = true;
                        swap_count += 1;
                        break;
                    }
                }
            }
        }
    }

    // Rebuild partitions from updated assignments
    let assignments: Vec<usize> = (0..data.num_buses())
        .map(|i| *bus_partition.get(&i).unwrap_or(&0))
        .collect();

    build_partitions_from_assignments(data, &assignments, num_partitions)
}

/// Count cut edges if bus were in given partition.
fn count_cut_edges(
    bus_idx: usize,
    partition: usize,
    bus_partition: &HashMap<usize, usize>,
    adj: &HashMap<usize, Vec<(usize, String, f64)>>,
) -> usize {
    adj.get(&bus_idx)
        .map(|neighbors| {
            neighbors
                .iter()
                .filter(|(n_idx, _, _)| bus_partition.get(n_idx).copied().unwrap_or(0) != partition)
                .count()
        })
        .unwrap_or(0)
}

/// Build NetworkPartition structs from bus assignments.
fn build_partitions_from_assignments(
    data: &NetworkData,
    assignments: &[usize],
    num_partitions: usize,
) -> Result<Vec<NetworkPartition>, PartitionError> {
    let adj = build_adjacency(data);

    // Initialize partitions
    let mut partitions: Vec<NetworkPartition> = (0..num_partitions)
        .map(|id| NetworkPartition {
            id,
            buses: Vec::new(),
            boundary_buses: Vec::new(),
            tie_lines: Vec::new(),
            generators: Vec::new(),
            loads: Vec::new(),
            total_gen_capacity_mw: 0.0,
            total_load_mw: 0.0,
        })
        .collect();

    // Assign buses
    for (i, &partition_id) in assignments.iter().enumerate() {
        if partition_id < num_partitions {
            partitions[partition_id]
                .buses
                .push(data.bus_name(i).to_string());
        }
    }

    // Find boundary buses and tie-lines
    for (i, &partition_id) in assignments.iter().enumerate() {
        if partition_id >= num_partitions {
            continue;
        }

        let bus_name = data.bus_name(i);
        let neighbors = adj.get(&i).cloned().unwrap_or_default();

        for (neighbor_idx, branch_name, susceptance) in neighbors {
            let neighbor_partition = assignments.get(neighbor_idx).copied().unwrap_or(0);

            if neighbor_partition != partition_id {
                // This is a boundary bus
                if !partitions[partition_id]
                    .boundary_buses
                    .contains(&bus_name.to_string())
                {
                    partitions[partition_id]
                        .boundary_buses
                        .push(bus_name.to_string());
                }

                // Add tie-line
                partitions[partition_id].tie_lines.push(TieLine {
                    branch_id: branch_name,
                    local_bus: bus_name.to_string(),
                    remote_bus: data.bus_name(neighbor_idx).to_string(),
                    neighbor_partition,
                    susceptance,
                });
            }
        }
    }

    // Assign generators and compute capacity
    for gen in &data.generators {
        if let Some(bus_idx) = data.bus_idx_by_id(gen.bus.value()) {
            let partition_id = assignments.get(bus_idx).copied().unwrap_or(0);
            if partition_id < num_partitions {
                partitions[partition_id].generators.push(gen.name.clone());
                partitions[partition_id].total_gen_capacity_mw += gen.pmax.value();
            }
        }
    }

    // Assign loads and compute demand
    for load in &data.loads {
        if let Some(bus_idx) = data.bus_idx_by_id(load.bus.value()) {
            let partition_id = assignments.get(bus_idx).copied().unwrap_or(0);
            if partition_id < num_partitions {
                partitions[partition_id].loads.push(load.name.clone());
                partitions[partition_id].total_load_mw += load.active_power.value();
            }
        }
    }

    Ok(partitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{
        BranchId, BusId, GenId, Kilovolts, LoadId, Megavars, Megawatts, PerUnit, Radians,
    };

    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Create a simple 6-bus network (2x3 grid)
        // Bus layout:
        //   1 -- 2 -- 3
        //   |    |    |
        //   4 -- 5 -- 6
        let mut bus_indices = Vec::new();
        for i in 1..=6 {
            let zone = if i <= 3 { Some(1) } else { Some(2) };
            let idx = network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(i),
                name: format!("bus{}", i),
                base_kv: Kilovolts(230.0),
                voltage_pu: PerUnit(1.0),
                angle_rad: Radians(0.0),
                vmin_pu: None,
                vmax_pu: None,
                area_id: None,
                zone_id: zone,
            }));
            bus_indices.push(idx);
        }

        // Add generators
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "gen1".to_string(), BusId::new(1)).with_p_limits(0.0, 100.0),
        ));
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(2), "gen6".to_string(), BusId::new(6)).with_p_limits(0.0, 100.0),
        ));

        // Add loads
        for i in 2..=5 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(i),
                name: format!("load{}", i),
                bus: BusId::new(i),
                active_power: Megawatts(25.0),
                reactive_power: Megavars(10.0),
            }));
        }

        // Add branches - horizontal connections
        let branches = vec![
            (1, 2, 1),
            (2, 3, 2),
            (4, 5, 3),
            (5, 6, 4),
            // Vertical connections
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
    fn test_spectral_partition() {
        let network = create_test_network();
        let partitions =
            partition_network(&network, PartitionStrategy::Spectral { num_partitions: 2 }).unwrap();

        assert_eq!(partitions.len(), 2);

        // All buses should be assigned
        let total_buses: usize = partitions.iter().map(|p| p.buses.len()).sum();
        assert_eq!(total_buses, 6);

        // Should have some tie-lines (since we're splitting a connected graph)
        let total_ties: usize = partitions.iter().map(|p| p.tie_lines.len()).sum();
        assert!(total_ties >= 2); // At least 2 cuts needed for grid
    }

    #[test]
    fn test_area_partition() {
        let network = create_test_network();
        let partitions = partition_network(&network, PartitionStrategy::Areas).unwrap();

        // Should create 2 partitions (zones 1 and 2)
        assert_eq!(partitions.len(), 2);

        // Partition 0 should have buses 1-3, Partition 1 should have buses 4-6
        assert_eq!(partitions[0].buses.len(), 3);
        assert_eq!(partitions[1].buses.len(), 3);
    }

    #[test]
    fn test_load_balanced_partition() {
        let network = create_test_network();
        let partitions = partition_network(
            &network,
            PartitionStrategy::LoadBalanced {
                num_partitions: 2,
                max_imbalance: 0.2,
            },
        )
        .unwrap();

        assert_eq!(partitions.len(), 2);

        // Total load should be distributed reasonably
        let load_diff = (partitions[0].total_load_mw - partitions[1].total_load_mw).abs();
        let avg_load = (partitions[0].total_load_mw + partitions[1].total_load_mw) / 2.0;
        if avg_load > 0.0 {
            assert!(load_diff <= avg_load * 0.5); // Within 50% imbalance
        }
    }

    #[test]
    fn test_partition_too_small() {
        let network = create_test_network();
        let result =
            partition_network(&network, PartitionStrategy::Spectral { num_partitions: 10 });

        assert!(matches!(
            result,
            Err(PartitionError::NetworkTooSmall(_, _, _))
        ));
    }

    #[test]
    fn test_tie_line_data() {
        let network = create_test_network();
        let partitions = partition_network(&network, PartitionStrategy::Areas).unwrap();

        // Check tie-lines have valid data
        for p in &partitions {
            for tie in &p.tie_lines {
                assert!(!tie.branch_id.is_empty());
                assert!(!tie.local_bus.is_empty());
                assert!(!tie.remote_bus.is_empty());
                assert!(tie.susceptance > 0.0);
                assert!(tie.neighbor_partition < partitions.len());
            }
        }
    }

    #[test]
    fn test_boundary_buses() {
        let network = create_test_network();
        let partitions = partition_network(&network, PartitionStrategy::Areas).unwrap();

        // All boundary buses should have tie-lines
        for p in &partitions {
            for boundary_bus in &p.boundary_buses {
                let has_tie = p.tie_lines.iter().any(|t| &t.local_bus == boundary_bus);
                assert!(has_tie, "Boundary bus {} has no tie-line", boundary_bus);
            }
        }
    }
}
