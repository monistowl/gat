//! Power flow compute shaders in WGSL.

/// WGSL compute shader for power mismatch calculation.
///
/// Computes per-bus active and reactive power mismatch:
/// - P_mismatch[i] = P_calc[i] - P_spec[i]
/// - Q_mismatch[i] = Q_calc[i] - Q_spec[i]
///
/// Where P_calc and Q_calc are computed from:
/// P_i = sum_j V_i * V_j * (G_ij * cos(θ_i - θ_j) + B_ij * sin(θ_i - θ_j))
/// Q_i = sum_j V_i * V_j * (G_ij * sin(θ_i - θ_j) - B_ij * cos(θ_i - θ_j))
pub const POWER_MISMATCH_SHADER: &str = r#"
// Uniforms for problem dimensions
struct Uniforms {
    n_bus: u32,
    nnz: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Bus data: voltage magnitude and angle
@group(0) @binding(1) var<storage, read> v_mag: array<f32>;
@group(0) @binding(2) var<storage, read> v_ang: array<f32>;

// Y-bus matrix in CSR format
@group(0) @binding(3) var<storage, read> g_data: array<f32>;  // G values (real part)
@group(0) @binding(4) var<storage, read> b_data: array<f32>;  // B values (imag part)
@group(0) @binding(5) var<storage, read> row_ptr: array<u32>; // CSR row pointers
@group(0) @binding(6) var<storage, read> col_idx: array<u32>; // CSR column indices

// Specified power injections
@group(0) @binding(7) var<storage, read> p_spec: array<f32>;
@group(0) @binding(8) var<storage, read> q_spec: array<f32>;

// Output: power mismatches
@group(0) @binding(9) var<storage, read_write> p_mismatch: array<f32>;
@group(0) @binding(10) var<storage, read_write> q_mismatch: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;

    if (i >= uniforms.n_bus) {
        return;
    }

    let vi = v_mag[i];
    let theta_i = v_ang[i];

    var p_calc: f32 = 0.0;
    var q_calc: f32 = 0.0;

    let row_start = row_ptr[i];
    let row_end = row_ptr[i + 1u];

    for (var k = row_start; k < row_end; k = k + 1u) {
        let j = col_idx[k];
        let g_ij = g_data[k];
        let b_ij = b_data[k];
        let vj = v_mag[j];
        let theta_j = v_ang[j];

        let angle_diff = theta_i - theta_j;
        let cos_val = cos(angle_diff);
        let sin_val = sin(angle_diff);

        // P_i contribution: V_i * V_j * (G_ij * cos + B_ij * sin)
        p_calc = p_calc + vi * vj * (g_ij * cos_val + b_ij * sin_val);

        // Q_i contribution: V_i * V_j * (G_ij * sin - B_ij * cos)
        q_calc = q_calc + vi * vj * (g_ij * sin_val - b_ij * cos_val);
    }

    p_mismatch[i] = p_calc - p_spec[i];
    q_mismatch[i] = q_calc - q_spec[i];
}
"#;

/// Simple test shader that doubles values in a buffer.
pub const DOUBLE_VALUES_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&data)) {
        data[idx] = data[idx] * 2.0;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GpuBuffer, GpuContext, KernelRunner};

    #[test]
    fn test_double_values_shader() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();
        let runner = KernelRunner::from_wgsl(&ctx, DOUBLE_VALUES_SHADER, "main").unwrap();

        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let buffer = GpuBuffer::new(&ctx, &data, "test_data");

        runner.dispatch(&buffer, 64).unwrap();

        let result = buffer.read(&ctx);
        assert_eq!(result, vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0]);
    }
}
