//! # Analytical Hessian for AC-OPF
//!
//! This module computes the Hessian of the Lagrangian for IPOPT's second-order
//! Newton convergence. Without an analytical Hessian, IPOPT falls back to BFGS
//! approximation which is slower (first-order convergence).
//!
//! ## Mathematical Background
//!
//! The Hessian of the Lagrangian is:
//!
//! ```text
//! H(x, λ) = σ · ∇²f(x) + Σᵢ λᵢ · ∇²gᵢ(x)
//!
//! where:
//!   σ = objective factor (scaling)
//!   f(x) = objective function (generation cost)
//!   gᵢ(x) = constraint functions (power balance)
//!   λᵢ = Lagrange multipliers (shadow prices / LMPs)
//! ```
//!
//! ## Structure of AC-OPF Hessian
//!
//! The variable vector is: x = [V | θ | P_g | Q_g]
//!
//! ```text
//!              │  V        │  θ        │  P_g      │  Q_g
//!     ─────────┼───────────┼───────────┼───────────┼───────────
//!       V      │ H_VV      │ H_Vθ      │ 0         │ 0
//!       θ      │ H_θV      │ H_θθ      │ 0         │ 0
//!       P_g    │ 0         │ 0         │ H_PP      │ 0
//!       Q_g    │ 0         │ 0         │ 0         │ 0
//! ```
//!
//! - **H_PP** comes from quadratic objective (diagonal: 2·c₂·S_base²)
//! - **H_VV, H_Vθ, H_θθ** come from power balance constraint Hessians
//!
//! ## Power Balance Constraint Hessians
//!
//! The power balance equations are:
//!
//! ```text
//! P_i = Σⱼ V_i V_j [G_ij cos(θ_ij) + B_ij sin(θ_ij)]
//! Q_i = Σⱼ V_i V_j [G_ij sin(θ_ij) - B_ij cos(θ_ij)]
//! ```
//!
//! Second derivatives involve:
//! - ∂²P/∂V_i∂V_j = G_ij cos(θ_ij) + B_ij sin(θ_ij)
//! - ∂²P/∂θ_i∂θ_j = -V_i V_j [G_ij cos(θ_ij) + B_ij sin(θ_ij)]  (i≠j)
//! - ∂²P/∂V_i∂θ_j = V_i [G_ij sin(θ_ij) - B_ij cos(θ_ij)]       (i≠j)
//!
//! ## Sparsity Pattern
//!
//! The Hessian is sparse because:
//! 1. Y-bus is sparse (only connected buses interact)
//! 2. Generator variables are decoupled from network variables
//! 3. P_g and Q_g don't interact (in the objective)
//!
//! For a network with n buses and m generators:
//! - Dense Hessian: O((2n + 2m)²)
//! - Sparse Hessian: O(nnz(Y-bus) + m)

use super::{AcOpfProblem, YBus};

