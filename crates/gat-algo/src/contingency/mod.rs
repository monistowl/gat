//! N-k contingency analysis with LODF-based fast screening.
//!
//! This module extends gat's existing N-1 contingency analysis to N-2+ using
//! Line Outage Distribution Factors (LODFs) for efficient pre-screening.
//!
//! ## Key Concepts
//!
//! - **PTDF (Power Transfer Distribution Factor):** Sensitivity of branch flow to bus injection.
//!   `PTDF[ℓ,n]` = ∂f_ℓ/∂P_n (change in flow on branch ℓ per MW injected at bus n)
//!
//! - **LODF (Line Outage Distribution Factor):** Redistribution of flow when a branch trips.
//!   `LODF[ℓ,m]` = flow increase on branch ℓ when branch m is outaged, as a fraction
//!   of the pre-outage flow on branch m.
//!
//! ## Algorithm
//!
//! For N-k analysis with k ≥ 2, the combinatorial explosion makes exhaustive power flow
//! infeasible. The LODF approach:
//!
//! 1. Pre-compute PTDF and LODF matrices (one-time O(n³) cost)
//! 2. For each contingency combination, estimate post-contingency flows using LODFs
//! 3. Flag combinations where estimated flows exceed 90% of limits
//! 4. Run full DC power flow only on flagged cases (~1-5% of total)
//!
//! ## References
//!
//! - Wood & Wollenberg, "Power Generation, Operation and Control", Ch. 9
//! - Alsac et al., "Fast Calculation of LODF and Application to Branch Outage Studies"

pub mod lodf;
pub mod n_k;

pub use lodf::{compute_lodf_matrix, compute_ptdf_matrix, LodfMatrix, PtdfMatrix};
pub use n_k::{
    collect_branch_limits, collect_branch_terminals, collect_injections,
    screen_nk_contingencies, BranchViolation, Contingency, ContingencyEvaluation,
    NkEvaluationResults, NkEvaluator, NkScreener, NkScreeningConfig, NkScreeningResults,
    OutageProbabilityConfig, ScreeningResult,
};
