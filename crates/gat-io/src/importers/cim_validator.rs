use anyhow::{anyhow, Result};
use gat_core::{Edge, Network, Node};

/// Represents a validation error or warning from CIM data
#[derive(Debug, Clone)]
pub struct CimValidationError {
    pub entity_type: String, // "Bus", "Branch", "Generator"
    pub entity_id: String,
    pub issue: String,
}

impl std::fmt::Display for CimValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.entity_type, self.entity_id, self.issue
        )
    }
}

/// Validate a network imported from CIM with strict requirements
pub fn validate_network_from_cim(network: &Network) -> Result<()> {
    let mut errors = Vec::new();

    // Check that network has at least one bus
    let bus_count = network
        .graph
        .node_indices()
        .filter(|idx| matches!(network.graph[*idx], Node::Bus(_)))
        .count();

    if bus_count == 0 {
        errors.push(CimValidationError {
            entity_type: "Network".to_string(),
            entity_id: "n/a".to_string(),
            issue: "Network has no buses".to_string(),
        });
    }

    // Validate buses
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            if bus.name.is_empty() {
                errors.push(CimValidationError {
                    entity_type: "Bus".to_string(),
                    entity_id: bus.id.value().to_string(),
                    issue: "Bus has empty name".to_string(),
                });
            }
            if bus.base_kv.value() <= 0.0 {
                errors.push(CimValidationError {
                    entity_type: "Bus".to_string(),
                    entity_id: bus.name.clone(),
                    issue: format!("Invalid voltage: {} kV", bus.base_kv.value()),
                });
            }
        }
    }

    // Validate branches
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if branch.name.is_empty() {
                errors.push(CimValidationError {
                    entity_type: "Branch".to_string(),
                    entity_id: branch.id.value().to_string(),
                    issue: "Branch has empty name".to_string(),
                });
            }
            if branch.resistance < 0.0 {
                errors.push(CimValidationError {
                    entity_type: "Branch".to_string(),
                    entity_id: branch.name.clone(),
                    issue: format!("Invalid resistance: {} (must be >= 0)", branch.resistance),
                });
            }
            if branch.reactance == 0.0 && branch.resistance == 0.0 {
                errors.push(CimValidationError {
                    entity_type: "Branch".to_string(),
                    entity_id: branch.name.clone(),
                    issue: "Branch has zero impedance (r=0, x=0)".to_string(),
                });
            }
        }
    }

    // Validate generators
    for node_idx in network.graph.node_indices() {
        if let Node::Gen(gen) = &network.graph[node_idx] {
            if gen.name.is_empty() {
                errors.push(CimValidationError {
                    entity_type: "Generator".to_string(),
                    entity_id: gen.id.value().to_string(),
                    issue: "Generator has empty name".to_string(),
                });
            }
        }
    }

    // Validate loads
    for node_idx in network.graph.node_indices() {
        if let Node::Load(load) = &network.graph[node_idx] {
            if load.name.is_empty() {
                errors.push(CimValidationError {
                    entity_type: "Load".to_string(),
                    entity_id: load.id.value().to_string(),
                    issue: "Load has empty name".to_string(),
                });
            }
        }
    }

    if !errors.is_empty() {
        let error_msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow!("CIM validation failed:\n{}", error_msg));
    }

    Ok(())
}

/// Validate network and return warnings for unusual but valid configurations
pub fn validate_cim_with_warnings(network: &Network) -> Vec<CimValidationError> {
    let mut warnings = Vec::new();

    // Check for unusual but valid configurations
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            let kv = bus.base_kv.value();
            if kv < 1.0 || kv > 1000.0 {
                warnings.push(CimValidationError {
                    entity_type: "Bus".to_string(),
                    entity_id: bus.name.clone(),
                    issue: format!("Unusual voltage level: {} kV", kv),
                });
            }
        }
    }

    // Check for branches with very high resistance relative to reactance
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if branch.reactance != 0.0 && (branch.resistance / branch.reactance.abs()) > 10.0 {
                warnings.push(CimValidationError {
                    entity_type: "Branch".to_string(),
                    entity_id: branch.name.clone(),
                    issue: format!(
                        "Unusual R/X ratio: {:.2}",
                        branch.resistance / branch.reactance.abs()
                    ),
                });
            }
        }
    }

    warnings
}
