//! Branch power flow compute shader in WGSL.

/// WGSL compute shader for parallel branch power flow calculation.
///
/// Computes per-branch active and reactive power flow using AC power flow equations:
/// - P_from = (Vm_from² × G / τ²) - (Vm_from × Vm_to / τ) × (G cos(θ) + B sin(θ))
/// - Q_from = -(Vm_from² × (B + Bc/2) / τ²) - (Vm_from × Vm_to / τ) × (G sin(θ) - B cos(θ))
///
/// Where θ = θ_from - θ_to - φ (phase shift)
///
/// # Buffer Layout
/// - Uniform: n_branches (u32)
/// - Read: branch_params [n_branches × 6]: [r, x, b_charging, tap, shift, _pad] per branch
/// - Read: branch_buses [n_branches × 2]: [from_bus_idx, to_bus_idx] per branch (u32)
/// - Read: bus_voltage [n_buses × 2]: [vm, va] per bus
/// - Write: branch_flow [n_branches × 3]: [p_from, q_from, p_to] per branch
pub const BRANCH_FLOW_SHADER: &str = r#"
struct Uniforms {
    n_branches: u32,
    n_buses: u32,
    base_mva: f32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Branch parameters: [r, x, b_charging, tap, shift, status] per branch
@group(0) @binding(1) var<storage, read> branch_params: array<f32>;

// Branch connectivity: [from_bus_idx, to_bus_idx] per branch (as f32 for simplicity)
@group(0) @binding(2) var<storage, read> branch_buses: array<f32>;

// Bus voltages: [vm, va] per bus
@group(0) @binding(3) var<storage, read> bus_voltage: array<f32>;

// Output: [p_from_mw, q_from_mvar, p_to_mw] per branch
@group(0) @binding(4) var<storage, read_write> branch_flow: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let branch_idx = global_id.x;

    if (branch_idx >= uniforms.n_branches) {
        return;
    }

    // Read branch parameters (6 floats per branch)
    let param_base = branch_idx * 6u;
    let r = branch_params[param_base + 0u];
    let x = branch_params[param_base + 1u];
    let b_charging = branch_params[param_base + 2u];
    let tap = branch_params[param_base + 3u];
    let shift = branch_params[param_base + 4u];
    let status = branch_params[param_base + 5u];

    // Read bus indices (2 floats per branch, stored as f32)
    let bus_base = branch_idx * 2u;
    let from_idx = u32(branch_buses[bus_base + 0u]);
    let to_idx = u32(branch_buses[bus_base + 1u]);

    // Read bus voltages (2 floats per bus: vm, va)
    let from_vm = bus_voltage[from_idx * 2u + 0u];
    let from_va = bus_voltage[from_idx * 2u + 1u];
    let to_vm = bus_voltage[to_idx * 2u + 0u];
    let to_va = bus_voltage[to_idx * 2u + 1u];

    // Compute admittance
    let z_sq = r * r + x * x;

    // Output base
    let out_base = branch_idx * 3u;

    // Handle zero impedance or offline branches
    if (z_sq < 1e-12 || status < 0.5) {
        branch_flow[out_base + 0u] = 0.0;
        branch_flow[out_base + 1u] = 0.0;
        branch_flow[out_base + 2u] = 0.0;
        return;
    }

    let g = r / z_sq;
    let b = -x / z_sq;

    // Angle difference with phase shift
    let angle_diff = from_va - to_va - shift;
    let cos_diff = cos(angle_diff);
    let sin_diff = sin(angle_diff);

    // From-bus power injection (per-unit)
    let p_from_pu = (from_vm * from_vm * g / (tap * tap))
        - (from_vm * to_vm / tap) * (g * cos_diff + b * sin_diff);
    let q_from_pu = -(from_vm * from_vm * (b + b_charging / 2.0) / (tap * tap))
        - (from_vm * to_vm / tap) * (g * sin_diff - b * cos_diff);

    // To-bus power injection (for loss calculation)
    let p_to_pu = (to_vm * to_vm * g)
        - (from_vm * to_vm / tap) * (g * cos_diff - b * sin_diff);

    // Convert to MW/MVAr
    branch_flow[out_base + 0u] = p_from_pu * uniforms.base_mva;
    branch_flow[out_base + 1u] = q_from_pu * uniforms.base_mva;
    branch_flow[out_base + 2u] = p_to_pu * uniforms.base_mva;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BufferBinding, GpuBuffer, GpuContext, MultiBufferKernel};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct BranchFlowUniforms {
        n_branches: u32,
        n_buses: u32,
        base_mva: f32,
        _padding: u32,
    }

    #[test]
    fn test_branch_flow_shader_basic() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();

        // Test a simple 2-bus, 1-branch case
        // Branch: R=0.01, X=0.1, B_charging=0.02, tap=1.0, shift=0.0, status=1.0
        // Bus 0: Vm=1.0, Va=0.0
        // Bus 1: Vm=0.98, Va=-0.05 rad

        let uniforms = BranchFlowUniforms {
            n_branches: 1,
            n_buses: 2,
            base_mva: 100.0,
            _padding: 0,
        };

        // Branch params: [r, x, b_charging, tap, shift, status]
        let branch_params: Vec<f32> = vec![0.01, 0.1, 0.02, 1.0, 0.0, 1.0];

        // Branch buses: [from_idx, to_idx]
        let branch_buses: Vec<f32> = vec![0.0, 1.0];

        // Bus voltages: [vm, va] for each bus
        let bus_voltage: Vec<f32> = vec![1.0, 0.0, 0.98, -0.05];

        // Output buffer
        let branch_flow: Vec<f32> = vec![0.0; 3];

        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_params = GpuBuffer::new(&ctx, &branch_params, "branch_params");
        let buf_buses = GpuBuffer::new(&ctx, &branch_buses, "branch_buses");
        let buf_voltage = GpuBuffer::new(&ctx, &bus_voltage, "bus_voltage");
        let buf_flow = GpuBuffer::new(&ctx, &branch_flow, "branch_flow");

        let kernel = MultiBufferKernel::new(
            &ctx,
            BRANCH_FLOW_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )
        .unwrap();

        kernel
            .dispatch(
                &ctx,
                &[
                    &buf_uniforms.buffer,
                    &buf_params.buffer,
                    &buf_buses.buffer,
                    &buf_voltage.buffer,
                    &buf_flow.buffer,
                ],
                1, // 1 branch
                64,
            )
            .unwrap();

        let result = buf_flow.read(&ctx);

        // Verify results are reasonable (non-zero flows for a real branch)
        // P_from should be positive (power flowing from bus 0 to bus 1)
        assert!(
            result[0].abs() > 0.01,
            "P_from should be non-zero: {}",
            result[0]
        );
        // Q_from can be positive or negative depending on reactive flow
        // P_to should be roughly negative of P_from minus losses
        assert!(
            result[2].abs() > 0.01,
            "P_to should be non-zero: {}",
            result[2]
        );

        // Verify losses are positive (P_from + P_to > 0 in our convention means loss)
        let losses = result[0] + result[2];
        assert!(losses >= 0.0, "Losses should be non-negative: {}", losses);
    }
}
