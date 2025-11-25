//! Full Nonlinear AC Optimal Power Flow Solver
//!
//! This module implements a full-space AC-OPF using interior-point methods.
//! Unlike the SOCP relaxation which uses squared variables, this formulation
//! uses the original polar variables (V, θ) with explicit nonlinear constraints.
//!
//! ## Mathematical Formulation
//!
//! Variables: V_i (voltage magnitude), θ_i (angle), P_g, Q_g (generator dispatch)
//!
//! Minimize: Σ (c₀ + c₁·P_g + c₂·P_g²)
//!
//! Subject to:
//!   - Power balance: P_inj = P_gen - P_load, Q_inj = Q_gen - Q_load
//!   - AC power flow: P_i = Σ V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
//!   - Voltage limits: V_min ≤ V ≤ V_max
//!   - Generator limits: P_min ≤ P_g ≤ P_max, Q_min ≤ Q_g ≤ Q_max
//!   - Thermal limits: P_ij² + Q_ij² ≤ S_max²

mod ybus;

pub use ybus::YBusBuilder;
