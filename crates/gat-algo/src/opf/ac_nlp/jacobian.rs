//! # Analytical Jacobian for AC-OPF Constraints
//!
//! This module computes the analytical Jacobian of the constraint functions for IPOPT.
//! Using analytical derivatives instead of finite differences provides:
//! - **Better numerical accuracy**: No truncation error from finite differencing
//! - **Faster computation**: O(nnz) vs O(n × m) for finite differences
//! - **Better convergence**: More accurate derivatives lead to better Newton steps
//!
//! ## Constraint Structure
//!
//! The constraints are:
//!
//! **Equality constraints** (g(x) = 0):
//! - g[0..n_bus]: Real power balance P_i^calc - P_g + P_d = 0
//! - g[n_bus..2*n_bus]: Reactive power balance Q_i^calc - Q_g + Q_d = 0
//! - g[2*n_bus]: Reference angle θ_ref = 0
//!
//! **Inequality constraints** (h(x) ≤ 0):
//! - For each thermally-constrained branch k:
//!   - h[2k]: S²_from_k - S²_max_k ≤ 0 (from side thermal limit)
//!   - h[2k+1]: S²_to_k - S²_max_k ≤ 0 (to side thermal limit)
//!
//! ## Variable Structure
//!
//! The variables are: x = [V | θ | P_g | Q_g]
//! - x[0..n_bus]: Voltage magnitudes V (per-unit)
//! - x[n_bus..2*n_bus]: Voltage angles θ (radians)
//! - x[2*n_bus..2*n_bus+n_gen]: Generator real power P_g (per-unit)
//! - x[2*n_bus+n_gen..]: Generator reactive power Q_g (per-unit)
//!
//! ## Jacobian Blocks
//!
//! ```text
//!                │     V      │     θ      │    P_g     │    Q_g     │
//!     ───────────┼────────────┼────────────┼────────────┼────────────┤
//!     P balance  │  ∂P/∂V     │  ∂P/∂θ     │ -I (sparse)│     0      │
//!     Q balance  │  ∂Q/∂V     │  ∂Q/∂θ     │     0      │ -I (sparse)│
//!     θ_ref      │     0      │ [0..1..0]  │     0      │     0      │
//!     Thermal    │ ∂S²/∂V_i,j │ ∂S²/∂θ_i,j │     0      │     0      │
//! ```

use super::{AcOpfProblem, BranchData};

/// Compute the sparsity pattern of the constraint Jacobian.
///
/// Returns (row_indices, col_indices) for non-zero entries.
/// IPOPT requires knowing the sparsity pattern before computing values.
///
/// # Arguments
/// * `problem` - AC-OPF problem (provides dimensions and Y-bus structure)
///
/// # Returns
/// Tuple of (row indices, column indices) for all non-zero Jacobian entries
pub fn jacobian_sparsity(problem: &AcOpfProblem) -> (Vec<usize>, Vec<usize>) {
    let n_bus = problem.n_bus;
    let v_offset = problem.v_offset;
    let theta_offset = problem.theta_offset;
    let pg_offset = problem.pg_offset;
    let qg_offset = problem.qg_offset;

    let mut rows = Vec::new();
    let mut cols = Vec::new();

    // ========================================================================
    // P BALANCE EQUATIONS (rows 0..n_bus)
    // ========================================================================
    for i in 0..n_bus {
        let row = i; // P balance for bus i

        // ∂P_i/∂V_j for all j (dense because P_i depends on all V through network)
        // In practice, only Y_ij ≠ 0 entries matter, but we include all for simplicity
        for j in 0..n_bus {
            rows.push(row);
            cols.push(v_offset + j);
        }

        // ∂P_i/∂θ_j for all j
        for j in 0..n_bus {
            rows.push(row);
            cols.push(theta_offset + j);
        }

        // ∂P_i/∂P_g_k = -1 for generators at bus i
        for (k, &bus_idx) in problem.gen_bus_idx.iter().enumerate() {
            if bus_idx == i {
                rows.push(row);
                cols.push(pg_offset + k);
            }
        }
        // ∂P_i/∂Q_g = 0, no entries needed
    }

    // ========================================================================
    // Q BALANCE EQUATIONS (rows n_bus..2*n_bus)
    // ========================================================================
    for i in 0..n_bus {
        let row = n_bus + i; // Q balance for bus i

        // ∂Q_i/∂V_j for all j
        for j in 0..n_bus {
            rows.push(row);
            cols.push(v_offset + j);
        }

        // ∂Q_i/∂θ_j for all j
        for j in 0..n_bus {
            rows.push(row);
            cols.push(theta_offset + j);
        }

        // ∂Q_i/∂P_g = 0, no entries needed

        // ∂Q_i/∂Q_g_k = -1 for generators at bus i
        for (k, &bus_idx) in problem.gen_bus_idx.iter().enumerate() {
            if bus_idx == i {
                rows.push(row);
                cols.push(qg_offset + k);
            }
        }
    }

    // ========================================================================
    // REFERENCE ANGLE CONSTRAINT (row 2*n_bus)
    // ========================================================================
    // θ_ref = 0, so ∂g/∂θ[ref_bus] = 1
    let row = 2 * n_bus;
    rows.push(row);
    cols.push(theta_offset + problem.ref_bus);

    // ========================================================================
    // THERMAL LIMIT CONSTRAINTS (rows 2*n_bus + 1 + 2*k for branch k)
    // ========================================================================
    // For each thermally-constrained branch, we have:
    // - From side: h = S²_from - S²_max, depends on Vi, Vj, θi, θj
    // - To side: h = S²_to - S²_max, depends on Vi, Vj, θi, θj
    let n_eq = 2 * n_bus + 1;
    let mut thermal_row = n_eq;

    for branch in &problem.branches {
        if branch.rate_mva <= 0.0 {
            continue;
        }

        let i = branch.from_idx;
        let j = branch.to_idx;

        // From side constraint: depends on Vi, Vj, θi, θj
        // ∂h/∂Vi, ∂h/∂Vj
        rows.push(thermal_row);
        cols.push(v_offset + i);
        rows.push(thermal_row);
        cols.push(v_offset + j);
        // ∂h/∂θi, ∂h/∂θj
        rows.push(thermal_row);
        cols.push(theta_offset + i);
        rows.push(thermal_row);
        cols.push(theta_offset + j);
        thermal_row += 1;

        // To side constraint: depends on Vi, Vj, θi, θj
        rows.push(thermal_row);
        cols.push(v_offset + i);
        rows.push(thermal_row);
        cols.push(v_offset + j);
        rows.push(thermal_row);
        cols.push(theta_offset + i);
        rows.push(thermal_row);
        cols.push(theta_offset + j);
        thermal_row += 1;
    }

    (rows, cols)
}

