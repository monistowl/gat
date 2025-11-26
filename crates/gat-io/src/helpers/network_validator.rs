//! Generic network validation that runs post-import for all formats.
//!
//! This validator performs consistency checks on the imported Network model,
//! catching issues that individual format parsers might miss. Results are
//! reported through the ImportDiagnostics infrastructure.

use super::diagnostics::ImportDiagnostics;
use gat_core::{Edge, Network, Node};
use std::collections::{HashMap, HashSet};

/// Configuration for network validation behavior
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    /// Treat warnings as errors (strict mode)
    pub strict: bool,
    /// Skip topological checks (for partial networks)
    pub skip_topology: bool,
    /// Custom voltage range (kV) for sanity checks
    pub voltage_range: Option<(f64, f64)>,
    /// Custom R/X ratio threshold for unusual branch warnings
    pub rx_ratio_threshold: Option<f64>,
}

impl ValidationConfig {
    /// Create strict validation config
    pub fn strict() -> Self {
        Self {
            strict: true,
            ..Default::default()
        }
    }
}

/// Validate a network and populate diagnostics with any issues found.
///
/// This function performs multiple categories of validation:
/// - **Structural**: Network has required elements (buses, branches)
/// - **Reference integrity**: All bus references point to existing buses
/// - **Topological**: Connectivity and island detection
/// - **Physical sanity**: Reasonable impedance, voltage, and power values
pub fn validate_network(network: &Network, diag: &mut ImportDiagnostics, config: &ValidationConfig) {
    // Phase 1: Structural validation
    validate_structure(network, diag);

    // Phase 2: Reference integrity
    validate_references(network, diag);

    // Phase 3: Topological validation (unless skipped)
    if !config.skip_topology {
        validate_topology(network, diag);
    }

    // Phase 4: Physical sanity checks
    validate_physical_sanity(network, diag, config);
}

/// Check basic network structure requirements
fn validate_structure(network: &Network, diag: &mut ImportDiagnostics) {
    let mut bus_count = 0;
    let mut gen_count = 0;
    let mut load_count = 0;
    let mut branch_count = 0;

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(_) => bus_count += 1,
            Node::Gen(_) => gen_count += 1,
            Node::Load(_) => load_count += 1,
        }
    }

    for edge in network.graph.edge_weights() {
        if matches!(edge, Edge::Branch(_)) {
            branch_count += 1;
        }
    }

    // Critical: Network must have buses
    if bus_count == 0 {
        diag.add_error("structure", "Network has no buses");
        return; // Can't validate further
    }

    // Warning: Network should have generators
    if gen_count == 0 {
        diag.add_validation_warning("Network", "No generators found - power flow will fail");
    }

    // Warning: Network should have loads for meaningful analysis
    if load_count == 0 {
        diag.add_validation_warning("Network", "No loads found");
    }

    // Warning: Multiple buses but no branches
    if bus_count > 1 && branch_count == 0 {
        diag.add_error("structure", "Multiple buses but no branches connecting them");
    }
}

/// Validate that all bus references point to existing buses
fn validate_references(network: &Network, diag: &mut ImportDiagnostics) {
    // Build set of valid bus IDs
    let mut valid_buses: HashSet<usize> = HashSet::new();
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            valid_buses.insert(bus.id.value());
        }
    }

    // Check generator bus references
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            if !valid_buses.contains(&gen.bus.value()) {
                diag.add_error(
                    "reference",
                    &format!(
                        "Generator '{}' references non-existent bus {}",
                        gen.name,
                        gen.bus.value()
                    ),
                );
            }
        }
    }

    // Check load bus references
    for node in network.graph.node_weights() {
        if let Node::Load(load) = node {
            if !valid_buses.contains(&load.bus.value()) {
                diag.add_error(
                    "reference",
                    &format!(
                        "Load '{}' references non-existent bus {}",
                        load.name,
                        load.bus.value()
                    ),
                );
            }
        }
    }

    // Check branch bus references
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !valid_buses.contains(&branch.from_bus.value()) {
                diag.add_error(
                    "reference",
                    &format!(
                        "Branch '{}' references non-existent from_bus {}",
                        branch.name,
                        branch.from_bus.value()
                    ),
                );
            }
            if !valid_buses.contains(&branch.to_bus.value()) {
                diag.add_error(
                    "reference",
                    &format!(
                        "Branch '{}' references non-existent to_bus {}",
                        branch.name,
                        branch.to_bus.value()
                    ),
                );
            }
        }
    }
}