/// Compute the sparsity pattern of the Hessian (lower triangular).
///
/// IPOPT requires only the lower triangular part of the symmetric Hessian.
/// Returns (row_indices, col_indices) for non-zero entries.
///
/// # Arguments
/// * `problem` - AC-OPF problem (provides dimensions and Y-bus structure)
///
/// # Returns
/// Tuple of (row indices, column indices) for lower triangular non-zeros
pub fn hessian_sparsity(problem: &AcOpfProblem) -> (Vec<usize>, Vec<usize>) {
    let n_bus = problem.n_bus;
    let n_gen = problem.n_gen;
    let _n_var = problem.n_var;

    let mut rows = Vec::new();
    let mut cols = Vec::new();

    // ========================================================================
    // BLOCK 1: V-V interactions (from power balance Hessians)
    // ========================================================================
    // ∂²g/∂V_i∂V_j exists if Y_ij ≠ 0 (buses are connected)
    // We use dense pattern for simplicity; sparse would use Y-bus structure

    for i in 0..n_bus {
        for j in 0..=i {
            // Lower triangular
            rows.push(problem.v_offset + i);
            cols.push(problem.v_offset + j);
        }
    }

    // ========================================================================
    // BLOCK 2: θ-V interactions (from power balance Hessians)
    // ========================================================================
    // ∂²g/∂θ_i∂V_j exists if Y_ij ≠ 0

    for i in 0..n_bus {
        for j in 0..n_bus {
            // θ comes after V in the variable vector
            let row = problem.theta_offset + i;
            let col = problem.v_offset + j;
            // Lower triangular: row >= col
            if row >= col {
                rows.push(row);
                cols.push(col);
            }
        }
    }

    // ========================================================================
    // BLOCK 3: θ-θ interactions (from power balance Hessians)
    // ========================================================================
    // ∂²g/∂θ_i∂θ_j exists if Y_ij ≠ 0

    for i in 0..n_bus {
        for j in 0..=i {
            // Lower triangular
            rows.push(problem.theta_offset + i);
            cols.push(problem.theta_offset + j);
        }
    }

    // ========================================================================
    // BLOCK 4: P_g diagonal (from quadratic objective)
    // ========================================================================
    // ∂²f/∂P_g² = 2·c₂·S_base² (diagonal only)

    for i in 0..n_gen {
        rows.push(problem.pg_offset + i);
        cols.push(problem.pg_offset + i);
    }

    // Q_g has no Hessian contribution (not in objective with quadratic terms)

    (rows, cols)
}

/// Compute the Hessian values at the given point.
///
/// Computes H(x, λ) = σ·∇²f(x) + Σᵢ λᵢ·∇²gᵢ(x)
///
/// # Arguments
/// * `problem` - AC-OPF problem definition
/// * `x` - Current variable values
/// * `obj_factor` - Scaling factor σ for objective Hessian
/// * `lambda` - Lagrange multipliers for constraints
///
/// # Returns
/// Hessian values in the same order as sparsity pattern (lower triangular)
pub fn hessian_values(
    problem: &AcOpfProblem,
    x: &[f64],
    obj_factor: f64,
    lambda: &[f64],
) -> Vec<f64> {
    let n_bus = problem.n_bus;
    let n_gen = problem.n_gen;

    // Pre-compute total number of entries from sparsity pattern
    // V-V: n_bus*(n_bus+1)/2
    // θ-V: n_bus*n_bus (but only lower triangular counted)
    // θ-θ: n_bus*(n_bus+1)/2
    // P_g: n_gen
    let n_vv = n_bus * (n_bus + 1) / 2;
    let n_theta_v = n_bus * n_bus; // Will be filtered to lower triangular
    let n_theta_theta = n_bus * (n_bus + 1) / 2;
    let n_pg = n_gen;

    let nnz_estimate = n_vv + n_theta_v + n_theta_theta + n_pg;
    let mut vals = vec![0.0; nnz_estimate];

    // Extract voltages and angles
    let (v, theta) = problem.extract_v_theta(x);

    // ========================================================================
    // CONSTRAINT HESSIANS (weighted by λ)
    // ========================================================================
    //
    // λ layout: [λ_P (n_bus) | λ_Q (n_bus) | λ_ref (1)]
    //
    // For each bus i, λ_P[i] and λ_Q[i] are the shadow prices (LMPs) for
    // real and reactive power balance respectively.

    compute_power_balance_hessian(
        &problem.ybus,
        &v,
        &theta,
        lambda,
        n_bus,
        &mut vals,
        problem.v_offset,
        problem.theta_offset,
    );

    // ========================================================================
    // OBJECTIVE HESSIAN (scaled by obj_factor)
    // ========================================================================
    //
    // For quadratic cost: f(P_g) = c₀ + c₁·P_g + c₂·P_g²
    // ∂²f/∂P_g² = 2·c₂
    //
    // But P_g is in per-unit, so we need to scale:
    // If P_MW = P_pu · S_base, then ∂f/∂P_pu = ∂f/∂P_MW · S_base
    // And ∂²f/∂P_pu² = ∂²f/∂P_MW² · S_base²

    let pg_start = n_vv + count_lower_triangular(n_bus, n_bus) + n_theta_theta;
    let s_base_sq = problem.base_mva * problem.base_mva;

    for (i, gen) in problem.generators.iter().enumerate() {
        // Get quadratic coefficient
        let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);

        // ∂²f/∂P_g² = 2·c₂·S_base²
        vals[pg_start + i] += obj_factor * 2.0 * c2 * s_base_sq;
    }

    vals.truncate(compute_actual_nnz(n_bus, n_gen));
    vals
}