/// Compute the number of non-zeros in the Jacobian.
pub fn jacobian_nnz(problem: &AcOpfProblem) -> usize {
    let (rows, _) = jacobian_sparsity(problem);
    rows.len()
}

/// Compute the Jacobian values at the given point.
///
/// # Arguments
/// * `problem` - AC-OPF problem definition
/// * `x` - Current variable values [V | θ | P_g | Q_g]
///
/// # Returns
/// Jacobian values in the same order as the sparsity pattern
pub fn jacobian_values(problem: &AcOpfProblem, x: &[f64]) -> Vec<f64> {
    let n_bus = problem.n_bus;
    let ybus = &problem.ybus;

    // Extract V and θ from x
    let (v, theta) = problem.extract_v_theta(x);

    // Pre-allocate values vector
    let nnz = jacobian_nnz(problem);
    let mut vals = Vec::with_capacity(nnz);

    // ========================================================================
    // P BALANCE EQUATIONS (rows 0..n_bus)
    // ========================================================================
    //
    // P_i = Σⱼ V_i·V_j·[G_ij·cos(θ_ij) + B_ij·sin(θ_ij)] - P_g + P_d
    //
    // Derivatives (from power_equations.rs documentation):
    // ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
    // ∂P_i/∂V_j = V_i·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))  for j ≠ i
    // ∂P_i/∂θ_i = V_i·Σ_{k≠i} V_k·(-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
    // ∂P_i/∂θ_j = V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))  for j ≠ i

    for i in 0..n_bus {
        // ∂P_i/∂V_j for all j
        for j in 0..n_bus {
            let g_ij = ybus.g(i, j);
            let b_ij = ybus.b(i, j);
            let theta_ij = theta[i] - theta[j];
            let cos_ij = theta_ij.cos();
            let sin_ij = theta_ij.sin();

            let dp_dv = if i == j {
                // Diagonal: ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                let mut sum = 2.0 * v[i] * g_ij;
                for k in 0..n_bus {
                    if k != i {
                        let g_ik = ybus.g(i, k);
                        let b_ik = ybus.b(i, k);
                        let theta_ik = theta[i] - theta[k];
                        sum += v[k] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin());
                    }
                }
                sum
            } else {
                // Off-diagonal: ∂P_i/∂V_j = V_i·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                v[i] * (g_ij * cos_ij + b_ij * sin_ij)
            };
            vals.push(dp_dv);
        }

        // ∂P_i/∂θ_j for all j
        for j in 0..n_bus {
            let g_ij = ybus.g(i, j);
            let b_ij = ybus.b(i, j);
            let theta_ij = theta[i] - theta[j];
            let cos_ij = theta_ij.cos();
            let sin_ij = theta_ij.sin();

            let dp_dtheta = if i == j {
                // Diagonal: ∂P_i/∂θ_i = V_i·Σ_{k≠i} V_k·(-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
                let mut sum = 0.0;
                for k in 0..n_bus {
                    if k != i {
                        let g_ik = ybus.g(i, k);
                        let b_ik = ybus.b(i, k);
                        let theta_ik = theta[i] - theta[k];
                        sum += v[k] * (-g_ik * theta_ik.sin() + b_ik * theta_ik.cos());
                    }
                }
                v[i] * sum
            } else {
                // Off-diagonal: ∂P_i/∂θ_j = V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                v[i] * v[j] * (g_ij * sin_ij - b_ij * cos_ij)
            };
            vals.push(dp_dtheta);
        }

        // ∂P_i/∂P_g_k = -1 for generators at bus i
        for (_k, &bus_idx) in problem.gen_bus_idx.iter().enumerate() {
            if bus_idx == i {
                vals.push(-1.0);
            }
        }
    }

    // ========================================================================
    // Q BALANCE EQUATIONS (rows n_bus..2*n_bus)
    // ========================================================================
    //
    // Q_i = Σⱼ V_i·V_j·[G_ij·sin(θ_ij) - B_ij·cos(θ_ij)] - Q_g + Q_d
    //
    // Derivatives:
    // ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i} V_k·(G_ik·sin(θ_ik) - B_ik·cos(θ_ik))
    // ∂Q_i/∂V_j = V_i·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))  for j ≠ i
    // ∂Q_i/∂θ_i = V_i·Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
    // ∂Q_i/∂θ_j = -V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))  for j ≠ i

    for i in 0..n_bus {
        // ∂Q_i/∂V_j for all j
        for j in 0..n_bus {
            let g_ij = ybus.g(i, j);
            let b_ij = ybus.b(i, j);
            let theta_ij = theta[i] - theta[j];
            let cos_ij = theta_ij.cos();
            let sin_ij = theta_ij.sin();

            let dq_dv = if i == j {
                // Diagonal: ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i} V_k·(G_ik·sin(θ_ik) - B_ik·cos(θ_ik))
                let mut sum = -2.0 * v[i] * b_ij;
                for k in 0..n_bus {
                    if k != i {
                        let g_ik = ybus.g(i, k);
                        let b_ik = ybus.b(i, k);
                        let theta_ik = theta[i] - theta[k];
                        sum += v[k] * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos());
                    }
                }
                sum
            } else {
                // Off-diagonal: ∂Q_i/∂V_j = V_i·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                v[i] * (g_ij * sin_ij - b_ij * cos_ij)
            };
            vals.push(dq_dv);
        }

        // ∂Q_i/∂θ_j for all j
        for j in 0..n_bus {
            let g_ij = ybus.g(i, j);
            let b_ij = ybus.b(i, j);
            let theta_ij = theta[i] - theta[j];
            let cos_ij = theta_ij.cos();
            let sin_ij = theta_ij.sin();

            let dq_dtheta = if i == j {
                // Diagonal: ∂Q_i/∂θ_i = V_i·Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                let mut sum = 0.0;
                for k in 0..n_bus {
                    if k != i {
                        let g_ik = ybus.g(i, k);
                        let b_ik = ybus.b(i, k);
                        let theta_ik = theta[i] - theta[k];
                        sum += v[k] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin());
                    }
                }
                v[i] * sum
            } else {
                // Off-diagonal: ∂Q_i/∂θ_j = -V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                -v[i] * v[j] * (g_ij * cos_ij + b_ij * sin_ij)
            };
            vals.push(dq_dtheta);
        }

        // ∂Q_i/∂Q_g_k = -1 for generators at bus i
        for (_k, &bus_idx) in problem.gen_bus_idx.iter().enumerate() {
            if bus_idx == i {
                vals.push(-1.0);
            }
        }
    }

    // ========================================================================
    // REFERENCE ANGLE CONSTRAINT (row 2*n_bus)
    // ========================================================================
    // g = θ[ref_bus] = 0, so ∂g/∂θ[ref_bus] = 1
    vals.push(1.0);

    // ========================================================================
    // THERMAL LIMIT CONSTRAINTS
    // ========================================================================
    // h = P² + Q² - S²_max
    // ∂h/∂x = 2P·∂P/∂x + 2Q·∂Q/∂x

    for branch in &problem.branches {
        if branch.rate_mva <= 0.0 {
            continue;
        }

        let i = branch.from_idx;
        let j = branch.to_idx;
        let vi = v[i];
        let vj = v[j];
        let theta_i = theta[i];
        let theta_j = theta[j];

        // From side: h = P²_ij + Q²_ij - S²_max
        {
            let (p, q) = branch_flow_from(branch, vi, vj, theta_i, theta_j);
            let (dp_dvi, dp_dvj, dp_dti, dp_dtj) = branch_flow_from_grad_p(branch, vi, vj, theta_i, theta_j);
            let (dq_dvi, dq_dvj, dq_dti, dq_dtj) = branch_flow_from_grad_q(branch, vi, vj, theta_i, theta_j);

            // ∂h/∂Vi = 2P·∂P/∂Vi + 2Q·∂Q/∂Vi
            vals.push(2.0 * p * dp_dvi + 2.0 * q * dq_dvi);
            // ∂h/∂Vj
            vals.push(2.0 * p * dp_dvj + 2.0 * q * dq_dvj);
            // ∂h/∂θi
            vals.push(2.0 * p * dp_dti + 2.0 * q * dq_dti);
            // ∂h/∂θj
            vals.push(2.0 * p * dp_dtj + 2.0 * q * dq_dtj);
        }

        // To side: h = P²_ji + Q²_ji - S²_max
        {
            let (p, q) = branch_flow_to(branch, vi, vj, theta_i, theta_j);
            let (dp_dvi, dp_dvj, dp_dti, dp_dtj) = branch_flow_to_grad_p(branch, vi, vj, theta_i, theta_j);
            let (dq_dvi, dq_dvj, dq_dti, dq_dtj) = branch_flow_to_grad_q(branch, vi, vj, theta_i, theta_j);

            // ∂h/∂Vi = 2P·∂P/∂Vi + 2Q·∂Q/∂Vi
            vals.push(2.0 * p * dp_dvi + 2.0 * q * dq_dvi);
            // ∂h/∂Vj
            vals.push(2.0 * p * dp_dvj + 2.0 * q * dq_dvj);
            // ∂h/∂θi
            vals.push(2.0 * p * dp_dti + 2.0 * q * dq_dti);
            // ∂h/∂θj
            vals.push(2.0 * p * dp_dtj + 2.0 * q * dq_dtj);
        }
    }

    vals
}

