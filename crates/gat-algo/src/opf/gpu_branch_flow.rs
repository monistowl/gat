//! GPU-accelerated branch power flow calculation.
//!
//! Provides GPU acceleration for computing branch flows in ADMM and other OPF solvers.
//! Falls back to CPU when GPU is unavailable.

#[cfg(feature = "gpu")]
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "gpu")]
use gat_gpu::shaders::BRANCH_FLOW_SHADER;
#[cfg(feature = "gpu")]
use gat_gpu::{BufferBinding, GpuBuffer, GpuContext, MultiBufferKernel};

use anyhow::Result;
use gat_core::{BusId, Edge, Network, Node};
use std::collections::HashMap;

/// Uniforms for branch flow shader.
#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BranchFlowUniforms {
    n_branches: u32,
    n_buses: u32,
    base_mva: f32,
    _padding: u32,
}

/// GPU-accelerated branch flow calculator.
///
/// Wraps the WGSL compute shader for parallel branch flow computation.
/// Falls back to CPU when GPU is unavailable or disabled.
pub struct GpuBranchFlowCalculator {
    /// Cached GPU context (reused across runs)
    #[cfg(feature = "gpu")]
    gpu_context: Option<GpuContext>,
}

impl GpuBranchFlowCalculator {
    /// Create a new GPU branch flow calculator.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gpu")]
            gpu_context: None,
        }
    }

    /// Check if GPU is available.
    #[cfg(feature = "gpu")]
    pub fn is_gpu_available(&self) -> bool {
        gat_gpu::is_gpu_available()
    }

    /// Check if GPU is available (always false without feature).
    #[cfg(not(feature = "gpu"))]
    pub fn is_gpu_available(&self) -> bool {
        false
    }

    /// Initialize GPU context if not already done.
    #[cfg(feature = "gpu")]
    fn ensure_gpu_context(&mut self) -> Result<&GpuContext> {
        if self.gpu_context.is_none() {
            self.gpu_context = Some(GpuContext::new()?);
        }
        Ok(self.gpu_context.as_ref().unwrap())
    }

    /// Compute branch power flows.
    ///
    /// Uses GPU acceleration when available, otherwise falls back to CPU.
    ///
    /// # Arguments
    /// * `network` - The power network with branch definitions
    /// * `bus_voltage_mag` - Voltage magnitudes by bus name
    /// * `bus_voltage_ang` - Voltage angles by bus name (radians)
    /// * `base_mva` - Base MVA for per-unit conversion
    ///
    /// # Returns
    /// (branch_p_flow, branch_q_flow, total_losses_mw)
    pub fn compute_branch_flows(
        &mut self,
        network: &Network,
        bus_voltage_mag: &HashMap<String, f64>,
        bus_voltage_ang: &HashMap<String, f64>,
        base_mva: f64,
    ) -> Result<(HashMap<String, f64>, HashMap<String, f64>, f64)> {
        #[cfg(feature = "gpu")]
        {
            if self.is_gpu_available() {
                match self.compute_branch_flows_gpu(
                    network,
                    bus_voltage_mag,
                    bus_voltage_ang,
                    base_mva,
                ) {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        eprintln!(
                            "[gat-gpu] GPU branch flow failed, falling back to CPU: {}",
                            e
                        );
                    }
                }
            }
        }

        // CPU fallback
        Ok(self.compute_branch_flows_cpu(network, bus_voltage_mag, bus_voltage_ang, base_mva))
    }

    /// GPU implementation of branch flow calculation.
    #[cfg(feature = "gpu")]
    fn compute_branch_flows_gpu(
        &mut self,
        network: &Network,
        bus_voltage_mag: &HashMap<String, f64>,
        bus_voltage_ang: &HashMap<String, f64>,
        base_mva: f64,
    ) -> Result<(HashMap<String, f64>, HashMap<String, f64>, f64)> {
        let ctx = self.ensure_gpu_context()?;

        // Build bus index mapping
        let mut bus_name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut idx = 0;
        for node in network.graph.node_weights() {
            if let Node::Bus(bus) = node {
                bus_name_to_idx.insert(bus.name.clone(), idx);
                idx += 1;
            }
        }
        let n_buses = idx;

        // Build voltage arrays
        let mut bus_voltage: Vec<f32> = vec![0.0; n_buses * 2];
        for (name, &vm) in bus_voltage_mag {
            if let Some(&idx) = bus_name_to_idx.get(name) {
                bus_voltage[idx * 2] = vm as f32;
            }
        }
        for (name, &va) in bus_voltage_ang {
            if let Some(&idx) = bus_name_to_idx.get(name) {
                bus_voltage[idx * 2 + 1] = va as f32;
            }
        }

        // Build bus ID to name mapping
        let mut bus_id_to_name: HashMap<BusId, String> = HashMap::new();
        for node in network.graph.node_weights() {
            if let Node::Bus(bus) = node {
                bus_id_to_name.insert(bus.id.clone(), bus.name.clone());
            }
        }

        // Collect branches and build parameter arrays
        let mut branch_names: Vec<String> = Vec::new();
        let mut branch_params: Vec<f32> = Vec::new();
        let mut branch_buses: Vec<f32> = Vec::new();

        for edge in network.graph.edge_weights() {
            if let Edge::Branch(branch) = edge {
                branch_names.push(branch.name.clone());

                // Get bus indices
                let from_name = bus_id_to_name.get(&branch.from_bus);
                let to_name = bus_id_to_name.get(&branch.to_bus);

                let from_idx = from_name
                    .and_then(|n| bus_name_to_idx.get(n))
                    .copied()
                    .unwrap_or(0);
                let to_idx = to_name
                    .and_then(|n| bus_name_to_idx.get(n))
                    .copied()
                    .unwrap_or(0);

                // Branch params: [r, x, b_charging, tap, shift, status]
                branch_params.push(branch.resistance as f32);
                branch_params.push(branch.reactance as f32);
                branch_params.push(branch.charging_b.0 as f32);
                branch_params.push(branch.tap_ratio as f32);
                branch_params.push(branch.phase_shift.0 as f32);
                branch_params.push(if branch.status { 1.0 } else { 0.0 });

                // Branch buses
                branch_buses.push(from_idx as f32);
                branch_buses.push(to_idx as f32);
            }
        }

        let n_branches = branch_names.len();
        if n_branches == 0 {
            return Ok((HashMap::new(), HashMap::new(), 0.0));
        }

        // Create GPU buffers
        let uniforms = BranchFlowUniforms {
            n_branches: n_branches as u32,
            n_buses: n_buses as u32,
            base_mva: base_mva as f32,
            _padding: 0,
        };

        let branch_flow: Vec<f32> = vec![0.0; n_branches * 3];

        let buf_uniforms = GpuBuffer::new_uniform(ctx, &[uniforms], "uniforms");
        let buf_params = GpuBuffer::new(ctx, &branch_params, "branch_params");
        let buf_buses = GpuBuffer::new(ctx, &branch_buses, "branch_buses");
        let buf_voltage = GpuBuffer::new(ctx, &bus_voltage, "bus_voltage");
        let buf_flow = GpuBuffer::new(ctx, &branch_flow, "branch_flow");

        // Create and dispatch kernel
        let kernel = MultiBufferKernel::new(
            ctx,
            BRANCH_FLOW_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )?;

        kernel.dispatch(
            ctx,
            &[
                &buf_uniforms.buffer,
                &buf_params.buffer,
                &buf_buses.buffer,
                &buf_voltage.buffer,
                &buf_flow.buffer,
            ],
            n_branches as u32,
            64,
        )?;

        // Read results
        let result = buf_flow.read(ctx);

        // Convert to HashMaps
        let mut branch_p_flow = HashMap::with_capacity(n_branches);
        let mut branch_q_flow = HashMap::with_capacity(n_branches);
        let mut total_losses = 0.0f64;

        for (i, name) in branch_names.iter().enumerate() {
            let p_from = result[i * 3] as f64;
            let q_from = result[i * 3 + 1] as f64;
            let p_to = result[i * 3 + 2] as f64;

            branch_p_flow.insert(name.clone(), p_from);
            branch_q_flow.insert(name.clone(), q_from);
            total_losses += p_from + p_to;
        }

        Ok((branch_p_flow, branch_q_flow, total_losses))
    }

    /// CPU implementation of branch flow calculation (fallback).
    fn compute_branch_flows_cpu(
        &self,
        network: &Network,
        bus_voltage_mag: &HashMap<String, f64>,
        bus_voltage_ang: &HashMap<String, f64>,
        base_mva: f64,
    ) -> (HashMap<String, f64>, HashMap<String, f64>, f64) {
        // Build bus ID to name mapping
        let mut bus_id_to_name: HashMap<BusId, String> = HashMap::new();
        for node in network.graph.node_weights() {
            if let Node::Bus(bus) = node {
                bus_id_to_name.insert(bus.id.clone(), bus.name.clone());
            }
        }

        let branch_count = network
            .graph
            .edge_weights()
            .filter(|e| matches!(e, Edge::Branch(_)))
            .count();

        let mut branch_p_flow = HashMap::with_capacity(branch_count);
        let mut branch_q_flow = HashMap::with_capacity(branch_count);
        let mut total_losses = 0.0;

        for edge in network.graph.edge_weights() {
            if let Edge::Branch(branch) = edge {
                let from_name = bus_id_to_name.get(&branch.from_bus);
                let to_name = bus_id_to_name.get(&branch.to_bus);

                if let (Some(from_name), Some(to_name)) = (from_name, to_name) {
                    let vm_from = bus_voltage_mag.get(from_name).copied().unwrap_or(1.0);
                    let va_from = bus_voltage_ang.get(from_name).copied().unwrap_or(0.0);
                    let vm_to = bus_voltage_mag.get(to_name).copied().unwrap_or(1.0);
                    let va_to = bus_voltage_ang.get(to_name).copied().unwrap_or(0.0);

                    let r = branch.resistance;
                    let x = branch.reactance;
                    let tap = branch.tap_ratio;
                    let shift = branch.phase_shift.0;
                    let b_charging = branch.charging_b.0;

                    let z_sq = r * r + x * x;
                    if z_sq < 1e-12 || !branch.status {
                        continue;
                    }

                    let g = r / z_sq;
                    let b = -x / z_sq;

                    let angle_diff = va_from - va_to - shift;
                    let cos_diff = angle_diff.cos();
                    let sin_diff = angle_diff.sin();

                    let p_from = (vm_from * vm_from * g / (tap * tap))
                        - (vm_from * vm_to / tap) * (g * cos_diff + b * sin_diff);
                    let q_from = -(vm_from * vm_from * (b + b_charging / 2.0) / (tap * tap))
                        - (vm_from * vm_to / tap) * (g * sin_diff - b * cos_diff);

                    let p_to = (vm_to * vm_to * g)
                        - (vm_from * vm_to / tap) * (g * cos_diff - b * sin_diff);

                    let p_from_mw = p_from * base_mva;
                    let q_from_mvar = q_from * base_mva;
                    let p_to_mw = p_to * base_mva;

                    total_losses += p_from_mw + p_to_mw;

                    branch_p_flow.insert(branch.name.clone(), p_from_mw);
                    branch_q_flow.insert(branch.name.clone(), q_from_mvar);
                }
            }
        }

        (branch_p_flow, branch_q_flow, total_losses)
    }
}