/// Compute the power balance constraint Hessians.
///
/// This computes the second derivatives of the power balance equations:
/// - ∂²P_i/∂x∂y for all bus pairs
/// - ∂²Q_i/∂x∂y for all bus pairs
///
/// Weighted by the Lagrange multipliers λ_P and λ_Q.
fn compute_power_balance_hessian(
    ybus: &YBus,
    v: &[f64],
    theta: &[f64],
    lambda: &[f64],
    n_bus: usize,
    vals: &mut [f64],
    v_offset: usize,
    theta_offset: usize,
) {
    // λ layout: [λ_P (n_bus) | λ_Q (n_bus) | λ_ref (1)]
    let lambda_p = &lambda[0..n_bus];
    let lambda_q = &lambda[n_bus..2 * n_bus];

    // ========================================================================
    // V-V BLOCK: ∂²g/∂V_i∂V_j
    // ========================================================================
    //
    // ∂²P_k/∂V_i∂V_j:
    //   - If i = j = k: 2·G_kk
    //   - If i = k ≠ j or j = k ≠ i: G_kj·cos(θ_kj) + B_kj·sin(θ_kj)
    //   - If i ≠ k, j ≠ k: 0
    //
    // Similar for Q with appropriate sign changes.

    let mut idx = 0;
    for i in 0..n_bus {
        for j in 0..=i {
            // Accumulate contributions from all constraint Hessians
            let mut h_vv = 0.0;

            // Contribution from P_k balance equation
            for k in 0..n_bus {
                h_vv += lambda_p[k] * d2p_dvi_dvj(ybus, v, theta, k, i, j);
                h_vv += lambda_q[k] * d2q_dvi_dvj(ybus, v, theta, k, i, j);
            }

            vals[idx] = h_vv;
            idx += 1;
        }
    }

    // ========================================================================
    // θ-V BLOCK: ∂²g/∂θ_i∂V_j
    // ========================================================================

    for i in 0..n_bus {
        for j in 0..n_bus {
            let row = theta_offset + i;
            let col = v_offset + j;
            if row >= col {
                let mut h_theta_v = 0.0;

                for k in 0..n_bus {
                    h_theta_v += lambda_p[k] * d2p_dtheta_i_dv_j(ybus, v, theta, k, i, j);
                    h_theta_v += lambda_q[k] * d2q_dtheta_i_dv_j(ybus, v, theta, k, i, j);
                }

                vals[idx] = h_theta_v;
                idx += 1;
            }
        }
    }

    // ========================================================================
    // θ-θ BLOCK: ∂²g/∂θ_i∂θ_j
    // ========================================================================

    for i in 0..n_bus {
        for j in 0..=i {
            let mut h_theta_theta = 0.0;

            for k in 0..n_bus {
                h_theta_theta += lambda_p[k] * d2p_dtheta_i_dtheta_j(ybus, v, theta, k, i, j);
                h_theta_theta += lambda_q[k] * d2q_dtheta_i_dtheta_j(ybus, v, theta, k, i, j);
            }

            vals[idx] = h_theta_theta;
            idx += 1;
        }
    }
}

