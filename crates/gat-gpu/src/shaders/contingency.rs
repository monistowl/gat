//! N-1 contingency screening shaders using LODF.

/// WGSL shader for N-1 contingency flow estimation using LODF.
///
/// For each monitored branch ℓ and each contingency (outaged branch m):
/// flow_post[ℓ] ≈ flow_pre[ℓ] + LODF[ℓ,m] × flow_pre[m]
///
/// Output: estimated post-contingency flow for each (contingency, monitored) pair.
/// f32-safe: LODF approximation already has ~5% error.
pub const LODF_SCREENING_SHADER: &str = r#"
struct Uniforms {
    n_branches: u32,
    n_contingencies: u32,
    _padding1: u32,
    _padding2: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// Pre-contingency branch flows (n_branches)
@group(0) @binding(1) var<storage, read> flow_pre: array<f32>;
// LODF matrix in row-major: LODF[l * n_branches + m] (n_branches × n_branches)
@group(0) @binding(2) var<storage, read> lodf: array<f32>;
// Which branches are contingencies (indices into flow_pre)
@group(0) @binding(3) var<storage, read> contingency_branches: array<u32>;
// Output: post-contingency flows (n_contingencies × n_branches)
@group(0) @binding(4) var<storage, read_write> flow_post: array<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let contingency_idx = global_id.x;  // Which contingency
    let branch_idx = global_id.y;       // Which monitored branch

    if (contingency_idx >= uniforms.n_contingencies || branch_idx >= uniforms.n_branches) {
        return;
    }

    let m = contingency_branches[contingency_idx];  // Outaged branch
    let l = branch_idx;                              // Monitored branch

    // Skip if monitoring the outaged branch itself
    if (l == m) {
        flow_post[contingency_idx * uniforms.n_branches + l] = 0.0;
        return;
    }

    let lodf_lm = lodf[l * uniforms.n_branches + m];
    let flow_m = flow_pre[m];
    let flow_l = flow_pre[l];

    // Post-contingency flow estimate
    flow_post[contingency_idx * uniforms.n_branches + l] = flow_l + lodf_lm * flow_m;
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
    struct LodfUniforms {
        n_branches: u32,
        n_contingencies: u32,
        _padding1: u32,
        _padding2: u32,
    }

    #[test]
    fn test_lodf_screening_shader() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();

        // 3 branches, 2 contingencies (branch 0 and branch 1 outaged)
        let n_branches = 3u32;
        let n_contingencies = 2u32;

        // Pre-contingency flows: [100, 50, 75] MW
        let flow_pre: Vec<f32> = vec![100.0, 50.0, 75.0];

        // LODF matrix (3x3):
        // When branch m trips, branch l sees: flow_l + LODF[l,m] * flow_m
        //       m=0    m=1    m=2
        // l=0  -1.0    0.3    0.1
        // l=1   0.4   -1.0    0.2
        // l=2   0.2    0.1   -1.0
        let lodf: Vec<f32> = vec![
            -1.0, 0.3, 0.1, // row 0
            0.4, -1.0, 0.2, // row 1
            0.2, 0.1, -1.0, // row 2
        ];

        // Contingency branches: [0, 1] (trip branch 0, then trip branch 1)
        let contingency_branches: Vec<u32> = vec![0, 1];

        let flow_post: Vec<f32> = vec![0.0; (n_contingencies * n_branches) as usize];

        let uniforms = LodfUniforms {
            n_branches,
            n_contingencies,
            _padding1: 0,
            _padding2: 0,
        };

        // IMPORTANT: Use new_uniform for the uniforms buffer
        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_flow_pre = GpuBuffer::new(&ctx, &flow_pre, "flow_pre");
        let buf_lodf = GpuBuffer::new(&ctx, &lodf, "lodf");
        let buf_contingencies = GpuBuffer::new(&ctx, &contingency_branches, "contingencies");
        let buf_flow_post = GpuBuffer::new(&ctx, &flow_post, "flow_post");

        let kernel = MultiBufferKernel::new(
            &ctx,
            LODF_SCREENING_SHADER,
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

        // Dispatch with 2D workgroups: (n_contingencies, n_branches)
        // Using workgroup_size(8,8), we need dispatch_workgroups(1, 1, 1) for this small case
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        {
            let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lodf_bind_group"),
                layout: &kernel.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf_uniforms.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buf_flow_pre.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buf_lodf.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buf_contingencies.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buf_flow_post.buffer.as_entire_binding(),
                    },
                ],
            });

            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&kernel.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        let result = buf_flow_post.read(&ctx);

        // Contingency 0 (branch 0 trips):
        // Branch 0: 0 (outaged)
        // Branch 1: 50 + 0.4 * 100 = 90
        // Branch 2: 75 + 0.2 * 100 = 95

        // Contingency 1 (branch 1 trips):
        // Branch 0: 100 + 0.3 * 50 = 115
        // Branch 1: 0 (outaged)
        // Branch 2: 75 + 0.1 * 50 = 80

        assert_eq!(result[0], 0.0); // C0, B0
        assert!((result[1] - 90.0).abs() < 0.01); // C0, B1
        assert!((result[2] - 95.0).abs() < 0.01); // C0, B2
        assert!((result[3] - 115.0).abs() < 0.01); // C1, B0
        assert_eq!(result[4], 0.0); // C1, B1
        assert!((result[5] - 80.0).abs() < 0.01); // C1, B2
    }
}