impl Default for GpuBranchFlowCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, Kilovolts, PerUnit, Radians};

    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Add two buses
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            base_kv: Kilovolts(138.0),
            voltage_pu: PerUnit(1.0),
            angle_rad: Radians(0.0),
            vmin_pu: Some(PerUnit(0.95)),
            vmax_pu: Some(PerUnit(1.05)),
            area_id: None,
            zone_id: None,
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            base_kv: Kilovolts(138.0),
            voltage_pu: PerUnit(0.98),
            angle_rad: Radians(-0.05),
            vmin_pu: Some(PerUnit(0.95)),
            vmax_pu: Some(PerUnit(1.05)),
            area_id: None,
            zone_id: None,
        }));

        // Add a branch
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Branch1".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b: PerUnit(0.02),
                tap_ratio: 1.0,
                phase_shift: Radians(0.0),
                status: true,
                ..Default::default()
            }),
        );

        network
    }

    #[test]
    fn test_gpu_branch_flow_calculator_creation() {
        let calc = GpuBranchFlowCalculator::new();
        // Should not panic regardless of GPU presence
        let _available = calc.is_gpu_available();
    }

    #[test]
    fn test_compute_branch_flows() {
        let network = create_test_network();

        let mut bus_voltage_mag = HashMap::new();
        bus_voltage_mag.insert("Bus1".to_string(), 1.0);
        bus_voltage_mag.insert("Bus2".to_string(), 0.98);

        let mut bus_voltage_ang = HashMap::new();
        bus_voltage_ang.insert("Bus1".to_string(), 0.0);
        bus_voltage_ang.insert("Bus2".to_string(), -0.05);

        let mut calc = GpuBranchFlowCalculator::new();
        let (p_flow, q_flow, losses) = calc
            .compute_branch_flows(&network, &bus_voltage_mag, &bus_voltage_ang, 100.0)
            .unwrap();

        // Verify we got results for the branch
        assert!(p_flow.contains_key("Branch1"), "Should have Branch1 P flow");
        assert!(q_flow.contains_key("Branch1"), "Should have Branch1 Q flow");

        // Verify flows are reasonable
        let p = p_flow.get("Branch1").unwrap();
        assert!(p.abs() > 0.01, "P flow should be non-zero: {}", p);

        // Losses should be non-negative
        assert!(losses >= 0.0, "Losses should be non-negative: {}", losses);
    }
}