// ============================================================================
// SECOND DERIVATIVES OF POWER BALANCE EQUATIONS
// ============================================================================
//
// These implement the analytical second derivatives of:
//   P_k = Σⱼ V_k V_j [G_kj cos(θ_k - θ_j) + B_kj sin(θ_k - θ_j)]
//   Q_k = Σⱼ V_k V_j [G_kj sin(θ_k - θ_j) - B_kj cos(θ_k - θ_j)]

/// ∂²P_k/∂V_i∂V_j
fn d2p_dvi_dvj(ybus: &YBus, _v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    if k != i && k != j {
        return 0.0;
    }

    if i == j {
        // Diagonal case: ∂²P_k/∂V_k² = 2·G_kk
        if i == k {
            return 2.0 * ybus.g(k, k);
        }
        return 0.0;
    }

    // Off-diagonal case where k == i or k == j
    let (m, n) = if k == i { (i, j) } else { (j, i) };
    let theta_mn = theta[m] - theta[n];
    ybus.g(m, n) * theta_mn.cos() + ybus.b(m, n) * theta_mn.sin()
}

/// ∂²Q_k/∂V_i∂V_j
fn d2q_dvi_dvj(ybus: &YBus, _v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    if k != i && k != j {
        return 0.0;
    }

    if i == j {
        // Diagonal case: ∂²Q_k/∂V_k² = -2·B_kk
        if i == k {
            return -2.0 * ybus.b(k, k);
        }
        return 0.0;
    }

    // Off-diagonal case where k == i or k == j
    let (m, n) = if k == i { (i, j) } else { (j, i) };
    let theta_mn = theta[m] - theta[n];
    ybus.g(m, n) * theta_mn.sin() - ybus.b(m, n) * theta_mn.cos()
}

/// ∂²P_k/∂θ_i∂V_j
fn d2p_dtheta_i_dv_j(ybus: &YBus, v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    // ∂P_k/∂θ_i = V_k Σₘ V_m [-G_km sin(θ_k-θ_m) + B_km cos(θ_k-θ_m)] · ∂(θ_k-θ_m)/∂θ_i
    //
    // The second derivative ∂²P_k/∂θ_i∂V_j involves:
    // - When j = k: derivative w.r.t. V_k in the first term
    // - When j ≠ k: derivative w.r.t. V_j in the sum

    if i == k {
        // ∂/∂V_j of [V_k Σₘ V_m (...)]
        if j == k {
            // Summing over all m: Σₘ V_m [-G_km sin(θ_k-θ_m) + B_km cos(θ_k-θ_m)]
            let mut sum = 0.0;
            for m in 0..v.len() {
                if m != k {
                    let theta_km = theta[k] - theta[m];
                    sum += v[m] * (-ybus.g(k, m) * theta_km.sin() + ybus.b(k, m) * theta_km.cos());
                }
            }
            return sum;
        } else {
            // j ≠ k: V_k · [-G_kj sin(θ_k-θ_j) + B_kj cos(θ_k-θ_j)]
            let theta_kj = theta[k] - theta[j];
            return v[k] * (-ybus.g(k, j) * theta_kj.sin() + ybus.b(k, j) * theta_kj.cos());
        }
    } else {
        // i ≠ k: The derivative picks out the term with m = i
        // ∂P_k/∂θ_i = V_k V_i [G_ki sin(θ_k-θ_i) - B_ki cos(θ_k-θ_i)]
        // Then ∂/∂V_j of this:
        if j == k {
            let theta_ki = theta[k] - theta[i];
            return v[i] * (ybus.g(k, i) * theta_ki.sin() - ybus.b(k, i) * theta_ki.cos());
        } else if j == i {
            let theta_ki = theta[k] - theta[i];
            return v[k] * (ybus.g(k, i) * theta_ki.sin() - ybus.b(k, i) * theta_ki.cos());
        }
    }

    0.0
}

