//! Branch power flow calculations for thermal limit constraints.
//!
//! Computes apparent power |S_ij| = sqrt(P_ij² + Q_ij²) at branch terminals.
//! These flows are used to enforce thermal limits on transmission lines and
//! transformers.
//!
//! ## Branch Flow Equations
//!
//! For a branch connecting buses i (from) and j (to) with series admittance
//! y = g + jb, tap ratio τ, and phase shift φ:
//!
//! ```text
//! P_from = (Vi²·g/τ²) - (Vi·Vj/τ)·[g·cos(θij-φ) + b·sin(θij-φ)]
//! Q_from = -(Vi²·(b + bc/2)/τ²) - (Vi·Vj/τ)·[g·sin(θij-φ) - b·cos(θij-φ)]
//!
//! P_to = (Vj²·g) - (Vi·Vj/τ)·[g·cos(θij-φ) - b·sin(θij-φ)]
//! Q_to = -(Vj²·(b + bc/2)) + (Vi·Vj/τ)·[g·sin(θij-φ) + b·cos(θij-φ)]
//! ```
//!
//! where bc is the total line charging susceptance.

use super::{AcOpfProblem, BranchData};

/// Compute branch power flows from voltage solution.
///
/// # Arguments
/// * `problem` - AC-OPF problem with branch data
/// * `v` - Voltage magnitudes (p.u.)
/// * `theta` - Voltage angles (radians)
///
/// # Returns
/// Vector of (P_from, Q_from, P_to, Q_to) for each branch in MW/MVAr
pub fn compute_branch_flows(
    problem: &AcOpfProblem,
    v: &[f64],
    theta: &[f64],
) -> Vec<(f64, f64, f64, f64)> {
    let base_mva = problem.base_mva;

    problem.branches.iter().map(|br| {
        let vi = v[br.from_idx];
        let vj = v[br.to_idx];
        let theta_ij = theta[br.from_idx] - theta[br.to_idx];

        // Series admittance: y = 1/z = 1/(r + jx) = (r - jx)/(r² + x²)
        let z_sq = br.r * br.r + br.x * br.x;
        let g = br.r / z_sq;  // Series conductance
        let b = -br.x / z_sq; // Series susceptance (negative!)

        let tap = br.tap;
        let shift = br.shift;
        let cos_ij = (theta_ij - shift).cos();
        let sin_ij = (theta_ij - shift).sin();

        // From-bus power injection (p.u.)
        let p_from = (vi * vi * g / (tap * tap))
                   - (vi * vj / tap) * (g * cos_ij + b * sin_ij);
        let q_from = -(vi * vi * (b + br.b_charging / 2.0) / (tap * tap))
                   - (vi * vj / tap) * (g * sin_ij - b * cos_ij);

        // To-bus power injection (p.u.)
        let p_to = (vj * vj * g)
                 - (vi * vj / tap) * (g * cos_ij - b * sin_ij);
        let q_to = -(vj * vj * (b + br.b_charging / 2.0))
                 + (vi * vj / tap) * (g * sin_ij + b * cos_ij);

        // Convert to MW/MVAr
        (p_from * base_mva, q_from * base_mva, p_to * base_mva, q_to * base_mva)
    }).collect()
}

/// Compute branch apparent power magnitudes |S| = sqrt(P² + Q²).
///
/// Returns (|S_from|, |S_to|) in MVA for each branch.
pub fn compute_branch_apparent_power(
    problem: &AcOpfProblem,
    v: &[f64],
    theta: &[f64],
) -> Vec<(f64, f64)> {
    compute_branch_flows(problem, v, theta)
        .iter()
        .map(|(pf, qf, pt, qt)| {
            let s_from = (pf * pf + qf * qf).sqrt();
            let s_to = (pt * pt + qt * qt).sqrt();
            (s_from, s_to)
        })
        .collect()
}

/// Compute thermal limit violations.
///
/// Returns vector of (branch_name, violation_mva, limit_mva) for violated branches.
pub fn compute_thermal_violations(
    problem: &AcOpfProblem,
    v: &[f64],
    theta: &[f64],
) -> Vec<(String, f64, f64)> {
    let apparent = compute_branch_apparent_power(problem, v, theta);

    problem.branches.iter().zip(apparent.iter())
        .filter_map(|(br, (s_from, s_to))| {
            if br.rate_mva <= 0.0 {
                return None; // No limit
            }
            let max_s = s_from.max(*s_to);
            if max_s > br.rate_mva {
                Some((br.name.clone(), max_s - br.rate_mva, br.rate_mva))
            } else {
                None
            }
        })
        .collect()
}

