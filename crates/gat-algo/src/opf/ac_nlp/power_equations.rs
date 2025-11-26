//! # AC Power Flow Equations
//!
//! This module computes the nonlinear AC power flow equations that form the
//! equality constraints of AC-OPF. These equations enforce Kirchhoff's laws:
//! the power injected at each bus must equal the power flowing out.
//!
//! ## The Power Balance Equations
//!
//! At each bus i, conservation of energy requires:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  REAL POWER BALANCE (P equation)                                         │
//! │  ───────────────────────────────                                         │
//! │                                                                           │
//! │  P_i^gen - P_i^load = P_i^calc                                           │
//! │                                                                           │
//! │  where P_i^calc = Σⱼ V_i · V_j · [ G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j) ]
//! │                   └─────────────────────────────────────────────────────────┘
//! │                    power flowing out through network (using Y-bus)         │
//! │                                                                           │
//! │  Physically: power generated - power consumed = power flowing to neighbors│
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  REACTIVE POWER BALANCE (Q equation)                                     │
//! │  ─────────────────────────────────                                       │
//! │                                                                           │
//! │  Q_i^gen - Q_i^load = Q_i^calc                                           │
//! │                                                                           │
//! │  where Q_i^calc = Σⱼ V_i · V_j · [ G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j) ]
//! │                                                                           │
//! │  Note: The sin/cos swap and sign change from P equation is due to         │
//! │        reactive power being 90° out of phase with real power.             │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Derivation from Circuit Theory
//!
//! Starting from **complex power** S = V · I*:
//!
//! ```text
//! S_i = V_i · I_i* = V_i · (Σⱼ Y_ij · V_j)*
//!     = V_i · Σⱼ Y_ij* · V_j*
//!
//! Let V_i = |V_i| · e^{jθ_i}  (polar form)
//!     Y_ij = G_ij + jB_ij     (rectangular form)
//!
//! Then:
//! S_i = Σⱼ |V_i||V_j| · e^{j(θ_i - θ_j)} · (G_ij - jB_ij)
//!
//! Expanding using Euler's formula e^{jθ} = cos(θ) + j·sin(θ):
//!
//! P_i = Re(S_i) = Σⱼ |V_i||V_j| · [G_ij·cos(θ_ij) + B_ij·sin(θ_ij)]
//! Q_i = Im(S_i) = Σⱼ |V_i||V_j| · [G_ij·sin(θ_ij) - B_ij·cos(θ_ij)]
//!
//! where θ_ij = θ_i - θ_j (angle difference)
//! ```
//!
//! ## The Jacobian Matrix
//!
//! Newton-Raphson power flow requires the Jacobian of the power equations.
//! For n buses, the Jacobian is a 2n × 2n matrix partitioned as:
//!
//! ```text
//!     ┌               ┐
//!     │  J₁   │  J₂   │       J₁ = ∂P/∂θ   (n×n)
//! J = │──────┼───────│       J₂ = ∂P/∂V   (n×n)
//!     │  J₃   │  J₄   │       J₃ = ∂Q/∂θ   (n×n)
//!     └               ┘       J₄ = ∂Q/∂V   (n×n)
//! ```
//!
//! **Diagonal elements** (i = j):
//! ```text
//! ∂P_i/∂θ_i = V_i · Σ_{k≠i} V_k · (-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
//! ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
//! ∂Q_i/∂θ_i = V_i · Σ_{k≠i} V_k · (G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
//! ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i} V_k·(G_ik·sin(θ_ik) - B_ik·cos(θ_ik))
//! ```
//!
//! **Off-diagonal elements** (i ≠ j):
//! ```text
//! ∂P_i/∂θ_j = V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
//! ∂P_i/∂V_j = V_i·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
//! ∂Q_i/∂θ_j = -V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
//! ∂Q_i/∂V_j = V_i·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
//! ```
//!
//! ## Computational Considerations
//!
//! The naive evaluation of P_i requires O(n) operations per bus, giving O(n²)
//! total. However, since Y_ij = 0 for non-adjacent buses, the actual complexity
//! is O(n + m) where m is the number of branches (typically m ≈ 1.5n).
//!
//! ## References
//!
//! - **Grainger & Stevenson**: "Power System Analysis", equations (9.14)-(9.15)
//!   The standard textbook derivation of power flow equations
//!
//! - **Stott (1974)**: "Review of Load-Flow Calculation Methods"
//!   Proceedings of the IEEE, 62(7), 916-929
//!   DOI: [10.1109/PROC.1974.9544](https://doi.org/10.1109/PROC.1974.9544)
//!
//! - **Tinney & Hart (1967)**: "Power Flow Solution by Newton's Method"
//!   IEEE Trans. PAS, 86(11), 1449-1460
//!   DOI: [10.1109/TPAS.1967.291823](https://doi.org/10.1109/TPAS.1967.291823)