/// Validate network topology (connectivity, islands)
fn validate_topology(network: &Network, diag: &mut ImportDiagnostics) {
    // Build a subgraph of just buses connected by active branches
    let bus_nodes: Vec<_> = network
        .graph
        .node_indices()
        .filter(|idx| matches!(network.graph[*idx], Node::Bus(_)))
        .collect();

    if bus_nodes.len() <= 1 {
        return; // Single bus or empty - no topology to check
    }

    // Build adjacency for buses only via active branches
    let mut bus_id_to_idx: HashMap<usize, usize> = HashMap::new();
    for (i, idx) in bus_nodes.iter().enumerate() {
        if let Node::Bus(bus) = &network.graph[*idx] {
            bus_id_to_idx.insert(bus.id.value(), i);
        }
    }

    // Union-find for bus connectivity
    let mut parent: Vec<usize> = (0..bus_nodes.len()).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let pa = find(parent, a);
        let pb = find(parent, b);
        if pa != pb {
            parent[pa] = pb;
        }
    }

    // Connect buses via active branches
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if branch.status {
                if let (Some(&a), Some(&b)) = (
                    bus_id_to_idx.get(&branch.from_bus.value()),
                    bus_id_to_idx.get(&branch.to_bus.value()),
                ) {
                    union(&mut parent, a, b);
                }
            }
        }
    }

    // Count distinct components
    let mut roots: HashSet<usize> = HashSet::new();
    for i in 0..bus_nodes.len() {
        roots.insert(find(&mut parent, i));
    }

    let island_count = roots.len();
    if island_count > 1 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "Network has {} electrical islands - power flow may fail or produce unexpected results",
                island_count
            ),
        );

        // Identify isolated buses (single-bus islands)
        let mut component_sizes: HashMap<usize, usize> = HashMap::new();
        for i in 0..bus_nodes.len() {
            let root = find(&mut parent, i);
            *component_sizes.entry(root).or_insert(0) += 1;
        }

        for (i, idx) in bus_nodes.iter().enumerate() {
            let root = find(&mut parent, i);
            if component_sizes[&root] == 1 {
                if let Node::Bus(bus) = &network.graph[*idx] {
                    diag.add_validation_warning(
                        &format!("Bus {}", bus.name),
                        "Isolated bus with no active connections",
                    );
                }
            }
        }
    }
}

/// Validate physical sanity of network parameters
fn validate_physical_sanity(
    network: &Network,
    diag: &mut ImportDiagnostics,
    config: &ValidationConfig,
) {
    let voltage_range = config.voltage_range.unwrap_or((0.1, 1500.0)); // 100V to 1500kV
    let rx_threshold = config.rx_ratio_threshold.unwrap_or(10.0);

    // Validate bus voltages
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            if bus.voltage_kv <= 0.0 {
                diag.add_error(
                    "physical",
                    &format!(
                        "Bus '{}' has invalid voltage: {} kV (must be > 0)",
                        bus.name, bus.voltage_kv
                    ),
                );
            } else if bus.voltage_kv < voltage_range.0 || bus.voltage_kv > voltage_range.1 {
                diag.add_validation_warning(
                    &format!("Bus {}", bus.name),
                    &format!(
                        "Unusual voltage level: {} kV (expected {}-{} kV)",
                        bus.voltage_kv, voltage_range.0, voltage_range.1
                    ),
                );
            }
        }
    }

    // Validate branch parameters
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            // Negative resistance (unless phase shifter)
            if branch.resistance < 0.0 && !branch.is_phase_shifter {
                diag.add_error(
                    "physical",
                    &format!(
                        "Branch '{}' has negative resistance: {} (not marked as phase shifter)",
                        branch.name, branch.resistance
                    ),
                );
            }

            // Zero impedance (short circuit)
            if branch.resistance == 0.0 && branch.reactance == 0.0 {
                diag.add_error(
                    "physical",
                    &format!(
                        "Branch '{}' has zero impedance (r=0, x=0) - creates singularity",
                        branch.name
                    ),
                );
            }

            // High R/X ratio (unusual for transmission)
            if branch.reactance.abs() > 1e-9 {
                let rx_ratio = branch.resistance / branch.reactance.abs();
                if rx_ratio > rx_threshold {
                    diag.add_validation_warning(
                        &format!("Branch {}", branch.name),
                        &format!(
                            "High R/X ratio: {:.2} (typical transmission < {})",
                            rx_ratio, rx_threshold
                        ),
                    );
                }
            }

            // Negative tap ratio
            if branch.tap_ratio < 0.0 {
                diag.add_error(
                    "physical",
                    &format!(
                        "Branch '{}' has negative tap ratio: {}",
                        branch.name, branch.tap_ratio
                    ),
                );
            }

            // Tap ratio of zero (invalid)
            if branch.tap_ratio == 0.0 {
                diag.add_error(
                    "physical",
                    &format!("Branch '{}' has zero tap ratio (must be > 0)", branch.name),
                );
            }

            // Negative thermal limit
            if let Some(s_max) = branch.s_max_mva {
                if s_max < 0.0 {
                    diag.add_error(
                        "physical",
                        &format!(
                            "Branch '{}' has negative thermal limit: {} MVA",
                            branch.name, s_max
                        ),
                    );
                }
            }
        }
    }

    // Validate generator parameters
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            // Pmax < Pmin (inverted limits)
            if gen.pmax_mw < gen.pmin_mw {
                diag.add_error(
                    "physical",
                    &format!(
                        "Generator '{}' has Pmax ({} MW) < Pmin ({} MW)",
                        gen.name, gen.pmax_mw, gen.pmin_mw
                    ),
                );
            }

            // Qmax < Qmin (inverted limits)
            if gen.qmax_mvar < gen.qmin_mvar {
                diag.add_error(
                    "physical",
                    &format!(
                        "Generator '{}' has Qmax ({} MVAr) < Qmin ({} MVAr)",
                        gen.name, gen.qmax_mvar, gen.qmin_mvar
                    ),
                );
            }

            // Negative Pmin for non-synchronous condenser
            if gen.pmin_mw < 0.0 && !gen.is_synchronous_condenser {
                diag.add_validation_warning(
                    &format!("Generator {}", gen.name),
                    &format!(
                        "Negative Pmin ({} MW) but not marked as synchronous condenser",
                        gen.pmin_mw
                    ),
                );
            }

            // Very large capacity warning
            if gen.pmax_mw > 10000.0 {
                diag.add_validation_warning(
                    &format!("Generator {}", gen.name),
                    &format!("Very large capacity: {} MW (> 10 GW)", gen.pmax_mw),
                );
            }
        }
    }

    // Check total generation vs load balance
    let mut total_load = 0.0;
    let mut total_gen_capacity = 0.0;
    let mut total_gen_min = 0.0;

    for node in network.graph.node_weights() {
        match node {
            Node::Load(load) => {
                total_load += load.active_power_mw;
            }
            Node::Gen(gen) => {
                total_gen_capacity += gen.pmax_mw;
                total_gen_min += gen.pmin_mw;
            }
            _ => {}
        }
    }

    // Warn if total gen capacity is less than total load
    if total_gen_capacity < total_load && total_load > 0.0 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "Total generation capacity ({:.1} MW) < total load ({:.1} MW) - infeasible dispatch",
                total_gen_capacity, total_load
            ),
        );
    }

    // Warn if minimum generation exceeds load
    if total_gen_min > total_load && total_load > 0.0 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "Minimum generation ({:.1} MW) > total load ({:.1} MW) - over-generation",
                total_gen_min, total_load
            ),
        );
    }
}

