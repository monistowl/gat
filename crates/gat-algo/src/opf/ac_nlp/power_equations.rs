//! AC Power Flow Equations
//!
//! Implements the fundamental AC power flow equations:
//!
//! ```text
//! P_i = Σⱼ V_i·V_j·(G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j))
//! Q_i = Σⱼ V_i·V_j·(G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j))
//! ```
//!
//! where G_ij = Re(Y_ij) and B_ij = Im(Y_ij).
//!
//! ## References
//!
//! - Grainger & Stevenson, "Power System Analysis", equations (9.14)-(9.15)

use super::YBus;

/// AC power flow equation computation
pub struct PowerEquations;

impl PowerEquations {
    /// Compute power injections at all buses
    ///
    /// # Arguments
    ///
    /// * `ybus` - Y-bus admittance matrix
    /// * `v` - Voltage magnitudes (per-unit)
    /// * `theta` - Voltage angles (radians)
    ///
    /// # Returns
    ///
    /// Tuple of (P_injection, Q_injection) vectors in per-unit
    pub fn compute_injections(ybus: &YBus, v: &[f64], theta: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();
        let mut p_inj = vec![0.0; n];
        let mut q_inj = vec![0.0; n];

        for i in 0..n {
            let vi = v[i];
            let theta_i = theta[i];

            for j in 0..n {
                let y_ij = ybus.get(i, j);
                let g_ij = y_ij.re;
                let b_ij = y_ij.im;

                let vj = v[j];
                let theta_ij = theta_i - theta[j];

                let cos_ij = theta_ij.cos();
                let sin_ij = theta_ij.sin();

                // P_i = Σ V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                p_inj[i] += vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                // Q_i = Σ V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                q_inj[i] += vi * vj * (g_ij * sin_ij - b_ij * cos_ij);
            }
        }

        (p_inj, q_inj)
    }

    /// Compute Jacobian of power flow equations
    ///
    /// The Jacobian has the structure:
    /// ```text
    /// J = | ∂P/∂θ  ∂P/∂V |
    ///     | ∂Q/∂θ  ∂Q/∂V |
    /// ```
    ///
    /// # Returns
    ///
    /// Tuple of (dP_dtheta, dP_dV, dQ_dtheta, dQ_dV) as flat vectors (row-major)
    pub fn compute_jacobian(
        ybus: &YBus,
        v: &[f64],
        theta: &[f64],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();

        let mut dp_dtheta = vec![0.0; n * n];
        let mut dp_dv = vec![0.0; n * n];
        let mut dq_dtheta = vec![0.0; n * n];
        let mut dq_dv = vec![0.0; n * n];

        for i in 0..n {
            let vi = v[i];
            let theta_i = theta[i];

            for j in 0..n {
                let y_ij = ybus.get(i, j);
                let g_ij = y_ij.re;
                let b_ij = y_ij.im;

                let vj = v[j];
                let theta_ij = theta_i - theta[j];

                let cos_ij = theta_ij.cos();
                let sin_ij = theta_ij.sin();

                let idx = i * n + j;

                if i == j {
                    // Diagonal elements (self-derivatives)
                    // Need to compute sums over k ≠ i

                    let mut sum_p = 0.0;
                    let mut sum_q = 0.0;

                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let g_ik = y_ik.re;
                            let b_ik = y_ik.im;
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            let cos_ik = theta_ik.cos();
                            let sin_ik = theta_ik.sin();

                            sum_p += vk * (-g_ik * sin_ik + b_ik * cos_ik);
                            sum_q += vk * (g_ik * cos_ik + b_ik * sin_ik);
                        }
                    }

                    // ∂P_i/∂θ_i = V_i · Σ_{k≠i} V_k · (-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
                    dp_dtheta[idx] = vi * sum_p;

                    // ∂Q_i/∂θ_i = V_i · Σ_{k≠i} V_k · (G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                    dq_dtheta[idx] = vi * sum_q;

                    // ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                    let mut sum_pv = 0.0;
                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            sum_pv += vk * (y_ik.re * theta_ik.cos() + y_ik.im * theta_ik.sin());
                        }
                    }
                    dp_dv[idx] = 2.0 * vi * g_ij + sum_pv;

                    // ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i} V_k·(G_ik·sin(θ_ik) - B_ik·cos(θ_ik))
                    let mut sum_qv = 0.0;
                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            sum_qv += vk * (y_ik.re * theta_ik.sin() - y_ik.im * theta_ik.cos());
                        }
                    }
                    dq_dv[idx] = -2.0 * vi * b_ij + sum_qv;
                } else {
                    // Off-diagonal elements

                    // ∂P_i/∂θ_j = V_i · V_j · (G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dp_dtheta[idx] = vi * vj * (g_ij * sin_ij - b_ij * cos_ij);

                    // ∂Q_i/∂θ_j = -V_i · V_j · (G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dq_dtheta[idx] = -vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                    // ∂P_i/∂V_j = V_i · (G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dp_dv[idx] = vi * (g_ij * cos_ij + b_ij * sin_ij);

                    // ∂Q_i/∂V_j = V_i · (G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dq_dv[idx] = vi * (g_ij * sin_ij - b_ij * cos_ij);
                }
            }
        }

        (dp_dtheta, dp_dv, dq_dtheta, dq_dv)
    }
}