use super::YBus;

/// Calculator for AC power flow equations P(V,θ) and Q(V,θ).
///
/// This is a stateless helper that provides methods for computing power injections
/// and their Jacobian given the Y-bus matrix and bus voltage/angle vectors.
///
/// # Equation Summary
///
/// For bus i with voltage V_i∠θ_i connected to buses j via admittance Y_ij = G_ij + jB_ij:
///
/// ```text
/// P_i = Σⱼ V_i·V_j·[G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j)]
/// Q_i = Σⱼ V_i·V_j·[G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j)]
/// ```
///
/// Note: All quantities are in per-unit on a common system base (typically 100 MVA).
pub struct PowerEquations;

impl PowerEquations {
    /// Compute real and reactive power injections at all buses.
    ///
    /// Given voltage magnitudes V and angles θ, computes:
    /// - P_i = Σⱼ V_i·V_j·[G_ij·cos(θ_ij) + B_ij·sin(θ_ij)]
    /// - Q_i = Σⱼ V_i·V_j·[G_ij·sin(θ_ij) - B_ij·cos(θ_ij)]
    ///
    /// # Arguments
    ///
    /// * `ybus` - Y-bus admittance matrix
    /// * `v` - Voltage magnitudes at each bus (per-unit, typically 0.9-1.1)
    /// * `theta` - Voltage angles at each bus (radians)
    ///
    /// # Returns
    ///
    /// Tuple `(P, Q)` where:
    /// * `P[i]` = Real power injection at bus i (per-unit MW on system base)
    /// * `Q[i]` = Reactive power injection at bus i (per-unit MVAr on system base)
    ///
    /// # Computational Complexity
    ///
    /// O(n²) for dense Y-bus. Each of n buses requires summing over n neighbors.
    /// In practice, only non-zero Y-bus entries contribute.
    ///
    /// # Physical Interpretation
    ///
    /// The returned P and Q values represent net power leaving each bus into
    /// the network. For a balanced system:
    /// - Σ P_i ≈ P_losses (total system losses)
    /// - Σ Q_i ≈ Q_losses + Q_charging (losses plus line charging VArs)
    pub fn compute_injections(ybus: &YBus, v: &[f64], theta: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();
        let mut p_inj = vec![0.0; n];
        let mut q_inj = vec![0.0; n];

        // ====================================================================
        // POWER FLOW COMPUTATION
        // ====================================================================
        //
        // For each bus i, we compute the power injection by summing over
        // all buses j. The key terms are:
        //
        //   θ_ij = θ_i - θ_j           (angle difference)
        //   V_prod = V_i · V_j          (voltage product)
        //
        // Then:
        //   P contribution from j: V_prod · [G_ij·cos(θ_ij) + B_ij·sin(θ_ij)]
        //   Q contribution from j: V_prod · [G_ij·sin(θ_ij) - B_ij·cos(θ_ij)]

        for i in 0..n {
            let vi = v[i];
            let theta_i = theta[i];

            for j in 0..n {
                // Get Y-bus element Y_ij = G_ij + jB_ij
                let y_ij = ybus.get(i, j);
                let g_ij = y_ij.re; // Conductance (real part)
                let b_ij = y_ij.im; // Susceptance (imaginary part)

                let vj = v[j];
                let theta_ij = theta_i - theta[j];

                // Precompute trig functions (expensive)
                let cos_ij = theta_ij.cos();
                let sin_ij = theta_ij.sin();

                // ============================================================
                // REAL POWER EQUATION
                // ============================================================
                //
                // P_i += V_i · V_j · [G_ij · cos(θ_ij) + B_ij · sin(θ_ij)]
                //
                // Physical meaning:
                // - G_ij · cos(θ_ij): Resistive power flow (in-phase component)
                // - B_ij · sin(θ_ij): Reactive power contribution to real flow
                //
                // When θ_ij is small (typical), cos(θ_ij) ≈ 1 and sin(θ_ij) ≈ θ_ij,
                // leading to the DC power flow approximation P_ij ≈ θ_ij / X_ij.

                p_inj[i] += vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                // ============================================================
                // REACTIVE POWER EQUATION
                // ============================================================
                //
                // Q_i += V_i · V_j · [G_ij · sin(θ_ij) - B_ij · cos(θ_ij)]
                //
                // Physical meaning:
                // - G_ij · sin(θ_ij): Cross-coupled term (usually small)
                // - B_ij · cos(θ_ij): Main reactive power term (susceptance × V²)
                //
                // The negative sign on B_ij·cos(θ_ij) means:
                // - Inductive elements (B < 0) absorb reactive power
                // - Capacitive elements (B > 0) supply reactive power

                q_inj[i] += vi * vj * (g_ij * sin_ij - b_ij * cos_ij);
            }
        }

        (p_inj, q_inj)
    }

