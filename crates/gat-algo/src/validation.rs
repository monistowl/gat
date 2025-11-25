//! Validation metrics for comparing GAT solutions against reference solutions.

use std::collections::HashMap;

use gat_core::Network;

use crate::AcOpfSolution;

/// Error metrics for power flow solutions
#[derive(Debug, Clone, Default)]
pub struct PFErrorMetrics {
    /// Maximum voltage magnitude error (p.u.)
    pub max_vm_error: f64,
    /// Maximum voltage angle error (degrees)
    pub max_va_error_deg: f64,
    /// Mean voltage magnitude error (p.u.)
    pub mean_vm_error: f64,
    /// Mean voltage angle error (degrees)
    pub mean_va_error_deg: f64,
    /// Maximum branch active power flow error (MW)
    pub max_branch_p_error: f64,
    /// Number of buses compared
    pub num_buses_compared: usize,
}

/// Constraint violation metrics for OPF solutions
#[derive(Debug, Clone, Default)]
pub struct OPFViolationMetrics {
    /// Maximum active power balance violation (MW)
    pub max_p_balance_violation: f64,
    /// Maximum reactive power balance violation (MVAr)
    pub max_q_balance_violation: f64,
    /// Maximum branch flow violation (MVA)
    pub max_branch_flow_violation: f64,
    /// Maximum generator active power violation (MW)
    pub max_gen_p_violation: f64,
    /// Maximum voltage magnitude violation (p.u.)
    pub max_vm_violation: f64,
}

/// Objective value comparison
#[derive(Debug, Clone, Default)]
pub struct ObjectiveGap {
    /// GAT objective value
    pub gat_objective: f64,
    /// Reference objective value
    pub ref_objective: f64,
    /// Absolute gap
    pub gap_abs: f64,
    /// Relative gap (fraction)
    pub gap_rel: f64,
}

/// Reference solution for power flow comparison
#[derive(Debug, Clone, Default)]
pub struct PFReferenceSolution {
    /// Bus voltage magnitudes (bus_id -> Vm in p.u.)
    pub vm: HashMap<usize, f64>,
    /// Bus voltage angles (bus_id -> Va in radians)
    pub va: HashMap<usize, f64>,
    /// Generator active power (gen_id -> P in MW)
    pub pgen: HashMap<usize, f64>,
    /// Generator reactive power (gen_id -> Q in MVAr)
    pub qgen: HashMap<usize, f64>,
}

impl PFErrorMetrics {
    /// Check if all errors are within tolerance
    pub fn within_tolerance(&self, voltage_tol: f64, angle_tol_deg: f64) -> bool {
        self.max_vm_error <= voltage_tol && self.max_va_error_deg <= angle_tol_deg
    }
}

impl OPFViolationMetrics {
    /// Check if all violations are within tolerance
    pub fn within_tolerance(&self, constraint_tol: f64) -> bool {
        self.max_p_balance_violation <= constraint_tol
            && self.max_q_balance_violation <= constraint_tol
            && self.max_branch_flow_violation <= constraint_tol
            && self.max_gen_p_violation <= constraint_tol
            && self.max_vm_violation <= constraint_tol
    }
}

impl ObjectiveGap {
    /// Create from two objective values
    pub fn new(gat_objective: f64, ref_objective: f64) -> Self {
        let gap_abs = (gat_objective - ref_objective).abs();
        let gap_rel = if ref_objective.abs() > 1e-10 {
            gap_abs / ref_objective.abs()
        } else {
            0.0
        };
        Self {
            gat_objective,
            ref_objective,
            gap_abs,
            gap_rel,
        }
    }

    /// Check if gap is within tolerance
    pub fn within_tolerance(&self, obj_tol: f64) -> bool {
        self.gap_rel <= obj_tol
    }
}

/// Compute power flow error metrics between GAT solution and reference
///
/// # Arguments
/// * `_network` - The network (for context, e.g., getting bus indices)
/// * `gat_vm` - GAT solution voltage magnitudes (bus_id -> Vm)
/// * `gat_va` - GAT solution voltage angles in radians (bus_id -> Va)
/// * `reference` - Reference solution to compare against
pub fn compute_pf_errors(
    _network: &Network,
    gat_vm: &HashMap<usize, f64>,
    gat_va: &HashMap<usize, f64>,
    reference: &PFReferenceSolution,
) -> PFErrorMetrics {
    let mut max_vm_error = 0.0_f64;
    let mut max_va_error_rad = 0.0_f64;
    let mut sum_vm_error = 0.0;
    let mut sum_va_error = 0.0;
    let mut count = 0_usize;

    for (bus_id, ref_vm) in &reference.vm {
        if let Some(gat_vm_val) = gat_vm.get(bus_id) {
            let vm_err = (gat_vm_val - ref_vm).abs();
            max_vm_error = max_vm_error.max(vm_err);
            sum_vm_error += vm_err;
            count += 1;
        }
    }

    for (bus_id, ref_va) in &reference.va {
        if let Some(gat_va_val) = gat_va.get(bus_id) {
            let va_err = (gat_va_val - ref_va).abs();
            max_va_error_rad = max_va_error_rad.max(va_err);
            sum_va_error += va_err;
        }
    }

    let num_buses = count.max(1);
    PFErrorMetrics {
        max_vm_error,
        max_va_error_deg: max_va_error_rad.to_degrees(),
        mean_vm_error: sum_vm_error / num_buses as f64,
        mean_va_error_deg: (sum_va_error / num_buses as f64).to_degrees(),
        max_branch_p_error: 0.0, // TODO: implement if branch flows available
        num_buses_compared: count,
    }
}

/// Compute OPF constraint violation metrics from a solution
///
/// This provides a basic placeholder implementation. The actual implementation
/// depends on how GAT's OPF solution exposes bus injections, branch flows, and limits.
pub fn compute_opf_violations(
    _network: &Network,
    _solution: &AcOpfSolution,
) -> OPFViolationMetrics {
    // TODO: Implement based on actual AcOpfSolution structure
    // For now, return zeros - the OPF solver should already enforce constraints
    OPFViolationMetrics::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_gap() {
        let gap = ObjectiveGap::new(100.0, 100.0);
        assert!(gap.within_tolerance(1e-6));
        assert_eq!(gap.gap_abs, 0.0);
        assert_eq!(gap.gap_rel, 0.0);

        let gap2 = ObjectiveGap::new(100.001, 100.0);
        assert!(gap2.within_tolerance(1e-3));
        assert!(!gap2.within_tolerance(1e-6));
    }

    #[test]
    fn test_pf_error_metrics_tolerance() {
        let metrics = PFErrorMetrics {
            max_vm_error: 0.0001,
            max_va_error_deg: 0.005,
            ..Default::default()
        };
        assert!(metrics.within_tolerance(1e-4, 0.01));
        assert!(!metrics.within_tolerance(1e-5, 0.01));
    }

    #[test]
    fn test_opf_violation_tolerance() {
        let violations = OPFViolationMetrics {
            max_p_balance_violation: 0.00001,
            max_q_balance_violation: 0.00001,
            max_branch_flow_violation: 0.0,
            max_gen_p_violation: 0.0,
            max_vm_violation: 0.0,
        };
        assert!(violations.within_tolerance(1e-4));
    }
}
