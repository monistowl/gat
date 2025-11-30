//! Validation metrics for comparing GAT solutions against reference solutions.

use std::collections::HashMap;

use gat_core::Network;

use crate::AcOpfSolution;

/// Expected unit for angle values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleUnit {
    Radians,
    Degrees,
}

/// Result of angle unit sanity check
#[derive(Debug, Clone)]
pub struct AngleSanityResult {
    /// Whether the angles appear to be in the expected unit
    pub likely_correct: bool,
    /// Detected unit based on heuristics
    pub detected_unit: AngleUnit,
    /// Expected unit
    pub expected_unit: AngleUnit,
    /// Maximum absolute angle value seen
    pub max_abs_value: f64,
    /// Number of values that look suspicious
    pub suspicious_count: usize,
    /// Total number of values checked
    pub total_count: usize,
}

impl AngleSanityResult {
    /// Log a warning if angles appear to be in wrong unit
    pub fn warn_if_suspicious(&self, context: &str) {
        if !self.likely_correct {
            eprintln!(
                "WARNING: {}: Angles may be in {:?} but {:?} expected. \
                 Max |angle|={:.4}, {}/{} values look suspicious. \
                 Check unit conversions!",
                context,
                self.detected_unit,
                self.expected_unit,
                self.max_abs_value,
                self.suspicious_count,
                self.total_count
            );
        }
    }
}

/// Check if angle values appear to be in the expected unit.
///
/// Heuristics:
/// - Radians: typical power flow angles are small, usually |θ| < π/2 (90°)
///   If we see many values > 1.5 (~86°), they might be degrees
/// - Degrees: values typically range -180 to 180
///   If most values are < 1.0, they might be radians
///
/// Returns a sanity check result with diagnostics.
pub fn check_angle_units(angles: &HashMap<usize, f64>, expected: AngleUnit) -> AngleSanityResult {
    if angles.is_empty() {
        return AngleSanityResult {
            likely_correct: true,
            detected_unit: expected,
            expected_unit: expected,
            max_abs_value: 0.0,
            suspicious_count: 0,
            total_count: 0,
        };
    }

    let mut max_abs = 0.0_f64;
    let mut large_count = 0_usize; // |θ| > 1.5 (suspicious for radians)
    let mut small_count = 0_usize; // |θ| < 1.0 (suspicious for degrees)

    for &angle in angles.values() {
        let abs_angle = angle.abs();
        max_abs = max_abs.max(abs_angle);

        if abs_angle > 1.5 {
            large_count += 1;
        }
        if abs_angle < 1.0 {
            small_count += 1;
        }
    }

    let total = angles.len();
    let large_fraction = large_count as f64 / total as f64;
    let small_fraction = small_count as f64 / total as f64;

    // Determine what unit the data looks like
    let detected_unit = if large_fraction > 0.3 {
        // Many large values -> probably degrees
        AngleUnit::Degrees
    } else if small_fraction > 0.9 && max_abs < 2.0 {
        // Almost all small values and max < 2 -> probably radians
        AngleUnit::Radians
    } else {
        // Ambiguous, assume expected is correct
        expected
    };

    let (likely_correct, suspicious_count) = match expected {
        AngleUnit::Radians => {
            // If expecting radians but detected degrees, flag it
            let correct = detected_unit == AngleUnit::Radians;
            (correct, large_count)
        }
        AngleUnit::Degrees => {
            // If expecting degrees but all values tiny, flag it
            let correct = detected_unit == AngleUnit::Degrees || max_abs > 1.5;
            (correct, if correct { 0 } else { small_count })
        }
    };

    AngleSanityResult {
        likely_correct,
        detected_unit,
        expected_unit: expected,
        max_abs_value: max_abs,
        suspicious_count,
        total_count: total,
    }
}

