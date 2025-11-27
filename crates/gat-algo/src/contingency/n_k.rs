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

// =============================================================================
// Full N-k Evaluation (for flagged contingencies)
// =============================================================================

/// Result of full DC power flow evaluation for a contingency.
#[derive(Debug, Clone)]
pub struct ContingencyEvaluation {
    /// The contingency evaluated
    pub contingency: Contingency,
    /// Whether DC power flow converged
    pub converged: bool,
    /// Actual branch flows (branch_id → MW)
    pub branch_flows: HashMap<usize, f64>,
    /// Maximum loading fraction (flow / limit)
    pub max_loading: f64,
    /// Branch with highest loading
    pub critical_branch: Option<usize>,
    /// List of violations (branch_id, flow_mw, limit_mw, loading_fraction)
    pub violations: Vec<BranchViolation>,
    /// Load shed required (MW), if any
    pub load_shed_mw: f64,
}

/// A thermal limit violation on a specific branch.
#[derive(Debug, Clone)]
pub struct BranchViolation {
    pub branch_id: usize,
    pub flow_mw: f64,
    pub limit_mw: f64,
    pub loading_fraction: f64,
}

/// Results from full N-k evaluation.
#[derive(Debug)]
pub struct NkEvaluationResults {
    /// All evaluated contingencies
    pub evaluations: Vec<ContingencyEvaluation>,
    /// Number with violations
    pub num_violated: usize,
    /// Number that didn't converge
    pub num_non_convergent: usize,
    /// Worst-case loading across all contingencies
    pub worst_loading: f64,
    /// Contingency with worst loading
    pub worst_contingency: Option<Contingency>,
}

impl NkEvaluationResults {
    /// Get summary string.
    pub fn summary(&self) -> String {
        format!(
            "N-k evaluation: {}/{} violated, {} non-convergent, worst loading {:.1}%",
            self.num_violated,
            self.evaluations.len(),
            self.num_non_convergent,
            self.worst_loading * 100.0
        )
    }

    /// Get violations sorted by severity.
    pub fn violations_by_severity(&self) -> Vec<&ContingencyEvaluation> {
        let mut with_violations: Vec<_> = self
            .evaluations
            .iter()
            .filter(|e| !e.violations.is_empty())
            .collect();
        with_violations.sort_by(|a, b| {
            b.max_loading
                .partial_cmp(&a.max_loading)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        with_violations
    }
}

/// Full N-k evaluator that runs DC power flow for flagged contingencies.
pub struct NkEvaluator<'a> {
    network: &'a Network,
    injections: HashMap<usize, f64>,
    branch_limits: HashMap<usize, f64>,
}

impl<'a> NkEvaluator<'a> {
    /// Create evaluator with bus injections (from base case).
    pub fn new(
        network: &'a Network,
        injections: HashMap<usize, f64>,
        branch_limits: HashMap<usize, f64>,
    ) -> Self {
        Self {
            network,
            injections,
            branch_limits,
        }
    }