/// ∂²Q_k/∂θ_i∂V_j
fn d2q_dtheta_i_dv_j(ybus: &YBus, v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    // Similar structure to P, with appropriate sign changes for Q formula

    if i == k {
        if j == k {
            let mut sum = 0.0;
            for m in 0..v.len() {
                if m != k {
                    let theta_km = theta[k] - theta[m];
                    sum += v[m] * (ybus.g(k, m) * theta_km.cos() + ybus.b(k, m) * theta_km.sin());
                }
            }
            return sum;
        } else {
            let theta_kj = theta[k] - theta[j];
            return v[k] * (ybus.g(k, j) * theta_kj.cos() + ybus.b(k, j) * theta_kj.sin());
        }
    } else {
        if j == k {
            let theta_ki = theta[k] - theta[i];
            return v[i] * (-ybus.g(k, i) * theta_ki.cos() - ybus.b(k, i) * theta_ki.sin());
        } else if j == i {
            let theta_ki = theta[k] - theta[i];
            return v[k] * (-ybus.g(k, i) * theta_ki.cos() - ybus.b(k, i) * theta_ki.sin());
        }
    }

    0.0
}

/// ∂²P_k/∂θ_i∂θ_j
fn d2p_dtheta_i_dtheta_j(ybus: &YBus, v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    // ∂P_k/∂θ_k = V_k Σₘ≠k V_m [-G_km sin(θ_k-θ_m) + B_km cos(θ_k-θ_m)]
    // ∂P_k/∂θ_i (i≠k) = V_k V_i [G_ki sin(θ_k-θ_i) - B_ki cos(θ_k-θ_i)]

    let v_k = v[k];

    if i == k && j == k {
        // Diagonal: ∂²P_k/∂θ_k² = V_k Σₘ≠k V_m [-G_km cos(θ_k-θ_m) - B_km sin(θ_k-θ_m)]
        let mut sum = 0.0;
        for m in 0..v.len() {
            if m != k {
                let theta_km = theta[k] - theta[m];
                sum += v[m] * (-ybus.g(k, m) * theta_km.cos() - ybus.b(k, m) * theta_km.sin());
            }
        }
        return v_k * sum;
    }

    if i == k && j != k {
        // ∂²P_k/∂θ_k∂θ_j = V_k V_j [G_kj cos(θ_k-θ_j) + B_kj sin(θ_k-θ_j)]
        let theta_kj = theta[k] - theta[j];
        return v_k * v[j] * (ybus.g(k, j) * theta_kj.cos() + ybus.b(k, j) * theta_kj.sin());
    }

    if i != k && j == k {
        // Symmetric to above
        let theta_ki = theta[k] - theta[i];
        return v_k * v[i] * (ybus.g(k, i) * theta_ki.cos() + ybus.b(k, i) * theta_ki.sin());
    }

    if i == j && i != k {
        // ∂²P_k/∂θ_i² = -V_k V_i [G_ki cos(θ_k-θ_i) + B_ki sin(θ_k-θ_i)]
        let theta_ki = theta[k] - theta[i];
        return -v_k * v[i] * (ybus.g(k, i) * theta_ki.cos() + ybus.b(k, i) * theta_ki.sin());
    }

    // Off-diagonal with i ≠ k, j ≠ k, i ≠ j: zero (no interaction)
    0.0
}

/// ∂²Q_k/∂θ_i∂θ_j
fn d2q_dtheta_i_dtheta_j(ybus: &YBus, v: &[f64], theta: &[f64], k: usize, i: usize, j: usize) -> f64 {
    // Similar structure to P with appropriate sign changes

    let v_k = v[k];

    if i == k && j == k {
        let mut sum = 0.0;
        for m in 0..v.len() {
            if m != k {
                let theta_km = theta[k] - theta[m];
                sum += v[m] * (-ybus.g(k, m) * theta_km.sin() + ybus.b(k, m) * theta_km.cos());
            }
        }
        return v_k * sum;
    }

    if i == k && j != k {
        let theta_kj = theta[k] - theta[j];
        return v_k * v[j] * (ybus.g(k, j) * theta_kj.sin() - ybus.b(k, j) * theta_kj.cos());
    }

    if i != k && j == k {
        let theta_ki = theta[k] - theta[i];
        return v_k * v[i] * (ybus.g(k, i) * theta_ki.sin() - ybus.b(k, i) * theta_ki.cos());
    }

    if i == j && i != k {
        let theta_ki = theta[k] - theta[i];
        return -v_k * v[i] * (ybus.g(k, i) * theta_ki.sin() - ybus.b(k, i) * theta_ki.cos());
    }

    0.0
}

