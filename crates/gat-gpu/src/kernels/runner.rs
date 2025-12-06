//! Generic compute kernel runner.

use crate::{buffers::GpuBuffer, GpuContext};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use std::sync::Arc;

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
