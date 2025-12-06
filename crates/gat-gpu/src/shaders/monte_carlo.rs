//! Monte Carlo reliability simulation shaders.

/// WGSL shader for batch capacity adequacy check.
///
/// For each scenario, checks if sum(available_gen) >= demand.
/// Output: 1.0 if adequate, 0.0 if shortfall.
///
/// This is f32-safe: only needs capacity > demand comparison.
pub const CAPACITY_CHECK_SHADER: &str = r#"
struct Uniforms {
    n_scenarios: u32,
    n_generators: u32,
    demand: f32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// Generator capacities (n_generators)
@group(0) @binding(1) var<storage, read> gen_capacity: array<f32>;
// Outage matrix: 1.0 = online, 0.0 = offline (n_scenarios * n_generators)
@group(0) @binding(2) var<storage, read> outage_state: array<f32>;
// Output: 1.0 = adequate, 0.0 = shortfall (n_scenarios)
@group(0) @binding(3) var<storage, read_write> adequate: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let scenario_idx = global_id.x;

    if (scenario_idx >= uniforms.n_scenarios) {
        return;
    }

    var available: f32 = 0.0;
    let base_idx = scenario_idx * uniforms.n_generators;

    for (var g = 0u; g < uniforms.n_generators; g = g + 1u) {
        let is_online = outage_state[base_idx + g];
        available = available + gen_capacity[g] * is_online;
    }

    adequate[scenario_idx] = select(0.0, 1.0, available >= uniforms.demand);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GpuBuffer, GpuContext};
    use crate::kernels::{BufferBinding, MultiBufferKernel};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct CapacityUniforms {
        n_scenarios: u32,
        n_generators: u32,
        demand: f32,
        _padding: u32,
    }

    #[test]
    fn test_capacity_check_shader() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();

        // 2 generators: 100 MW and 50 MW
        let gen_capacity: Vec<f32> = vec![100.0, 50.0];

        // 4 scenarios:
        // 0: both online (150 MW available)
        // 1: gen 0 offline (50 MW available)
        // 2: gen 1 offline (100 MW available)
        // 3: both offline (0 MW available)
        let outage_state: Vec<f32> = vec![
            1.0, 1.0,  // scenario 0
            0.0, 1.0,  // scenario 1
            1.0, 0.0,  // scenario 2
            0.0, 0.0,  // scenario 3
        ];

        let adequate: Vec<f32> = vec![0.0; 4];

        let uniforms = CapacityUniforms {
            n_scenarios: 4,
            n_generators: 2,
            demand: 80.0,  // Need 80 MW
            _padding: 0,
        };

        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_capacity = GpuBuffer::new(&ctx, &gen_capacity, "gen_capacity");
        let buf_outage = GpuBuffer::new(&ctx, &outage_state, "outage_state");
        let buf_adequate = GpuBuffer::new(&ctx, &adequate, "adequate");

        let kernel = MultiBufferKernel::new(
            &ctx,
            CAPACITY_CHECK_SHADER,
            "main",
            &[
                BufferBinding::Uniform,    // uniforms
                BufferBinding::ReadOnly,   // gen_capacity
                BufferBinding::ReadOnly,   // outage_state
                BufferBinding::ReadWrite,  // adequate
            ],
        ).unwrap();

        kernel.dispatch(
            &ctx,
            &[&buf_uniforms.buffer, &buf_capacity.buffer, &buf_outage.buffer, &buf_adequate.buffer],
            4,  // 4 scenarios
            64,
        ).unwrap();

        let result = buf_adequate.read(&ctx);
        // Scenario 0: 150 >= 80 -> adequate (1.0)
        // Scenario 1: 50 < 80 -> shortfall (0.0)
        // Scenario 2: 100 >= 80 -> adequate (1.0)
        // Scenario 3: 0 < 80 -> shortfall (0.0)
        assert_eq!(result, vec![1.0, 0.0, 1.0, 0.0]);
    }
}
