//! Incremental matrix solvers using Woodbury formula for N-1 analysis.
//!
//! In N-1 contingency analysis, we need to solve many systems where only
//! 1-2 matrix entries change (branch outages). The Woodbury formula enables
//! efficient rank-k updates without full matrix refactorization.
//!
//! ## Woodbury Formula
//!
//! For a matrix A with low-rank update UCV:
//! ```text
//! (A + UCV)⁻¹ = A⁻¹ - A⁻¹U(C⁻¹ + VA⁻¹U)⁻¹VA⁻¹
//! ```
//!
//! For branch outages in power systems:
//! - U and V are sparse vectors (only 2 non-zero entries)
//! - C is typically scalar (the susceptance being removed)
//! - This reduces O(n³) refactorization to O(n²) update
//!
//! ## Usage
//!
//! ```ignore
//! use gat_algo::sparse::{SparseSusceptance, IncrementalSolver};
//!
//! let b_prime = SparseSusceptance::from_network(&network)?;
//! let solver = IncrementalSolver::new(&b_prime)?;
//!
//! // Solve base case
//! let theta_base = solver.solve(&p_injection)?;
//!
//! // For each contingency, apply Woodbury update
//! for branch_id in monitored_branches {
//!     let update = WoodburyUpdate::branch_outage(&b_prime, branch_id)?;
//!     let theta_contingency = solver.solve_with_update(&p_injection, &update)?;
//! }
//! ```

use gat_core::BranchId;
use sprs::{CsMat, CsVec};
use thiserror::Error;

use super::susceptance::SparseSusceptance;

/// Errors from incremental solver operations
#[derive(Debug, Error)]
pub enum IncrementalError {
    #[error("Matrix factorization failed: {0}")]
    FactorizationFailed(String),

    #[error("Singular matrix encountered")]
    SingularMatrix,

    #[error("Unknown branch ID: {0}")]
    UnknownBranch(usize),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Woodbury update is singular (branch already disconnected?)")]
    SingularUpdate,
}

/// A low-rank matrix update for Woodbury formula.
///
/// Represents the update `ΔB = u × c × vᵀ` where:
/// - u, v are sparse column vectors
/// - c is a scalar (susceptance change)
#[derive(Debug, Clone)]
pub struct WoodburyUpdate {
    /// Sparse vector u (typically has 2 non-zeros for branch outage)
    pub u: CsVec<f64>,
    /// Sparse vector v (typically equal to u for symmetric updates)
    pub v: CsVec<f64>,
    /// Scalar multiplier (susceptance being removed)
    pub c: f64,
    /// Branch being modified (for diagnostics)
    pub branch_id: Option<BranchId>,
}

impl WoodburyUpdate {
    /// Create update for a branch outage.
    ///
    /// When branch k between buses i and j trips, the B' matrix changes:
    /// ```text
    /// ΔB'[i,i] = -b_k    (diagonal decreases)
    /// ΔB'[j,j] = -b_k    (diagonal decreases)
    /// ΔB'[i,j] = +b_k    (off-diagonal increases)
    /// ΔB'[j,i] = +b_k    (symmetric)
    /// ```
    ///
    /// This can be represented as rank-1 update: `ΔB' = -b_k × e × eᵀ`
    /// where e[i] = 1, e[j] = -1.
    pub fn branch_outage(
        b_prime: &SparseSusceptance,
        branch_id: BranchId,
    ) -> Result<Self, IncrementalError> {
        let (from_idx, to_idx, susceptance) = b_prime
            .branch_data(branch_id)
            .ok_or(IncrementalError::UnknownBranch(branch_id.value()))?;

        let n = b_prime.n_bus();
        let slack_idx = b_prime.slack_idx();

        // Build sparse vector e where e[from] = 1, e[to] = -1
        // Skip entries for slack bus (reduced system)
        let mut indices = Vec::new();
        let mut values = Vec::new();

        // Map full indices to reduced indices (skip slack)
        let reduced_from = if from_idx < slack_idx {
            Some(from_idx)
        } else if from_idx > slack_idx {
            Some(from_idx - 1)
        } else {
            None
        };

        let reduced_to = if to_idx < slack_idx {
            Some(to_idx)
        } else if to_idx > slack_idx {
            Some(to_idx - 1)
        } else {
            None
        };

        if let Some(idx) = reduced_from {
            indices.push(idx);
            values.push(1.0);
        }
        if let Some(idx) = reduced_to {
            indices.push(idx);
            values.push(-1.0);
        }

        // Sort by index (required for CsVec)
        let mut pairs: Vec<_> = indices.into_iter().zip(values).collect();
        pairs.sort_by_key(|(idx, _)| *idx);

        let (sorted_indices, sorted_values): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();

        let reduced_dim = n - 1;
        let u = CsVec::new(reduced_dim, sorted_indices.clone(), sorted_values.clone());
        let v = CsVec::new(reduced_dim, sorted_indices, sorted_values);

        Ok(Self {
            u,
            v,
            c: -susceptance, // Negative because we're removing the branch
            branch_id: Some(branch_id),
        })
    }