    /// Compute the Jacobian matrix of power flow equations.
    ///
    /// The Jacobian relates small changes in voltage/angle to changes in power:
    ///
    /// ```text
    /// [ ΔP ]   [ J₁  J₂ ] [ Δθ ]
    /// [    ] = [        ] [    ]
    /// [ ΔQ ]   [ J₃  J₄ ] [ ΔV ]
    /// ```
    ///
    /// This is used in Newton-Raphson iteration to solve power flow.
    ///
    /// # Arguments
    ///
    /// * `ybus` - Y-bus admittance matrix
    /// * `v` - Voltage magnitudes (per-unit)
    /// * `theta` - Voltage angles (radians)
    ///
    /// # Returns
    ///
    /// Tuple of four n×n matrices in row-major flat format:
    /// * `J₁ = ∂P/∂θ` - How real power changes with angle
    /// * `J₂ = ∂P/∂V` - How real power changes with voltage magnitude
    /// * `J₃ = ∂Q/∂θ` - How reactive power changes with angle
    /// * `J₄ = ∂Q/∂V` - How reactive power changes with voltage magnitude
    ///
    /// # Computational Complexity
    ///
    /// O(n³) due to nested loops for diagonal elements. Could be optimized
    /// to O(n·m) using sparse Y-bus, where m = number of branches.
    ///
    /// # Physical Interpretation
    ///
    /// **J₁ (∂P/∂θ)** is typically dominant: angle differences drive real power flow.
    /// This is why DC power flow (which ignores J₂, J₃, J₄) works well.
    ///
    /// **J₄ (∂Q/∂V)** is important for voltage control: reactive power injection
    /// strongly affects local voltage magnitude.
    ///
    /// **J₂, J₃** represent coupling between P-V and Q-θ relationships.
    /// In weak systems (high impedance), this coupling becomes significant.
    pub fn compute_jacobian(
        ybus: &YBus,
        v: &[f64],
        theta: &[f64],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();

        // Allocate Jacobian submatrices (n × n each, row-major)
        let mut dp_dtheta = vec![0.0; n * n]; // J₁
        let mut dp_dv = vec![0.0; n * n]; // J₂
        let mut dq_dtheta = vec![0.0; n * n]; // J₃
        let mut dq_dv = vec![0.0; n * n]; // J₄

        // ====================================================================
        // JACOBIAN COMPUTATION
        // ====================================================================
        //
        // The Jacobian has different formulas for diagonal vs off-diagonal elements.
        //
        // DIAGONAL (i = i): Involves sums over all neighbors k ≠ i
        //   These represent how bus i's power injection changes with its own V and θ
        //
        // OFF-DIAGONAL (i ≠ j): Simple expressions involving only buses i and j
        //   These represent how bus i's power injection changes with bus j's V and θ

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

                // Matrix index for element (i, j) in row-major format
                let idx = i * n + j;

                if i == j {
                    // ========================================================
                    // DIAGONAL ELEMENTS
                    // ========================================================
                    //
                    // Diagonal elements require summing contributions from
                    // all neighboring buses k ≠ i. This is because:
                    //
                    //   ∂P_i/∂θ_i = ∂/∂θ_i [Σⱼ V_i·V_j·(G_ij·cos(θ_i-θ_j) + B_ij·sin(θ_i-θ_j))]
                    //
                    // The θ_i appears in EVERY term of the sum, so we must sum
                    // the partial derivatives over all j.

                    // Compute sums for ∂P/∂θ and ∂Q/∂θ
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

                            // ∂(P term)/∂θ_i = V_k · (-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
                            sum_p += vk * (-g_ik * sin_ik + b_ik * cos_ik);

                            // ∂(Q term)/∂θ_i = V_k · (G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                            sum_q += vk * (g_ik * cos_ik + b_ik * sin_ik);
                        }
                    }

