//! Generic network validation that runs post-import for all formats.
//!
//! This validator performs consistency checks on the imported Network model,
//! catching issues that individual format parsers might miss. Results are
//! reported through the Diagnostics infrastructure from gat-core.

use gat_core::{Edge, ImportDiagnostics, Network, Node};
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
pub fn validate_network(
    network: &Network,
    diag: &mut ImportDiagnostics,
    config: &ValidationConfig,
) {
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

    // Phase 5: Distribution anomaly detection (catches mapping bugs)
    validate_distribution_anomalies(network, diag);

    // Phase 6: Power balance sanity checks
    validate_power_balance(network, diag);
}

/// Check basic network structure requirements
fn validate_structure(network: &Network, diag: &mut ImportDiagnostics) {
    let mut bus_count = 0;
    let mut gen_count = 0;
    let mut load_count = 0;
    let mut _shunt_count = 0;
    let mut branch_count = 0;

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(_) => bus_count += 1,
            Node::Gen(_) => gen_count += 1,
            Node::Load(_) => load_count += 1,
            Node::Shunt(_) => _shunt_count += 1,
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
        diag.add_error(
            "structure",
            "Multiple buses but no branches connecting them",
        );
    }
}

/// Validate that all bus references point to existing buses
fn validate_references(network: &Network, diag: &mut ImportDiagnostics) {
    // Count buses for capacity hint
    let bus_count = network
        .graph
        .node_weights()
        .filter(|n| matches!(n, Node::Bus(_)))
        .count();

    // Build set of valid bus IDs with pre-allocated capacity
    let mut valid_buses: HashSet<usize> = HashSet::with_capacity(bus_count);
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

    // Build adjacency for buses only via active branches (pre-allocated)
    let mut bus_id_to_idx: HashMap<usize, usize> = HashMap::with_capacity(bus_nodes.len());
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

    // Count distinct components (pre-allocated for worst case: all isolated)
    let mut roots: HashSet<usize> = HashSet::with_capacity(bus_nodes.len());
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
        let mut component_sizes: HashMap<usize, usize> = HashMap::with_capacity(island_count);
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
            let kv = bus.base_kv.value();
            if kv <= 0.0 {
                diag.add_error(
                    "physical",
                    &format!(
                        "Bus '{}' has invalid voltage: {} kV (must be > 0)",
                        bus.name, kv
                    ),
                );
            } else if kv < voltage_range.0 || kv > voltage_range.1 {
                diag.add_validation_warning(
                    &format!("Bus {}", bus.name),
                    &format!(
                        "Unusual voltage level: {} kV (expected {}-{} kV)",
                        kv, voltage_range.0, voltage_range.1
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
            if let Some(s_max) = &branch.s_max {
                if s_max.value() < 0.0 {
                    diag.add_error(
                        "physical",
                        &format!(
                            "Branch '{}' has negative thermal limit: {} MVA",
                            branch.name, s_max.value()
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
            if gen.pmax.value() < gen.pmin.value() {
                diag.add_error(
                    "physical",
                    &format!(
                        "Generator '{}' has Pmax ({} MW) < Pmin ({} MW)",
                        gen.name, gen.pmax.value(), gen.pmin.value()
                    ),
                );
            }

            // Qmax < Qmin (inverted limits)
            if gen.qmax.value() < gen.qmin.value() {
                diag.add_error(
                    "physical",
                    &format!(
                        "Generator '{}' has Qmax ({} MVAr) < Qmin ({} MVAr)",
                        gen.name, gen.qmax.value(), gen.qmin.value()
                    ),
                );
            }

            // Negative Pmin for non-synchronous condenser
            if gen.pmin.value() < 0.0 && !gen.is_synchronous_condenser {
                diag.add_validation_warning(
                    &format!("Generator {}", gen.name),
                    &format!(
                        "Negative Pmin ({} MW) but not marked as synchronous condenser",
                        gen.pmin.value()
                    ),
                );
            }

            // Very large capacity warning
            if gen.pmax.value() > 10000.0 {
                diag.add_validation_warning(
                    &format!("Generator {}", gen.name),
                    &format!("Very large capacity: {} MW (> 10 GW)", gen.pmax.value()),
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
                total_load += load.active_power.value();
            }
            Node::Gen(gen) => {
                total_gen_capacity += gen.pmax.value();
                total_gen_min += gen.pmin.value();
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

/// Detect distribution anomalies that indicate mapping bugs
///
/// These checks catch common parser bugs where all components of one type
/// end up at the same bus due to missing or incorrect bus mappings.
fn validate_distribution_anomalies(network: &Network, diag: &mut ImportDiagnostics) {
    // Collect generator bus distribution
    let mut gen_buses: HashSet<usize> = HashSet::new();
    let mut gen_count = 0;
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            gen_buses.insert(gen.bus.value());
            gen_count += 1;
        }
    }

    // All generators at same bus is highly suspicious (unless single gen)
    if gen_count > 1 && gen_buses.len() == 1 {
        let bus_id = gen_buses.iter().next().unwrap();
        diag.add_validation_warning(
            "Network",
            &format!(
                "All {} generators are at bus {} - likely a bus mapping bug",
                gen_count, bus_id
            ),
        );
    }

    // Collect load bus distribution
    let mut load_buses: HashSet<usize> = HashSet::new();
    let mut load_count = 0;
    for node in network.graph.node_weights() {
        if let Node::Load(load) = node {
            load_buses.insert(load.bus.value());
            load_count += 1;
        }
    }

    // All loads at same bus is suspicious (unless single load)
    if load_count > 1 && load_buses.len() == 1 {
        let bus_id = load_buses.iter().next().unwrap();
        diag.add_validation_warning(
            "Network",
            &format!(
                "All {} loads are at bus {} - likely a bus mapping bug",
                load_count, bus_id
            ),
        );
    }

    // Check for generators/loads at bus 0 (often indicates failed mapping)
    // Bus 0 is typically invalid in most formats (1-indexed)
    let gens_at_bus_0 = network
        .graph
        .node_weights()
        .filter(|n| matches!(n, Node::Gen(g) if g.bus.value() == 0))
        .count();
    if gens_at_bus_0 > 0 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "{} generator(s) at bus 0 - likely a bus mapping failure (buses are typically 1-indexed)",
                gens_at_bus_0
            ),
        );
    }

    let loads_at_bus_0 = network
        .graph
        .node_weights()
        .filter(|n| matches!(n, Node::Load(l) if l.bus.value() == 0))
        .count();
    if loads_at_bus_0 > 0 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "{} load(s) at bus 0 - likely a bus mapping failure (buses are typically 1-indexed)",
                loads_at_bus_0
            ),
        );
    }

    // Collect branch endpoint distribution
    let mut branch_from_buses: HashMap<usize, usize> = HashMap::new();
    let mut branch_to_buses: HashMap<usize, usize> = HashMap::new();
    let mut branch_count = 0;
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(br) = edge {
            *branch_from_buses.entry(br.from_bus.value()).or_insert(0) += 1;
            *branch_to_buses.entry(br.to_bus.value()).or_insert(0) += 1;
            branch_count += 1;
        }
    }

    // All branches from same bus is suspicious (star topology with single source)
    if branch_count > 2 && branch_from_buses.len() == 1 {
        let bus_id = branch_from_buses.keys().next().unwrap();
        diag.add_validation_warning(
            "Network",
            &format!(
                "All {} branches have from_bus {} - likely a bus mapping bug",
                branch_count, bus_id
            ),
        );
    }

    // All branches to same bus is equally suspicious
    if branch_count > 2 && branch_to_buses.len() == 1 {
        let bus_id = branch_to_buses.keys().next().unwrap();
        diag.add_validation_warning(
            "Network",
            &format!(
                "All {} branches have to_bus {} - likely a bus mapping bug",
                branch_count, bus_id
            ),
        );
    }

    // Check for branches with from_bus or to_bus at bus 0
    let branches_with_bus_0 = network
        .graph
        .edge_weights()
        .filter(
            |e| matches!(e, Edge::Branch(br) if br.from_bus.value() == 0 || br.to_bus.value() == 0),
        )
        .count();
    if branches_with_bus_0 > 0 {
        diag.add_validation_warning(
            "Network",
            &format!(
                "{} branch(es) connected to bus 0 - likely a bus mapping failure",
                branches_with_bus_0
            ),
        );
    }
}

/// Phase 6: Power balance sanity checks
///
/// Detects infeasible or suspicious power balance conditions that indicate
/// either data issues or physically impossible operating points.
fn validate_power_balance(network: &Network, diag: &mut ImportDiagnostics) {
    let mut total_pmax = 0.0;
    let mut total_pmin = 0.0;
    let mut _total_qmax = 0.0;
    let mut _total_qmin = 0.0;
    let mut total_load_p = 0.0;
    let mut _total_load_q = 0.0;
    let mut gen_count = 0;
    let mut load_count = 0;

    for node in network.graph.node_weights() {
        match node {
            Node::Gen(gen) => {
                total_pmax += gen.pmax.value();
                total_pmin += gen.pmin.value();
                _total_qmax += gen.qmax.value();
                _total_qmin += gen.qmin.value();
                gen_count += 1;

                // Check for negative Pmax (suspicious)
                if gen.pmax.value() < 0.0 {
                    diag.add_validation_warning(
                        "PowerBalance",
                        &format!(
                            "Generator {} has negative Pmax ({:.2} MW) - likely a data error",
                            gen.id.value(),
                            gen.pmax.value()
                        ),
                    );
                }

                // Check for Pmin > Pmax (invalid)
                if gen.pmin.value() > gen.pmax.value() {
                    diag.add_error(
                        "PowerBalance",
                        &format!(
                            "Generator {} has Pmin ({:.2}) > Pmax ({:.2}) - invalid limits",
                            gen.id.value(),
                            gen.pmin.value(),
                            gen.pmax.value()
                        ),
                    );
                }
            }
            Node::Load(load) => {
                total_load_p += load.active_power.value();
                _total_load_q += load.reactive_power.value();
                load_count += 1;

                // Check for negative load (could be valid for distributed gen, but warn)
                if load.active_power.value() < 0.0 {
                    diag.add_validation_warning(
                        "PowerBalance",
                        &format!(
                            "Load {} has negative P ({:.2} MW) - verify if intentional (e.g., DER)",
                            load.id.value(),
                            load.active_power.value()
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    // Skip balance checks if no loads or generators
    if gen_count == 0 || load_count == 0 {
        return;
    }

    // Check for insufficient generation capacity
    if total_pmax < total_load_p {
        diag.add_error(
            "PowerBalance",
            &format!(
                "Total Pmax ({:.2} MW) < Total Load ({:.2} MW) - network is infeasible",
                total_pmax, total_load_p
            ),
        );
    }

    // Check for over-generation at minimum output
    if total_pmin > total_load_p {
        diag.add_validation_warning(
            "PowerBalance",
            &format!(
                "Total Pmin ({:.2} MW) > Total Load ({:.2} MW) - curtailment required",
                total_pmin, total_load_p
            ),
        );
    }

    // Check for extremely high reserve margin (may indicate data issues)
    if total_load_p > 0.0 {
        let reserve_margin = (total_pmax - total_load_p) / total_load_p;
        if reserve_margin > 5.0 {
            // More than 500% reserve is suspicious
            diag.add_validation_warning(
                "PowerBalance",
                &format!(
                    "Very high reserve margin ({:.0}%) - verify generation capacity data",
                    reserve_margin * 100.0
                ),
            );
        }
    }

    // Check for zero total load (suspicious unless islanding study)
    if total_load_p.abs() < 1e-6 && load_count > 0 {
        diag.add_validation_warning(
            "PowerBalance",
            &format!(
                "Total active load is zero across {} loads - verify load data",
                load_count
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
    use gat_core::{Branch, BranchId, Bus, BusId, Gen, GenId, Kilovolts, Load, LoadId};

    fn make_simple_network() -> Network {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));

        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Gen 1".to_string(), BusId::new(1)).with_p_limits(0.0, 100.0),
        ));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power: gat_core::Megawatts(50.0),
            reactive_power: gat_core::Megavars(10.0),
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
        assert!(diag
            .issues
            .iter()
            .any(|i| i.message.contains("zero impedance")));
    }

    #[test]
    fn test_invalid_bus_reference_error() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator references non-existent bus 999
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Bad Gen".to_string(), BusId::new(999))
                .with_p_limits(0.0, 100.0),
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.error_count() > 0);
        assert!(diag
            .issues
            .iter()
            .any(|i| i.message.contains("non-existent bus")));
    }

    #[test]
    fn test_island_detection() {
        let mut network = Network::new();

        // Create two disconnected buses
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
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
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator with Pmax < Pmin
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Bad Gen".to_string(), BusId::new(1))
                .with_p_limits(100.0, 50.0), // Inverted!
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.error_count() > 0);
        assert!(diag
            .issues
            .iter()
            .any(|i| i.message.contains("Pmax") && i.message.contains("Pmin")));
    }

    #[test]
    fn test_unusual_voltage_warning() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Weird Bus".to_string(),
            base_kv: Kilovolts(0.05), // 50V - very unusual
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            area_id: None,
            zone_id: None,
        }));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(diag.warning_count() > 0);
        assert!(diag
            .issues
            .iter()
            .any(|i| i.message.contains("Unusual voltage")));
    }

    #[test]
    fn test_quick_validate_counts() {
        let network = make_simple_network();
        let (_warnings, errors) = validate_network_quick(&network);

        assert_eq!(errors, 0, "Valid network should have no errors")
    }

    #[test]
    fn test_all_generators_same_bus_warning() {
        let mut network = Network::new();

        // Single bus
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Multiple generators all at the same bus - suspicious!
        for i in 0..5 {
            network.graph.add_node(Node::Gen(
                Gen::new(GenId::new(i), format!("Gen {}", i), BusId::new(1))
                    .with_p_limits(0.0, 100.0),
            ));
        }

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        // Should warn about all generators at same bus
        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("All 5 generators are at bus 1")),
            "Should warn about all generators at same bus: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_generators_at_bus_0_warning() {
        let mut network = Network::new();

        // Valid bus
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator at bus 0 - invalid, likely mapping failure
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Bad Gen".to_string(), BusId::new(0)).with_p_limits(0.0, 100.0),
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        // Should warn about generator at bus 0
        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("generator(s) at bus 0")),
            "Should warn about generators at bus 0: {:?}",
            diag.issues
        );
    }

    // ==================== Branch Clustering Tests ====================

    #[test]
    fn test_all_branches_from_same_bus_warning() {
        let mut network = Network::new();

        // Create 3 buses
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus3_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus 3".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // All branches from bus 1 - suspicious star topology
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
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            bus1_idx,
            bus3_idx,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                name: "Branch 1-3".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                name: "Branch 1-2b".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("branches have from_bus 1")),
            "Should warn about all branches from same bus: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_branches_at_bus_0_warning() {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Branch connected to bus 0 - mapping failure
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Bad Branch".to_string(),
                from_bus: BusId::new(0), // Invalid!
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("branch(es) connected to bus 0")),
            "Should warn about branches at bus 0: {:?}",
            diag.issues
        );
    }

    // ==================== Power Balance Tests ====================

    #[test]
    fn test_insufficient_generation_capacity_error() {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator with only 50 MW capacity
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Small Gen".to_string(), BusId::new(1))
                .with_p_limits(0.0, 50.0),
        ));

        // Load requiring 100 MW - infeasible!
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Big Load".to_string(),
            bus: BusId::new(2),
            active_power: gat_core::Megawatts(100.0),
            reactive_power: gat_core::Megavars(10.0),
        }));

        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("Pmax") && i.message.contains("infeasible")),
            "Should error on insufficient generation: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_over_generation_at_pmin_warning() {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator with Pmin of 100 MW (can't go lower)
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Must-run Gen".to_string(), BusId::new(1))
                .with_p_limits(100.0, 200.0),
        ));

        // Load only needs 50 MW - curtailment needed!
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Small Load".to_string(),
            bus: BusId::new(2),
            active_power: gat_core::Megawatts(50.0),
            reactive_power: gat_core::Megavars(10.0),
        }));

        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("Pmin") && i.message.contains("curtailment")),
            "Should warn on over-generation: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_negative_pmax_warning() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator with negative Pmax - data error
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Bad Gen".to_string(),
            bus: BusId::new(1),
            pmin: gat_core::Megawatts(0.0),
            pmax: gat_core::Megawatts(-100.0), // Invalid!
            ..Gen::default()
        }));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("negative Pmax")),
            "Should warn on negative Pmax: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_negative_load_warning() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Load with negative power - could be DER
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "DER Load".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(-50.0), // Negative!
            reactive_power: gat_core::Megavars(0.0),
        }));

        // Need a gen to avoid "no generators" warning
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Gen".to_string(), BusId::new(1)).with_p_limits(0.0, 100.0),
        ));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues.iter().any(|i| i.message.contains("negative P")),
            "Should warn on negative load: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_zero_total_load_warning() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Gen".to_string(), BusId::new(1)).with_p_limits(0.0, 100.0),
        ));

        // Load with zero power
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Zero Load".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(0.0),
            reactive_power: gat_core::Megavars(0.0),
        }));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        assert!(
            diag.issues
                .iter()
                .any(|i| i.message.contains("Total active load is zero")),
            "Should warn on zero total load: {:?}",
            diag.issues
        );
    }

    #[test]
    fn test_pmin_greater_than_pmax_error() {
        let mut network = Network::new();

        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Generator with Pmin > Pmax (caught by power balance validator)
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Invalid Gen".to_string(),
            bus: BusId::new(1),
            pmin: gat_core::Megawatts(100.0),
            pmax: gat_core::Megawatts(50.0), // Less than Pmin!
            ..Gen::default()
        }));

        let mut diag = ImportDiagnostics::new();
        validate_network(&network, &mut diag, &ValidationConfig::default());

        // Should have error from power balance validation
        assert!(
            diag.issues.iter().any(|i| {
                i.message.contains("Pmin")
                    && i.message.contains("Pmax")
                    && i.category == "PowerBalance"
            }),
            "Should error on Pmin > Pmax in power balance: {:?}",
            diag.issues
        );
    }
}