    /// Create a custom rank-1 update.
    pub fn custom(u: CsVec<f64>, v: CsVec<f64>, c: f64) -> Self {
        Self {
            u,
            v,
            c,
            branch_id: None,
        }
    }
}

/// Incremental solver using pre-factorized base case and Woodbury updates.
///
/// Stores the LU factorization of the reduced B' matrix and enables
/// efficient solutions for modified systems.
#[derive(Debug)]
pub struct IncrementalSolver {
    /// Dimension of reduced system (n_bus - 1)
    dim: usize,
    /// LU factors of reduced B' matrix (L and U stored together)
    lu_factors: Vec<f64>,
    /// Pivot indices from LU decomposition
    pivots: Vec<usize>,
    /// Reduced B' matrix (stored for potential sparse matrix-vector ops)
    #[allow(dead_code)]
    reduced_b_prime: CsMat<f64>,
}

impl IncrementalSolver {
    /// Create solver from susceptance matrix.
    ///
    /// Performs LU factorization of the reduced B' matrix (slack removed).
    pub fn new(b_prime: &SparseSusceptance) -> Result<Self, IncrementalError> {
        let (reduced_matrix, _reduced_order) = b_prime.reduced_matrix();
        let dim = reduced_matrix.rows();

        if dim == 0 {
            return Err(IncrementalError::FactorizationFailed(
                "Empty reduced matrix".to_string(),
            ));
        }

        // Convert sparse to dense for LU factorization
        // (For large systems, consider using sparse LU from sprs or faer)
        let mut dense = vec![0.0; dim * dim];
        for (val, (i, j)) in reduced_matrix.iter() {
            dense[i * dim + j] = *val;
        }

        // LU factorization with partial pivoting
        let (lu_factors, pivots) =
            Self::lu_factorize(&dense, dim).map_err(|e| IncrementalError::FactorizationFailed(e))?;

        Ok(Self {
            dim,
            lu_factors,
            pivots,
            reduced_b_prime: reduced_matrix,
        })
    }

    /// Solve B' × θ = P for the base case (no contingencies).
    ///
    /// Input `p` should be the reduced injection vector (slack bus removed).
    pub fn solve(&self, p: &[f64]) -> Result<Vec<f64>, IncrementalError> {
        if p.len() != self.dim {
            return Err(IncrementalError::DimensionMismatch {
                expected: self.dim,
                got: p.len(),
            });
        }

        let mut x = p.to_vec();
        Self::lu_solve(&self.lu_factors, &self.pivots, &mut x, self.dim);
        Ok(x)
    }

    /// Solve (B' + ΔB') × θ = P using Woodbury formula.
    ///
    /// Uses: `(A + ucvᵀ)⁻¹b = A⁻¹b - A⁻¹u(c⁻¹ + vᵀA⁻¹u)⁻¹vᵀA⁻¹b`
    pub fn solve_with_update(
        &self,
        p: &[f64],
        update: &WoodburyUpdate,
    ) -> Result<Vec<f64>, IncrementalError> {
        if p.len() != self.dim {
            return Err(IncrementalError::DimensionMismatch {
                expected: self.dim,
                got: p.len(),
            });
        }

        // Step 1: Compute A⁻¹b (base solution)
        let a_inv_b = self.solve(p)?;

        // Step 2: Compute A⁻¹u
        let u_dense: Vec<f64> = (0..self.dim)
            .map(|i| update.u.get(i).copied().unwrap_or(0.0))
            .collect();
        let a_inv_u = self.solve(&u_dense)?;

        // Step 3: Compute vᵀA⁻¹b (scalar)
        let v_t_a_inv_b: f64 = (0..self.dim)
            .map(|i| update.v.get(i).copied().unwrap_or(0.0) * a_inv_b[i])
            .sum();

        // Step 4: Compute vᵀA⁻¹u (scalar)
        let v_t_a_inv_u: f64 = (0..self.dim)
            .map(|i| update.v.get(i).copied().unwrap_or(0.0) * a_inv_u[i])
            .sum();

        // Step 5: Compute (c⁻¹ + vᵀA⁻¹u)⁻¹
        let c_inv = if update.c.abs() > 1e-12 {
            1.0 / update.c
        } else {
            return Err(IncrementalError::SingularUpdate);
        };

        let denominator = c_inv + v_t_a_inv_u;
        if denominator.abs() < 1e-12 {
            return Err(IncrementalError::SingularUpdate);
        }

        let woodbury_scalar = v_t_a_inv_b / denominator;

        // Step 6: Compute result = A⁻¹b - woodbury_scalar × A⁻¹u
        let result: Vec<f64> = a_inv_b
            .iter()
            .zip(a_inv_u.iter())
            .map(|(&ab, &au)| ab - woodbury_scalar * au)
            .collect();

        Ok(result)
    }