/// Compute single branch flow (helper for penalty calculations).
///
/// Returns (P_from, Q_from, P_to, Q_to) in MVA.
pub fn compute_single_branch_flow(
    br: &BranchData,
    vi: f64,
    vj: f64,
    theta_ij: f64,
    base_mva: f64,
) -> (f64, f64, f64, f64) {
    let z_sq = br.r * br.r + br.x * br.x;
    let g = br.r / z_sq;
    let b = -br.x / z_sq;

    let tap = br.tap;
    let shift = br.shift;
    let cos_ij = (theta_ij - shift).cos();
    let sin_ij = (theta_ij - shift).sin();

    let p_from = (vi * vi * g / (tap * tap))
               - (vi * vj / tap) * (g * cos_ij + b * sin_ij);
    let q_from = -(vi * vi * (b + br.b_charging / 2.0) / (tap * tap))
               - (vi * vj / tap) * (g * sin_ij - b * cos_ij);

    let p_to = (vj * vj * g)
             - (vi * vj / tap) * (g * cos_ij - b * sin_ij);
    let q_to = -(vj * vj * (b + br.b_charging / 2.0))
             + (vi * vj / tap) * (g * sin_ij + b * cos_ij);

    (p_from * base_mva, q_from * base_mva, p_to * base_mva, q_to * base_mva)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_branch_flow_flat_start() {
        // With V=1, θ=0, no phase shift, flows should be zero
        // (ignoring line charging which produces small reactive power)
        let br = BranchData {
            name: "test".to_string(),
            from_idx: 0,
            to_idx: 1,
            r: 0.01,
            x: 0.1,
            b_charging: 0.0, // No line charging
            tap: 1.0,
            shift: 0.0,
            rate_mva: 100.0,
        };

        let (pf, qf, pt, qt) = compute_single_branch_flow(&br, 1.0, 1.0, 0.0, 100.0);

        // With identical voltages and zero angle difference, P should be ~0
        assert!(pf.abs() < 1e-10, "P_from should be ~0, got {}", pf);
        assert!(pt.abs() < 1e-10, "P_to should be ~0, got {}", pt);
        // Q should also be ~0 without line charging
        assert!(qf.abs() < 1e-10, "Q_from should be ~0, got {}", qf);
        assert!(qt.abs() < 1e-10, "Q_to should be ~0, got {}", qt);
    }

    #[test]
    fn test_single_branch_flow_with_angle() {
        // With angle difference, power should flow
        let br = BranchData {
            name: "test".to_string(),
            from_idx: 0,
            to_idx: 1,
            r: 0.01,
            x: 0.1,
            b_charging: 0.0,
            tap: 1.0,
            shift: 0.0,
            rate_mva: 100.0,
        };

        // θ_ij = 0.1 radians (~5.7°)
        let (pf, _qf, pt, _qt) = compute_single_branch_flow(&br, 1.0, 1.0, 0.1, 100.0);

        // Power should flow from higher angle to lower angle
        // P_from should be positive (sending power)
        assert!(pf > 0.0, "P_from should be positive, got {}", pf);
        // P_to should be negative (receiving power, but conventions vary)
        // Actually in power flow convention, both can be positive/negative
        // The key is P_from + P_to ≈ losses > 0
        assert!(pf + pt > 0.0, "Total should be losses (positive)");
    }

    #[test]
    fn test_power_conservation() {
        // Losses should be positive and small for typical parameters
        let br = BranchData {
            name: "test".to_string(),
            from_idx: 0,
            to_idx: 1,
            r: 0.01,
            x: 0.1,
            b_charging: 0.02,
            tap: 1.0,
            shift: 0.0,
            rate_mva: 100.0,
        };

        let (pf, _qf, pt, _qt) = compute_single_branch_flow(&br, 1.0, 0.98, 0.05, 100.0);

        // Losses = P_from + P_to (in power flow sign convention, sending is +)
        // Note: This depends on the sign convention used
        let losses = pf + pt;
        // Losses should be positive and small (< 5% of flow typically)
        // With these parameters, we expect some losses
        assert!(losses.abs() < pf.abs() * 0.1, "Losses should be < 10% of flow");
    }
}