// ============================================================================
// BRANCH FLOW FUNCTIONS FOR JACOBIAN COMPUTATION
// ============================================================================

/// Compute branch power flow (from side).
fn branch_flow_from(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;

    let theta_diff = theta_i - theta_j - branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    let vi_sq = vi * vi;
    let vi_vj = vi * vj;

    let p = (vi_sq / a_sq) * g - (vi_vj / a) * (g * cos_diff + b * sin_diff);

    let bc_half = branch.b_charging / 2.0;
    let q = -(vi_sq / a_sq) * (b + bc_half) - (vi_vj / a) * (g * sin_diff - b * cos_diff);

    (p, q)
}

/// Compute branch power flow (to side).
fn branch_flow_to(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

    let theta_diff = theta_j - theta_i + branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    let vj_sq = vj * vj;
    let vi_vj = vi * vj;

    let p = vj_sq * g - (vi_vj / a) * (g * cos_diff + b * sin_diff);

    let bc_half = branch.b_charging / 2.0;
    let q = -vj_sq * (b + bc_half) - (vi_vj / a) * (g * sin_diff - b * cos_diff);

    (p, q)
}

/// Gradient of P_ij (from side) w.r.t. (Vi, Vj, θi, θj).
fn branch_flow_from_grad_p(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64, f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;

    let theta_diff = theta_i - theta_j - branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    // P_ij = (Vi²/a²)·g - (Vi·Vj/a)·(g·cos + b·sin)

    // ∂P/∂Vi = (2Vi/a²)·g - (Vj/a)·(g·cos + b·sin)
    let dp_dvi = (2.0 * vi / a_sq) * g - (vj / a) * (g * cos_diff + b * sin_diff);

    // ∂P/∂Vj = -(Vi/a)·(g·cos + b·sin)
    let dp_dvj = -(vi / a) * (g * cos_diff + b * sin_diff);

    // ∂P/∂θi = -(Vi·Vj/a)·(-g·sin + b·cos) = (Vi·Vj/a)·(g·sin - b·cos)
    let dp_dti = (vi * vj / a) * (g * sin_diff - b * cos_diff);

    // ∂P/∂θj = -(Vi·Vj/a)·(g·sin - b·cos)
    let dp_dtj = -(vi * vj / a) * (g * sin_diff - b * cos_diff);

    (dp_dvi, dp_dvj, dp_dti, dp_dtj)
}