    /// Get the dimension of the reduced system.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// LU factorization with partial pivoting (in-place).
    fn lu_factorize(matrix: &[f64], n: usize) -> Result<(Vec<f64>, Vec<usize>), String> {
        let mut lu = matrix.to_vec();
        let mut pivots = vec![0usize; n];

        for k in 0..n {
            // Find pivot
            let mut max_val = lu[k * n + k].abs();
            let mut max_idx = k;

            for i in (k + 1)..n {
                let val = lu[i * n + k].abs();
                if val > max_val {
                    max_val = val;
                    max_idx = i;
                }
            }

            if max_val < 1e-14 {
                return Err(format!("Singular matrix at column {}", k));
            }

            pivots[k] = max_idx;

            // Swap rows if needed
            if max_idx != k {
                for j in 0..n {
                    lu.swap(k * n + j, max_idx * n + j);
                }
            }

            // Elimination
            let pivot = lu[k * n + k];
            for i in (k + 1)..n {
                let factor = lu[i * n + k] / pivot;
                lu[i * n + k] = factor; // Store L factor

                for j in (k + 1)..n {
                    lu[i * n + j] -= factor * lu[k * n + j];
                }
            }
        }

        Ok((lu, pivots))
    }

    /// Solve using LU factors (in-place).
    fn lu_solve(lu: &[f64], pivots: &[usize], b: &mut [f64], n: usize) {
        // Apply row permutations
        for k in 0..n {
            if pivots[k] != k {
                b.swap(k, pivots[k]);
            }
        }

        // Forward substitution (L × y = b)
        for i in 1..n {
            for j in 0..i {
                b[i] -= lu[i * n + j] * b[j];
            }
        }

        // Back substitution (U × x = y)
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                b[i] -= lu[i * n + j] * b[j];
            }
            b[i] /= lu[i * n + i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};

    fn create_3bus_network() -> Network {
        let mut network = Network::new();

        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));
        let b3 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));

        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                reactance: 0.1,
                status: true,
                ..Default::default()
            }),
        );
        network.graph.add_edge(
            b2,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                name: "Line2-3".to_string(),
                from_bus: BusId::new(2),
                to_bus: BusId::new(3),
                reactance: 0.1,
                status: true,
                ..Default::default()
            }),
        );
        network.graph.add_edge(
            b1,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                name: "Line1-3".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                reactance: 0.2,
                status: true,
                ..Default::default()
            }),
        );

        network
    }

    #[test]
    fn test_solver_creation() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();
        let solver = IncrementalSolver::new(&b_prime).unwrap();

        assert_eq!(solver.dim(), 2); // 3 buses - 1 slack
    }

    #[test]
    fn test_base_case_solve() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();
        let solver = IncrementalSolver::new(&b_prime).unwrap();

        // Inject 1 pu at bus 2, withdraw at bus 3 (bus 1 is slack)
        let p = vec![1.0, -1.0];
        let theta = solver.solve(&p).unwrap();

        assert_eq!(theta.len(), 2);
        // Angles should be non-zero
        assert!(theta[0].abs() > 1e-10 || theta[1].abs() > 1e-10);
    }

    #[test]
    fn test_woodbury_update_creation() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();

        let update = WoodburyUpdate::branch_outage(&b_prime, BranchId::new(1)).unwrap();

        assert!(update.c.abs() > 0.0); // Non-zero susceptance
        assert!(update.u.nnz() > 0); // Non-empty vectors
        assert!(update.v.nnz() > 0);
    }

    #[test]
    fn test_woodbury_solve() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();
        let solver = IncrementalSolver::new(&b_prime).unwrap();

        let p = vec![1.0, -1.0];

        // Base case
        let theta_base = solver.solve(&p).unwrap();

        // With branch outage
        let update = WoodburyUpdate::branch_outage(&b_prime, BranchId::new(2)).unwrap();
        let theta_outage = solver.solve_with_update(&p, &update).unwrap();

        // Solutions should differ
        let diff: f64 = theta_base
            .iter()
            .zip(theta_outage.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();

        assert!(diff > 1e-10, "Outage should change the solution");
    }
}