/// Count number of lower triangular entries for θ-V block.
fn count_lower_triangular(n_rows: usize, n_cols: usize) -> usize {
    // θ indices are [n_cols, n_cols + n_rows)
    // V indices are [0, n_cols)
    // Count pairs where θ_i >= V_j, i.e., (n_cols + i) >= j
    // Since n_cols + i >= n_cols > any j in [0, n_cols), all pairs are valid
    n_rows * n_cols
}

/// Compute actual number of non-zeros (for truncation).
fn compute_actual_nnz(n_bus: usize, n_gen: usize) -> usize {
    let n_vv = n_bus * (n_bus + 1) / 2;
    let n_theta_v = n_bus * n_bus;
    let n_theta_theta = n_bus * (n_bus + 1) / 2;
    let n_pg = n_gen;
    n_vv + n_theta_v + n_theta_theta + n_pg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hessian_sparsity_pattern_dimensions() {
        use gat_core::{Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Network, Node, CostModel};

        // Create minimal 2-bus, 1-gen network
        let mut network = Network::new();
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            ..Bus::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            pmax_mw: 100.0,
            cost_model: CostModel::quadratic(0.0, 20.0, 0.01),
            ..Gen::default()
        }));
        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                status: true,
                ..Branch::default()
            }),
        );

        let problem = AcOpfProblem::from_network(&network).unwrap();
        let (rows, cols) = hessian_sparsity(&problem);

        // Check dimensions
        assert_eq!(rows.len(), cols.len());

        // n_bus = 2, n_gen = 1
        // V-V block: 2*(2+1)/2 = 3 entries
        // θ-V block: 2*2 = 4 entries
        // θ-θ block: 2*(2+1)/2 = 3 entries
        // P_g block: 1 entry
        // Total: 3 + 4 + 3 + 1 = 11 entries
        assert_eq!(rows.len(), 11);

        // All entries should be in lower triangular (row >= col)
        for (r, c) in rows.iter().zip(cols.iter()) {
            assert!(r >= c, "Entry ({}, {}) is not in lower triangular", r, c);
        }
    }

    #[test]
    fn test_objective_hessian_diagonal() {
        use gat_core::{Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Network, Node, CostModel};

        // Create network with quadratic cost
        let mut network = Network::new();
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            ..Bus::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            pmax_mw: 100.0,
            cost_model: CostModel::quadratic(0.0, 20.0, 0.05), // c2 = 0.05
            ..Gen::default()
        }));
        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                status: true,
                ..Branch::default()
            }),
        );

        let problem = AcOpfProblem::from_network(&network).unwrap();
        let x0 = problem.initial_point();

        // Zero lambda = no constraint Hessian contribution
        let lambda = vec![0.0; 2 * problem.n_bus + 1];

        let vals = hessian_values(&problem, &x0, 1.0, &lambda);

        // The P_g Hessian entry should be 2 * c2 * S_base^2
        // c2 = 0.05, S_base = 100
        // Expected: 2 * 0.05 * 100^2 = 1000
        let pg_idx = compute_actual_nnz(problem.n_bus, problem.n_gen) - 1;
        assert!(
            (vals[pg_idx] - 1000.0).abs() < 1.0,
            "P_g Hessian entry: expected ~1000, got {}",
            vals[pg_idx]
        );
    }
}
