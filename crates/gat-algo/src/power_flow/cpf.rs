//! Continuation Power Flow (CPF) for Voltage Stability Analysis
//!
//! CPF traces the PV curve (nose curve) as system loading increases, finding the
//! maximum loading point before voltage collapse. Uses predictor-corrector iteration.
//!
//! ## Algorithm
//!
//! 1. **Predictor**: Take a step along the tangent direction
//! 2. **Corrector**: Solve power flow with continuation parameter fixed
//! 3. Repeat until nose point detected (dλ/ds < 0)
//!
//! ## References
//!
//! - Ajjarapu & Christy (1992): "The continuation power flow: A tool for steady
//!   state voltage stability analysis"
//!   IEEE Trans. Power Systems, 7(1), 416-423
//!   DOI: [10.1109/59.141737](https://doi.org/10.1109/59.141737)

use std::collections::HashMap;
use gat_core::BusId;

/// A point on the PV (nose) curve
#[derive(Debug, Clone, Default)]
pub struct CpfPoint {
    /// Loading parameter λ (1.0 = base case)
    pub loading: f64,
    /// Voltage magnitude at critical bus (p.u.)
    pub voltage: f64,
}

/// Result of continuation power flow analysis
#[derive(Debug, Clone, Default)]
pub struct CpfResult {
    /// Did the CPF converge to find the nose point?
    pub converged: bool,
    /// Maximum loading factor λ_max before voltage collapse
    pub max_loading: f64,
    /// Bus with lowest voltage at max loading (critical bus)
    pub critical_bus: Option<BusId>,
    /// Loading margin (λ_max - 1.0) as fraction of base load
    pub loading_margin: f64,
    /// Complete nose curve data for plotting
    pub nose_curve: Vec<CpfPoint>,
    /// Voltage magnitudes at each bus at max loading
    pub voltage_at_max: HashMap<BusId, f64>,
    /// Number of CPF steps taken
    pub steps: usize,
}

/// CPF solver configuration
#[derive(Debug, Clone)]
pub struct CpfSolver {
    /// Step size for predictor (initial)
    pub step_size: f64,
    /// Minimum step size
    pub min_step: f64,
    /// Maximum step size
    pub max_step: f64,
    /// Convergence tolerance for corrector
    pub tolerance: f64,
    /// Maximum corrector iterations per step
    pub max_corrector_iter: usize,
    /// Maximum number of CPF steps
    pub max_steps: usize,
    /// Target bus for voltage monitoring (optional)
    pub target_bus: Option<BusId>,
}

impl Default for CpfSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CpfSolver {
    pub fn new() -> Self {
        Self {
            step_size: 0.1,
            min_step: 0.001,
            max_step: 0.5,
            tolerance: 1e-6,
            max_corrector_iter: 20,
            max_steps: 100,
            target_bus: None,
        }
    }

    pub fn with_step_size(mut self, step: f64) -> Self {
        self.step_size = step;
        self
    }

    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    pub fn with_target_bus(mut self, bus: BusId) -> Self {
        self.target_bus = Some(bus);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::BusId;

    #[test]
    fn test_cpf_result_stores_nose_curve() {
        let result = CpfResult {
            converged: true,
            max_loading: 1.5,
            critical_bus: Some(BusId::new(2)),
            nose_curve: vec![
                CpfPoint { loading: 1.0, voltage: 1.0 },
                CpfPoint { loading: 1.25, voltage: 0.95 },
                CpfPoint { loading: 1.5, voltage: 0.85 },
            ],
            ..Default::default()
        };

        assert_eq!(result.nose_curve.len(), 3);
        assert!((result.max_loading - 1.5).abs() < 0.01);
    }
}
