//! Transmission Expansion Planning (TEP)
//!
//! This module implements a DC-based Mixed-Integer Linear Programming (MILP)
//! formulation for Transmission Expansion Planning.
//!
//! ## Problem Overview
//!
//! TEP determines which candidate transmission lines to build to minimize
//! total cost (investment + operation) while meeting demand and respecting
//! physical constraints.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  TRANSMISSION EXPANSION PLANNING (TEP)                                   │
//! │  ──────────────────────────────────────                                  │
//! │                                                                          │
//! │  Given:                                                                  │
//! │    • Existing network with generators and loads                         │
//! │    • Set of candidate transmission lines                                │
//! │    • Investment costs per candidate line                                │
//! │    • Operating scenarios (loads, generation)                            │
//! │                                                                          │
//! │  Decide:                                                                 │
//! │    • Which candidate lines to build (binary decisions)                  │
//! │    • Generator dispatch (continuous)                                    │
//! │                                                                          │
//! │  Minimize:                                                               │
//! │    Total cost = Investment cost + Operating cost                        │
//! │                                                                          │
//! │  Subject to:                                                             │
//! │    • Power balance at each bus                                          │
//! │    • Generator limits                                                   │
//! │    • Branch flow limits (existing and built candidates)                 │
//! │    • DC power flow physics (linearized)                                 │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## MILP Formulation
//!
//! The TEP problem is formulated as:
//!
//! ```text
//! minimize    Σ_k c_k · x_k + Σ_g c_g · P_g
//!             └──────────────┘ └────────────┘
//!             investment cost   operating cost
//!
//! subject to:
//!   Σ P_gen(i) - Σ P_load(i) = Σ_j P_ij(i,j)     Power balance (Kirchhoff's current law)
//!   P_ij = b_ij · (θ_i - θ_j)                    DC power flow (existing lines)
//!   -M(1-x_k) ≤ P_k - b_k(θ_i-θ_j) ≤ M(1-x_k)   Candidate lines (Big-M disjunctive)
//!   |P_ij| ≤ P_ij^max                            Line flow limits
//!   |P_k| ≤ P_k^max · x_k                        Candidate flow limits (active only if built)
//!   x_k ∈ {0,1}                                  Binary build decisions
//! ```
//!
//! ## Big-M Constraints
//!
//! The key modeling challenge is that power flow on candidate lines should:
//! - Follow physics (P = b·Δθ) when the line is built (x=1)
//! - Be zero when the line is not built (x=0)
//!
//! Big-M formulation achieves this:
//! - When x=1: -M·0 ≤ P - b·Δθ ≤ M·0 → P = b·Δθ (enforced)
//! - When x=0: -M ≤ P - b·Δθ ≤ M (relaxed, but P=0 from flow limit)
//!
//! ## References
//!
//! - **Garver (1970)**: "Transmission Network Estimation Using Linear Programming"
//!   - Classic disjunctive formulation
//!   - 6-bus benchmark problem
//!
//! - **Romero et al. (2002)**: "Analysis of heuristic algorithms for transmission network expansion planning"
//!   - Comparison of MILP vs. heuristics
//!
//! - **Alguacil et al. (2003)**: "Transmission network expansion planning: A mixed-integer LP approach"
//!   - Modern MILP techniques for large-scale TEP

mod problem;
mod solution;
mod solver;

pub use problem::{CandidateId, CandidateLine, TepProblem, TepProblemBuilder};
pub use solution::{LineBuildDecision, TepSolution};
pub use solver::{solve_tep, TepError, TepSolverConfig};