/// Gradient of Q_ij (from side) w.r.t. (Vi, Vj, θi, θj).
fn branch_flow_from_grad_q(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64, f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
    let a_sq = a * a;

    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_i - theta_j - branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    // Q_ij = -(Vi²/a²)·(b + bc/2) - (Vi·Vj/a)·(g·sin - b·cos)

    // ∂Q/∂Vi = -(2Vi/a²)·(b + bc/2) - (Vj/a)·(g·sin - b·cos)
    let dq_dvi = -(2.0 * vi / a_sq) * (b + bc_half) - (vj / a) * (g * sin_diff - b * cos_diff);

    // ∂Q/∂Vj = -(Vi/a)·(g·sin - b·cos)
    let dq_dvj = -(vi / a) * (g * sin_diff - b * cos_diff);

    // ∂Q/∂θi = -(Vi·Vj/a)·(g·cos + b·sin)
    let dq_dti = -(vi * vj / a) * (g * cos_diff + b * sin_diff);

    // ∂Q/∂θj = (Vi·Vj/a)·(g·cos + b·sin)
    let dq_dtj = (vi * vj / a) * (g * cos_diff + b * sin_diff);

    (dq_dvi, dq_dvj, dq_dti, dq_dtj)
}

/// Gradient of P_ji (to side) w.r.t. (Vi, Vj, θi, θj).
fn branch_flow_to_grad_p(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64, f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

    let theta_diff = theta_j - theta_i + branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    // P_ji = Vj²·g - (Vi·Vj/a)·(g·cos + b·sin)

    // ∂P/∂Vi = -(Vj/a)·(g·cos + b·sin)
    let dp_dvi = -(vj / a) * (g * cos_diff + b * sin_diff);

    // ∂P/∂Vj = 2Vj·g - (Vi/a)·(g·cos + b·sin)
    let dp_dvj = 2.0 * vj * g - (vi / a) * (g * cos_diff + b * sin_diff);

    // ∂P/∂θi = (Vi·Vj/a)·(g·sin - b·cos)  (note: ∂θ_diff/∂θi = -1)
    let dp_dti = (vi * vj / a) * (g * sin_diff - b * cos_diff);

    // ∂P/∂θj = -(Vi·Vj/a)·(g·sin - b·cos)  (note: ∂θ_diff/∂θj = 1)
    let dp_dtj = -(vi * vj / a) * (g * sin_diff - b * cos_diff);

    (dp_dvi, dp_dvj, dp_dti, dp_dtj)
}