                    // J₁[i,i] = ∂P_i/∂θ_i = V_i · Σ_{k≠i} V_k · (-G_ik·sin + B_ik·cos)
                    dp_dtheta[idx] = vi * sum_p;

                    // J₃[i,i] = ∂Q_i/∂θ_i = V_i · Σ_{k≠i} V_k · (G_ik·cos + B_ik·sin)
                    dq_dtheta[idx] = vi * sum_q;

                    // Compute sums for ∂P/∂V and ∂Q/∂V
                    let mut sum_pv = 0.0;
                    let mut sum_qv = 0.0;

                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            sum_pv += vk * (y_ik.re * theta_ik.cos() + y_ik.im * theta_ik.sin());
                            sum_qv += vk * (y_ik.re * theta_ik.sin() - y_ik.im * theta_ik.cos());
                        }
                    }

                    // J₂[i,i] = ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i}...
                    // The 2·V_i·G_ii comes from ∂/∂V_i (V_i² · G_ii) = 2·V_i·G_ii
                    dp_dv[idx] = 2.0 * vi * g_ij + sum_pv;

                    // J₄[i,i] = ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i}...
                    // The -2·V_i·B_ii comes from ∂/∂V_i (V_i² · (-B_ii)) = -2·V_i·B_ii
                    dq_dv[idx] = -2.0 * vi * b_ij + sum_qv;
                } else {
                    // ========================================================
                    // OFF-DIAGONAL ELEMENTS
                    // ========================================================
                    //
                    // Off-diagonal elements are simpler because bus j's V and θ
                    // only appear in one term of bus i's power injection sum.
                    //
                    // ∂P_i/∂θ_j = ∂/∂θ_j [V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))]
                    //
                    // Since θ_ij = θ_i - θ_j, we have ∂θ_ij/∂θ_j = -1

                    // J₁[i,j] = ∂P_i/∂θ_j = V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dp_dtheta[idx] = vi * vj * (g_ij * sin_ij - b_ij * cos_ij);

                    // J₃[i,j] = ∂Q_i/∂θ_j = -V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dq_dtheta[idx] = -vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                    // J₂[i,j] = ∂P_i/∂V_j = V_i·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dp_dv[idx] = vi * (g_ij * cos_ij + b_ij * sin_ij);

                    // J₄[i,j] = ∂Q_i/∂V_j = V_i·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dq_dv[idx] = vi * (g_ij * sin_ij - b_ij * cos_ij);
                }
            }
        }

        (dp_dtheta, dp_dv, dq_dtheta, dq_dv)
    }
}
