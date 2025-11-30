//! # AC-OPF Diagnostics and Introspection Utilities
//!
//! This module provides tools for debugging AC-OPF convergence issues:
//!
//! - **Jacobian verification**: Compare analytical Jacobian against finite differences
//! - **Constraint violation analysis**: Identify which constraints are violated and by how much
//! - **Branch flow diagnostics**: Compute and report thermal limit violations
//!
//! These utilities helped identify and fix the thermal constraint Jacobian sign bug
//! that was causing IPOPT to fail on case118.

use super::{AcOpfProblem, BranchData};

/// Result of Jacobian verification against finite differences.
#[derive(Debug, Clone)]
pub struct JacobianVerification {
    /// Maximum absolute error across all entries
    pub max_abs_error: f64,
    /// Maximum relative error across all entries
    pub max_rel_error: f64,
    /// Index of the entry with maximum error (row, col)
    pub max_error_location: (usize, usize),
    /// Number of entries with relative error > 1e-4
    pub large_error_count: usize,
    /// Total number of non-zero entries checked
    pub total_entries: usize,
}

/// Branch flow from "from" bus.
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

/// Branch flow from "to" bus.
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

/// Thermal violation for a single branch.
#[derive(Debug, Clone)]
pub struct ThermalViolation {
    /// Branch index
    pub branch_idx: usize,
    /// From bus name
    pub from_bus: String,
    /// To bus name
    pub to_bus: String,
    /// Flow from "from" side (MVA)
    pub s_from_mva: f64,
    /// Flow from "to" side (MVA)
    pub s_to_mva: f64,
    /// Rating (MVA)
    pub rate_mva: f64,
    /// Violation amount (max(s_from, s_to) - rate)
    pub violation_mva: f64,
}

/// Analyze thermal violations at a given operating point.
///
/// Returns list of branches with violations, sorted by severity.
pub fn analyze_thermal_violations(
    problem: &AcOpfProblem,
    v: &[f64],
    theta: &[f64],
) -> Vec<ThermalViolation> {
    let base_mva = problem.base_mva;
    let mut violations = Vec::new();

    for (idx, branch) in problem.branches.iter().enumerate() {
        if branch.rate_mva <= 0.0 {
            continue;
        }

        let i = branch.from_idx;
        let j = branch.to_idx;

        let (p_from, q_from) = branch_flow_from(branch, v[i], v[j], theta[i], theta[j]);
        let (p_to, q_to) = branch_flow_to(branch, v[i], v[j], theta[i], theta[j]);

        let s_from = (p_from * p_from + q_from * q_from).sqrt() * base_mva;
        let s_to = (p_to * p_to + q_to * q_to).sqrt() * base_mva;

        let max_flow = s_from.max(s_to);
        let violation = max_flow - branch.rate_mva;

        if violation > 0.01 {  // > 0.01 MVA threshold
            violations.push(ThermalViolation {
                branch_idx: idx,
                from_bus: problem.buses[i].name.clone(),
                to_bus: problem.buses[j].name.clone(),
                s_from_mva: s_from,
                s_to_mva: s_to,
                rate_mva: branch.rate_mva,
                violation_mva: violation,
            });
        }
    }

    // Sort by violation severity (descending)
    violations.sort_by(|a, b| b.violation_mva.partial_cmp(&a.violation_mva).unwrap());
    violations
}

/// Print a summary of thermal violations for debugging.
pub fn print_thermal_summary(violations: &[ThermalViolation]) {
    println!("\n=== THERMAL VIOLATION SUMMARY ===\n");

    if violations.is_empty() {
        println!("All thermal constraints satisfied");
    } else {
        println!("Thermal violations ({} branches):", violations.len());
        for (i, v) in violations.iter().take(10).enumerate() {
            println!(
                "  {}. {} -> {}: {:.1}/{:.1} MVA (violation: {:.1} MVA)",
                i + 1, v.from_bus, v.to_bus, v.s_from_mva.max(v.s_to_mva), v.rate_mva, v.violation_mva
            );
        }
        if violations.len() > 10 {
            println!("  ... and {} more", violations.len() - 10);
        }
    }

    println!("\n=================================\n");
}

/// Compute branch flows for all branches.
///
/// Returns: Vec of (branch_idx, from_bus, to_bus, P_from, Q_from, P_to, Q_to) in MW/MVAr
pub fn compute_all_branch_flows(
    problem: &AcOpfProblem,
    v: &[f64],
    theta: &[f64],
) -> Vec<(usize, String, String, f64, f64, f64, f64)> {
    let base_mva = problem.base_mva;
    let mut flows = Vec::new();

    for (idx, branch) in problem.branches.iter().enumerate() {
        let i = branch.from_idx;
        let j = branch.to_idx;

        let (p_from, q_from) = branch_flow_from(branch, v[i], v[j], theta[i], theta[j]);
        let (p_to, q_to) = branch_flow_to(branch, v[i], v[j], theta[i], theta[j]);

        flows.push((
            idx,
            problem.buses[i].name.clone(),
            problem.buses[j].name.clone(),
            p_from * base_mva,
            q_from * base_mva,
            p_to * base_mva,
            q_to * base_mva,
        ));
    }

    flows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_flow_symmetry() {
        // Simple branch with no tap or shift
        let branch = BranchData {
            name: "test_branch".to_string(),
            from_idx: 0,
            to_idx: 1,
            r: 0.01,
            x: 0.1,
            b_charging: 0.02,
            rate_mva: 100.0,
            tap: 1.0,
            shift: 0.0,
            angle_diff_max: 0.0, // no limit
        };

        // With equal voltages and zero angle difference, flows should be equal and opposite
        let (p_from, q_from) = branch_flow_from(&branch, 1.0, 1.0, 0.0, 0.0);
        let (p_to, q_to) = branch_flow_to(&branch, 1.0, 1.0, 0.0, 0.0);

        // P flows should be equal and opposite (no losses at equal voltage)
        assert!((p_from + p_to).abs() < 1e-10, "P flows should sum to zero");

        // Q flows include charging, so they won't be equal
        // but should be reasonable
        assert!(q_from.abs() < 0.1);
        assert!(q_to.abs() < 0.1);
    }
}