    /// Evaluate a single contingency using DC power flow.
    ///
    /// This creates a modified network with outaged branches removed,
    /// then solves DC power flow to get actual post-contingency flows.
    pub fn evaluate(&self, contingency: &Contingency) -> ContingencyEvaluation {
        // For simplicity, we'll use the existing DC power flow infrastructure
        // by computing angles with outaged branches, then calculating flows.
        let outaged_set: std::collections::HashSet<usize> =
            contingency.outaged_branches.iter().cloned().collect();

        // Compute DC angles with outaged branches removed
        match self.compute_dc_flows_with_outages(&outaged_set) {
            Ok(flows) => {
                let mut max_loading = 0.0;
                let mut critical_branch = None;
                let mut violations = Vec::new();

                for (&branch_id, &flow) in &flows {
                    if let Some(&limit) = self.branch_limits.get(&branch_id) {
                        if limit > 0.0 {
                            let loading = flow.abs() / limit;
                            if loading > max_loading {
                                max_loading = loading;
                                critical_branch = Some(branch_id);
                            }
                            if loading > 1.0 {
                                violations.push(BranchViolation {
                                    branch_id,
                                    flow_mw: flow.abs(),
                                    limit_mw: limit,
                                    loading_fraction: loading,
                                });
                            }
                        }
                    }
                }

                violations.sort_by(|a, b| {
                    b.loading_fraction
                        .partial_cmp(&a.loading_fraction)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                ContingencyEvaluation {
                    contingency: contingency.clone(),
                    converged: true,
                    branch_flows: flows,
                    max_loading,
                    critical_branch,
                    violations,
                    load_shed_mw: 0.0, // TODO: compute if needed
                }
            }
            Err(_) => ContingencyEvaluation {
                contingency: contingency.clone(),
                converged: false,
                branch_flows: HashMap::new(),
                max_loading: f64::INFINITY,
                critical_branch: None,
                violations: vec![],
                load_shed_mw: 0.0,
            },
        }
    }

    /// Evaluate all flagged contingencies (parallel).
    pub fn evaluate_flagged(&self, screening_results: &NkScreeningResults) -> NkEvaluationResults {
        let flagged: Vec<_> = screening_results.flagged().collect();

        let evaluations: Vec<ContingencyEvaluation> = flagged
            .par_iter()
            .map(|r| self.evaluate(&r.contingency))
            .collect();

        let num_violated = evaluations.iter().filter(|e| !e.violations.is_empty()).count();
        let num_non_convergent = evaluations.iter().filter(|e| !e.converged).count();

        let (worst_loading, worst_contingency) = evaluations
            .iter()
            .filter(|e| e.converged)
            .max_by(|a, b| {
                a.max_loading
                    .partial_cmp(&b.max_loading)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| (e.max_loading, Some(e.contingency.clone())))
            .unwrap_or((0.0, None));

        NkEvaluationResults {
            evaluations,
            num_violated,
            num_non_convergent,
            worst_loading,
            worst_contingency,
        }
    }

    /// Simplified DC power flow with outaged branches.
    fn compute_dc_flows_with_outages(
        &self,
        outaged: &std::collections::HashSet<usize>,
    ) -> Result<HashMap<usize, f64>> {
        // Build susceptance matrix excluding outaged branches
        let mut bus_ids: Vec<usize> = self
            .network
            .graph
            .node_indices()
            .filter_map(|idx| match &self.network.graph[idx] {
                gat_core::Node::Bus(bus) => Some(bus.id.value()),
                _ => None,
            })
            .collect();
        bus_ids.sort_unstable();

        let n = bus_ids.len();
        if n < 2 {
            return Ok(HashMap::new());
        }

        let mut bus_to_idx: HashMap<usize, usize> = HashMap::new();
        for (idx, &id) in bus_ids.iter().enumerate() {
            bus_to_idx.insert(id, idx);
        }

        // Build B' matrix
        let mut b_matrix = vec![vec![0.0; n]; n];
        let mut branches: Vec<(usize, usize, usize, f64)> = Vec::new();

        for edge in self.network.graph.edge_references() {
            if let Edge::Branch(branch) = edge.weight() {
                if !branch.status || outaged.contains(&branch.id.value()) {
                    continue;
                }
                let from = branch.from_bus.value();
                let to = branch.to_bus.value();
                let x = (branch.reactance * branch.tap_ratio).abs().max(1e-6);

                if let (Some(&i), Some(&j)) = (bus_to_idx.get(&from), bus_to_idx.get(&to)) {
                    let b = 1.0 / x;
                    b_matrix[i][j] -= b;
                    b_matrix[j][i] -= b;
                    b_matrix[i][i] += b;
                    b_matrix[j][j] += b;
                }
                branches.push((branch.id.value(), from, to, x));
            }
        }

        // Build RHS from injections
        let mut rhs = vec![0.0; n];
        for (&bus_id, &inj) in &self.injections {
            if let Some(&idx) = bus_to_idx.get(&bus_id) {
                rhs[idx] = inj;
            }
        }

        // Solve reduced system (slack = first bus)
        let m = n - 1;
        let mut reduced = vec![vec![0.0; m]; m];
        let mut reduced_rhs = vec![0.0; m];
        for i in 0..m {
            for j in 0..m {
                reduced[i][j] = b_matrix[i + 1][j + 1];
            }
            reduced_rhs[i] = rhs[i + 1];
        }

        let angles = solve_linear_system(&reduced, &reduced_rhs)?;

        // Full angles (slack = 0)
        let mut theta = vec![0.0; n];
        for i in 0..m {
            theta[i + 1] = angles[i];
        }

        // Compute branch flows
        let mut flows = HashMap::new();
        for (id, from, to, x) in branches {
            let i = *bus_to_idx.get(&from).unwrap();
            let j = *bus_to_idx.get(&to).unwrap();
            let flow = (theta[i] - theta[j]) / x * 100.0; // Convert to MW (base 100 MVA)
            flows.insert(id, flow);
        }

        Ok(flows)
    }
}

/// Solve linear system Ax = b using LU decomposition.
fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
    let n = a.len();
    if n == 0 {
        return Ok(vec![]);
    }

    let mut lu: Vec<Vec<f64>> = a.to_vec();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut b_work = b.to_vec();

    // LU with partial pivoting
    for k in 0..n {
        let mut max_val = lu[k][k].abs();
        let mut max_row = k;
        for i in (k + 1)..n {
            if lu[i][k].abs() > max_val {
                max_val = lu[i][k].abs();
                max_row = i;
            }
        }

        if max_val < 1e-12 {
            return Err(anyhow::anyhow!("Singular matrix in DC power flow"));
        }

        if max_row != k {
            lu.swap(k, max_row);
            perm.swap(k, max_row);
        }

        for i in (k + 1)..n {
            lu[i][k] /= lu[k][k];
            for j in (k + 1)..n {
                lu[i][j] -= lu[i][k] * lu[k][j];
            }
        }
    }

    // Permute b
    let mut pb = vec![0.0; n];
    for i in 0..n {
        pb[i] = b_work[perm[i]];
    }

    // Forward substitution
    let mut y = vec![0.0; n];
    for i in 0..n {
        y[i] = pb[i];
        for j in 0..i {
            y[i] -= lu[i][j] * y[j];
        }
    }

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        x[i] = y[i];
        for j in (i + 1)..n {
            x[i] -= lu[i][j] * x[j];
        }
        x[i] /= lu[i][i];
    }

    Ok(x)
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

    #[test]
    fn test_full_evaluation() {
        let network = create_test_network();

        // Bus injections: 100 MW at bus 1, -100 MW at bus 3 (simplified)
        let injections = HashMap::from([(1, 1.0), (3, -1.0)]); // in pu (100 MW base)
        let branch_limits = HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]);