/// Convenience function to check and warn about angle units
pub fn warn_if_wrong_angle_unit(
    angles: &HashMap<usize, f64>,
    expected: AngleUnit,
    context: &str,
) -> bool {
    let result = check_angle_units(angles, expected);
    result.warn_if_suspicious(context);
    result.likely_correct
}

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
/// * `reference` - Reference solution to compare against (angles in radians)
///
/// # Panics
/// Will log warnings if angle values appear to be in the wrong unit.
pub fn compute_pf_errors(
    _network: &Network,
    gat_vm: &HashMap<usize, f64>,
    gat_va: &HashMap<usize, f64>,
    reference: &PFReferenceSolution,
) -> PFErrorMetrics {
    // Sanity check: both angle sets should be in radians
    warn_if_wrong_angle_unit(gat_va, AngleUnit::Radians, "GAT solution angles");
    warn_if_wrong_angle_unit(
        &reference.va,
        AngleUnit::Radians,
        "Reference solution angles",
    );

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

/// Compute OPF constraint violation metrics from an OpfSolution
///
/// Violations are computed as the amount by which constraints are exceeded:
/// - Voltage: max(0, V - Vmax) + max(0, Vmin - V)
/// - Generator P: max(0, P - Pmax) + max(0, Pmin - P)
/// - Branch flow: max(0, S - Smax) where S = sqrt(P² + Q²)
///
/// Returns metrics in the original units (p.u. for voltage, MW/MVAr for power).
pub fn compute_opf_violations_from_solution(
    network: &Network,
    solution: &crate::OpfSolution,
) -> OPFViolationMetrics {
    use gat_core::Node;

    let mut max_vm_violation = 0.0_f64;
    let mut max_gen_p_violation = 0.0_f64;
    let mut max_branch_flow_violation = 0.0_f64;

    // Check voltage violations
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            let bus_name = bus.name.clone();
            if let Some(&vm) = solution.bus_voltage_mag.get(&bus_name) {
                // Check upper bound
                if let Some(vmax) = bus.vmax_pu {
                    if vm > vmax {
                        max_vm_violation = max_vm_violation.max(vm - vmax);
                    }
                }
                // Check lower bound
                if let Some(vmin) = bus.vmin_pu {
                    if vm < vmin {
                        max_vm_violation = max_vm_violation.max(vmin - vm);
                    }
                }
            }
        }
    }

    // Check generator P violations
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            if !gen.status {
                continue;
            }
            let gen_name = gen.name.clone();
            if let Some(&pg) = solution.generator_p.get(&gen_name) {
                // Check upper bound (if not infinite)
                if gen.pmax_mw.is_finite() && pg > gen.pmax_mw {
                    max_gen_p_violation = max_gen_p_violation.max(pg - gen.pmax_mw);
                }
                // Check lower bound
                if pg < gen.pmin_mw {
                    max_gen_p_violation = max_gen_p_violation.max(gen.pmin_mw - pg);
                }
            }
        }
    }

    // Check branch flow violations
    for edge in network.graph.edge_weights() {
        use gat_core::Edge;

        // Only branches (not transformers) have thermal limits in our model
        if let Edge::Branch(branch) = edge {
            let branch_name = branch.name.clone();
            let p_flow = solution
                .branch_p_flow
                .get(&branch_name)
                .copied()
                .unwrap_or(0.0);
            let q_flow = solution
                .branch_q_flow
                .get(&branch_name)
                .copied()
                .unwrap_or(0.0);
            let s_flow = (p_flow.powi(2) + q_flow.powi(2)).sqrt();

            // Get thermal limit (prefer s_max_mva, fall back to rating_a_mva)
            let s_limit = branch.s_max_mva.or(branch.rating_a_mva);
            if let Some(s_max) = s_limit {
                if s_max > 0.0 && s_flow > s_max {
                    max_branch_flow_violation = max_branch_flow_violation.max(s_flow - s_max);
                }
            }
        }
    }

    OPFViolationMetrics {
        max_p_balance_violation: 0.0, // Would require computing nodal injection balance
        max_q_balance_violation: 0.0, // Would require computing nodal injection balance
        max_branch_flow_violation,
        max_gen_p_violation,
        max_vm_violation,
    }
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

    #[test]
    fn test_angle_sanity_check_radians() {
        // Typical power flow angles in radians (small values)
        let radians: HashMap<usize, f64> =
            [(1, 0.0), (2, -0.17), (3, -0.30), (4, -0.33), (5, -0.32)]
                .into_iter()
                .collect();

        let result = check_angle_units(&radians, AngleUnit::Radians);
        assert!(result.likely_correct, "Should detect radians correctly");
        assert_eq!(result.detected_unit, AngleUnit::Radians);
    }

    #[test]
    fn test_angle_sanity_check_degrees_when_expecting_radians() {
        // Angles in degrees (larger values) but we expect radians
        let degrees: HashMap<usize, f64> = [
            (1, 0.0),
            (2, -9.7),  // ~-0.17 rad
            (3, -17.2), // ~-0.30 rad
            (4, -19.2), // ~-0.33 rad
            (5, -18.4), // ~-0.32 rad
        ]
        .into_iter()
        .collect();

        let result = check_angle_units(&degrees, AngleUnit::Radians);
        assert!(
            !result.likely_correct,
            "Should detect degrees when radians expected"
        );
        assert_eq!(result.detected_unit, AngleUnit::Degrees);
    }

    #[test]
    fn test_angle_sanity_check_empty() {
        let empty: HashMap<usize, f64> = HashMap::new();
        let result = check_angle_units(&empty, AngleUnit::Radians);
        assert!(result.likely_correct, "Empty should pass");
    }

    #[test]
    fn test_angle_sanity_check_mixed() {
        // Edge case: some large angles that could be valid radians (approaching π)
        let mixed: HashMap<usize, f64> = [(1, 0.0), (2, -0.5), (3, -1.0), (4, -1.2), (5, 0.8)]
            .into_iter()
            .collect();

        let result = check_angle_units(&mixed, AngleUnit::Radians);
        // These are still plausible radians (max ~69°)
        assert!(result.likely_correct, "Should accept borderline radians");
    }
}
