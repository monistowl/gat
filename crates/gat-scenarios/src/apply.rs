use anyhow::{anyhow, Result};
use gat_core::{Branch, Edge, Gen, Network, Node};
use petgraph::graph::EdgeIndex;

use crate::spec::{OutageSpec, ResolvedScenario};

/// Options for applying a scenario to a network topology.
///
/// Controls how outages are handled: whether outaged elements (branches, generators)
/// are removed from the graph or simply disabled.
#[derive(Debug, Clone)]
pub struct ScenarioApplyOptions {
    /// If true, remove outaged branches from the graph entirely.
    /// If false, branches remain but are effectively disabled (future: set flow limits to 0).
    pub drop_outaged_elements: bool,
}

impl Default for ScenarioApplyOptions {
    fn default() -> Self {
        Self {
            drop_outaged_elements: true,
        }
    }
}

/// Apply a resolved scenario to a network, modifying it in-place.
///
/// **Algorithm:**
/// 1. Apply outages: remove branches or disable generators based on outage specs.
/// 2. Scale loads: multiply all load P/Q by `scenario.load_scale`.
/// 3. Scale renewables: multiply all generator P/Q by `scenario.renewable_scale`.
///
/// This implements the standard N-1/N-k contingency analysis pattern used in reliability
/// assessment (see doi:10.1109/TPWRS.2007.899019 for DC contingency analysis).
///
/// **Note:** Bus outages are not yet supported; model them as branch/gen outages.
pub fn apply_scenario_to_network(
    network: &mut Network,
    scenario: &ResolvedScenario,
    opts: &ScenarioApplyOptions,
) -> Result<()> {
    // Step 1: Apply branch outages by removing edges from the graph (if drop_outaged_elements is true)
    // This models N-1/N-k contingencies where transmission lines are out of service.
    if !scenario.outages.is_empty() && opts.drop_outaged_elements {
        let mut branch_ids_to_remove = Vec::new();
        for outage in &scenario.outages {
            if let OutageSpec::Branch { id } = outage {
                branch_ids_to_remove.extend(find_matching_branches(network, id));
            }
        }
        for edge in branch_ids_to_remove {
            network.graph.remove_edge(edge);
        }
    }

    // Step 2: Apply generator outages and other non-branch outages
    for outage in &scenario.outages {
        match outage {
            OutageSpec::Branch { .. } => {
                // Already handled above if drop_outaged_elements is true
            }
            OutageSpec::Gen { id } => {
                // Disable generator by setting P/Q to zero (models generator outage)
                disable_generator(network, id);
            }
            OutageSpec::Bus { id } => {
                return Err(anyhow!(
                    "bus outages are not supported yet ({}); consider modeling as branch/gen outages",
                    id
                ));
            }
        }
    }

    // Step 3: Scale loads and renewable generation according to scenario multipliers
    // This models demand growth scenarios, renewable penetration scenarios, etc.
    for node_idx in network.graph.node_indices() {
        if let Some(node) = network.graph.node_weight_mut(node_idx) {
            match node {
                Node::Load(load) => {
                    // Scale load by scenario's load_scale (e.g., 1.1 = 10% demand growth)
                    load.active_power_mw *= scenario.load_scale;
                    load.reactive_power_mvar *= scenario.load_scale;
                }
                Node::Gen(gen) => {
                    // Scale renewable generation by scenario's renewable_scale
                    // Note: This applies to all generators; in v1 we may want per-generator scaling
                    gen.active_power_mw *= scenario.renewable_scale;
                    gen.reactive_power_mvar *= scenario.renewable_scale;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Find all branch edges matching the given identifier (by name or ID).
///
/// **Matching logic:** Matches if branch name equals `needle`, or if `needle` parses as an integer
/// and equals the branch ID. This allows flexible identification by name or numeric ID.
fn find_matching_branches(network: &Network, needle: &str) -> Vec<EdgeIndex> {
    network
        .graph
        .edge_indices()
        .filter(|idx| {
            network.graph[*idx]
                .as_branch()
                .map(|branch| branch_matches(branch, needle))
                .unwrap_or(false)
        })
        .collect()
}

/// Disable a generator by setting its active and reactive power to zero.
///
/// **Purpose:** Models generator outages in contingency analysis. The generator remains in the
/// network topology but produces no power.
fn disable_generator(network: &mut Network, needle: &str) {
    for node_idx in network.graph.node_indices() {
        if let Some(Node::Gen(gen)) = network.graph.node_weight_mut(node_idx) {
            if generator_matches(gen, needle) {
                gen.active_power_mw = 0.0;
                gen.reactive_power_mvar = 0.0;
            }
        }
    }
}

/// Check if a branch matches the given identifier (name or numeric ID).
fn branch_matches(branch: &Branch, needle: &str) -> bool {
    if branch.name == needle {
        return true;
    }
    if let Ok(idx) = needle.parse::<usize>() {
        return branch.id.value() == idx;
    }
    false
}

/// Check if a generator matches the given identifier (name or numeric ID).
fn generator_matches(gen: &Gen, needle: &str) -> bool {
    if gen.name == needle {
        return true;
    }
    if let Ok(idx) = needle.parse::<usize>() {
        return gen.id.value() == idx;
    }
    false
}

trait EdgeBranchExt {
    fn as_branch(&self) -> Option<&Branch>;
}

impl EdgeBranchExt for Edge {
    fn as_branch(&self) -> Option<&Branch> {
        match self {
            Edge::Branch(branch) => Some(branch),
            _ => None,
        }
    }
}
