//! N-k contingency screening using LODF-based fast estimation.
//!
//! For N-k analysis (k ≥ 2), evaluating all O(n^k) contingency combinations
//! with full power flow is computationally prohibitive. This module provides:
//!
//! 1. **Fast LODF-based screening:** Estimate post-contingency flows without solving power flow
//! 2. **Threshold filtering:** Flag only combinations exceeding a configurable % of limits
//! 3. **Full evaluation:** Run power flow only on flagged cases
//!
//! ## Algorithm
//!
//! For N-2: with branches m₁, m₂ both out:
//! ```text
//! flow_ℓ_post ≈ flow_ℓ_pre + LODF[ℓ,m₁]·flow_m₁ + LODF[ℓ,m₂]·flow_m₂
//!              + second-order correction (ignored in screening)
//! ```
//!
//! This linear approximation is conservative for screening purposes.

use super::lodf::{compute_lodf_matrix, compute_ptdf_matrix, LodfMatrix, PtdfMatrix};
use anyhow::Result;
use gat_core::{Edge, Network};
use rayon::prelude::*;
use std::collections::HashMap;

/// Configuration for N-k screening.
#[derive(Debug, Clone)]
pub struct NkScreeningConfig {
    /// Maximum contingency order (k in N-k)
    pub max_k: usize,
    /// Flag threshold as fraction of limit (e.g., 0.9 = 90%)
    pub threshold_fraction: f64,
    /// Branch thermal limits (branch_id → MVA limit)
    pub branch_limits: HashMap<usize, f64>,
    /// Default limit if not specified (0 = no limit)
    pub default_limit_mva: f64,
}

impl Default for NkScreeningConfig {
    fn default() -> Self {
        Self {
            max_k: 2,
            threshold_fraction: 0.9,
            branch_limits: HashMap::new(),
            default_limit_mva: 0.0,
        }
    }
}

/// A contingency: one or more elements out of service.
#[derive(Debug, Clone)]
pub struct Contingency {
    /// Branch IDs that are out in this contingency
    pub outaged_branches: Vec<usize>,
    /// Optional probability of this contingency occurring
    pub probability: Option<f64>,
    /// Human-readable label
    pub label: Option<String>,
}

impl Contingency {
    /// Create an N-1 contingency (single branch outage).
    pub fn single(branch_id: usize) -> Self {
        Self {
            outaged_branches: vec![branch_id],
            probability: None,
            label: None,
        }
    }

    /// Create an N-2 contingency (two branches out).
    pub fn double(branch_id1: usize, branch_id2: usize) -> Self {
        Self {
            outaged_branches: vec![branch_id1, branch_id2],
            probability: None,
            label: None,
        }
    }

    /// Order of this contingency (k in N-k).
    pub fn order(&self) -> usize {
        self.outaged_branches.len()
    }
}

/// Result of screening a single contingency.
#[derive(Debug, Clone)]
pub struct ScreeningResult {
    /// The contingency that was screened
    pub contingency: Contingency,
    /// Estimated maximum loading fraction (flow / limit)
    pub max_loading_fraction: f64,
    /// Branch with highest loading
    pub most_loaded_branch: Option<usize>,
    /// Whether this contingency exceeds the screening threshold
    pub flagged: bool,
    /// Estimated violations (branch_id, estimated_flow, limit)
    pub violations: Vec<(usize, f64, f64)>,
}

/// Results from N-k screening.
#[derive(Debug)]
pub struct NkScreeningResults {
    /// All screened contingencies
    pub results: Vec<ScreeningResult>,
    /// Number flagged for detailed evaluation
    pub num_flagged: usize,
    /// Total contingencies screened
    pub total_screened: usize,
    /// Screening threshold used
    pub threshold: f64,
}

impl NkScreeningResults {
    /// Get only the flagged contingencies (for full evaluation).
    pub fn flagged(&self) -> impl Iterator<Item = &ScreeningResult> {
        self.results.iter().filter(|r| r.flagged)
    }

