//! Generic compute kernel runner.

use crate::{buffers::GpuBuffer, GpuContext};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use std::sync::Arc;

/// Buffer binding type for multi-buffer kernels
#[derive(Debug, Clone, Copy)]
pub enum BufferBinding {
    /// Read-only storage buffer
    ReadOnly,
    /// Read-write storage buffer
    ReadWrite,
    /// Uniform buffer (small, constant data)
    Uniform,
}

/// Runs a compute shader with the given buffers
pub struct KernelRunner {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl KernelRunner {
    /// Create a new kernel runner from WGSL source code
    pub fn from_wgsl(ctx: &GpuContext, wgsl_source: &str, entry_point: &str) -> Result<Self> {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(entry_point),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl_source)),
        });

        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("kernel_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("kernel_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            pipeline,
            bind_group_layout,
            device: Arc::clone(&ctx.device),
            queue: Arc::clone(&ctx.queue),
        })
    }

    /// Run kernel on a single buffer
    pub fn dispatch<T: Pod + Zeroable>(
        &self,
        buffer: &GpuBuffer<T>,
        workgroup_size: u32,
    ) -> Result<()> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kernel_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.buffer.as_entire_binding(),
            }],
        });

        let workgroups = (buffer.len() as u32).div_ceil(workgroup_size);

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        Ok(())
    }
}

/// Kernel runner supporting multiple input/output buffers
pub struct MultiBufferKernel {
    /// The compiled compute pipeline
    pub pipeline: wgpu::ComputePipeline,
    /// Layout defining how buffers are bound to the shader
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Buffer binding types (read-only, read-write, uniform)
    pub bindings: Vec<BufferBinding>,
}

impl MultiBufferKernel {
    /// Create a new multi-buffer kernel from WGSL source code.
    ///
    /// # Arguments
    /// * `ctx` - GPU context for device access
    /// * `wgsl_source` - WGSL shader source code
    /// * `entry_point` - Shader entry point function name
    /// * `bindings` - Buffer binding types in order of shader bindings
    ///
    /// # Returns
    /// A configured kernel ready to dispatch with the specified buffer layout
    pub fn new(
        ctx: &GpuContext,
        wgsl_source: &str,
        entry_point: &str,
        bindings: &[BufferBinding],
    ) -> Result<Self> {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(entry_point),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl_source)),
        });

        let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings
            .iter()
            .enumerate()
            .map(|(i, binding)| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: match binding {
                    BufferBinding::ReadOnly => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    BufferBinding::ReadWrite => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    BufferBinding::Uniform => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                count: None,
            })
            .collect();

        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("multi_buffer_layout"),
                    entries: &entries,
                });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("multi_buffer_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            pipeline,
            bind_group_layout,
            bindings: bindings.to_vec(),
        })
    }

    /// Dispatch the kernel with multiple buffers.
    ///
    /// # Arguments
    /// * `ctx` - GPU context for device and queue access
    /// * `buffers` - Slice of buffer references matching the binding layout
    /// * `element_count` - Total number of elements to process
    /// * `workgroup_size` - Number of threads per workgroup (must match shader)
    ///
    /// # Returns
    /// Ok(()) on success, error if buffer count mismatches binding layout
    ///
    /// # Example
    /// ```ignore
    /// kernel.dispatch(&ctx, &[&buf_a.buffer, &buf_b.buffer], 1024, 64)?;
    /// ```
    pub fn dispatch(
        &self,
        ctx: &GpuContext,
        buffers: &[&wgpu::Buffer],
        element_count: u32,
        workgroup_size: u32,
    ) -> Result<()> {
        if buffers.len() != self.bindings.len() {
            anyhow::bail!(
                "Expected {} buffers, got {}",
                self.bindings.len(),
                buffers.len()
            );
        }

        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buf.as_entire_binding(),
            })
            .collect();

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("multi_buffer_bind_group"),
            layout: &self.bind_group_layout,
            entries: &entries,
        });

        let workgroups = element_count.div_ceil(workgroup_size);

        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuContext;

    const ADD_ARRAYS_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> result: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&a)) {
        result[idx] = a[idx] + b[idx];
    }
}
"#;

    #[test]
    fn test_multi_buffer_dispatch() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();

        let a: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let b: Vec<f32> = vec![10.0, 20.0, 30.0, 40.0];
        let result: Vec<f32> = vec![0.0; 4];

        let buf_a = crate::GpuBuffer::new(&ctx, &a, "a");
        let buf_b = crate::GpuBuffer::new(&ctx, &b, "b");
        let buf_result = crate::GpuBuffer::new(&ctx, &result, "result");

        let runner = MultiBufferKernel::new(
            &ctx,
            ADD_ARRAYS_SHADER,
            "main",
            &[
                BufferBinding::ReadOnly,   // a
                BufferBinding::ReadOnly,   // b
                BufferBinding::ReadWrite,  // result
            ],
        ).unwrap();

        runner.dispatch(&ctx, &[&buf_a.buffer, &buf_b.buffer, &buf_result.buffer], 4, 64).unwrap();

        let output = buf_result.read(&ctx);
        assert_eq!(output, vec![11.0, 22.0, 33.0, 44.0]);
    }
}