/// Convenience function: validate network and return issues count
pub fn validate_network_quick(network: &Network) -> (usize, usize) {
    let mut diag = ImportDiagnostics::new();
    validate_network(network, &mut diag, &ValidationConfig::default());
    (diag.warning_count(), diag.error_count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId, Gen, GenId, Load, LoadId};

    fn make_simple_network() -> Network {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            voltage_kv: 138.0,
        }));

        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Gen 1".to_string(), BusId::new(1))
                .with_p_limits(0.0, 100.0),
        ));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power_mw: 50.0,
            reactive_power_mvar: 10.0,
        }));

        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Branch 1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                tap_ratio: 1.0,
                ..Branch::default()
            }),
        );

        network
    }

    #[test]
    fn test_valid_network_no_errors() {
        let network = make_simple_network();
        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert_eq!(diag.error_count(), 0, "Valid network should have no errors");
    }

    #[test]
    fn test_empty_network_error() {
        let network = Network::new();
        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.error_count() > 0, "Empty network should error");
        assert!(diag.issues.iter().any(|i| i.message.contains("no buses")));
    }

    #[test]
    fn test_zero_impedance_error() {
        let mut network = make_simple_network();

        // Find and modify the branch to have zero impedance
        for edge in network.graph.edge_weights_mut() {
            if let Edge::Branch(branch) = edge {
                branch.resistance = 0.0;
                branch.reactance = 0.0;
            }
        }

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.error_count() > 0,
            "Zero impedance should produce error"
        );
        assert!(diag.issues.iter().any(|i| i.message.contains("zero impedance")));
    }

    #[test]
    fn test_invalid_bus_reference_error() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));

        // Generator references non-existent bus 999
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Bad Gen".to_string(), BusId::new(999))
                .with_p_limits(0.0, 100.0),
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.error_count() > 0);
        assert!(diag.issues.iter().any(|i| i.message.contains("non-existent bus")));
    }

    #[test]
    fn test_island_detection() {
        let mut network = Network::new();

        // Create two disconnected buses
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            voltage_kv: 138.0,
        }));
        // No branch connecting them

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        // Should warn about islands and multiple buses with no branches
        assert!(diag.warning_count() > 0 || diag.error_count() > 0);
    }

    #[test]
    fn test_inverted_gen_limits_error() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));

        // Generator with Pmax < Pmin
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Bad Gen".to_string(), BusId::new(1))
                .with_p_limits(100.0, 50.0), // Inverted!
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.error_count() > 0);
        assert!(diag.issues.iter().any(|i| i.message.contains("Pmax") && i.message.contains("Pmin")));
    }

    #[test]
    fn test_unusual_voltage_warning() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Weird Bus".to_string(),
            voltage_kv: 0.05, // 50V - very unusual
        }));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.warning_count() > 0);
        assert!(diag.issues.iter().any(|i| i.message.contains("Unusual voltage")));
    }

    #[test]
    fn test_quick_validate_counts() {
        let network = make_simple_network();
        let (_warnings, errors) = validate_network_quick(&network);

        assert_eq!(errors, 0, "Valid network should have no errors")
    }
}