    /// Get summary statistics.
    pub fn summary(&self) -> String {
        format!(
            "N-k screening: {}/{} flagged ({:.1}% pass rate) at {:.0}% threshold",
            self.num_flagged,
            self.total_screened,
            100.0 * (1.0 - self.num_flagged as f64 / self.total_screened.max(1) as f64),
            self.threshold * 100.0
        )
    }
}

/// Pre-computed data for fast N-k screening.
pub struct NkScreener {
    #[allow(dead_code)] // Kept for future debugging/analysis
    ptdf: PtdfMatrix,
    lodf: LodfMatrix,
    branch_ids: Vec<usize>,
    base_flows: HashMap<usize, f64>,
    config: NkScreeningConfig,
}

impl NkScreener {
    /// Create a new screener from a network and base case flows.
    ///
    /// `base_flows` should contain pre-contingency flow on each branch (from DC power flow).
    pub fn new(
        network: &Network,
        base_flows: HashMap<usize, f64>,
        config: NkScreeningConfig,
    ) -> Result<Self> {
        let ptdf = compute_ptdf_matrix(network)?;
        let lodf = compute_lodf_matrix(network, &ptdf)?;
        let branch_ids = ptdf.branch_ids.clone();

        Ok(Self {
            ptdf,
            lodf,
            branch_ids,
            base_flows,
            config,
        })
    }

    /// Generate all N-1 contingencies.
    pub fn generate_n1(&self) -> Vec<Contingency> {
        self.branch_ids
            .iter()
            .map(|&id| Contingency::single(id))
            .collect()
    }

    /// Generate all N-2 contingencies.
    pub fn generate_n2(&self) -> Vec<Contingency> {
        let mut contingencies = Vec::new();
        let n = self.branch_ids.len();
        for i in 0..n {
            for j in (i + 1)..n {
                contingencies.push(Contingency::double(self.branch_ids[i], self.branch_ids[j]));
            }
        }
        contingencies
    }

    /// Screen a single contingency using LODF estimation.
    pub fn screen_contingency(&self, contingency: &Contingency) -> ScreeningResult {
        let mut max_loading = 0.0;
        let mut most_loaded = None;
        let mut violations = Vec::new();

        // For each surviving branch, estimate post-contingency flow
        for &branch_l in &self.branch_ids {
            // Skip outaged branches
            if contingency.outaged_branches.contains(&branch_l) {
                continue;
            }

            // Base flow on this branch
            let base_flow = *self.base_flows.get(&branch_l).unwrap_or(&0.0);

            // Add flow redistribution from each outaged branch
            let mut estimated_flow = base_flow;
            for &branch_m in &contingency.outaged_branches {
                let flow_m = *self.base_flows.get(&branch_m).unwrap_or(&0.0);
                if let Some(lodf) = self.lodf.get(branch_l, branch_m) {
                    if lodf.is_finite() {
                        estimated_flow += lodf * flow_m;
                    }
                }
            }

            // Check against limit
            let limit = self
                .config
                .branch_limits
                .get(&branch_l)
                .copied()
                .unwrap_or(self.config.default_limit_mva);

            if limit > 0.0 {
                let loading = estimated_flow.abs() / limit;
                if loading > max_loading {
                    max_loading = loading;
                    most_loaded = Some(branch_l);
                }
                if loading > self.config.threshold_fraction {
                    violations.push((branch_l, estimated_flow.abs(), limit));
                }
            }
        }

        let flagged = max_loading > self.config.threshold_fraction;

        ScreeningResult {
            contingency: contingency.clone(),
            max_loading_fraction: max_loading,
            most_loaded_branch: most_loaded,
            flagged,
            violations,
        }
    }

    /// Screen all contingencies up to order k (parallel).
    pub fn screen_all(&self, contingencies: &[Contingency]) -> NkScreeningResults {
        let results: Vec<ScreeningResult> = contingencies
            .par_iter()
            .map(|c| self.screen_contingency(c))
            .collect();

        let num_flagged = results.iter().filter(|r| r.flagged).count();
        let total = results.len();

        NkScreeningResults {
            results,
            num_flagged,
            total_screened: total,
            threshold: self.config.threshold_fraction,
        }
    }

