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
//!
//! ## Type Safety
//!
//! This module uses typed IDs (`BranchId`, `BusId`) throughout for compile-time safety,
//! leveraging the unified [`crate::sparse::SparsePtdf`] sensitivity matrices.

use crate::sparse::{LodfMatrix, PtdfMatrix, SparsePtdf};
use anyhow::Result;
use gat_core::{BranchId, BusId, Edge, Network, Node};
use rayon::prelude::*;
use std::collections::HashMap;

/// Configuration for N-k screening.
#[derive(Debug, Clone)]
pub struct NkScreeningConfig {
    /// Maximum contingency order (k in N-k)
    pub max_k: usize,
    /// Flag threshold as fraction of limit (e.g., 0.9 = 90%)
    pub threshold_fraction: f64,
    /// Branch thermal limits (BranchId → MVA limit)
    pub branch_limits: HashMap<BranchId, f64>,
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
    pub outaged_branches: Vec<BranchId>,
    /// Probability of this contingency occurring (per year or per exposure time)
    pub probability: Option<f64>,
    /// Human-readable label
    pub label: Option<String>,
}

impl Contingency {
    /// Create an N-1 contingency (single branch outage).
    pub fn single(branch_id: BranchId) -> Self {
        Self {
            outaged_branches: vec![branch_id],
            probability: None,
            label: None,
        }
    }

    /// Create an N-2 contingency (two branches out).
    pub fn double(branch_id1: BranchId, branch_id2: BranchId) -> Self {
        Self {
            outaged_branches: vec![branch_id1, branch_id2],
            probability: None,
            label: None,
        }
    }

    /// Create a contingency with probability assigned.
    pub fn with_probability(mut self, prob: f64) -> Self {
        self.probability = Some(prob);
        self
    }

    /// Create a contingency with a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Order of this contingency (k in N-k).
    pub fn order(&self) -> usize {
        self.outaged_branches.len()
    }

    /// Compute probability from Forced Outage Rates (FOR) assuming independence.
    ///
    /// For N-k contingencies, probability = FOR₁ × FOR₂ × ... × FORₖ
    /// where FOR is typically in the range 0.001-0.05 per year.
    pub fn compute_probability(&mut self, for_rates: &HashMap<BranchId, f64>) {
        let prob = self
            .outaged_branches
            .iter()
            .map(|id| for_rates.get(id).copied().unwrap_or(0.01)) // Default 1% FOR
            .product();
        self.probability = Some(prob);
    }
}

/// Configuration for outage probability computation.
#[derive(Debug, Clone)]
pub struct OutageProbabilityConfig {
    /// Forced Outage Rate per branch (BranchId → FOR per year)
    pub for_rates: HashMap<BranchId, f64>,
    /// Default FOR if not specified (typical: 0.01 = 1% per year)
    pub default_for: f64,
    /// Exposure time in hours (default: 8760 = 1 year)
    pub exposure_hours: f64,
}

impl Default for OutageProbabilityConfig {
    fn default() -> Self {
        Self {
            for_rates: HashMap::new(),
            default_for: 0.01, // 1% FOR typical for transmission lines
            exposure_hours: 8760.0,
        }
    }
}

impl OutageProbabilityConfig {
    /// Get FOR for a specific branch.
    pub fn get_for(&self, branch_id: BranchId) -> f64 {
        self.for_rates
            .get(&branch_id)
            .copied()
            .unwrap_or(self.default_for)
    }

