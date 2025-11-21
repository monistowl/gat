use anyhow::{anyhow, Result};
use gat_core::{Branch, Edge, Gen, Network, Node};
use petgraph::graph::EdgeIndex;

use crate::spec::{OutageSpec, ResolvedScenario};

#[derive(Debug, Clone)]
pub struct ScenarioApplyOptions {
    pub drop_outaged_elements: bool,
}

impl Default for ScenarioApplyOptions {
    fn default() -> Self {
        Self {
            drop_outaged_elements: true,
        }
    }
}

pub fn apply_scenario_to_network(
    network: &mut Network,
    scenario: &ResolvedScenario,
    opts: &ScenarioApplyOptions,
) -> Result<()> {
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
    for outage in &scenario.outages {
        match outage {
            OutageSpec::Branch { .. } => {}
            OutageSpec::Gen { id } => {
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
    for node_idx in network.graph.node_indices() {
        if let Some(node) = network.graph.node_weight_mut(node_idx) {
            match node {
                Node::Load(load) => {
                    load.active_power_mw *= scenario.load_scale;
                    load.reactive_power_mvar *= scenario.load_scale;
                }
                Node::Gen(gen) => {
                    gen.active_power_mw *= scenario.renewable_scale;
                    gen.reactive_power_mvar *= scenario.renewable_scale;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

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

fn branch_matches(branch: &Branch, needle: &str) -> bool {
    if branch.name == needle {
        return true;
    }
    if let Ok(idx) = needle.parse::<usize>() {
        return branch.id.value() == idx;
    }
    false
}

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
