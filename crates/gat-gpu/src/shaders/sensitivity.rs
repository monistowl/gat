//! Sensitivity factor computation shaders (PTDF, LODF).

/// WGSL shader for PTDF matrix computation.
///
/// PTDF[branch, bus] = (X[from_bus, bus] - X[to_bus, bus]) / branch_reactance
///
/// Where X = (B')^(-1) is the inverse susceptance matrix.
///
/// f32 with warning: denominators near zero need clamping.
pub const PTDF_SHADER: &str = r#"
struct Uniforms {
    n_branches: u32,
    n_buses: u32,
    _padding1: u32,
    _padding2: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// X matrix (B' inverse): row-major n_buses × n_buses
@group(0) @binding(1) var<storage, read> x_matrix: array<f32>;
// Branch data: [from_bus, to_bus, reactance] per branch (n_branches × 3)
@group(0) @binding(2) var<storage, read> branch_data: array<f32>;
// Output: PTDF matrix row-major n_branches × n_buses
@group(0) @binding(3) var<storage, read_write> ptdf: array<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let branch_idx = global_id.x;
    let bus_idx = global_id.y;

    if (branch_idx >= uniforms.n_branches || bus_idx >= uniforms.n_buses) {
        return;
    }

    // Get branch terminals and reactance
    let base = branch_idx * 3u;
    let from_bus = u32(branch_data[base]);
    let to_bus = u32(branch_data[base + 1u]);
    let reactance = branch_data[base + 2u];

    // Clamp small reactances to avoid division issues
    let x_clamped = max(abs(reactance), 1e-6);
    let x_signed = select(-x_clamped, x_clamped, reactance >= 0.0);

    // X[from_bus, bus_idx] - X[to_bus, bus_idx]
    let x_from = x_matrix[from_bus * uniforms.n_buses + bus_idx];
    let x_to = x_matrix[to_bus * uniforms.n_buses + bus_idx];

    ptdf[branch_idx * uniforms.n_buses + bus_idx] = (x_from - x_to) / x_signed;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::{BufferBinding, MultiBufferKernel};
    use crate::{GpuBuffer, GpuContext};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct PtdfUniforms {
        n_branches: u32,
        n_buses: u32,
        _padding1: u32,
        _padding2: u32,
    }

    #[test]
    fn test_ptdf_shader() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();

        // Simple 3-bus system
        let n_buses = 3u32;
        let n_branches = 2u32;

        // X matrix (B' inverse) - identity-like for simple test
        let x_matrix: Vec<f32> = vec![
            1.0, 0.2, 0.1, // row 0
            0.2, 1.0, 0.3, // row 1
            0.1, 0.3, 1.0, // row 2
        ];

        // Branch data: [from, to, reactance]
        // Branch 0: bus 0 -> bus 1, x=0.1
        // Branch 1: bus 1 -> bus 2, x=0.2
        let branch_data: Vec<f32> = vec![
            0.0, 1.0, 0.1, // branch 0
            1.0, 2.0, 0.2, // branch 1
        ];

        let ptdf: Vec<f32> = vec![0.0; (n_branches * n_buses) as usize];

        let uniforms = PtdfUniforms {
            n_branches,
            n_buses,
            _padding1: 0,
            _padding2: 0,
        };

        // IMPORTANT: Use new_uniform for the uniforms buffer
        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_x = GpuBuffer::new(&ctx, &x_matrix, "x_matrix");
        let buf_branch = GpuBuffer::new(&ctx, &branch_data, "branch_data");
        let buf_ptdf = GpuBuffer::new(&ctx, &ptdf, "ptdf");

        let kernel = MultiBufferKernel::new(
            &ctx,
            PTDF_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )
        .unwrap();

        // 2D dispatch for branches × buses
        // Use manual dispatch for 2D workgroups (8,8)
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        {
            let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ptdf_bind_group"),
                layout: &kernel.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf_uniforms.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buf_x.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buf_branch.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buf_ptdf.buffer.as_entire_binding(),
                    },
                ],
            });

            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&kernel.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch enough workgroups for n_branches × n_buses
            pass.dispatch_workgroups(1, 1, 1); // (8,8) is enough for 2×3
        }
        ctx.queue.submit(Some(encoder.finish()));
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        let result = buf_ptdf.read(&ctx);

        // Branch 0 (0->1, x=0.1):
        // PTDF[0,0] = (X[0,0] - X[1,0]) / 0.1 = (1.0 - 0.2) / 0.1 = 8.0
        // PTDF[0,1] = (X[0,1] - X[1,1]) / 0.1 = (0.2 - 1.0) / 0.1 = -8.0
        // PTDF[0,2] = (X[0,2] - X[1,2]) / 0.1 = (0.1 - 0.3) / 0.1 = -2.0
        assert!(
            (result[0] - 8.0).abs() < 0.01,
            "PTDF[0,0] expected 8.0, got {}",
            result[0]
        );
        assert!(
            (result[1] - (-8.0)).abs() < 0.01,
            "PTDF[0,1] expected -8.0, got {}",
            result[1]
        );
        assert!(
            (result[2] - (-2.0)).abs() < 0.01,
            "PTDF[0,2] expected -2.0, got {}",
            result[2]
        );

        // Branch 1 (1->2, x=0.2):
        // PTDF[1,0] = (X[1,0] - X[2,0]) / 0.2 = (0.2 - 0.1) / 0.2 = 0.5
        // PTDF[1,1] = (X[1,1] - X[2,1]) / 0.2 = (1.0 - 0.3) / 0.2 = 3.5
        // PTDF[1,2] = (X[1,2] - X[2,2]) / 0.2 = (0.3 - 1.0) / 0.2 = -3.5
        assert!(
            (result[3] - 0.5).abs() < 0.01,
            "PTDF[1,0] expected 0.5, got {}",
            result[3]
        );
        assert!(
            (result[4] - 3.5).abs() < 0.01,
            "PTDF[1,1] expected 3.5, got {}",
            result[4]
        );
        assert!(
            (result[5] - (-3.5)).abs() < 0.01,
            "PTDF[1,2] expected -3.5, got {}",
            result[5]
        );
    }
}