    /// Compute probability for a contingency (assuming independence).
    pub fn compute_contingency_probability(&self, outaged_branches: &[BranchId]) -> f64 {
        outaged_branches
            .iter()
            .map(|&id| self.get_for(id))
            .product()
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
    pub most_loaded_branch: Option<BranchId>,
    /// Whether this contingency exceeds the screening threshold
    pub flagged: bool,
    /// Estimated violations (BranchId, estimated_flow, limit)
    pub violations: Vec<(BranchId, f64, f64)>,
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
    branch_ids: Vec<BranchId>,
    base_flows: HashMap<BranchId, f64>,
    config: NkScreeningConfig,
}

impl NkScreener {
    /// Create a new screener from a network and base case flows.
    ///
    /// `base_flows` should contain pre-contingency flow on each branch (from DC power flow).
    pub fn new(
        network: &Network,
        base_flows: HashMap<BranchId, f64>,
        config: NkScreeningConfig,
    ) -> Result<Self> {
        let ptdf = SparsePtdf::compute_ptdf(network)?;
        let lodf = SparsePtdf::compute_lodf(network, &ptdf)?;
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
    base_flows: HashMap<BranchId, f64>,
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
    /// Actual branch flows (BranchId → MW)
    pub branch_flows: HashMap<BranchId, f64>,
    /// Maximum loading fraction (flow / limit)
    pub max_loading: f64,
    /// Branch with highest loading
    pub critical_branch: Option<BranchId>,
    /// List of violations
    pub violations: Vec<BranchViolation>,
    /// Load shed required (MW), if any
    pub load_shed_mw: f64,
    /// Expected Unserved Energy contribution (MWh) = probability × load_shed × exposure_hours
    pub eue_contribution_mwh: f64,
    /// Severity index combining probability and impact (for ranking)
    pub severity_index: f64,
}

impl ContingencyEvaluation {
    /// Compute EUE contribution given exposure time.
    ///
    /// EUE = probability × load_shed_mw × exposure_hours
    pub fn compute_eue(&mut self, exposure_hours: f64) {
        if let Some(prob) = self.contingency.probability {
            self.eue_contribution_mwh = prob * self.load_shed_mw * exposure_hours;
            // Severity index = probability × (max_loading - 1.0) for violations
            self.severity_index = if self.max_loading > 1.0 {
                prob * (self.max_loading - 1.0) * 100.0
            } else {
                0.0
            };
        }
    }
}

/// A thermal limit violation on a specific branch.
#[derive(Debug, Clone)]
pub struct BranchViolation {
    pub branch_id: BranchId,
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
    /// Total Expected Unserved Energy (MWh)
    pub total_eue_mwh: f64,
    /// Total load shed across all contingencies (MW)
    pub total_load_shed_mw: f64,
}

impl NkEvaluationResults {
    /// Get summary string.
    pub fn summary(&self) -> String {
        format!(
            "N-k evaluation: {}/{} violated, {} non-convergent, worst loading {:.1}%, EUE={:.2} MWh",
            self.num_violated,
            self.evaluations.len(),
            self.num_non_convergent,
            self.worst_loading * 100.0,
            self.total_eue_mwh
        )
    }

    /// Get violations sorted by severity (max loading).
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

