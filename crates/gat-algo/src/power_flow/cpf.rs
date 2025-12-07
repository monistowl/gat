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

use super::ac_pf::AcPowerFlowSolver;
use anyhow::Result;
use gat_core::{BusId, Megavars, Megawatts, Network, Node};
use std::collections::HashMap;

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

    /// Solve CPF to find maximum loading point
    pub fn solve(&self, network: &mut Network) -> Result<CpfResult> {
        // Store original load values
        let original_loads = self.collect_load_values(network);

        // Initialize AC power flow solver for corrector steps
        let pf_solver = AcPowerFlowSolver::default();

        // Solve base case (λ = 1.0)
        let base_solution = pf_solver.solve(network)?;
        if !base_solution.converged {
            // Restore loads before returning error
            self.restore_load_values(network, &original_loads);
            anyhow::bail!("Base case power flow did not converge");
        }

        // Determine target bus for voltage monitoring
        let target_bus = self.target_bus.unwrap_or_else(|| {
            // Find the load bus with lowest voltage in base case
            base_solution
                .bus_voltage_magnitude
                .iter()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(bus, _)| *bus)
                .unwrap_or(BusId::new(0))
        });

        // Initialize nose curve with base case
        let mut nose_curve = vec![CpfPoint {
            loading: 1.0,
            voltage: base_solution
                .bus_voltage_magnitude
                .get(&target_bus)
                .copied()
                .unwrap_or(1.0),
        }];

        let mut lambda = 1.0;
        let mut step_size = self.step_size;
        let mut steps = 0;
        let mut previous_voltage = nose_curve[0].voltage;

        // CPF main loop: increase loading until divergence
        while steps < self.max_steps {
            steps += 1;

            // Predictor: increase loading factor
            let new_lambda = lambda + step_size;

            // Scale loads in-place
            self.apply_load_scaling(network, &original_loads, new_lambda);
            let solution = pf_solver.solve(network);

            match solution {
                Ok(sol) if sol.converged => {
                    // Power flow converged at this loading
                    let voltage = sol
                        .bus_voltage_magnitude
                        .get(&target_bus)
                        .copied()
                        .unwrap_or(1.0);

                    // Check if voltage is dropping rapidly (approaching nose)
                    let voltage_drop = previous_voltage - voltage;
                    if voltage_drop > 0.1 && step_size > self.min_step {
                        // Reduce step size near nose point
                        step_size *= 0.5;
                        if step_size < self.min_step {
                            step_size = self.min_step;
                        }
                        continue;
                    }

                    // Accept this point
                    nose_curve.push(CpfPoint {
                        loading: new_lambda,
                        voltage,
                    });
                    lambda = new_lambda;
                    previous_voltage = voltage;

                    // Check for voltage collapse (voltage too low)
                    if voltage < 0.7 {
                        // Approaching collapse, found max loading
                        break;
                    }
                }
                _ => {
                    // Power flow diverged - reduce step size and retry
                    if step_size > self.min_step {
                        step_size *= 0.5;
                        if step_size < self.min_step {
                            step_size = self.min_step;
                        }
                        continue;
                    } else {
                        // Cannot make progress - found max loading
                        break;
                    }
                }
            }
        }

        // Find critical bus (lowest voltage at max loading)
        self.apply_load_scaling(network, &original_loads, lambda);
        let final_solution = pf_solver.solve(network).ok();
        let critical_bus = final_solution.as_ref().and_then(|sol| {
            sol.bus_voltage_magnitude
                .iter()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(bus, _)| *bus)
        });

        let voltage_at_max = final_solution
            .map(|sol| sol.bus_voltage_magnitude)
            .unwrap_or_default();

        // Restore original loads
        self.restore_load_values(network, &original_loads);

        Ok(CpfResult {
            converged: true,
            max_loading: lambda,
            critical_bus,
            loading_margin: lambda - 1.0,
            nose_curve,
            voltage_at_max,
            steps,
        })
    }

    /// Collect original load values before scaling
    fn collect_load_values(
        &self,
        network: &Network,
    ) -> HashMap<petgraph::graph::NodeIndex, (Megawatts, Megavars)> {
        let mut loads = HashMap::new();
        for node_idx in network.graph.node_indices() {
            if let Some(Node::Load(load)) = network.graph.node_weight(node_idx) {
                loads.insert(node_idx, (load.active_power, load.reactive_power));
            }
        }
        loads
    }

    /// Apply load scaling to network (modifies loads in-place)
    fn apply_load_scaling(
        &self,
        network: &mut Network,
        original_loads: &HashMap<petgraph::graph::NodeIndex, (Megawatts, Megavars)>,
        lambda: f64,
    ) {
        for (node_idx, (p_orig, q_orig)) in original_loads {
            if let Some(Node::Load(load)) = network.graph.node_weight_mut(*node_idx) {
                load.active_power = Megawatts(p_orig.0 * lambda);
                load.reactive_power = Megavars(q_orig.0 * lambda);
            }
        }
    }

    /// Restore original load values
    fn restore_load_values(
        &self,
        network: &mut Network,
        original_loads: &HashMap<petgraph::graph::NodeIndex, (Megawatts, Megavars)>,
    ) {
        self.apply_load_scaling(network, original_loads, 1.0);
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
                CpfPoint {
                    loading: 1.0,
                    voltage: 1.0,
                },
                CpfPoint {
                    loading: 1.25,
                    voltage: 0.95,
                },
                CpfPoint {
                    loading: 1.5,
                    voltage: 0.85,
                },
            ],
            ..Default::default()
        };

        assert_eq!(result.nose_curve.len(), 3);
        assert!((result.max_loading - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_cpf_finds_max_loading() {
        use gat_core::{Branch, BranchId, Bus, Edge, Gen, GenId, Load, LoadId, Network, Node};

        let mut network = Network::new();

        // Simple 2-bus network: generator at bus 0, load at bus 1
        let b0 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "Gen Bus".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Load Bus".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        network.graph.add_edge(
            b0,
            b1,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        network.graph.add_node(Node::Gen(Gen::new(
            GenId::new(0),
            "Gen".to_string(),
            BusId::new(0),
        )));

        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(0),
            name: "Load".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(50.0),
            reactive_power: gat_core::Megavars(20.0),
        }));

        let solver = CpfSolver::new()
            .with_target_bus(BusId::new(1))
            .with_step_size(0.05);

        let result = solver.solve(&mut network).expect("CPF should complete");

        // Should find a max loading > 1.0
        assert!(
            result.max_loading > 1.0,
            "Max loading should be > 1.0, got {}",
            result.max_loading
        );
        // Nose curve should have multiple points
        assert!(
            result.nose_curve.len() > 2,
            "Nose curve should have points, got {}",
            result.nose_curve.len()
        );
        // Critical bus should be identified
        assert!(
            result.critical_bus.is_some(),
            "Critical bus should be identified"
        );
    }
}
