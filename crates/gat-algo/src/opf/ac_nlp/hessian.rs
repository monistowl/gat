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

use super::{AcOpfProblem, BranchData, BusData, YBus};

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

    // ========================================================================
    // BLOCK 5: Thermal constraint Hessians (already covered by dense pattern)
    // ========================================================================
    // For each thermally-constrained branch (i -> j), the flow constraint is:
    //   h(x) = P_ij² + Q_ij² - S_max²
    //
    // The Hessian involves 4 variables: V_i, V_j, θ_i, θ_j
    // Lower triangular entries (10 per constraint):
    //   (V_i, V_i), (V_j, V_i), (V_j, V_j)
    //   (θ_i, V_i), (θ_i, V_j), (θ_j, V_i), (θ_j, V_j)
    //   (θ_i, θ_i), (θ_j, θ_i), (θ_j, θ_j)
    //
    // Since we use dense patterns for V-V, θ-V, and θ-θ blocks (BLOCK 1-3),
    // all thermal constraint entries are already included. The thermal Hessian
    // values are accumulated in hessian_values() at the same positions as the
    // power balance Hessian values.

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
    // λ layout: [λ_P (n_bus) | λ_Q (n_bus) | λ_ref (1) | λ_thermal (2 per constrained branch)]
    //
    // For each bus i, λ_P[i] and λ_Q[i] are the shadow prices (LMPs) for
    // real and reactive power balance respectively.
    // λ_thermal contains the Lagrange multipliers for thermal limit constraints.

    compute_power_balance_hessian(
        &problem.ybus,
        &problem.buses,
        &v,
        &theta,
        lambda,
        n_bus,
        &mut vals,
        problem.v_offset,
        problem.theta_offset,
    );

    // ========================================================================
    // THERMAL CONSTRAINT HESSIANS (weighted by λ_thermal)
    // ========================================================================
    //
    // For h = P² + Q² - S²_max:
    // ∂²h/∂x∂y = 2·(∂P/∂x·∂P/∂y + ∂Q/∂x·∂Q/∂y) + 2P·∂²P/∂x∂y + 2Q·∂²Q/∂x∂y
    //
    // The thermal constraint Hessian contributes to V-V, θ-V, θ-θ blocks.

    let lambda_thermal_start = 2 * n_bus + 1; // After equality constraints
    compute_thermal_hessian(
        &problem.branches,
        &v,
        &theta,
        lambda,
        lambda_thermal_start,
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
///
/// IMPORTANT: Includes shunt contributions! The constraints include:
/// - P: gs_pu * V² term → ∂²(gs*V²)/∂V² = 2*gs (diagonal only)
/// - Q: -bs_pu * V² term → ∂²(-bs*V²)/∂V² = -2*bs (diagonal only)
#[allow(clippy::too_many_arguments)]
fn compute_power_balance_hessian(
    ybus: &YBus,
    buses: &[BusData],
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
    //   - If i = j = k: 2·G_kk + 2·gs_pu (shunt contribution!)
    //   - If i = k ≠ j or j = k ≠ i: G_kj·cos(θ_kj) + B_kj·sin(θ_kj)
    //   - If i ≠ k, j ≠ k: 0
    //
    // ∂²Q_k/∂V_i∂V_j:
    //   - If i = j = k: -2·B_kk - 2·bs_pu (shunt contribution!)
    //   - Similar structure otherwise
    //
    // Similar for Q with appropriate sign changes.

    let mut idx = 0;
    for i in 0..n_bus {
        for j in 0..=i {
            // Accumulate contributions from all constraint Hessians
            let mut h_vv = 0.0;

            // Contribution from P_k and Q_k balance equations
            for k in 0..n_bus {
                h_vv += lambda_p[k] * d2p_dvi_dvj(ybus, v, theta, k, i, j);
                h_vv += lambda_q[k] * d2q_dvi_dvj(ybus, v, theta, k, i, j);
            }

            // Add shunt contributions on diagonal (i == j)
            // P constraint: gs_pu * V² → ∂²/∂V² = 2*gs_pu
            // Q constraint: -bs_pu * V² → ∂²/∂V² = -2*bs_pu
            if i == j {
                h_vv += lambda_p[i] * 2.0 * buses[i].gs_pu;
                h_vv += lambda_q[i] * (-2.0 * buses[i].bs_pu);
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

/// Compute the thermal constraint Hessians and add to the appropriate blocks.
///
/// For constraint h = P² + Q² - S²_max:
/// ∂²h/∂x∂y = 2·(∂P/∂x·∂P/∂y + ∂Q/∂x·∂Q/∂y) + 2P·∂²P/∂x∂y + 2Q·∂²Q/∂x∂y
///
/// This function adds thermal constraint Hessian contributions (weighted by λ)
/// to the existing V-V, θ-V, θ-θ blocks in the vals array.
#[allow(clippy::too_many_arguments)]
fn compute_thermal_hessian(
    branches: &[BranchData],
    v: &[f64],
    theta: &[f64],
    lambda: &[f64],
    lambda_start: usize,
    n_bus: usize,
    vals: &mut [f64],
    _v_offset: usize,
    _theta_offset: usize,
) {
    let mut lambda_idx = lambda_start;

    for branch in branches {
        if branch.rate_mva <= 0.0 {
            continue;
        }

        let i = branch.from_idx;
        let j = branch.to_idx;
        let vi = v[i];
        let vj = v[j];
        let theta_i = theta[i];
        let theta_j = theta[j];

        // From side constraint: h = P²_ij + Q²_ij - S²_max
        {
            let lambda_from = lambda[lambda_idx];
            lambda_idx += 1;

            // Skip if multiplier is zero (constraint not active)
            if lambda_from.abs() > 1e-12 {
                // Get branch flows and gradients
                let (p, q) = branch_flow_from(branch, vi, vj, theta_i, theta_j);
                let dp = branch_grad_p_from(branch, vi, vj, theta_i, theta_j);
                let dq = branch_grad_q_from(branch, vi, vj, theta_i, theta_j);
                let d2p = branch_hess_p_from(branch, vi, vj, theta_i, theta_j);
                let d2q = branch_hess_q_from(branch, vi, vj, theta_i, theta_j);

                // Add contributions to Hessian blocks
                add_thermal_hessian_contributions(
                    lambda_from, p, q, &dp, &dq, &d2p, &d2q,
                    i, j, n_bus, vals,
                );
            }
        }

        // To side constraint: h = P²_ji + Q²_ji - S²_max
        {
            let lambda_to = lambda[lambda_idx];
            lambda_idx += 1;

            if lambda_to.abs() > 1e-12 {
                let (p, q) = branch_flow_to(branch, vi, vj, theta_i, theta_j);
                let dp = branch_grad_p_to(branch, vi, vj, theta_i, theta_j);
                let dq = branch_grad_q_to(branch, vi, vj, theta_i, theta_j);
                let d2p = branch_hess_p_to(branch, vi, vj, theta_i, theta_j);
                let d2q = branch_hess_q_to(branch, vi, vj, theta_i, theta_j);

                add_thermal_hessian_contributions(
                    lambda_to, p, q, &dp, &dq, &d2p, &d2q,
                    i, j, n_bus, vals,
                );
            }
        }
    }
}

/// Add thermal constraint Hessian contributions to the appropriate blocks.
///
/// The Hessian entry (row, col) maps to:
/// - V-V block: lower triangular with offset 0
/// - θ-V block: starts after V-V
/// - θ-θ block: lower triangular after θ-V
fn add_thermal_hessian_contributions(
    lambda: f64,
    p: f64,
    q: f64,
    dp: &[f64; 4],  // [dvi, dvj, dti, dtj]
    dq: &[f64; 4],
    d2p: &[f64; 10], // Lower triangular: [vi_vi, vj_vi, vj_vj, ti_vi, ti_vj, ti_ti, tj_vi, tj_vj, tj_ti, tj_tj]
    d2q: &[f64; 10],
    bus_i: usize,
    bus_j: usize,
    n_bus: usize,
    vals: &mut [f64],
) {
    // Compute the 10 unique Hessian entries for this constraint
    // ∂²h/∂x∂y = 2·(∂P/∂x·∂P/∂y + ∂Q/∂x·∂Q/∂y) + 2P·∂²P/∂x∂y + 2Q·∂²Q/∂x∂y

    // Gradient indices: 0=Vi, 1=Vj, 2=θi, 3=θj
    // We compute the 4x4 lower triangular part

    // Variable indices in the full problem
    let vars = [bus_i, bus_j, n_bus + bus_i, n_bus + bus_j]; // [Vi, Vj, θi, θj]

    // Hessian entry indices (lower triangular within the 4 variables)
    // (0,0), (1,0), (1,1), (2,0), (2,1), (2,2), (3,0), (3,1), (3,2), (3,3)
    let hess_pairs = [
        (0, 0), (1, 0), (1, 1), (2, 0), (2, 1), (2, 2), (3, 0), (3, 1), (3, 2), (3, 3),
    ];

    for (k, &(r, c)) in hess_pairs.iter().enumerate() {
        // Compute ∂²h/∂x_r∂x_c
        let grad_term = 2.0 * (dp[r] * dp[c] + dq[r] * dq[c]);
        let hess_term = 2.0 * (p * d2p[k] + q * d2q[k]);
        let h_val = lambda * (grad_term + hess_term);

        if h_val.abs() < 1e-15 {
            continue;
        }

        // Map to global row/col indices
        let global_row = vars[r];
        let global_col = vars[c];

        // Ensure lower triangular
        let (row, col) = if global_row >= global_col {
            (global_row, global_col)
        } else {
            (global_col, global_row)
        };

        // Find the index in the vals array
        // V-V block: row, col both in [0, n_bus)
        // θ-V block: row in [n_bus, 2*n_bus), col in [0, n_bus)
        // θ-θ block: row, col both in [n_bus, 2*n_bus)

        let idx = if row < n_bus {
            // V-V block: lower triangular, row*(row+1)/2 + col
            row * (row + 1) / 2 + col
        } else if col < n_bus {
            // θ-V block: dense, after V-V block
            let n_vv = n_bus * (n_bus + 1) / 2;
            let theta_row = row - n_bus;
            n_vv + theta_row * n_bus + col
        } else {
            // θ-θ block: lower triangular, after V-V and θ-V blocks
            let n_vv = n_bus * (n_bus + 1) / 2;
            let n_theta_v = n_bus * n_bus;
            let theta_row = row - n_bus;
            let theta_col = col - n_bus;
            n_vv + n_theta_v + theta_row * (theta_row + 1) / 2 + theta_col
        };

        vals[idx] += h_val;
    }
}

// ============================================================================
// BRANCH FLOW FUNCTIONS FOR HESSIAN COMPUTATION
// ============================================================================

/// Branch flow (from side) - same as jacobian.rs
fn branch_flow_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_i - theta_j - branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let p = (vi * vi / a_sq) * g - (vi * vj / a) * (g * cos_d + b * sin_d);
    let q = -(vi * vi / a_sq) * (b + bc_half) - (vi * vj / a) * (g * sin_d - b * cos_d);
    (p, q)
}

/// Branch flow (to side)
fn branch_flow_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_j - theta_i + branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let p = vj * vj * g - (vi * vj / a) * (g * cos_d + b * sin_d);
    let q = -vj * vj * (b + bc_half) - (vi * vj / a) * (g * sin_d - b * cos_d);
    (p, q)
}

/// Gradient of P (from side): [∂P/∂Vi, ∂P/∂Vj, ∂P/∂θi, ∂P/∂θj]
fn branch_grad_p_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 4] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;

    let theta_diff = theta_i - theta_j - branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let dp_dvi = (2.0 * vi / a_sq) * g - (vj / a) * (g * cos_d + b * sin_d);
    let dp_dvj = -(vi / a) * (g * cos_d + b * sin_d);
    let dp_dti = (vi * vj / a) * (g * sin_d - b * cos_d);
    let dp_dtj = -(vi * vj / a) * (g * sin_d - b * cos_d);

    [dp_dvi, dp_dvj, dp_dti, dp_dtj]
}

/// Gradient of Q (from side)
fn branch_grad_q_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 4] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_i - theta_j - branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let dq_dvi = -(2.0 * vi / a_sq) * (b + bc_half) - (vj / a) * (g * sin_d - b * cos_d);
    let dq_dvj = -(vi / a) * (g * sin_d - b * cos_d);
    let dq_dti = -(vi * vj / a) * (g * cos_d + b * sin_d);
    let dq_dtj = (vi * vj / a) * (g * cos_d + b * sin_d);

    [dq_dvi, dq_dvj, dq_dti, dq_dtj]
}