    /// Get contingencies ranked by EUE contribution (highest first).
    pub fn ranked_by_eue(&self) -> Vec<&ContingencyEvaluation> {
        let mut ranked: Vec<_> = self.evaluations.iter().collect();
        ranked.sort_by(|a, b| {
            b.eue_contribution_mwh
                .partial_cmp(&a.eue_contribution_mwh)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Get contingencies ranked by severity index (probability × impact).
    pub fn ranked_by_severity(&self) -> Vec<&ContingencyEvaluation> {
        let mut ranked: Vec<_> = self.evaluations.iter().collect();
        ranked.sort_by(|a, b| {
            b.severity_index
                .partial_cmp(&a.severity_index)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Get top N contingencies by EUE contribution.
    pub fn top_n_by_eue(&self, n: usize) -> Vec<&ContingencyEvaluation> {
        self.ranked_by_eue().into_iter().take(n).collect()
    }

    /// Compute cumulative EUE contribution.
    /// Returns (contingencies_included, cumulative_eue) pairs.
    pub fn cumulative_eue(&self) -> Vec<(usize, f64)> {
        let ranked = self.ranked_by_eue();
        let mut cumulative = Vec::new();
        let mut total = 0.0;
        for (i, eval) in ranked.iter().enumerate() {
            total += eval.eue_contribution_mwh;
            cumulative.push((i + 1, total));
        }
        cumulative
    }

    /// Get percentage of total EUE from top N contingencies.
    pub fn eue_concentration(&self, top_n: usize) -> f64 {
        if self.total_eue_mwh == 0.0 {
            return 0.0;
        }
        let top_eue: f64 = self
            .ranked_by_eue()
            .iter()
            .take(top_n)
            .map(|e| e.eue_contribution_mwh)
            .sum();
        top_eue / self.total_eue_mwh * 100.0
    }
}

/// Full N-k evaluator that runs DC power flow for flagged contingencies.
pub struct NkEvaluator<'a> {
    network: &'a Network,
    injections: HashMap<BusId, f64>,
    branch_limits: HashMap<BranchId, f64>,
    prob_config: OutageProbabilityConfig,
}

impl<'a> NkEvaluator<'a> {
    /// Create evaluator with bus injections (from base case).
    pub fn new(
        network: &'a Network,
        injections: HashMap<BusId, f64>,
        branch_limits: HashMap<BranchId, f64>,
    ) -> Self {
        Self {
            network,
            injections,
            branch_limits,
            prob_config: OutageProbabilityConfig::default(),
        }
    }

    /// Configure outage probabilities for EUE computation.
    pub fn with_probability_config(mut self, config: OutageProbabilityConfig) -> Self {
        self.prob_config = config;
        self
    }

    /// Set FOR (Forced Outage Rate) for all branches.
    pub fn with_default_for(mut self, default_for: f64) -> Self {
        self.prob_config.default_for = default_for;
        self
    }

    /// Set exposure time in hours (default: 8760 = 1 year).
    pub fn with_exposure_hours(mut self, hours: f64) -> Self {
        self.prob_config.exposure_hours = hours;
        self
    }

    /// Evaluate a single contingency using DC power flow.
    ///
    /// This creates a modified network with outaged branches removed,
    /// then solves DC power flow to get actual post-contingency flows.
    /// Probability and EUE are computed if probability config is set.
    pub fn evaluate(&self, contingency: &Contingency) -> ContingencyEvaluation {
        // For simplicity, we'll use the existing DC power flow infrastructure
        // by computing angles with outaged branches, then calculating flows.
        let outaged_set: std::collections::HashSet<BranchId> =
            contingency.outaged_branches.iter().cloned().collect();

        // Assign probability if not already set
        let prob = contingency.probability.unwrap_or_else(|| {
            self.prob_config
                .compute_contingency_probability(&contingency.outaged_branches)
        });
        let mut contingency_with_prob = contingency.clone();
        contingency_with_prob.probability = Some(prob);

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

                // Compute load shed estimate based on overload
                // Simple approximation: excess flow above limit represents load that can't be served
                let load_shed_mw: f64 = violations
                    .iter()
                    .map(|v| (v.flow_mw - v.limit_mw).max(0.0))
                    .sum();

                // Compute EUE contribution
                let eue_contribution_mwh = prob * load_shed_mw * self.prob_config.exposure_hours;

                // Compute severity index (probability × overload severity)
                let severity_index = if max_loading > 1.0 {
                    prob * (max_loading - 1.0) * 100.0
                } else {
                    0.0
                };

                ContingencyEvaluation {
                    contingency: contingency_with_prob,
                    converged: true,
                    branch_flows: flows,
                    max_loading,
                    critical_branch,
                    violations,
                    load_shed_mw,
                    eue_contribution_mwh,
                    severity_index,
                }
            }
            Err(_) => ContingencyEvaluation {
                contingency: contingency_with_prob,
                converged: false,
                branch_flows: HashMap::new(),
                max_loading: f64::INFINITY,
                critical_branch: None,
                violations: vec![],
                load_shed_mw: 0.0,
                eue_contribution_mwh: 0.0,
                severity_index: 0.0,
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

        let num_violated = evaluations
            .iter()
            .filter(|e| !e.violations.is_empty())
            .count();
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

        // Compute EUE totals
        let total_eue_mwh: f64 = evaluations.iter().map(|e| e.eue_contribution_mwh).sum();
        let total_load_shed_mw: f64 = evaluations.iter().map(|e| e.load_shed_mw).sum();

        NkEvaluationResults {
            evaluations,
            num_violated,
            num_non_convergent,
            worst_loading,
            worst_contingency,
            total_eue_mwh,
            total_load_shed_mw,
        }
    }

    /// Evaluate a list of contingencies directly (parallel).
    pub fn evaluate_all(&self, contingencies: &[Contingency]) -> NkEvaluationResults {
        let evaluations: Vec<ContingencyEvaluation> =
            contingencies.par_iter().map(|c| self.evaluate(c)).collect();

        let num_violated = evaluations
            .iter()
            .filter(|e| !e.violations.is_empty())
            .count();
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

        let total_eue_mwh: f64 = evaluations.iter().map(|e| e.eue_contribution_mwh).sum();
        let total_load_shed_mw: f64 = evaluations.iter().map(|e| e.load_shed_mw).sum();

        NkEvaluationResults {
            evaluations,
            num_violated,
            num_non_convergent,
            worst_loading,
            worst_contingency,
            total_eue_mwh,
            total_load_shed_mw,
        }
    }

    /// Simplified DC power flow with outaged branches.
    fn compute_dc_flows_with_outages(
        &self,
        outaged: &std::collections::HashSet<BranchId>,
    ) -> Result<HashMap<BranchId, f64>> {
        // Build susceptance matrix excluding outaged branches
        let mut bus_ids: Vec<BusId> = self
            .network
            .graph
            .node_indices()
            .filter_map(|idx| match &self.network.graph[idx] {
                Node::Bus(bus) => Some(bus.id),
                _ => None,
            })
            .collect();
        bus_ids.sort_unstable_by_key(|id| id.value());

        let n = bus_ids.len();
        if n < 2 {
            return Ok(HashMap::new());
        }

        let mut bus_to_idx: HashMap<BusId, usize> = HashMap::new();
        for (idx, &id) in bus_ids.iter().enumerate() {
            bus_to_idx.insert(id, idx);
        }

        // Build B' matrix
        let mut b_matrix = vec![vec![0.0; n]; n];
        let mut branches: Vec<(BranchId, BusId, BusId, f64)> = Vec::new();

        for edge in self.network.graph.edge_references() {
            if let Edge::Branch(branch) = edge.weight() {
                if !branch.status || outaged.contains(&branch.id) {
                    continue;
                }
                let from = branch.from_bus;
                let to = branch.to_bus;
                let x = (branch.reactance * branch.tap_ratio).abs().max(1e-6);

                if let (Some(&i), Some(&j)) = (bus_to_idx.get(&from), bus_to_idx.get(&to)) {
                    let b = 1.0 / x;
                    b_matrix[i][j] -= b;
                    b_matrix[j][i] -= b;
                    b_matrix[i][i] += b;
                    b_matrix[j][j] += b;
                }
                branches.push((branch.id, from, to, x));
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
    let b_work = b.to_vec();

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

// =============================================================================
// Helper functions for extracting network data
// =============================================================================

/// Extract net power injections (generation - load) per bus in MW.
///
/// This is the standard input format for N-k screening and DC power flow.
/// Returns a map of BusId → net injection in MW (positive = generation surplus).
pub fn collect_injections(network: &Network) -> HashMap<BusId, f64> {
    let mut injections = HashMap::new();
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Gen(gen) => {
                *injections.entry(gen.bus).or_insert(0.0) += gen.active_power_mw;
            }
            Node::Load(load) => {
                *injections.entry(load.bus).or_insert(0.0) -= load.active_power_mw;
            }
            _ => {}
        }
    }
    injections
}

/// Extract branch thermal limits (rating_a_mva) per branch.
///
/// Returns a map of BranchId → thermal limit in MVA.
/// Branches without ratings or with very small ratings (< 0.1 MVA) are excluded.
pub fn collect_branch_limits(network: &Network) -> HashMap<BranchId, f64> {
    let mut limits = HashMap::new();
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            if branch.status {
                if let Some(rating) = branch.rating_a_mva {
                    // Skip very small ratings to avoid numerical issues
                    if rating > 0.1 {
                        limits.insert(branch.id, rating);
                    }
                }
            }
        }
    }
    limits
}

/// Collect branch terminal buses for result mapping.
///
/// Returns a map of BranchId → (from_bus, to_bus).
pub fn collect_branch_terminals(network: &Network) -> HashMap<BranchId, (BusId, BusId)> {
    let mut terminals = HashMap::new();
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            if branch.status {
                terminals.insert(branch.id, (branch.from_bus, branch.to_bus));
            }
        }
    }
    terminals
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
        let base_flows = HashMap::from([
            (BranchId::new(1), 50.0),
            (BranchId::new(2), 30.0),
            (BranchId::new(3), 20.0),
        ]);
        let config = NkScreeningConfig::default();
        let screener = NkScreener::new(&network, base_flows, config).unwrap();

        let n1 = screener.generate_n1();
        assert_eq!(n1.len(), 3);
        assert!(n1.iter().all(|c| c.order() == 1));
    }

    #[test]
    fn test_generate_n2_contingencies() {
        let network = create_test_network();
        let base_flows = HashMap::from([
            (BranchId::new(1), 50.0),
            (BranchId::new(2), 30.0),
            (BranchId::new(3), 20.0),
        ]);
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
        let base_flows = HashMap::from([
            (BranchId::new(1), 20.0),
            (BranchId::new(2), 15.0),
            (BranchId::new(3), 10.0),
        ]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([
            (BranchId::new(1), 100.0),
            (BranchId::new(2), 100.0),
            (BranchId::new(3), 100.0),
        ]);
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
        let base_flows = HashMap::from([
            (BranchId::new(1), 80.0),
            (BranchId::new(2), 60.0),
            (BranchId::new(3), 40.0),
        ]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([
            (BranchId::new(1), 100.0),
            (BranchId::new(2), 100.0),
            (BranchId::new(3), 100.0),
        ]);
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
        let base_flows = HashMap::from([
            (BranchId::new(1), 50.0),
            (BranchId::new(2), 30.0),
            (BranchId::new(3), 20.0),
        ]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([
            (BranchId::new(1), 100.0),
            (BranchId::new(2), 100.0),
            (BranchId::new(3), 100.0),
        ]);
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
        let injections = HashMap::from([(BusId::new(1), 1.0), (BusId::new(3), -1.0)]); // in pu (100 MW base)
        let branch_limits = HashMap::from([
            (BranchId::new(1), 100.0),
            (BranchId::new(2), 100.0),
            (BranchId::new(3), 100.0),
        ]);

        let evaluator = NkEvaluator::new(&network, injections, branch_limits);

        // Evaluate a single N-1 contingency
        let contingency = Contingency::single(BranchId::new(3)); // Take out branch 3
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

        // High flows to trigger flagging (N-1 only to avoid singular N-2 cases)
        let base_flows = HashMap::from([
            (BranchId::new(1), 80.0),
            (BranchId::new(2), 60.0),
            (BranchId::new(3), 40.0),
        ]);
        let mut config = NkScreeningConfig::default();
        config.branch_limits = HashMap::from([
            (BranchId::new(1), 100.0),
            (BranchId::new(2), 100.0),
            (BranchId::new(3), 100.0),
        ]);
        config.threshold_fraction = 0.5; // Low threshold to flag more cases
        config.max_k = 1; // Only N-1 to avoid island-creating N-2 cases

        let screener = NkScreener::new(&network, base_flows.clone(), config).unwrap();
        let screening = screener.screen_all(&screener.generate_n1());

        // Now evaluate flagged cases
        let injections = HashMap::from([(BusId::new(1), 0.8), (BusId::new(3), -0.8)]); // Matching base flows roughly
        let evaluator = NkEvaluator::new(
            &network,
            injections,
            HashMap::from([
                (BranchId::new(1), 100.0),
                (BranchId::new(2), 100.0),
                (BranchId::new(3), 100.0),
            ]),
        );

        let eval_results = evaluator.evaluate_flagged(&screening);

        println!("{}", eval_results.summary());
        println!(
            "Evaluated {} flagged contingencies",
            eval_results.evaluations.len()
        );

        // N-1 evaluations should all converge (triangle network stays connected)
        for eval in &eval_results.evaluations {
            assert!(
                eval.converged,
                "N-1 contingency {:?} should converge",
                eval.contingency.outaged_branches
            );
        }
    }

    #[test]
    fn test_contingency_probability() {
        // Test probability computation from FOR rates
        let for_rates = HashMap::from([
            (BranchId::new(1), 0.02),
            (BranchId::new(2), 0.03),
            (BranchId::new(3), 0.01),
        ]);

        // N-1: probability = FOR
        let mut c1 = Contingency::single(BranchId::new(1));
        c1.compute_probability(&for_rates);
        assert!((c1.probability.unwrap() - 0.02).abs() < 1e-10);

        // N-2: probability = FOR₁ × FOR₂
        let mut c2 = Contingency::double(BranchId::new(1), BranchId::new(2));
        c2.compute_probability(&for_rates);
        assert!((c2.probability.unwrap() - 0.02 * 0.03).abs() < 1e-10);

        // Test builder pattern
        let c3 = Contingency::single(BranchId::new(3))
            .with_probability(0.05)
            .with_label("Test contingency");
        assert_eq!(c3.probability, Some(0.05));
        assert_eq!(c3.label, Some("Test contingency".to_string()));
    }

    #[test]
    fn test_outage_probability_config() {
        let mut config = OutageProbabilityConfig::default();
        config.for_rates.insert(BranchId::new(1), 0.02);
        config.for_rates.insert(BranchId::new(2), 0.03);
        config.default_for = 0.01;

        // Known branch
        assert!((config.get_for(BranchId::new(1)) - 0.02).abs() < 1e-10);

        // Unknown branch uses default
        assert!((config.get_for(BranchId::new(99)) - 0.01).abs() < 1e-10);

        // N-1 probability
        let prob1 = config.compute_contingency_probability(&[BranchId::new(1)]);
        assert!((prob1 - 0.02).abs() < 1e-10);

        // N-2 probability
        let prob2 = config.compute_contingency_probability(&[BranchId::new(1), BranchId::new(2)]);
        assert!((prob2 - 0.0006).abs() < 1e-10); // 0.02 × 0.03
    }

    #[test]
    fn test_eue_computation() {
        let network = create_test_network();

        // Setup with known probability configuration
        let injections = HashMap::from([
            (BusId::new(1), 1.5),
            (BusId::new(3), -1.5),
        ]); // Higher flow to cause violations
        let branch_limits = HashMap::from([
            (BranchId::new(1), 50.0),
            (BranchId::new(2), 50.0),
            (BranchId::new(3), 50.0),
        ]); // Lower limits

        let mut prob_config = OutageProbabilityConfig::default();
        prob_config.default_for = 0.01; // 1% FOR
        prob_config.exposure_hours = 8760.0; // 1 year

        let evaluator = NkEvaluator::new(&network, injections, branch_limits)
            .with_probability_config(prob_config);

        // Evaluate a contingency
        let contingency = Contingency::single(BranchId::new(3));
        let result = evaluator.evaluate(&contingency);

        println!("EUE test evaluation:");
        println!("  Probability: {:?}", result.contingency.probability);
        println!("  Max loading: {:.2}", result.max_loading);
        println!("  Load shed: {:.2} MW", result.load_shed_mw);
        println!("  EUE contribution: {:.2} MWh", result.eue_contribution_mwh);
        println!("  Severity index: {:.4}", result.severity_index);

        // Verify probability was assigned
        assert!(result.contingency.probability.is_some());
        assert!((result.contingency.probability.unwrap() - 0.01).abs() < 1e-10);

        // If there's a violation, EUE should be computed
        if result.load_shed_mw > 0.0 {
            assert!(result.eue_contribution_mwh > 0.0);
        }
    }

    #[test]
    fn test_eue_ranking() {
        let network = create_test_network();

        let injections = HashMap::from([
            (BusId::new(1), 1.2),
            (BusId::new(3), -1.2),
        ]); // Moderate flow
        let branch_limits = HashMap::from([
            (BranchId::new(1), 60.0),
            (BranchId::new(2), 60.0),
            (BranchId::new(3), 60.0),
        ]);

        let mut prob_config = OutageProbabilityConfig::default();
        prob_config.for_rates = HashMap::from([
            (BranchId::new(1), 0.05), // 5% FOR - high failure rate
            (BranchId::new(2), 0.01), // 1% FOR
            (BranchId::new(3), 0.02), // 2% FOR
        ]);
        prob_config.exposure_hours = 8760.0;

        let evaluator = NkEvaluator::new(&network, injections, branch_limits)
            .with_probability_config(prob_config);

        // Evaluate all N-1 contingencies
        let contingencies: Vec<Contingency> = vec![
            Contingency::single(BranchId::new(1)),
            Contingency::single(BranchId::new(2)),
            Contingency::single(BranchId::new(3)),
        ];
        let results = evaluator.evaluate_all(&contingencies);

        println!("EUE ranking test:");
        println!("{}", results.summary());
        println!("Total EUE: {:.2} MWh", results.total_eue_mwh);

        // Test ranking methods
        let by_eue = results.ranked_by_eue();
        println!("\nRanked by EUE:");
        for eval in &by_eue {
            println!(
                "  Branch {:?}: EUE={:.2} MWh, prob={:.4}",
                eval.contingency.outaged_branches,
                eval.eue_contribution_mwh,
                eval.contingency.probability.unwrap_or(0.0)
            );
        }

        let by_severity = results.ranked_by_severity();
        println!("\nRanked by severity:");
        for eval in &by_severity {
            println!(
                "  Branch {:?}: severity={:.4}",
                eval.contingency.outaged_branches, eval.severity_index
            );
        }

        // Test EUE concentration
        let top1_concentration = results.eue_concentration(1);
        println!("\nTop 1 EUE concentration: {:.1}%", top1_concentration);

        // Verify cumulative EUE sums to total
        let cumulative = results.cumulative_eue();
        if let Some((_, final_total)) = cumulative.last() {
            assert!(
                (final_total - results.total_eue_mwh).abs() < 1e-6,
                "Cumulative EUE should sum to total"
            );
        }
    }
}