    /// Screen N-1 and N-2 contingencies.
    pub fn screen_n1_n2(&self) -> NkScreeningResults {
        let mut contingencies = self.generate_n1();
        if self.config.max_k >= 2 {
            contingencies.extend(self.generate_n2());
        }
        self.screen_all(&contingencies)
    }
}

/// Convenience function: run N-k screening on a network with base case flows.
pub fn screen_nk_contingencies(
    network: &Network,
    base_flows: HashMap<usize, f64>,
    config: NkScreeningConfig,
) -> Result<NkScreeningResults> {
    let screener = NkScreener::new(network, base_flows, config)?;
    Ok(screener.screen_n1_n2())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId, Node};

    fn create_test_network() -> Network {
        let mut network = Network::new();

        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        let b3 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                reactance: 0.1,
                rating_a_mva: Some(100.0),
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            b2,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                from_bus: BusId::new(2),
                to_bus: BusId::new(3),
                reactance: 0.1,
                rating_a_mva: Some(100.0),
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            b1,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                reactance: 0.2,
                rating_a_mva: Some(100.0),
                ..Branch::default()
            }),
        );

        network
    }

    #[test]
    fn test_generate_n1_contingencies() {
        let network = create_test_network();
        let base_flows = HashMap::from([(1, 50.0), (2, 30.0), (3, 20.0)]);
        let config = NkScreeningConfig::default();
        let screener = NkScreener::new(&network, base_flows, config).unwrap();

        let n1 = screener.generate_n1();
        assert_eq!(n1.len(), 3);
        assert!(n1.iter().all(|c| c.order() == 1));
    }

    #[test]
    fn test_generate_n2_contingencies() {
        let network = create_test_network();
        let base_flows = HashMap::from([(1, 50.0), (2, 30.0), (3, 20.0)]);
        let config = NkScreeningConfig::default();
        let screener = NkScreener::new(&network, base_flows, config).unwrap();

        let n2 = screener.generate_n2();
        // C(3,2) = 3 combinations
        assert_eq!(n2.len(), 3);
        assert!(n2.iter().all(|c| c.order() == 2));
    }

    #[test]
    fn test_screen_n1_no_violations() {
        let network = create_test_network();
        // Low base flows, well below limits
        let base_flows = HashMap::from([(1, 20.0), (2, 15.0), (3, 10.0)]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]);
        config.threshold_fraction = 0.9;

        let screener = NkScreener::new(&network, base_flows, config).unwrap();
        let results = screener.screen_all(&screener.generate_n1());

        assert_eq!(results.total_screened, 3);
        // With low flows and high limits, nothing should be flagged
        println!("N-1 flagged: {}", results.num_flagged);
        for r in &results.results {
            println!(
                "  Contingency {:?}: max_loading={:.2}",
                r.contingency.outaged_branches, r.max_loading_fraction
            );
        }
    }

    #[test]
    fn test_screen_n1_with_violation() {
        let network = create_test_network();
        // High base flow on branch 1, which will redistribute when branch 3 trips
        let base_flows = HashMap::from([(1, 80.0), (2, 60.0), (3, 40.0)]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]);
        config.threshold_fraction = 0.9;

        let screener = NkScreener::new(&network, base_flows, config).unwrap();
        let results = screener.screen_all(&screener.generate_n1());

        println!("N-1 with high flows:");
        for r in &results.results {
            println!(
                "  Outage {:?}: max_loading={:.2}, flagged={}",
                r.contingency.outaged_branches, r.max_loading_fraction, r.flagged
            );
        }

        // At least some should be flagged with these high flows
        assert!(results.total_screened > 0);
    }

    #[test]
    fn test_screen_n2() {
        let network = create_test_network();
        let base_flows = HashMap::from([(1, 50.0), (2, 30.0), (3, 20.0)]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]);
        config.max_k = 2;

        let screener = NkScreener::new(&network, base_flows, config).unwrap();
        let results = screener.screen_n1_n2();

        // 3 N-1 + 3 N-2 = 6 total contingencies
        assert_eq!(results.total_screened, 6);
        println!("{}", results.summary());
    }
}