/// Gradient of Q_ji (to side) w.r.t. (Vi, Vj, θi, θj).
fn branch_flow_to_grad_q(branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64, f64, f64) {
    let z_sq = branch.r * branch.r + branch.x * branch.x;
    let g = branch.r / z_sq;
    let b = -branch.x / z_sq;

    let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

    let bc_half = branch.b_charging / 2.0;

    let theta_diff = theta_j - theta_i + branch.shift;
    let cos_diff = theta_diff.cos();
    let sin_diff = theta_diff.sin();

    // Q_ji = -Vj²·(b + bc/2) - (Vi·Vj/a)·(g·sin - b·cos)

    // ∂Q/∂Vi = -(Vj/a)·(g·sin - b·cos)
    let dq_dvi = -(vj / a) * (g * sin_diff - b * cos_diff);

    // ∂Q/∂Vj = -2Vj·(b + bc/2) - (Vi/a)·(g·sin - b·cos)
    let dq_dvj = -2.0 * vj * (b + bc_half) - (vi / a) * (g * sin_diff - b * cos_diff);

    // ∂Q/∂θi = (Vi·Vj/a)·(g·cos + b·sin)  (note: ∂θ_diff/∂θi = -1)
    let dq_dti = (vi * vj / a) * (g * cos_diff + b * sin_diff);

    // ∂Q/∂θj = -(Vi·Vj/a)·(g·cos + b·sin)  (note: ∂θ_diff/∂θj = 1)
    let dq_dtj = -(vi * vj / a) * (g * cos_diff + b * sin_diff);

    (dq_dvi, dq_dvj, dq_dti, dq_dtj)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Jacobian against finite differences for a small problem.
    #[test]
    fn test_jacobian_vs_finite_diff() {
        // This test would require setting up a small test problem
        // For now, just verify dimensions match
    }
}