/// Gradient of P (to side)
fn branch_grad_p_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 4] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

    let theta_diff = theta_j - theta_i + branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let dp_dvi = -(vj / a) * (g * cos_d + b * sin_d);
    let dp_dvj = 2.0 * vj * g - (vi / a) * (g * cos_d + b * sin_d);
    let dp_dti = (vi * vj / a) * (g * sin_d - b * cos_d);
    let dp_dtj = -(vi * vj / a) * (g * sin_d - b * cos_d);

    [dp_dvi, dp_dvj, dp_dti, dp_dtj]
}

/// Gradient of Q (to side)
fn branch_grad_q_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 4] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_j - theta_i + branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    let dq_dvi = -(vj / a) * (g * sin_d - b * cos_d);
    let dq_dvj = -2.0 * vj * (b + bc_half) - (vi / a) * (g * sin_d - b * cos_d);
    let dq_dti = (vi * vj / a) * (g * cos_d + b * sin_d);
    let dq_dtj = -(vi * vj / a) * (g * cos_d + b * sin_d);

    [dq_dvi, dq_dvj, dq_dti, dq_dtj]
}

/// Hessian of P (from side) - lower triangular
/// Returns [∂²P/∂Vi², ∂²P/∂Vj∂Vi, ∂²P/∂Vj², ∂²P/∂θi∂Vi, ∂²P/∂θi∂Vj, ∂²P/∂θi²,
///          ∂²P/∂θj∂Vi, ∂²P/∂θj∂Vj, ∂²P/∂θj∂θi, ∂²P/∂θj²]
fn branch_hess_p_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 10] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;

    let theta_diff = theta_i - theta_j - branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    // P_ij = (Vi²/a²)·g - (Vi·Vj/a)·(g·cos + b·sin)

    // ∂²P/∂Vi² = (2/a²)·g
    let d2p_vi_vi = (2.0 / a_sq) * g;

    // ∂²P/∂Vj∂Vi = -(1/a)·(g·cos + b·sin)
    let d2p_vj_vi = -(1.0 / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂Vj² = 0
    let d2p_vj_vj = 0.0;

    // ∂²P/∂θi∂Vi = (Vj/a)·(g·sin - b·cos)
    let d2p_ti_vi = (vj / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θi∂Vj = (Vi/a)·(g·sin - b·cos)
    let d2p_ti_vj = (vi / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θi² = (Vi·Vj/a)·(g·cos + b·sin)
    let d2p_ti_ti = (vi * vj / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂θj∂Vi = -(Vj/a)·(g·sin - b·cos)
    let d2p_tj_vi = -(vj / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θj∂Vj = -(Vi/a)·(g·sin - b·cos)
    let d2p_tj_vj = -(vi / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θj∂θi = -(Vi·Vj/a)·(g·cos + b·sin)
    let d2p_tj_ti = -(vi * vj / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂θj² = (Vi·Vj/a)·(g·cos + b·sin)
    let d2p_tj_tj = (vi * vj / a) * (g * cos_d + b * sin_d);

    [d2p_vi_vi, d2p_vj_vi, d2p_vj_vj, d2p_ti_vi, d2p_ti_vj, d2p_ti_ti,
     d2p_tj_vi, d2p_tj_vj, d2p_tj_ti, d2p_tj_tj]
}

/// Hessian of Q (from side) - lower triangular
fn branch_hess_q_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 10] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_i - theta_j - branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    // Q_ij = -(Vi²/a²)·(b + bc/2) - (Vi·Vj/a)·(g·sin - b·cos)

    // ∂²Q/∂Vi² = -(2/a²)·(b + bc/2)
    let d2q_vi_vi = -(2.0 / a_sq) * (b + bc_half);

    // ∂²Q/∂Vj∂Vi = -(1/a)·(g·sin - b·cos)
    let d2q_vj_vi = -(1.0 / a) * (g * sin_d - b * cos_d);

    // ∂²Q/∂Vj² = 0
    let d2q_vj_vj = 0.0;

    // ∂²Q/∂θi∂Vi = -(Vj/a)·(g·cos + b·sin)
    let d2q_ti_vi = -(vj / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θi∂Vj = -(Vi/a)·(g·cos + b·sin)
    let d2q_ti_vj = -(vi / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θi² = -(Vi·Vj/a)·(g·sin - b·cos) = (Vi·Vj/a)·(b·cos - g·sin)
    let d2q_ti_ti = (vi * vj / a) * (b * cos_d - g * sin_d);

    // ∂²Q/∂θj∂Vi = (Vj/a)·(g·cos + b·sin)
    let d2q_tj_vi = (vj / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θj∂Vj = (Vi/a)·(g·cos + b·sin)
    let d2q_tj_vj = (vi / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θj∂θi = (Vi·Vj/a)·(g·sin - b·cos) = -(Vi·Vj/a)·(b·cos - g·sin)
    let d2q_tj_ti = -(vi * vj / a) * (b * cos_d - g * sin_d);

    // ∂²Q/∂θj² = -(Vi·Vj/a)·(g·sin - b·cos) = (Vi·Vj/a)·(b·cos - g·sin)
    let d2q_tj_tj = (vi * vj / a) * (b * cos_d - g * sin_d);

    [d2q_vi_vi, d2q_vj_vi, d2q_vj_vj, d2q_ti_vi, d2q_ti_vj, d2q_ti_ti,
     d2q_tj_vi, d2q_tj_vj, d2q_tj_ti, d2q_tj_tj]
}

/// Hessian of P (to side) - lower triangular
fn branch_hess_p_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 10] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

    let theta_diff = theta_j - theta_i + branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    // P_ji = Vj²·g - (Vi·Vj/a)·(g·cos + b·sin)

    // ∂²P/∂Vi² = 0
    let d2p_vi_vi = 0.0;

    // ∂²P/∂Vj∂Vi = -(1/a)·(g·cos + b·sin)
    let d2p_vj_vi = -(1.0 / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂Vj² = 2·g
    let d2p_vj_vj = 2.0 * g;

    // ∂²P/∂θi∂Vi = (Vj/a)·(g·sin - b·cos)  (note: ∂θ_diff/∂θi = -1)
    let d2p_ti_vi = (vj / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θi∂Vj = (Vi/a)·(g·sin - b·cos)
    let d2p_ti_vj = (vi / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θi² = (Vi·Vj/a)·(g·cos + b·sin)
    let d2p_ti_ti = (vi * vj / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂θj∂Vi = -(Vj/a)·(g·sin - b·cos)
    let d2p_tj_vi = -(vj / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θj∂Vj = -(Vi/a)·(g·sin - b·cos)
    let d2p_tj_vj = -(vi / a) * (g * sin_d - b * cos_d);

    // ∂²P/∂θj∂θi = -(Vi·Vj/a)·(g·cos + b·sin)
    let d2p_tj_ti = -(vi * vj / a) * (g * cos_d + b * sin_d);

    // ∂²P/∂θj² = (Vi·Vj/a)·(g·cos + b·sin)
    let d2p_tj_tj = (vi * vj / a) * (g * cos_d + b * sin_d);

    [d2p_vi_vi, d2p_vj_vi, d2p_vj_vj, d2p_ti_vi, d2p_ti_vj, d2p_ti_ti,
     d2p_tj_vi, d2p_tj_vj, d2p_tj_ti, d2p_tj_tj]
}

/// Hessian of Q (to side) - lower triangular
fn branch_hess_q_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> [f64; 10] {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;
    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_j - theta_i + branch.shift;
    let (sin_d, cos_d) = theta_diff.sin_cos();

    // Q_ji = -Vj²·(b + bc/2) - (Vi·Vj/a)·(g·sin - b·cos)

    // ∂²Q/∂Vi² = 0
    let d2q_vi_vi = 0.0;

    // ∂²Q/∂Vj∂Vi = -(1/a)·(g·sin - b·cos)
    let d2q_vj_vi = -(1.0 / a) * (g * sin_d - b * cos_d);

    // ∂²Q/∂Vj² = -2·(b + bc/2)
    let d2q_vj_vj = -2.0 * (b + bc_half);

    // ∂²Q/∂θi∂Vi = (Vj/a)·(g·cos + b·sin)
    let d2q_ti_vi = (vj / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θi∂Vj = (Vi/a)·(g·cos + b·sin)
    let d2q_ti_vj = (vi / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θi² = (Vi·Vj/a)·(b·cos - g·sin)
    let d2q_ti_ti = (vi * vj / a) * (b * cos_d - g * sin_d);

    // ∂²Q/∂θj∂Vi = -(Vj/a)·(g·cos + b·sin)
    let d2q_tj_vi = -(vj / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θj∂Vj = -(Vi/a)·(g·cos + b·sin)
    let d2q_tj_vj = -(vi / a) * (g * cos_d + b * sin_d);

    // ∂²Q/∂θj∂θi = -(Vi·Vj/a)·(b·cos - g·sin)
    let d2q_tj_ti = -(vi * vj / a) * (b * cos_d - g * sin_d);

    // ∂²Q/∂θj² = (Vi·Vj/a)·(b·cos - g·sin)
    let d2q_tj_tj = (vi * vj / a) * (b * cos_d - g * sin_d);

    [d2q_vi_vi, d2q_vj_vi, d2q_vj_vj, d2q_ti_vi, d2q_ti_vj, d2q_ti_ti,
     d2q_tj_vi, d2q_tj_vj, d2q_tj_ti, d2q_tj_tj]
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
fn d2p_dtheta_i_dtheta_j(
    ybus: &YBus,
    v: &[f64],
    theta: &[f64],
    k: usize,
    i: usize,
    j: usize,
) -> f64 {
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
fn d2q_dtheta_i_dtheta_j(
    ybus: &YBus,
    v: &[f64],
    theta: &[f64],
    k: usize,
    i: usize,
    j: usize,
) -> f64 {
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
        use gat_core::{Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Network, Node};

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
        use gat_core::{Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Network, Node};

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