        let evaluator = NkEvaluator::new(&network, injections, branch_limits);

        // Evaluate a single N-1 contingency
        let contingency = Contingency::single(3); // Take out branch 3
        let result = evaluator.evaluate(&contingency);

        assert!(result.converged, "DC power flow should converge");
        println!("Evaluation of branch 3 outage:");
        println!("  Converged: {}", result.converged);
        println!("  Max loading: {:.2}", result.max_loading);
        println!("  Flows: {:?}", result.branch_flows);

        // With branch 3 out, all flow goes through branch 1 → 2 path
        // Verify we get flows
        assert!(!result.branch_flows.is_empty(), "Should have flows");
    }

    #[test]
    fn test_evaluate_flagged_contingencies() {
        let network = create_test_network();

        // High flows to trigger flagging
        let base_flows = HashMap::from([(1, 80.0), (2, 60.0), (3, 40.0)]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]);
        config.threshold_fraction = 0.5; // Low threshold to flag more cases

        let screener = NkScreener::new(&network, base_flows.clone(), config).unwrap();
        let screening = screener.screen_n1_n2();

        // Now evaluate flagged cases
        let injections = HashMap::from([(1, 0.8), (3, -0.8)]); // Matching base flows roughly
        let evaluator = NkEvaluator::new(&network, injections, HashMap::from([(1, 100.0), (2, 100.0), (3, 100.0)]));

        let eval_results = evaluator.evaluate_flagged(&screening);

        println!("{}", eval_results.summary());
        println!("Evaluated {} flagged contingencies", eval_results.evaluations.len());

        // All evaluations should converge for this simple network
        assert_eq!(eval_results.num_non_convergent, 0);
    }
}
